//! The lobby's extras: every player's card (class icon, level, title, mount and hat) beside their
//! name, the ready-check marks, and the side bets' row in the Multiplayer panel ([`super::bets`]).

use std::collections::BTreeMap;

use bevy::prelude::*;
use bevy_replicon::prelude::*;

use super::super::super::faire::catalog::equipped_title;
use super::super::super::faire::Kind;
use super::super::super::hud::HudFont;
use super::super::super::profile::Profile;
use super::super::super::{stable, talents, Phase, Selection};
use super::super::host::Peer;
use super::super::{Link, PlayerName, Session};
use super::bets::{bet_line, Market, Wager, MAX_STAKE, STEP};
use super::ready::ReadyShown;
use super::wire::{Book, Card, Heard, Say, Speech};
use super::{class_icon, label, player_of, to_all, Art, CLASS_ICONS, TICKET};
use crate::glue::art::GOLD;
use crate::glue::GlueClicks;

/// The panel's container the social rows are built into ([`super::super::ui`] places it).
#[derive(Component)]
pub(in crate::survivors) struct SocialSlot;
/// Lobby line `i`'s class icon, and its ready-check mark.
#[derive(Component)]
pub(in crate::survivors) struct LobbyIcon(pub usize);
#[derive(Component)]
pub(in crate::survivors) struct LobbyMark(pub usize);

/// The host's book: every player's card and bet.
#[derive(Resource, Default)]
pub(super) struct HostBook {
    pub cards: BTreeMap<u8, Card>,
    pub bets: BTreeMap<u8, Wager>,
    pub dirty: bool,
    sent: Option<(Book, usize)>,
}

/// The book as this machine last heard it.
#[derive(Resource, Default)]
pub(in crate::survivors) struct BookShown(pub Book);

impl BookShown {
    pub(in crate::survivors) fn card(&self, player: u8) -> Option<&Card> {
        self.0.cards.iter().find(|c| c.player == player)
    }
}

/// The bet row's picks: the market, whom (a place in the lobby) and the stake.
#[derive(Resource, Default)]
struct Draft {
    market: u8,
    on: usize,
    stake: u32,
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum BetAction {
    MarketPrev,
    MarketNext,
    OnPrev,
    OnNext,
    Less,
    More,
}

#[derive(Component)]
struct BetBuilt;
#[derive(Component, Clone, Copy)]
enum BetText {
    Market,
    On,
    Stake,
    Purse,
}
#[derive(Component)]
struct BookLine(usize);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<HostBook>()
        .init_resource::<BookShown>()
        .init_resource::<Draft>()
        .add_systems(
            Update,
            (
                my_card.run_if(super::linked),
                host_book.run_if(super::hosting),
                hear_book,
                lobby_icons,
                build_bets,
                bet_clicks,
                send_bet,
                refresh_bets,
            )
                .chain(),
        );
}

/// A class's level as its talents make it: one talent point a level from 10, so a class with
/// none is level 1 and a full tree is 60.
pub(super) fn level_for(points: u32) -> u32 {
    if points == 0 {
        1
    } else {
        (points + 9).min(60)
    }
}

/// The name of the prize `key` of `kind`.
fn prize_name(kind: Kind, key: Option<&str>) -> Option<String> {
    let key = key?;
    stable::prizes(kind)
        .find(|p| p.key == key)
        .map(|p| p.name.clone())
}

/// This player's card, from the menu's pick and its own profile.
pub(super) fn own_card(profile: &Profile, selection: &Selection, name: &str) -> Card {
    let class = selection.class.unwrap_or(0);
    let points = talents::saved(profile, class).map_or(0, |s| talents::earned_points(s, 0));
    Card {
        player: 0,
        name: name.to_string(),
        class: class.min(255) as u8,
        level: level_for(points),
        title: equipped_title(profile).map(str::to_string),
        mount: prize_name(Kind::Mount, profile.stable.mount.as_deref()),
        hat: prize_name(Kind::MountHat, profile.stable.mount_hat.as_deref()),
        tabard: prize_name(Kind::Tabard, profile.faire.wardrobe.tabard.as_deref()),
    }
}

