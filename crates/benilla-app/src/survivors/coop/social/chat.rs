//! Party chat, `/roll`, the voice emotes and the Faire's news.
//!
//! In a co-op lobby or run **Enter** opens a one-line input at the bottom left (the lobby takes
//! it before Enter World or the ready check). While it is open every key is the
//! input's: the hero stands, and Esc closes it without pausing. A line goes to the whole party in
//! party colour; `/roll [max]` has the host roll 1 to max for everyone to see, and `/dance /wave
//! /cheer /charge /flex /lol /bye /thank /roar` play the emote's real animation on the hero
//! (`EmotesText` → `Emotes` → `AnimationData`) with its voice for the hero's race and sex
//! (`EmotesTextSound`). A Faire pull in the lobby is told to everyone, an Epic or Legendary with a
//! fanfare and a banner.

use std::collections::VecDeque;

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy_replicon::prelude::*;

use super::super::super::faire::{FairePulled, PrizeRarity};
use super::super::super::gear;
use super::super::super::hud::HudFont;
use super::super::super::{Phase, Rng};
use super::super::host::Peer;
use super::super::wire::NetHero;
use super::super::Session;
use super::wire::{Body, Heard, Line, Say, Speech};
use super::{label, linked, name_of, player_of, to_all, Art, EMOTE, LOOT, PARTY, SYSTEM};
use crate::sound::{EmoteSounds, KitCue, SoundCue};

/// `/command`, its `EmotesText` row, and what the chat says of it (`EmotesTextData`'s line with
/// no target). `/lol` is the client's alias of `/laugh`.
pub(super) const EMOTES: [(&str, &str, &str); 9] = [
    ("dance", "DANCE", "bursts into dance."),
    ("wave", "WAVE", "waves."),
    ("cheer", "CHEER", "cheers!"),
    ("charge", "CHARGE", "starts to charge."),
    ("flex", "FLEX", "flexes. Oooooh so strong!"),
    ("lol", "LAUGH", "laughs."),
    ("bye", "BYE", "waves goodbye to everyone. Farewell!"),
    ("thank", "THANK", "thanks everyone around."),
    ("roar", "ROAR", "roars with bestial vigor. So fierce!"),
];

/// The chat input's frame, `ChatFrameEditBox`'s.
pub(super) const INPUT_BORDER: &str = "Interface\\ChatFrame\\UI-ChatInputBorder";
/// The fanfares a Faire pull plays for the lobby: the quest-complete chime for an Epic, the
/// battleground victory for a Legendary.
pub(super) const FANFARE_EPIC: &str = "QUESTCOMPLETED";
pub(super) const FANFARE_LEGENDARY: &str = "PVPVictoryAllianceMono";

/// The longest line a player may say.
const MAX_LINE: usize = 200;
/// The most `/roll` takes.
const MAX_ROLL: u32 = 1_000_000;
/// Lines the frame keeps, and shows.
const KEEP: usize = 60;
const SHOWN: usize = 8;
/// Seconds a line stays up with the input shut.
const LINE_SECS: f32 = 20.0;
/// Seconds a Faire banner holds.
const BANNER_SECS: f32 = 4.5;

/// What a line typed into the chat asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Command {
    Say(String),
    Roll {
        max: u32,
    },
    /// An index into [`EMOTES`].
    Emote(u8),
    /// A `/command` the chat does not know.
    Unknown(String),
    Nothing,
}

