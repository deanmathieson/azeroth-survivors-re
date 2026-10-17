//! Need, Greed or Pass on boss gear. A co-op host with friends in the run puts each piece a boss
//! chest drops up for the party's roll ([`gear::GroupDrops`]) instead of keeping it, and every
//! player gets 1.12's group-loot frame for it: the item's icon and name in its quality colour, the
//! timer bar, and the Need (dice), Greed (coins) and Pass buttons. Need is only for a piece the
//! player's class can wear. After [`ROLL_SECS`], or once everyone has chosen, the host rolls 1 to
//! 100 for each Need (else each Greed), rolls ties again, and tells everyone "[Name] won: [Item]
//! (Need 87)"; the winner's own machine keeps the piece in its own profile ([`gear::WonLoot`]).
//! Solo, a chest is auto-looted as it always was.

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

use super::super::super::armory::item_tip;
use super::super::super::gear::{self, GearRead, GroupDrops, ItemIcon, WonLoot};
use super::super::super::hud::HudFont;
use super::super::super::{Phase, Rng, Selection};
use super::super::host::Peer;
use super::super::wire::NetHero;
use super::super::Session;
use super::wire::{Body, Heard, Line, Say, Speech};
use super::{label, name_of, player_of, to_all, Art};
use crate::glue::GlueClicks;
use crate::sound::SoundCue;

/// Seconds a roll stays open.
pub(super) const ROLL_SECS: f32 = 20.0;
/// The dice: 1.12 has no dice sound, so the gems' clatter as they are put down stands in.
pub(super) const DICE: &str = "PutDownGems";
/// `GroupLootFrame`'s timer, `UI-Character-Skills-Bar` tinted gold.
pub(super) const TIMER_BAR: &str = "Interface\\PaperDollInfoFrame\\UI-Character-Skills-Bar";
/// Need, Greed and Pass: each button's up and down art.
pub(super) const BUTTONS: [(&str, &str); 3] = [
    (
        "Interface\\Buttons\\UI-GroupLoot-Dice-Up",
        "Interface\\Buttons\\UI-GroupLoot-Dice-Down",
    ),
    (
        "Interface\\Buttons\\UI-GroupLoot-Coin-Up",
        "Interface\\Buttons\\UI-GroupLoot-Coin-Down",
    ),
    (
        "Interface\\Buttons\\UI-GroupLoot-Pass-Up",
        "Interface\\Buttons\\UI-GroupLoot-Pass-Down",
    ),
];

/// The roll bar is a child of this clipped card, never a sibling that can draw over its border.
const FRAME_WIDTH: f32 = 330.0;

fn timer_percent(left: f32, secs: f32) -> f32 {
    100.0 * (left / secs.max(0.01)).clamp(0.0, 1.0)
}

/// A choice at a group roll.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::survivors) enum RollPick {
    Need,
    Greed,
    Pass,
}

impl RollPick {
    pub(super) const ALL: [Self; 3] = [Self::Need, Self::Greed, Self::Pass];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Need => "Need",
            Self::Greed => "Greed",
            Self::Pass => "Pass",
        }
    }
}

/// The heaviest armour a 1.12 class can train. Hunters and shamans reach mail at 40; group loot
/// asks what a class can ultimately use, rather than its level at this instant.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Armor {
    Cloth,
    Leather,
    Mail,
    Plate,
}

fn max_armor(class: usize) -> Armor {
    match class {
        0 | 1 => Armor::Plate,
        2 | 5 => Armor::Mail,
        3 | 8 => Armor::Leather,
        _ => Armor::Cloth,
    }
}

/// A main-hand's 1.12 weapon family. The catalog's display icon is the durable source for this:
/// it keeps the loot rule with the real item rather than the class whose collection introduced it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Weapon {
    Axe,
    Dagger,
    Mace,
    Polearm,
    Staff,
    Sword,
    Wand,
}

fn weapon(item: &gear::Item) -> Weapon {
    let icon = item.icon;
    if icon.starts_with("INV_Axe") {
        Weapon::Axe
    } else if icon.starts_with("INV_Mace") || icon.starts_with("INV_Hammer") {
        Weapon::Mace
    } else if icon.starts_with("INV_Weapon_Halberd") {
        Weapon::Polearm
    } else if icon.starts_with("INV_Staff") {
        Weapon::Staff
    } else if icon.starts_with("INV_Sword") {
        Weapon::Sword
    } else if icon.starts_with("INV_Weapon_Short") {
        Weapon::Dagger
    } else {
        // The priest talisman is a wand in this nine-slot catalog.
        Weapon::Wand
    }
}