/// What the lobby line adds of a card: "Lv 23 · <Carnie> · Swift Zulian Tiger · Top Hat".
pub(in crate::survivors) fn member_extra(world_card: Option<&Card>) -> String {
    let Some(c) = world_card else {
        return String::new();
    };
    let mut parts = vec![format!("Lv {}", c.level)];
    if let Some(t) = &c.title {
        parts.push(format!("<{t}>"));
    }
    parts.extend(c.mount.iter().cloned());
    parts.extend(c.hat.iter().cloned());
    format!("  ·  {}", parts.join("  ·  "))
}

/// This player's card goes to the host on joining, and again whenever it changes.
fn my_card(
    session: Res<Session>,
    profile: Res<Profile>,
    selection: Res<Selection>,
    name: Res<PlayerName>,
    mut sent: Local<Option<(Card, u32)>>,
    mut say: MessageWriter<Say>,
) {
    let card = own_card(&profile, &selection, &name.0);
    let fresh = sent
        .as_ref()
        .is_none_or(|(c, attempt)| *c != card || *attempt != session.attempt);
    if fresh {
        *sent = Some((card.clone(), session.attempt));
        say.write(Say(Speech::Card(card)));
    }
}

/// The host keeps everyone's card and bet, and tells everyone whenever the book or the party
/// changes. Bets are taken only before the run.
fn host_book(
    mut said: MessageReader<FromClient<Say>>,
    peers: Query<&Peer>,
    session: Res<Session>,
    mut book: ResMut<HostBook>,
    mut out: MessageWriter<ToClients<Heard>>,
) {
    for m in said.read() {
        let Some(player) = player_of(m.client_id, &peers) else {
            continue;
        };
        match &m.message.0 {
            Speech::Card(card) => {
                let mut card = card.clone();
                card.player = player;
                if let Some(member) = session.lobby.iter().find(|x| x.player == player) {
                    card.name.clone_from(&member.name);
                    card.class = member.class;
                }
                book.cards.insert(player, card);
                book.dirty = true;
            }
            Speech::Bet(bet) if !session.in_run => {
                let valid = bet.filter(|w| {
                    w.stake > 0
                        && w.stake <= MAX_STAKE
                        && usize::from(w.market) < Market::ALL.len()
                        && session.lobby.iter().any(|x| x.player == w.on)
                });
                match valid {
                    Some(w) => book.bets.insert(player, w),
                    None => book.bets.remove(&player),
                };
                book.dirty = true;
            }
            _ => {}
        }
    }
    // The gone take their card and bet with them.
    if !session.lobby.is_empty() {
        let here = |p: &u8| session.lobby.iter().any(|m| m.player == *p);
        let (cards, bets) = (book.cards.len(), book.bets.len());
        book.cards.retain(|p, _| here(p));
        book.bets.retain(|p, w| here(p) && here(&w.on));
        if cards != book.cards.len() || bets != book.bets.len() {
            book.dirty = true;
        }
    }
    let party = session.lobby.len();
    if !book.dirty && book.sent.as_ref().is_some_and(|s| s.1 == party) {
        return;
    }
    book.dirty = false;
    let now = Book {
        cards: book.cards.values().cloned().collect(),
        bets: book.bets.iter().map(|(&p, &w)| (p, w)).collect(),
    };
    if book
        .sent
        .as_ref()
        .is_some_and(|s| s.0 == now && s.1 == party)
    {
        return;
    }
    book.sent = Some((now.clone(), party));
    out.write(to_all(Heard::Book(now)));
}

fn hear_book(mut heard: MessageReader<Heard>, mut shown: ResMut<BookShown>, session: Res<Session>) {
    for h in heard.read() {
        if let Heard::Book(book) = h {
            shown.0 = book.clone();
        }
    }
    if session.link == Link::Off && shown.0 != Book::default() {
        shown.0 = Book::default();
    }
}

impl BookShown {
    /// Player `player`'s bet as the host has it.
    pub(super) fn bet(&self, player: u8) -> Option<Wager> {
        self.0
            .bets
            .iter()
            .find(|(p, _)| *p == player)
            .map(|(_, w)| *w)
    }
}