/// Read a typed line: `/roll [max]` (also `/random`), the emotes, `/s` and `/p` before a line, any
/// other `/command` unknown, and anything else said as it is.
pub(super) fn parse(input: &str) -> Command {
    let text = input.trim();
    if text.is_empty() {
        return Command::Nothing;
    }
    let Some(rest) = text.strip_prefix('/') else {
        return Command::Say(text.chars().take(MAX_LINE).collect());
    };
    let (command, arg) = rest
        .split_once(char::is_whitespace)
        .map_or((rest, ""), |(c, a)| (c, a.trim()));
    let command = command.to_ascii_lowercase();
    match command.as_str() {
        "roll" | "random" | "rnd" => {
            let max = match arg {
                "" => 100,
                a => match a.parse::<u32>() {
                    Ok(n) => n.clamp(1, MAX_ROLL),
                    Err(_) => 100,
                },
            };
            Command::Roll { max }
        }
        "s" | "say" | "p" | "party" => match arg {
            "" => Command::Nothing,
            a => Command::Say(a.chars().take(MAX_LINE).collect()),
        },
        c => match EMOTES.iter().position(|e| e.0 == c) {
            Some(i) => Command::Emote(i as u8),
            None if c == "laugh" => Command::Emote(5),
            None => Command::Unknown(c.to_string()),
        },
    }
}

/// The prizes of a pull worth telling the lobby: every Epic and Legendary, or else the best one.
pub(super) fn announced(prizes: &[(String, u8)]) -> Vec<(String, u8)> {
    let big: Vec<_> = prizes.iter().filter(|p| p.1 >= 3).cloned().collect();
    if !big.is_empty() {
        return big;
    }
    prizes
        .iter()
        .max_by_key(|p| p.1)
        .cloned()
        .into_iter()
        .collect()
}

/// A Faire prize rarity's colour, from its index (0 Common to 4 Legendary).
fn prize_color(rarity: u8) -> Color {
    PrizeRarity::ALL
        .get(usize::from(rarity))
        .copied()
        .unwrap_or(PrizeRarity::Common)
        .color()
}

/// A piece of gear's name in brackets and its quality colour.
fn item_span(item: u16) -> (String, Color) {
    match gear::item(item) {
        Some(i) => (format!("[{}]", i.name), i.rarity.color()),
        None => ("[something shiny]".into(), Color::WHITE),
    }
}

/// How `line` reads, in coloured runs: "[Name]: hi" in party blue, "[Name] rolls 42 (1-100)",
/// "[Name] won: [Item] (Need 87)" with the item in its quality colour, and so on.
pub(super) fn spans(line: &Line) -> Vec<(String, Color)> {
    let who = format!("[{}]", line.name);
    match &line.body {
        Body::Said(text) => vec![(format!("{who}: {text}"), PARTY)],
        Body::Rolled { value, max } => vec![(format!("{who} rolls {value} (1-{max})"), SYSTEM)],
        Body::Emoted(i) => {
            let words = EMOTES.get(usize::from(*i)).map_or("emotes.", |e| e.2);
            vec![(format!("{who} {words}"), EMOTE)]
        }
        Body::Won { item, pick, value } => vec![
            (format!("{who} won: "), LOOT),
            item_span(*item),
            (format!(" ({} {value})", pick.label()), LOOT),
        ],
        Body::AllPassed { item } => vec![("Everyone passed on: ".into(), LOOT), item_span(*item)],
        Body::Pulled { prize, rarity } => vec![
            (format!("{who} pulled "), TICKET_LINE),
            (format!("[{prize}]"), prize_color(*rarity)),
            ("!".into(), TICKET_LINE),
        ],
        Body::Pinged(kind) => vec![(format!("{who}: {}", kind.label()), PARTY)],
        Body::Note(text) => vec![(text.clone(), SYSTEM)],
    }
}

/// A Faire line's colour: the Faire's own gold.
const TICKET_LINE: Color = super::TICKET;

/// The chat: its lines (coloured runs and their age), and the input while it is open.
#[derive(Resource, Default)]
pub(super) struct Chat {
    lines: VecDeque<(Vec<(String, Color)>, f32)>,
    pub(super) input: Option<String>,
    /// Bumped by every new line, so the frame rebuilds once.
    version: u32,
}

impl Chat {
    pub(super) fn push(&mut self, runs: Vec<(String, Color)>) {
        self.lines.push_back((runs, 0.0));
        while self.lines.len() > KEEP {
            self.lines.pop_front();
        }
        self.version = self.version.wrapping_add(1);
    }