fn can_use_weapon(class: usize, weapon: Weapon) -> bool {
    match weapon {
        Weapon::Axe => matches!(class, 0 | 2 | 5),
        Weapon::Dagger => matches!(class, 0 | 2 | 3 | 4 | 5 | 6 | 7),
        Weapon::Mace => matches!(class, 0 | 1 | 3 | 4 | 5 | 8),
        Weapon::Polearm => matches!(class, 0..=2),
        Weapon::Staff => matches!(class, 2 | 4 | 5 | 6 | 7 | 8),
        Weapon::Sword => matches!(class, 0..=3),
        Weapon::Wand => matches!(class, 4 | 6 | 7),
    }
}

/// Whether `hero` may Need `item`: every class can use lighter armour, and weapons follow the
/// 1.12 class weapon skills. Greed and Pass deliberately do not consult this rule.
pub(super) fn may_need(item: u16, hero: usize) -> bool {
    let Some(item) = gear::item(item) else {
        return false;
    };
    if item.slot == gear::Slot::MainHand {
        can_use_weapon(hero, weapon(item))
    } else {
        max_armor(hero) >= max_armor(item.class)
    }
}

/// One player at a roll: its class, and its choice once made.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Seat {
    player: u8,
    class: usize,
    pick: Option<RollPick>,
}

/// A roll's winner, how it won and its roll.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Win {
    pub player: u8,
    pub pick: RollPick,
    pub value: u32,
}

/// One group roll on the host.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct GroupRoll {
    pub id: u32,
    pub item: u16,
    pub left: f32,
    seats: Vec<Seat>,
}

impl GroupRoll {
    /// A roll on piece `item` among `party`'s players and their classes.
    pub(super) fn new(id: u32, item: u16, party: &[(u8, usize)]) -> Self {
        Self {
            id,
            item,
            left: ROLL_SECS,
            seats: party
                .iter()
                .map(|&(player, class)| Seat {
                    player,
                    class,
                    pick: None,
                })
                .collect(),
        }
    }

    /// `player` chooses, once: a Need from a class that cannot wear the piece is refused.
    pub(super) fn choose(&mut self, player: u8, pick: RollPick) -> bool {
        let Some(seat) = self.seats.iter_mut().find(|s| s.player == player) else {
            return false;
        };
        if seat.pick.is_some() || (pick == RollPick::Need && !may_need(self.item, seat.class)) {
            return false;
        }
        seat.pick = Some(pick);
        true
    }

    pub(super) fn tick(&mut self, dt: f32) {
        self.left -= dt;
    }

    /// Everyone has chosen, or the time is up (who has not, passes).
    pub(super) fn decided(&self) -> bool {
        self.left <= 0.0 || self.seats.iter().all(|s| s.pick.is_some())
    }

    /// The winner: Need beats Greed; within that, the highest of `d100`'s rolls, ties rolled again
    /// among the tied. `None` when everyone passed (or let the time run out).
    pub(super) fn settle(&self, mut d100: impl FnMut() -> u32) -> Option<Win> {
        let tier = [RollPick::Need, RollPick::Greed]
            .into_iter()
            .find(|&p| self.seats.iter().any(|s| s.pick == Some(p)))?;
        let mut contenders: Vec<u8> = self
            .seats
            .iter()
            .filter(|s| s.pick == Some(tier))
            .map(|s| s.player)
            .collect();
        loop {
            let rolls: Vec<(u8, u32)> = contenders.iter().map(|&p| (p, d100())).collect();
            let best = rolls.iter().map(|r| r.1).max()?;
            let top: Vec<u8> = rolls.iter().filter(|r| r.1 == best).map(|r| r.0).collect();
            if let [player] = top[..] {
                return Some(Win {
                    player,
                    pick: tier,
                    value: best,
                });
            }
            contenders = top;
        }
    }
}

/// The host's open rolls.
#[derive(Resource, Default)]
pub(super) struct HostRolls {
    rolls: Vec<GroupRoll>,
    next: u32,
}