/// Each lobby line's class icon and ready-check mark.
#[allow(clippy::type_complexity)]
fn lobby_icons(
    session: Res<Session>,
    ready: Res<ReadyShown>,
    mut icons: Query<(&LobbyIcon, &mut Art, &mut Node), Without<LobbyMark>>,
    mut marks: Query<(&LobbyMark, &mut Art, &mut Node), Without<LobbyIcon>>,
) {
    for (i, mut art, mut node) in &mut icons {
        let member = session.lobby.get(i.0);
        let display = if member.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        if let Some(m) = member {
            let want = Art::crop(CLASS_ICONS, class_icon(usize::from(m.class)));
            if *art != want {
                *art = want;
            }
        }
    }
    for (i, mut art, mut node) in &mut marks {
        let mark = session.lobby.get(i.0).and_then(|m| ready.mark(m.player));
        let display = if mark.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        if let Some(mark) = mark {
            let want = mark.art();
            if *art != want {
                *art = want;
            }
        }
    }
}

/// A small boxed button for the bet row.
fn bet_button(p: &mut ChildSpawnerCommands, font: &Handle<Font>, action: BetAction, caption: &str) {
    p.spawn((
        action,
        Button,
        Interaction::default(),
        Node {
            width: Val::Px(22.0),
            height: Val::Px(22.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
        BorderColor::all(Color::srgb(0.45, 0.4, 0.3)),
    ))
    .with_children(|b| {
        b.spawn(label(font, caption, 14.0, GOLD));
    });
}

/// The side bets' rows, built once into the panel's [`SocialSlot`].
fn build_bets(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    slots: Query<Entity, (With<SocialSlot>, Without<BetBuilt>)>,
) {
    let (Some(font), Ok(slot)) = (font, slots.single()) else {
        return;
    };
    let f = &font.0;
    commands.entity(slot).insert(BetBuilt).with_children(|s| {
        s.spawn(label(f, "Side bets, in Darkmoon Prize Tickets", 15.0, GOLD));
        s.spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|r| {
            bet_button(r, f, BetAction::MarketPrev, "<");
            r.spawn(Node {
                width: Val::Px(118.0),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|t| {
                t.spawn((BetText::Market, label(f, "", 13.0, Color::WHITE)));
            });
            bet_button(r, f, BetAction::MarketNext, ">");
            r.spawn(label(f, "on", 13.0, GOLD));
            bet_button(r, f, BetAction::OnPrev, "<");
            r.spawn(Node {
                width: Val::Px(96.0),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|t| {
                t.spawn((BetText::On, label(f, "", 13.0, Color::WHITE)));
            });
            bet_button(r, f, BetAction::OnNext, ">");
            bet_button(r, f, BetAction::Less, "-");
            r.spawn(Node {
                width: Val::Px(26.0),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|t| {
                t.spawn((BetText::Stake, label(f, "0", 13.0, TICKET)));
            });
            bet_button(r, f, BetAction::More, "+");
        });
        s.spawn((
            BetText::Purse,
            label(f, "", 12.0, Color::srgb(0.8, 0.8, 0.8)),
        ));
        for i in 0..usize::from(super::super::super::party::MAX_PARTY) {
            s.spawn((BookLine(i), label(f, "", 12.0, Color::WHITE)));
        }
    });
}

/// The row's arrows and steps, before the ready check only.
fn bet_clicks(
    clicks: Res<GlueClicks>,
    actions: Query<(Entity, &BetAction)>,
    session: Res<Session>,
    ready: Res<ReadyShown>,
    profile: Res<Profile>,
    mut draft: ResMut<Draft>,
) {
    let Some(action) = actions
        .iter()
        .find(|(e, _)| clicks.hit(*e))
        .map(|(_, a)| *a)
    else {
        return;
    };
    if session.in_run || ready.view.open || session.link == Link::Off {
        return;
    }
    let markets = Market::ALL.len() as u8;
    let party = session.lobby.len().max(1);
    let most = MAX_STAKE.min(profile.faire.tickets / STEP * STEP);
    match action {
        BetAction::MarketPrev => draft.market = (draft.market + markets - 1) % markets,
        BetAction::MarketNext => draft.market = (draft.market + 1) % markets,
        BetAction::OnPrev => draft.on = (draft.on + party - 1) % party,
        BetAction::OnNext => draft.on = (draft.on + 1) % party,
        BetAction::Less => draft.stake = draft.stake.saturating_sub(STEP),
        BetAction::More => draft.stake = (draft.stake + STEP).min(most),
    }
}

/// The draft as a bet, in the lobby: sent to the host whenever it differs from the host's book
/// as the draft or the book changes. A settled bet rides again on the next run unless changed.
fn send_bet(
    session: Res<Session>,
    phase: Res<State<Phase>>,
    draft: Res<Draft>,
    book: Res<BookShown>,
    profile: Res<Profile>,
    mut say: MessageWriter<Say>,
) {
    let lobby = !session.in_run && session.link != Link::Off && *phase.get() == Phase::Menu;
    if !lobby || !(draft.is_changed() || book.is_changed() || phase.is_changed()) {
        return;
    }
    // Never more than this player still has.
    let stake = draft.stake.min(MAX_STAKE.min(profile.faire.tickets));
    let want = session
        .lobby
        .get(draft.on.min(session.lobby.len().saturating_sub(1)))
        .filter(|_| stake > 0)
        .map(|m| Wager {
            market: draft.market,
            on: m.player,
            stake,
        });
    if book.bet(session.me) != want {
        say.write(Say(Speech::Bet(want)));
    }
}

/// The row's words and the book's lines.
#[allow(clippy::type_complexity)]
fn refresh_bets(
    session: Res<Session>,
    draft: Res<Draft>,
    book: Res<BookShown>,
    profile: Res<Profile>,
    ready: Res<ReadyShown>,
    mut texts: Query<(&BetText, &mut Text), Without<BookLine>>,
    mut lines: Query<(&BookLine, &mut Text), Without<BetText>>,
) {
    let on = session
        .lobby
        .get(draft.on.min(session.lobby.len().saturating_sub(1)))
        .map_or_else(|| "-".to_string(), |m| m.name.clone());
    let locked = session.in_run || ready.view.open;
    for (which, mut t) in &mut texts {
        let want = match which {
            BetText::Market => Market::of(draft.market).label().to_string(),
            BetText::On => on.clone(),
            BetText::Stake => draft.stake.to_string(),
            BetText::Purse => {
                let lock = if locked { "  (bets are closed)" } else { "" };
                format!(
                    "You have {} tickets. Each market's pool goes to its winners; no winner, stakes back.{lock}",
                    profile.faire.tickets
                )
            }
        };
        if t.0 != want {
            t.0 = want;
        }
    }
    let name = |p: u8| {
        session
            .lobby
            .iter()
            .find(|m| m.player == p)
            .map_or_else(|| format!("Player {}", p + 1), |m| m.name.clone())
    };
    for (l, mut t) in &mut lines {
        let want = book
            .0
            .bets
            .get(l.0)
            .map(|(p, w)| bet_line(&name(*p), &name(w.on), w))
            .unwrap_or_default();
        if t.0 != want {
            t.0 = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_follow_the_talent_points() {
        assert_eq!(level_for(0), 1);
        assert_eq!(level_for(1), 10);
        assert_eq!(level_for(14), 23);
        assert_eq!(level_for(51), 60);
        assert_eq!(level_for(400), 60);
    }

    #[test]
    fn a_card_reads_its_profile_and_shows_on_the_lobby_line() {
        let mut profile = Profile::default();
        let selection = Selection {
            class: Some(6),
            ..default()
        };
        let card = own_card(&profile, &selection, "Jaina");
        assert_eq!(
            (card.class, card.level, card.name.as_str()),
            (6, 1, "Jaina")
        );
        assert_eq!(card.mount, None);
        let mount = stable::prizes(Kind::Mount).next().unwrap();
        profile.stable.mount = Some(mount.key.clone());
        let card = own_card(&profile, &selection, "Jaina");
        assert_eq!(card.mount.as_deref(), Some(mount.name.as_str()));
        let line = member_extra(Some(&card));
        assert!(
            line.contains("Lv 1") && line.contains(&mount.name),
            "{line}"
        );
        assert_eq!(member_extra(None), "");
    }
}