    /// A plain system line, only here.
    pub(super) fn note(&mut self, text: impl Into<String>) {
        self.push(vec![(text.into(), SYSTEM)]);
    }

    /// Whether the input is open: every key is the chat's.
    pub(super) fn typing(&self) -> bool {
        self.input.is_some()
    }
}

/// A Faire pull's banner across the screen, while it holds.
#[derive(Resource, Default)]
struct FaireBanner(Option<(String, Color, f32)>);

#[derive(Component)]
struct ChatRoot;
#[derive(Component)]
struct ChatLines;
#[derive(Component)]
struct ChatInput;
#[derive(Component)]
struct ChatInputText;
#[derive(Component)]
struct BannerNode;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Chat>()
        .init_resource::<FaireBanner>()
        .add_systems(
            PreUpdate,
            chat_keys
                .after(bevy::input::InputSystems)
                .run_if(resource_exists::<HudFont>),
        )
        .add_systems(
            Update,
            (
                host_chat.run_if(super::hosting),
                faire_pulls.run_if(linked),
                hear_chat,
                build_chat,
                refresh_chat,
            )
                .chain(),
        );
}

/// Enter opens the input in a co-op lobby or run; while it is open the typed keys are its, Enter
/// says the line, Esc drops it, and no other system sees a key.
fn chat_keys(
    mut chat: ResMut<Chat>,
    mut typed: MessageReader<KeyboardInput>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    phase: Res<State<Phase>>,
    session: Res<Session>,
    mut say: MessageWriter<Say>,
) {
    let linked = matches!(
        session.link,
        super::super::Link::Hosting { .. } | super::super::Link::Joined { .. }
    );
    let active = linked
        && ((session.in_run && *phase.get() == Phase::Playing) || *phase.get() == Phase::Menu);
    if !active {
        if chat.input.is_some() {
            chat.input = None;
        }
        return;
    }
    if chat.input.is_none() {
        if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) {
            chat.input = Some(String::new());
            typed.clear();
            keys.reset_all();
        }
        return;
    }
    let mut submit = None;
    let mut close = false;
    for k in typed.read() {
        if k.state != ButtonState::Pressed || close {
            continue;
        }
        let Some(text) = chat.input.as_mut() else {
            break;
        };
        match &k.logical_key {
            Key::Enter => {
                submit = Some(std::mem::take(text));
                close = true;
            }
            Key::Escape => close = true,
            Key::Backspace => {
                text.pop();
            }
            _ => {
                if let Some(t) = &k.text {
                    for c in t.chars().filter(|c| !c.is_control()) {
                        if text.chars().count() < MAX_LINE {
                            text.push(c);
                        }
                    }
                }
            }
        }
    }
    keys.reset_all();
    if close {
        chat.input = None;
    }
    let Some(line) = submit else { return };
    match parse(&line) {
        Command::Say(text) => {
            say.write(Say(Speech::Text(text)));
        }
        Command::Roll { max } => {
            say.write(Say(Speech::Roll { max }));
        }
        Command::Emote(i) => {
            say.write(Say(Speech::Emote(i)));
        }
        Command::Unknown(c) => chat.note(format!("Unknown command: /{c}")),
        Command::Nothing => {}
    }
}

/// The host hears every player's lines, rolls and emotes and tells everyone; it rolls the dice.
fn host_chat(
    mut said: MessageReader<FromClient<Say>>,
    peers: Query<&Peer>,
    session: Res<Session>,
    mut rng: ResMut<Rng>,
    mut out: MessageWriter<ToClients<Heard>>,
) {
    for m in said.read() {
        let Some(player) = player_of(m.client_id, &peers) else {
            continue;
        };
        let line = |body| {
            to_all(Heard::Line(Line {
                player,
                name: name_of(&session, player),
                body,
            }))
        };
        match &m.message.0 {
            Speech::Text(text) => {
                let text: String = text.trim().chars().take(MAX_LINE).collect();
                if !text.is_empty() {
                    out.write(line(Body::Said(text)));
                }
            }
            Speech::Roll { max } => {
                let max = (*max).clamp(1, MAX_ROLL);
                let value = rng.index(max as usize) as u32 + 1;
                out.write(line(Body::Rolled { value, max }));
            }
            Speech::Emote(i) if usize::from(*i) < EMOTES.len() => {
                out.write(to_all(Heard::Emote { player, emote: *i }));
                out.write(line(Body::Emoted(*i)));
            }
            Speech::Pulled(prizes) => {
                for (prize, rarity) in announced(prizes) {
                    out.write(line(Body::Pulled { prize, rarity }));
                }
            }
            _ => {}
        }
    }
}