/// A roll this machine shows: its frame, its time left, and this player's choice.
struct Shown {
    id: u32,
    item: u16,
    secs: f32,
    left: f32,
    picked: Option<RollPick>,
    frame: Option<Entity>,
}

#[derive(Resource, Default)]
struct ShownRolls(Vec<Shown>);

#[derive(Component)]
struct RollFrame;
/// A roll's Need, Greed or Pass button.
#[derive(Component, Clone, Copy)]
struct RollButton {
    id: u32,
    pick: RollPick,
}
#[derive(Component)]
struct TimerFill(u32);
#[derive(Component)]
struct ChoiceText(u32);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<HostRolls>()
        .init_resource::<ShownRolls>()
        .add_systems(
            Update,
            (
                host_rolls.run_if(super::hosting),
                hear_rolls,
                roll_clicks,
                roll_frames,
            )
                .chain(),
        )
        .add_systems(OnEnter(Phase::Menu), clear_rolls);
}

/// The host opens a roll for each piece put up, takes the choices, and settles each roll once
/// decided: the line for everyone and the piece to its winner's machine.
#[allow(clippy::too_many_arguments)]
fn host_rolls(
    time: Res<Time<Real>>,
    mut drops: ResMut<GroupDrops>,
    mut host: ResMut<HostRolls>,
    mut said: MessageReader<FromClient<Say>>,
    peers: Query<&Peer>,
    clients: Query<(Entity, &Peer)>,
    heroes: Query<&NetHero>,
    session: Res<Session>,
    mut rng: ResMut<Rng>,
    mut out: MessageWriter<ToClients<Heard>>,
) {
    for drop in std::mem::take(&mut drops.0) {
        let Some(piece) = gear::item(drop.item) else {
            continue;
        };
        let party: Vec<(u8, usize)> = heroes
            .iter()
            .map(|h| (h.player, usize::from(h.class)))
            .collect();
        let id = host.next;
        host.next = host.next.wrapping_add(1);
        host.rolls.push(GroupRoll::new(id, piece.id, &party));
        out.write(to_all(Heard::RollStart {
            id,
            item: piece.id,
            secs: ROLL_SECS,
        }));
    }
    for m in said.read() {
        let Speech::Loot { roll, pick } = m.message.0 else {
            continue;
        };
        let Some(player) = player_of(m.client_id, &peers) else {
            continue;
        };
        if let Some(r) = host.rolls.iter_mut().find(|r| r.id == roll) {
            r.choose(player, pick);
        }
    }
    let dt = time.delta_secs();
    let mut done = Vec::new();
    for r in &mut host.rolls {
        r.tick(dt);
        if r.decided() {
            done.push(r.clone());
        }
    }
    host.rolls.retain(|r| !r.decided());
    for r in done {
        out.write(to_all(Heard::RollEnd { id: r.id }));
        let win = r.settle(|| rng.index(100) as u32 + 1);
        let Some(win) = win else {
            out.write(to_all(Heard::Line(Line {
                player: 0,
                name: String::new(),
                body: Body::AllPassed { item: r.item },
            })));
            continue;
        };
        out.write(to_all(Heard::Line(Line {
            player: win.player,
            name: name_of(&session, win.player),
            body: Body::Won {
                item: r.item,
                pick: win.pick,
                value: win.value,
            },
        })));
        let target = if win.player == 0 {
            Some(ClientId::Server)
        } else {
            clients
                .iter()
                .find(|(_, p)| p.player == win.player)
                .map(|(e, _)| ClientId::Client(e))
        };
        if let Some(target) = target {
            out.write(ToClients {
                targets: SendTargets::Single(target),
                message: Heard::Grant { item: r.item },
            });
        }
    }
}

/// Rolls open and close here as the host says; a piece won here goes into this profile.
fn hear_rolls(
    mut heard: MessageReader<Heard>,
    mut shown: ResMut<ShownRolls>,
    mut won: ResMut<WonLoot>,
    mut commands: Commands,
) {
    for h in heard.read() {
        match h {
            Heard::RollStart { id, item, secs } => shown.0.push(Shown {
                id: *id,
                item: *item,
                secs: *secs,
                left: *secs,
                picked: None,
                frame: None,
            }),
            Heard::RollEnd { id } => {
                for s in shown.0.iter().filter(|s| s.id == *id) {
                    if let Some(f) = s.frame {
                        commands.entity(f).try_despawn();
                    }
                }
                shown.0.retain(|s| s.id != *id);
            }
            Heard::Grant { item } => won.0.push(*item),
            _ => {}
        }
    }
}