/// This player's Faire pulls, while in a session, go to the host to be told.
fn faire_pulls(mut pulls: MessageReader<FairePulled>, mut say: MessageWriter<Say>) {
    for p in pulls.read() {
        let prizes =
            p.0.iter()
                .map(|(name, rarity)| (name.clone(), *rarity as u8))
                .collect();
        say.write(Say(Speech::Pulled(prizes)));
    }
}

/// Lines into the chat; an emote played on its hero; a big Faire pull's banner and fanfare.
#[allow(clippy::too_many_arguments)]
fn hear_chat(
    mut heard: MessageReader<Heard>,
    mut chat: ResMut<Chat>,
    mut banner: ResMut<FaireBanner>,
    heroes: Query<(Entity, &NetHero, &Transform, Option<&crate::net::Guid>)>,
    emotes: Option<Res<EmoteSounds>>,
    mut anims: MessageWriter<crate::net::EmoteMessage>,
    mut voices: MessageWriter<KitCue>,
    mut cues: MessageWriter<SoundCue>,
) {
    for h in heard.read() {
        match h {
            Heard::Line(line) => {
                if let Body::Pulled { prize, rarity } = &line.body {
                    if *rarity >= 3 {
                        let text = format!("{} pulled [{prize}]!", line.name);
                        banner.0 = Some((text, prize_color(*rarity), BANNER_SECS));
                        cues.write(SoundCue::flat(if *rarity >= 4 {
                            FANFARE_LEGENDARY
                        } else {
                            FANFARE_EPIC
                        }));
                    }
                }
                chat.push(spans(line));
            }
            Heard::Emote { player, emote } => {
                let (Some(emotes), Some((_, row, _))) =
                    (emotes.as_deref(), EMOTES.get(usize::from(*emote)))
                else {
                    continue;
                };
                let Some((e, hero, tf, guid)) = heroes.iter().find(|h| h.1.player == *player)
                else {
                    continue;
                };
                let Some(text) = emotes.text_id(row) else {
                    continue;
                };
                if let Some(anim) = emotes.text_emote(text) {
                    anims.write(crate::net::EmoteMessage {
                        guid: guid.map_or(0, |g| g.0),
                        source: Some(e),
                        kind: crate::net::EmoteKind::Anim(anim),
                    });
                }
                let voice = emotes
                    .0
                    .voice(text, u32::from(hero.race), u32::from(hero.sex))
                    .filter(|&k| k != 0);
                if let Some(kit) = voice {
                    voices.write(KitCue {
                        kit,
                        at: Some(tf.translation + Vec3::Y * 1.6),
                    });
                }
            }
            _ => {}
        }
    }
}

/// The chat frame at the bottom left and the Faire banner, built once.
fn build_chat(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    built: Query<(), With<ChatRoot>>,
) {
    let Some(font) = font else { return };
    if !built.is_empty() {
        return;
    }
    let f = &font.0;
    commands
        .spawn((
            ChatRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                bottom: Val::Px(150.0),
                width: Val::Px(430.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                ..default()
            },
            GlobalZIndex(300),
            Pickable::IGNORE,
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((
                ChatLines,
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(4.0)),
                    row_gap: Val::Px(1.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ));
            root.spawn((
                ChatInput,
                Node {
                    height: Val::Px(26.0),
                    padding: UiRect::horizontal(Val::Px(10.0)),
                    align_items: AlignItems::Center,
                    display: Display::None,
                    ..default()
                },
                ImageNode::default(),
                Art::whole(INPUT_BORDER),
            ))
            .with_children(|i| {
                i.spawn((ChatInputText, label(f, "", 14.0, PARTY)));
            });
        });
    commands
        .spawn((
            BannerNode,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(22.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(320),
            Pickable::IGNORE,
            Visibility::Hidden,
        ))
        .with_children(|b| {
            b.spawn((label(f, "", 30.0, Color::WHITE), TextShadow::default()));
        });
}

/// The frame shows in a session: the last lines (older ones fade unless the input is open), and the
/// input with its caret; the banner while it holds.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn refresh_chat(
    mut commands: Commands,
    time: Res<Time<Real>>,
    session: Res<Session>,
    font: Option<Res<HudFont>>,
    mut chat: ResMut<Chat>,
    mut banner: ResMut<FaireBanner>,
    mut shown: Local<(u32, bool, usize)>,
    mut root: Query<&mut Visibility, (With<ChatRoot>, Without<BannerNode>)>,
    mut lines: Query<(Entity, &mut BackgroundColor), With<ChatLines>>,
    mut input: Query<&mut Node, With<ChatInput>>,
    mut input_text: Query<&mut Text, With<ChatInputText>>,
    mut banners: Query<(&mut Visibility, &Children), (With<BannerNode>, Without<ChatRoot>)>,
    mut banner_text: Query<(&mut Text, &mut TextColor), Without<ChatInputText>>,
) {
    let Some(font) = font else { return };
    let dt = time.delta_secs();
    let on = matches!(
        session.link,
        super::super::Link::Hosting { .. } | super::super::Link::Joined { .. }
    );
    for (_, age) in &mut chat.lines {
        *age += dt;
    }
    let want = if on {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut v in &mut root {
        v.set_if_neq(want);
    }
    let typing = chat.input.is_some();
    // Lines young enough to show, the newest last.
    let fresh = chat
        .lines
        .iter()
        .rev()
        .take(SHOWN)
        .filter(|(_, age)| typing || *age < LINE_SECS)
        .count();
    if shown.0 != chat.version || shown.1 != typing || shown.2 != fresh {
        *shown = (chat.version, typing, fresh);
        for (e, mut bg) in &mut lines {
            commands.entity(e).despawn_related::<Children>();
            bg.0 = if fresh > 0 || typing {
                Color::srgba(0.0, 0.0, 0.0, 0.35)
            } else {
                Color::NONE
            };
            let start = chat.lines.len() - fresh;
            commands.entity(e).with_children(|l| {
                for (runs, _) in chat.lines.iter().skip(start) {
                    l.spawn((
                        Text::default(),
                        TextFont {
                            font: font.0.clone(),
                            font_size: 14.0,
                            ..default()
                        },
                        TextShadow::default(),
                    ))
                    .with_children(|t| {
                        for (s, c) in runs {
                            t.spawn((
                                TextSpan::new(s.clone()),
                                TextFont {
                                    font: font.0.clone(),
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(*c),
                            ));
                        }
                    });
                }
            });
        }
    }
    let display = if typing { Display::Flex } else { Display::None };
    for mut node in &mut input {
        if node.display != display {
            node.display = display;
        }
    }
    if let Some(text) = &chat.input {
        let want = format!("Party: {text}|");
        for mut t in &mut input_text {
            if t.0 != want {
                t.0 = want.clone();
            }
        }
    }
    // The banner holds a few seconds.
    if let Some((_, _, t)) = banner.0.as_mut() {
        *t -= dt;
        if *t <= 0.0 {
            banner.0 = None;
        }
    }
    for (mut v, children) in &mut banners {
        let want = if banner.0.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        v.set_if_neq(want);
        let Some((line, color, _)) = &banner.0 else {
            continue;
        };
        for c in children.iter() {
            if let Ok((mut t, mut tc)) = banner_text.get_mut(c) {
                if t.0 != *line {
                    t.0 = line.clone();
                }
                tc.0 = *color;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::loot::RollPick;
    use super::super::ping::PingKind;
    use super::*;

    #[test]
    fn typed_lines_read_as_commands() {
        assert_eq!(parse("   "), Command::Nothing);
        assert_eq!(parse(" hello there "), Command::Say("hello there".into()));
        assert_eq!(parse("/roll"), Command::Roll { max: 100 });
        assert_eq!(parse("/ROLL 20"), Command::Roll { max: 20 });
        assert_eq!(parse("/random 6"), Command::Roll { max: 6 });
        assert_eq!(parse("/roll 0"), Command::Roll { max: 1 });
        assert_eq!(parse("/roll lots"), Command::Roll { max: 100 });
        assert_eq!(
            parse("/roll 99999999999"),
            Command::Roll { max: 100 },
            "past u32 reads as no number"
        );
        assert_eq!(parse("/roll 5000000"), Command::Roll { max: MAX_ROLL });
        assert_eq!(parse("/dance"), Command::Emote(0));
        assert_eq!(parse("/Cheer"), Command::Emote(2));
        assert_eq!(parse("/lol"), Command::Emote(5));
        assert_eq!(parse("/laugh"), Command::Emote(5));
        assert_eq!(parse("/roar now"), Command::Emote(8));
        assert_eq!(
            parse("/p pull the boss"),
            Command::Say("pull the boss".into())
        );
        assert_eq!(parse("/s"), Command::Nothing);
        assert_eq!(parse("/kiss"), Command::Unknown("kiss".into()));
        let long = "a".repeat(500);
        assert_eq!(parse(&long), Command::Say("a".repeat(MAX_LINE)));
        // Every emote's command reads back to itself.
        for (i, (command, ..)) in EMOTES.iter().enumerate() {
            assert_eq!(parse(&format!("/{command}")), Command::Emote(i as u8));
        }
    }

    #[test]
    fn lines_read_as_wow_words_them() {
        let line = |body| Line {
            player: 1,
            name: "Jaina".into(),
            body,
        };
        let text = |l: &Line| spans(l).into_iter().map(|s| s.0).collect::<String>();
        assert_eq!(
            text(&line(Body::Rolled {
                value: 42,
                max: 100
            })),
            "[Jaina] rolls 42 (1-100)"
        );
        assert_eq!(text(&line(Body::Said("hi".into()))), "[Jaina]: hi");
        assert_eq!(spans(&line(Body::Said("hi".into())))[0].1, PARTY);
        assert_eq!(text(&line(Body::Emoted(0))), "[Jaina] bursts into dance.");
        let piece = &gear::items_for(0)[0];
        let won = spans(&line(Body::Won {
            item: piece.id,
            pick: RollPick::Need,
            value: 87,
        }));
        assert_eq!(
            won.iter().map(|s| s.0.as_str()).collect::<String>(),
            format!("[Jaina] won: [{}] (Need 87)", piece.name)
        );
        assert_eq!(
            won[1].1,
            piece.rarity.color(),
            "the item in its quality colour"
        );
        assert_eq!(
            text(&line(Body::Pulled {
                prize: "Swift Zulian Tiger".into(),
                rarity: 4
            })),
            "[Jaina] pulled [Swift Zulian Tiger]!"
        );
        assert_eq!(text(&line(Body::Pinged(PingKind::Help))), "[Jaina]: Help!");
    }

    #[test]
    fn a_pull_tells_its_big_prizes_or_its_best() {
        let p = |n: &str, r: u8| (n.to_string(), r);
        assert_eq!(
            announced(&[p("a", 0), p("b", 3), p("c", 1), p("d", 4)]),
            vec![p("b", 3), p("d", 4)]
        );
        assert_eq!(
            announced(&[p("a", 0), p("b", 2), p("c", 1)]),
            vec![p("b", 2)]
        );
        assert!(announced(&[]).is_empty());
    }
}