/// This player's class, by the menu's pick.
fn my_class(selection: &Selection) -> usize {
    selection.class.unwrap_or(0)
}

/// Need, Greed or Pass clicked: the choice goes to the host (a Need for another class's piece is
/// refused here as there), and Need or Greed rolls the dice.
fn roll_clicks(
    clicks: Res<GlueClicks>,
    buttons: Query<(Entity, &RollButton)>,
    mut shown: ResMut<ShownRolls>,
    selection: Res<Selection>,
    mut say: MessageWriter<Say>,
    mut cues: MessageWriter<SoundCue>,
) {
    let Some(b) = buttons
        .iter()
        .find(|(e, _)| clicks.hit(*e))
        .map(|(_, b)| *b)
    else {
        return;
    };
    let Some(s) = shown.0.iter_mut().find(|s| s.id == b.id) else {
        return;
    };
    if s.picked.is_some() {
        return;
    }
    if b.pick == RollPick::Need && !may_need(s.item, my_class(&selection)) {
        return;
    }
    s.picked = Some(b.pick);
    say.write(Say(Speech::Loot {
        roll: b.id,
        pick: b.pick,
    }));
    if b.pick != RollPick::Pass {
        cues.write(SoundCue::flat(DICE));
    }
}

/// Each open roll's frame: built as it opens, its timer running down, its buttons pressed in and
/// put away once this player has chosen; stacked up from above the action bar.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn roll_frames(
    mut commands: Commands,
    time: Res<Time<Real>>,
    font: Option<Res<HudFont>>,
    selection: Res<Selection>,
    gear: GearRead,
    mut shown: ResMut<ShownRolls>,
    mut frames: Query<&mut Node, With<RollFrame>>,
    mut fills: Query<(&TimerFill, &mut Node), Without<RollFrame>>,
    mut buttons: Query<
        (&RollButton, &Interaction, &mut Art, &mut Node),
        (Without<RollFrame>, Without<TimerFill>),
    >,
    mut choices: Query<(&ChoiceText, &mut Text)>,
) {
    let Some(font) = font else { return };
    let dt = time.delta_secs();
    let mine = my_class(&selection);
    for (i, s) in shown.0.iter_mut().enumerate() {
        s.left = (s.left - dt).max(0.0);
        let bottom = Val::Px(170.0 + i as f32 * 82.0);
        match s.frame {
            Some(f) => {
                if let Ok(mut node) = frames.get_mut(f) {
                    if node.bottom != bottom {
                        node.bottom = bottom;
                    }
                }
            }
            None => s.frame = Some(spawn_frame(&mut commands, &font.0, s, mine, &gear, bottom)),
        }
    }
    for (fill, mut node) in &mut fills {
        let Some(s) = shown.0.iter().find(|s| s.id == fill.0) else {
            continue;
        };
        let w = Val::Percent(timer_percent(s.left, s.secs));
        if node.width != w {
            node.width = w;
        }
    }
    for (b, interaction, mut art, mut node) in &mut buttons {
        let Some(s) = shown.0.iter().find(|s| s.id == b.id) else {
            continue;
        };
        let display = Display::Flex;
        if node.display != display {
            node.display = display;
        }
        let (up, down) = BUTTONS[RollPick::ALL.iter().position(|&p| p == b.pick).unwrap_or(2)];
        let path = if *interaction == Interaction::Pressed {
            down
        } else {
            up
        };
        if art.path != path {
            art.path = path;
        }
        let tint = match *interaction {
            _ if s.picked.is_some_and(|picked| picked != b.pick) => {
                Color::srgba(0.35, 0.35, 0.35, 0.7)
            }
            Interaction::Hovered => Color::srgb(1.0, 0.9, 0.45),
            _ if b.pick == RollPick::Need && !may_need(s.item, mine) => {
                Color::srgba(0.35, 0.35, 0.35, 0.7)
            }
            _ => Color::WHITE,
        };
        if art.tint != tint {
            art.tint = tint;
        }
    }
    for (c, mut t) in &mut choices {
        let Some(s) = shown.0.iter().find(|s| s.id == c.0) else {
            continue;
        };
        let want = s.picked.map_or("", RollPick::label);
        if t.0 != want {
            t.0 = want.to_string();
        }
    }
}

/// A `GroupLootFrame`: a vertical, fixed-width 1.12-style card. Its timer lives in its own clipped
/// row, and its controls in their own row, so neither can escape the frame or hide the buttons.
fn spawn_frame(
    commands: &mut Commands,
    font: &Handle<Font>,
    s: &Shown,
    mine: usize,
    gear: &GearRead,
    bottom: Val,
) -> Entity {
    let piece = gear::item(s.item);
    let (name, color, icon) = piece.map_or(
        ("Something shiny", Color::WHITE, "INV_Misc_QuestionMark"),
        |p| (p.name, p.rarity.color(), p.icon),
    );
    let need_ok = may_need(s.item, mine);
    let tip = piece.map(|p| item_tip(p, gear.get(mine)));
    let px = Val::Px;
    commands
        .spawn((
            RollFrame,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                bottom,
                margin: UiRect::left(px(-160.0)),
                width: px(FRAME_WIDTH),
                height: px(126.0),
                padding: UiRect::all(px(8.0)),
                column_gap: px(8.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.05, 0.9)),
            BorderColor::all(Color::srgb(0.75, 0.6, 0.25)),
            GlobalZIndex(310),
        ))
        .with_children(|f| {
            f.spawn(Node {
                width: Val::Percent(100.0),
                column_gap: px(8.0),
                ..default()
            })
            .with_children(|top| {
                let mut icon_node = top.spawn((
                    Button,
                    Node {
                        width: px(44.0),
                        height: px(44.0),
                        border: UiRect::all(px(1.0)),
                        ..default()
                    },
                    BorderColor::all(color),
                    ImageNode {
                        color: Color::srgb(0.1, 0.1, 0.1),
                        ..default()
                    },
                    ItemIcon {
                        icon,
                        tint: Color::WHITE,
                    },
                ));
                if let Some(tip) = tip.clone() {
                    icon_node.insert(tip);
                }
                top.spawn(Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4.0),
                    ..default()
                })
                .with_children(|c| {
                    let mut title = c.spawn((
                        Button,
                        label(font, name, 14.0, color),
                        TextShadow::default(),
                    ));
                    if let Some(tip) = tip.clone() {
                        title.insert(tip);
                    }
                    c.spawn((
                        Node {
                            height: px(10.0),
                            width: Val::Percent(100.0),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
                    ))
                    .with_children(|bar| {
                        bar.spawn((
                            TimerFill(s.id),
                            Node {
                                height: Val::Percent(100.0),
                                width: Val::Percent(100.0),
                                ..default()
                            },
                            Art::crop(TIMER_BAR, [0.0, 1.0, 0.0, 0.25])
                                .tinted(Color::srgb(1.0, 0.82, 0.0)),
                        ));
                    });
                });
            });
            f.spawn(Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                column_gap: px(18.0),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|controls| {
                for (pick, (up, _)) in RollPick::ALL.into_iter().zip(BUTTONS) {
                    let tint = if pick == RollPick::Need && !need_ok {
                        Color::srgba(0.35, 0.35, 0.35, 0.7)
                    } else {
                        Color::WHITE
                    };
                    controls.spawn((
                        RollButton { id: s.id, pick },
                        Button,
                        Interaction::default(),
                        Node {
                            width: px(32.0),
                            height: px(32.0),
                            ..default()
                        },
                        Art::whole(up).tinted(tint),
                    ));
                }
                controls.spawn((
                    ChoiceText(s.id),
                    label(font, "", 12.0, Color::srgb(0.8, 0.8, 0.8)),
                ));
            });
        })
        .id()
}

/// Back at the menu, every roll still open is gone.
fn clear_rolls(mut commands: Commands, mut shown: ResMut<ShownRolls>, mut host: ResMut<HostRolls>) {
    for s in shown.0.drain(..) {
        if let Some(f) = s.frame {
            commands.entity(f).try_despawn();
        }
    }
    host.rolls.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A d100 that answers from a script.
    fn dice(rolls: &[u32]) -> impl FnMut() -> u32 + '_ {
        let mut i = 0;
        move || {
            let r = rolls[i % rolls.len()];
            i += 1;
            r
        }
    }

    const MAGE: usize = 6;
    const PRIEST: usize = 4;
    const WARRIOR: usize = 0;
    const ROGUE: usize = 3;

    #[test]
    fn need_follows_armor_and_weapon_skills_not_the_drop_class() {
        // Warrior plate (100) is usable by a paladin too, but not a rogue; mage cloth (700) is
        // usable by every armour class. Hunters and shamans use their level-40 mail maximum.
        assert!(may_need(100, 1));
        assert!(!may_need(100, ROGUE));
        assert!(may_need(700, WARRIOR));
        assert!(may_need(300, 5));
        // A warrior sword is valid for a rogue, while the hunter axe is not.
        assert!(may_need(108, ROGUE));
        assert!(!may_need(308, ROGUE));

        let mut r = GroupRoll::new(1, 700, &[(0, MAGE), (1, PRIEST)]);
        assert!(
            r.choose(1, RollPick::Need),
            "a priest can need a cloth robe"
        );
        assert!(!r.choose(1, RollPick::Pass), "one choice each");
        assert!(r.choose(0, RollPick::Need));
        assert!(!r.choose(7, RollPick::Greed), "not at the roll");
        assert!(r.decided());
        // Both are eligible Need rolls, so the higher die wins.
        assert_eq!(
            r.settle(dice(&[3, 4])),
            Some(Win {
                player: 1,
                pick: RollPick::Need,
                value: 4
            })
        );
    }

    #[test]
    fn ties_roll_again_among_the_tied() {
        let mut r = GroupRoll::new(1, 700, &[(0, MAGE), (1, MAGE), (2, MAGE)]);
        for p in 0..3 {
            r.choose(p, RollPick::Need);
        }
        // 50, 50 and 20: the two fifties roll again, 30 against 70.
        assert_eq!(
            r.settle(dice(&[50, 50, 20, 30, 70])),
            Some(Win {
                player: 1,
                pick: RollPick::Need,
                value: 70
            })
        );
        // A greed-only roll is settled the same way.
        let mut g = GroupRoll::new(2, 700, &[(0, PRIEST), (1, PRIEST)]);
        g.choose(0, RollPick::Greed);
        g.choose(1, RollPick::Greed);
        assert_eq!(g.settle(dice(&[99, 12])).map(|w| w.player), Some(0));
    }

    #[test]
    fn the_timer_passes_for_whoever_has_not_chosen() {
        let mut r = GroupRoll::new(1, 700, &[(0, MAGE), (1, PRIEST)]);
        r.choose(1, RollPick::Greed);
        r.tick(ROLL_SECS - 0.5);
        assert!(!r.decided(), "still open with a player to choose");
        r.tick(1.0);
        assert!(r.decided(), "the time is up");
        // The mage never chose: the priest's greed takes it.
        assert_eq!(
            r.settle(dice(&[40])).map(|w| (w.player, w.pick)),
            Some((1, RollPick::Greed))
        );
    }

    #[test]
    fn everyone_passing_leaves_it_on_the_ground() {
        let mut r = GroupRoll::new(1, 700, &[(0, MAGE), (1, PRIEST)]);
        r.choose(0, RollPick::Pass);
        r.choose(1, RollPick::Pass);
        assert!(r.decided());
        assert_eq!(r.settle(dice(&[1])), None);
        // Nobody choosing at all is everyone passing.
        let mut idle = GroupRoll::new(2, 700, &[(0, MAGE)]);
        idle.tick(ROLL_SECS);
        assert_eq!(idle.settle(dice(&[1])), None);
    }

    #[test]
    fn roll_frame_has_three_buttons_and_a_clamped_timer() {
        assert_eq!(RollPick::ALL.len(), 3);
        assert_eq!(BUTTONS.len(), 3);
        assert_eq!(FRAME_WIDTH, 330.0);
        assert_eq!(timer_percent(ROLL_SECS, ROLL_SECS), 100.0);
        assert_eq!(timer_percent(-1.0, ROLL_SECS), 0.0);
        assert_eq!(timer_percent(ROLL_SECS * 2.0, ROLL_SECS), 100.0);
    }
}
