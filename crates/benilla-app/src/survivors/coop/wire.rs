//! What travels between a co-op host and its clients: the replicated components (on heroes,
//! enemies, pickups and the run), and the messages either way. Everything here is plain data that
//! postcard packs; the `&'static` tables (abilities, passives, classes, rosters) travel as their keys
//! or indices, which both machines' binaries share.

use bevy::ecs::entity::{EntityMapper, MapEntities};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::super::bonus::Bonuses;
use super::super::data;
use super::super::gear::Features;
use super::super::gear::Slot;
use super::super::hand::Struck;
use super::super::hud::Choice;
use super::super::party::HandDecision;
use super::super::raid::{ForcedMove, HeroRaid};
use super::super::Overcharge;

/// An entity inside a message, packed as replicon packs its own.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) struct Ent(
    #[serde(with = "bevy_replicon::compact_entity")] pub(in crate::survivors) Entity,
);

// ---------------------------------------------------------------------------------------------
// Replicated components
// ---------------------------------------------------------------------------------------------

/// Where a unit or object stands, quantised: centimetres, a 256-step facing, and the walk the
/// animation shows (pace in cm/s and the movement flags).
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::survivors) struct NetPos {
    pub(in crate::survivors) at: [i32; 3],
    pub(in crate::survivors) yaw: u8,
    pub(in crate::survivors) pace: u16,
    pub(in crate::survivors) flags: u32,
}

impl NetPos {
    pub(in crate::survivors) fn new(pos: Vec3, yaw: f32, pace: f32, flags: u32) -> Self {
        let cm = |v: f32| (v * 100.0).round() as i32;
        let turn = yaw.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU;
        Self {
            at: [cm(pos.x), cm(pos.y), cm(pos.z)],
            yaw: ((turn * 256.0).round() as u32 % 256) as u8,
            pace: (pace.max(0.0) * 100.0).round().min(f32::from(u16::MAX)) as u16,
            flags,
        }
    }

    pub(in crate::survivors) fn pos(&self) -> Vec3 {
        Vec3::new(
            self.at[0] as f32 / 100.0,
            self.at[1] as f32 / 100.0,
            self.at[2] as f32 / 100.0,
        )
    }

    /// The facing in radians, in `[0, τ)`.
    pub(in crate::survivors) fn yaw(&self) -> f32 {
        f32::from(self.yaw) / 256.0 * std::f32::consts::TAU
    }

    pub(in crate::survivors) fn pace(&self) -> f32 {
        f32::from(self.pace) / 100.0
    }
}

/// Who a hero is: its player, name, and the class, race and sex its body is built from.
/// `trainer` is the class trainer's display, real on every machine; a dressed display is a local
/// one, so each machine dresses only its own hero.
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(in crate::survivors) struct NetHero {
    pub(in crate::survivors) player: u8,
    pub(in crate::survivors) name: String,
    /// An index into [`data::CLASSES`].
    pub(in crate::survivors) class: u8,
    pub(in crate::survivors) race: u8,
    pub(in crate::survivors) sex: u8,
    pub(in crate::survivors) trainer: u32,
    /// The selected riding creature, or zero when standing.
    pub(in crate::survivors) mount_display: u32,
    /// The mount hat model and its mount-specific seat nudge.
    pub(in crate::survivors) mount_hat: Option<super::super::stable::CosmeticHat>,
}

/// The character-create choices and equipped display ids belonging to a hero.  This is separate
/// from [`NetHero`] so a body can be re-dressed without changing its identity.
#[derive(Component, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::survivors) struct NetLooks {
    /// `None` means use the trainer's features for this body.
    pub(in crate::survivors) features: Option<Features>,
    /// Armour and held-weapon display ids in [`Slot::ALL`] order; zero means the starter outfit.
    pub(in crate::survivors) gear: [u32; Slot::ALL.len()],
}

/// A hero's health bar and whether it is down.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::survivors) struct NetVitals {
    pub(in crate::survivors) hp: f32,
    pub(in crate::survivors) max_hp: f32,
    pub(in crate::survivors) shield: f32,
    pub(in crate::survivors) downed: bool,
    /// A downed hero's revive, 0 to 1 ([`HeroState::revive_t`](super::super::HeroState)).
    pub(in crate::survivors) revive: f32,
    /// Focused: its co-op hand strip is open, under the Divine Protection glow.
    pub(in crate::survivors) focus: bool,
}

/// A hero's build: what its player's HUD, cards and tools read.
#[derive(Component, Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub(in crate::survivors) struct NetBuild {
    pub(in crate::survivors) abilities: Vec<(String, u8)>,
    pub(in crate::survivors) passives: Vec<(String, u8)>,
    pub(in crate::survivors) bonuses: Bonuses,
    pub(in crate::survivors) picks: u32,
    pub(in crate::survivors) rerolls: u32,
    pub(in crate::survivors) banishes: u32,
    pub(in crate::survivors) banished: Vec<WireStruck>,
    /// Overcharges taken, by [`Overcharge::index`].
    pub(in crate::survivors) overcharges: [u32; 6],
    /// Jewelry choices banked.
    pub(in crate::survivors) jewels: u32,
}

/// The party's group buffs on a hero ([`super::super::synergy`]), each by its index in the
/// buff table and its strength: its stats and Character panel count them, as the host's do.
/// Sent at most [`super::SLOW_HZ`] times a second.
#[derive(Component, Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub(in crate::survivors) struct NetBuffs(pub(in crate::survivors) Vec<(u8, f32)>);

/// The auras a unit shows, in their slots' order: a hero's buffs, stance, soulstone and marks, an
/// enemy's shared debuffs and elite mark. Sent at most [`super::SLOW_HZ`] times a second.
#[derive(Component, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::survivors) struct NetAuras(pub(in crate::survivors) Vec<u32>);

/// A hero's share of the run so far, for the damage meter and the results' awards: sent at most
/// [`super::SLOW_HZ`] times a second, and once more as the run ends.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::survivors) struct NetStats {
    pub(in crate::survivors) damage_done: f64,
    pub(in crate::survivors) healing_done: f64,
    pub(in crate::survivors) damage_taken: f64,
    pub(in crate::survivors) revives: u32,
    pub(in crate::survivors) gold: u32,
    pub(in crate::survivors) downs: u32,
}

/// A hero's part in a raid boss's fight that its own machine plays: the latest forced move (fear,
/// knockback, mind control) by its count, and the seconds of fear or mind control left, in tenths
/// (a dispel ends one early). Its debuffs travel as its auras ([`NetAuras`]).
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::survivors) struct NetRaid {
    pub(in crate::survivors) moves: u16,
    pub(in crate::survivors) last: Option<ForcedMove>,
    pub(in crate::survivors) left_tenths: u16,
}

impl NetRaid {
    pub(in crate::survivors) fn of(raid: &HeroRaid) -> Self {
        Self {
            moves: raid.moves,
            last: raid.last_move,
            left_tenths: (raid.forced_left.max(0.0) * 10.0)
                .ceil()
                .min(f32::from(u16::MAX)) as u16,
        }
    }

    /// Into the mirror hero's raid part, which its walk reads.
    pub(in crate::survivors) fn apply(&self, raid: &mut HeroRaid) {
        raid.moves = self.moves;
        raid.last_move = self.last;
        raid.forced_left = f32::from(self.left_tenths) / 10.0;
    }
}

/// An enemy's kind (its place in the stage's roster) and rank, sent once. A raid add is numbered on
/// past the horde ([`data::Roster::kind`]).
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) struct NetEnemy {
    pub(in crate::survivors) boss: bool,
    pub(in crate::survivors) index: u16,
    pub(in crate::survivors) elite: bool,
    pub(in crate::survivors) guardian: bool,
    pub(in crate::survivors) max_hp: f32,
}

/// An enemy's health and death: the lean part that changes.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) struct NetLife {
    pub(in crate::survivors) hp: f32,
    pub(in crate::survivors) dead: bool,
}

/// The party's shared run, on one replicated entity: the clock, kills, the XP bar, gold and the
/// announcement.
#[derive(Component, Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub(in crate::survivors) struct NetRun {
    pub(in crate::survivors) elapsed: f32,
    pub(in crate::survivors) kills: u32,
    pub(in crate::survivors) level: u32,
    pub(in crate::survivors) xp: f32,
    pub(in crate::survivors) gold: u32,
    pub(in crate::survivors) elites: u32,
    pub(in crate::survivors) caches: u32,
    pub(in crate::survivors) bosses: u32,
    pub(in crate::survivors) won: bool,
    pub(in crate::survivors) banner: Option<String>,
    /// Every living player is choosing: the world runs slowed.
    pub(in crate::survivors) slow: bool,
    /// The authoritative camp timer and ready-check state.
    pub(in crate::survivors) camp_left: f32,
    pub(in crate::survivors) camp_ready: Vec<u8>,
    pub(in crate::survivors) camp_fished: Vec<u8>,
    pub(in crate::survivors) fishing_dip: f32,
    pub(in crate::survivors) camp_rolls: Vec<(u8, u8)>,
    pub(in crate::survivors) camp_roll_winner: Option<u8>,
    /// Where the camp's fire burns: every machine builds the camp's props round it.
    pub(in crate::survivors) camp_fire: Option<[f32; 3]>,
}

/// A banished choice, by its key.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum WireStruck {
    Ability(String),
    Passive(String),
}

impl WireStruck {
    pub(in crate::survivors) fn of(s: Struck) -> Self {
        match s {
            Struck::Ability(k) => Self::Ability(k.to_string()),
            Struck::Passive(k) => Self::Passive(k.to_string()),
        }
    }

    /// The struck choice, if this binary knows the key.
    pub(in crate::survivors) fn struck(&self) -> Option<Struck> {
        match self {
            Self::Ability(k) => ability(k).map(|a| Struck::Ability(a.key)),
            Self::Passive(k) => passive(k).map(|p| Struck::Passive(p.key)),
        }
    }
}

/// The ability keyed `key`, if there is one (unlike [`data::ability`], which panics).
pub(in crate::survivors) fn ability(key: &str) -> Option<&'static data::AbilityDef> {
    data::ABILITIES.iter().find(|a| a.key == key)
}

pub(in crate::survivors) fn passive(key: &str) -> Option<&'static data::PassiveDef> {
    data::PASSIVES.iter().find(|p| p.key == key)
}

/// A level-up card, by key or slot.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum WireChoice {
    NewAbility(String),
    Upgrade(u8),
    NewPassive(String),
    UpPassive(u8),
    Heal,
    /// An [`Overcharge`], by its index.
    Overcharge(u8),
    /// A piece of jewelry, by its key.
    Jewel(String),
}

impl WireChoice {
    pub(in crate::survivors) fn of(choice: Choice) -> Self {
        match choice {
            Choice::NewAbility(def) => Self::NewAbility(def.key.to_string()),
            Choice::Upgrade(i) => Self::Upgrade(i as u8),
            Choice::NewPassive(def) => Self::NewPassive(def.key.to_string()),
            Choice::UpPassive(i) => Self::UpPassive(i as u8),
            Choice::Heal => Self::Heal,
            Choice::Overcharge(kind) => Self::Overcharge(kind.index() as u8),
            Choice::Jewel(piece) => Self::Jewel(piece.key.to_string()),
        }
    }

    /// The card, if this binary knows its key.
    pub(in crate::survivors) fn choice(&self) -> Option<Choice> {
        Some(match self {
            Self::NewAbility(k) => Choice::NewAbility(ability(k)?),
            Self::Upgrade(i) => Choice::Upgrade(usize::from(*i)),
            Self::NewPassive(k) => Choice::NewPassive(passive(k)?),
            Self::UpPassive(i) => Choice::UpPassive(usize::from(*i)),
            Self::Heal => Choice::Heal,
            Self::Jewel(key) => Choice::Jewel(super::super::jewelry::piece(key)?),
            Self::Overcharge(i) => Choice::Overcharge(*Overcharge::ALL.get(usize::from(*i))?),
        })
    }
}

/// A choice at the hand, as it travels.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum WireDecision {
    Pick(u8),
    Reroll,
    Banish(u8),
    Skip,
}

impl WireDecision {
    pub(in crate::survivors) fn of(d: HandDecision) -> Self {
        match d {
            HandDecision::Pick(i) => Self::Pick(i.min(255) as u8),
            HandDecision::Reroll => Self::Reroll,
            HandDecision::Banish(i) => Self::Banish(i.min(255) as u8),
            HandDecision::Skip => Self::Skip,
        }
    }

    pub(in crate::survivors) fn decision(self) -> HandDecision {
        match self {
            Self::Pick(i) => HandDecision::Pick(usize::from(i)),
            Self::Reroll => HandDecision::Reroll,
            Self::Banish(i) => HandDecision::Banish(usize::from(i)),
            Self::Skip => HandDecision::Skip,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Client → host
// ---------------------------------------------------------------------------------------------

/// A client introduces itself on connecting: its name, pick, and profile's bonuses and learned
/// talent ranks (the host builds its hero and spec on them).
#[derive(Message, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(in crate::survivors) struct Hello {
    pub(in crate::survivors) name: String,
    pub(in crate::survivors) class: u8,
    pub(in crate::survivors) race: u8,
    pub(in crate::survivors) sex: u8,
    pub(in crate::survivors) trainer: u32,
    pub(in crate::survivors) looks: NetLooks,
    pub(in crate::survivors) bonuses: Bonuses,
    /// Talent id and learned rank, from the picked class's saved allocation.
    pub(in crate::survivors) talents: Vec<(u32, u8)>,
    pub(in crate::survivors) mount_display: u32,
    pub(in crate::survivors) mount_hat: Option<super::super::stable::CosmeticHat>,
    /// Completed-deck trinket pinned by this player's own profile.
    pub(in crate::survivors) pinned_trinket: Option<String>,
    /// The build it runs ([`super::build_tag`]): the host turns away any other.
    pub(in crate::survivors) build: String,
}

/// The client's own hero, where its player walked it (~30 Hz, unreliable: the next one replaces a
/// lost one).
#[derive(Message, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) struct MoveInput {
    pub(in crate::survivors) at: [f32; 3],
    pub(in crate::survivors) yaw: f32,
    pub(in crate::survivors) pace: f32,
    pub(in crate::survivors) flags: u32,
}

/// A joined client's changed stable selection.  It is reliable so the lobby and a waiting run
/// show the owner's current cosmetics without making a new connection.
#[derive(Message, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) struct Cosmetics {
    pub(in crate::survivors) mount_display: u32,
    pub(in crate::survivors) mount_hat: Option<super::super::stable::CosmeticHat>,
}

/// The client's hand strip opened (true) or shut: its hero is Focused while open.
#[derive(Message, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) struct FocusInput(pub(in crate::survivors) bool);

/// A choice at the client's level-up hand.
#[derive(Message, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) struct HandAction(pub(in crate::survivors) WireDecision);

/// The client's player pressed the self-resurrection key while its hero is down: a soulstone or
/// an ankh gets it up on the host, as the local player's key does there.
#[derive(Message, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) struct SelfRes;

/// A camp interaction requested by one client. The host checks the camp, owner, timing and purse,
/// and rolls the dice itself.
#[derive(Message, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum CampAction {
    Ready,
    Fish,
    Roll,
    /// A [`VendorItem`](super::super::campfire::VendorItem), by its place.
    Buy(u8),
}

// ---------------------------------------------------------------------------------------------
// Host → client
// ---------------------------------------------------------------------------------------------

/// The host's answer to [`Hello`]: the player number it gave this client.
#[derive(Message, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(in crate::survivors) struct Welcome {
    pub(in crate::survivors) player: u8,
    /// The host's name and build: a client on another build leaves.
    pub(in crate::survivors) host: String,
    pub(in crate::survivors) build: String,
}

/// The host turned this client away, and why; it is let go after this.
#[derive(Message, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(in crate::survivors) struct Refused {
    pub(in crate::survivors) reason: String,
}

/// One player in the lobby.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(in crate::survivors) struct Member {
    pub(in crate::survivors) player: u8,
    pub(in crate::survivors) name: String,
    pub(in crate::survivors) class: u8,
    pub(in crate::survivors) race: u8,
}

/// Everyone connected, the host first.
#[derive(Message, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::survivors) struct Lobby {
    pub(in crate::survivors) players: Vec<Member>,
}

/// The host started a run: everyone enters this stage under these Run Options (density, health,
/// damage, speed).
#[derive(Message, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) struct StartRun {
    pub(in crate::survivors) stage: u8,
    pub(in crate::survivors) options: [f32; 4],
}

/// The run ended (to the results), or the host went back to the menu.
#[derive(Message, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum RunPhase {
    Over { won: bool },
    Lobby,
}

/// The client's next level-up hand, rolled by the host; empty when it has none left. It carries the
/// build its slots index into, which may not have replicated yet.
#[derive(Message, Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub(in crate::survivors) struct Hand {
    pub(in crate::survivors) choices: Vec<WireChoice>,
    pub(in crate::survivors) build: NetBuild,
}

/// A boss's yell or emote: its text and its sound's place in [`super::super::raid::SOUNDS`].
#[derive(Message, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(in crate::survivors) struct NetYell {
    pub(in crate::survivors) text: String,
    pub(in crate::survivors) sound: u8,
}

/// A cast edge's kind, as [`crate::creature_anim::CastEventKind`].
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) enum WireCast {
    Start,
    Go,
    Fail,
    Impact { weapon_visual: Option<u32> },
    GroundImpact { at: [f32; 3] },
}

/// A spell visual or combat number the host played, for clients to play the same: the host decides
/// every hit, so these are its only trace on a client.
#[derive(Message, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(in crate::survivors) enum NetFx {
    /// `SMSG_SPELL_GO` with its targets: the cast kit on the caster, a missile or an impact at each
    /// hit.
    Go {
        caster: Ent,
        spell: u32,
        hits: Vec<Ent>,
        dest: Option<[f32; 3]>,
    },
    /// Any other cast edge.
    Cast {
        entity: Ent,
        spell: u32,
        kind: WireCast,
    },
    /// A melee swing.
    Swing {
        attacker: Ent,
        victim: Option<Ent>,
        hit_info: u32,
        victim_state: u32,
        damage: u32,
    },
    /// A floating number.
    Text {
        anchor: Ent,
        text: String,
        category: u8,
        color: Option<u32>,
    },
}

impl NetFx {
    /// Every entity the effect names.
    pub(in crate::survivors) fn entities(&self) -> Vec<Entity> {
        match self {
            Self::Go { caster, hits, .. } => std::iter::once(caster.0)
                .chain(hits.iter().map(|h| h.0))
                .collect(),
            Self::Cast { entity, .. } => vec![entity.0],
            Self::Swing {
                attacker, victim, ..
            } => std::iter::once(attacker.0)
                .chain(victim.map(|v| v.0))
                .collect(),
            Self::Text { anchor, .. } => vec![anchor.0],
        }
    }
}

impl MapEntities for NetFx {
    fn map_entities<E: EntityMapper>(&mut self, m: &mut E) {
        let mut map = |e: &mut Ent| e.0 = m.get_mapped(e.0);
        match self {
            Self::Go { caster, hits, .. } => {
                map(caster);
                hits.iter_mut().for_each(map);
            }
            Self::Cast { entity, .. } => map(entity),
            Self::Swing {
                attacker, victim, ..
            } => {
                map(attacker);
                if let Some(v) = victim {
                    map(v);
                }
            }
            Self::Text { anchor, .. } => map(anchor),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::survivors::bonus::{Bonus, StatKey};

    /// Through postcard, as replicon sends it, and back.
    fn round<T: Serialize + for<'de> Deserialize<'de>>(v: &T) -> T {
        let mut buf = [0u8; 2048];
        let bytes = bevy_replicon::postcard::to_slice(v, &mut buf).expect("serializes");
        bevy_replicon::postcard::from_bytes(bytes).expect("deserializes")
    }

    fn size<T: Serialize>(v: &T) -> usize {
        let mut buf = [0u8; 2048];
        bevy_replicon::postcard::to_slice(v, &mut buf)
            .unwrap()
            .len()
    }

    #[test]
    fn positions_quantise_to_centimetres_and_a_256th_turn() {
        let p = NetPos::new(Vec3::new(-1234.567, 88.004, 3.21), 1.0, 7.0, 0x1);
        assert!(p.pos().distance(Vec3::new(-1234.57, 88.0, 3.21)) < 0.006);
        assert!((p.yaw() - 1.0).abs() < std::f32::consts::TAU / 256.0);
        assert!((p.pace() - 7.0).abs() < 0.01);
        // Negative facings wrap into the turn; a full turn is no turn.
        assert_eq!(
            NetPos::new(Vec3::ZERO, -std::f32::consts::PI, 0.0, 0).yaw,
            128
        );
        assert_eq!(
            NetPos::new(Vec3::ZERO, std::f32::consts::TAU, 0.0, 0).yaw,
            0
        );
        assert_eq!(round(&p), p);
        // Lean: a still enemy is a handful of bytes.
        let still = NetPos::new(Vec3::new(500.0, 20.0, -300.0), 2.0, 0.0, 0);
        assert!(size(&still) <= 14);
    }

    #[test]
    fn every_card_round_trips_by_key() {
        let hero = super::super::super::HeroState::default();
        let mut cards = vec![Choice::Heal, Choice::Upgrade(2), Choice::UpPassive(1)];
        cards.extend(Overcharge::ALL.map(Choice::Overcharge));
        cards.extend(
            super::super::super::jewelry::pieces()
                .iter()
                .map(Choice::Jewel),
        );
        cards.extend(data::ABILITIES.iter().map(Choice::NewAbility));
        cards.extend(data::PASSIVES.iter().map(Choice::NewPassive));
        for card in cards {
            let wire = round(&WireChoice::of(card));
            let back = wire.choice().expect("a known key");
            assert_eq!(
                super::super::super::hand::struck(back, &hero),
                super::super::super::hand::struck(card, &hero)
            );
            assert_eq!(WireChoice::of(back), WireChoice::of(card));
        }
        assert!(WireChoice::NewAbility("no such spell".into())
            .choice()
            .is_none());
    }

    #[test]
    fn messages_survive_the_wire() {
        let mut bonuses = Bonuses::default();
        bonuses.add(&Bonus::new(StatKey::MaxHealth, 30.0));
        let hello = Hello {
            name: "Thrall".into(),
            class: 3,
            race: 2,
            sex: 0,
            trainer: 4000,
            looks: NetLooks {
                features: Some(Features::from_array([2, 1, 3, 4, 5])),
                gear: [100, 101, 102, 103, 104, 105, 106, 107, 108],
            },
            bonuses: bonuses.clone(),
            talents: vec![(36, 1)],
            mount_display: 2404,
            mount_hat: Some(super::super::super::stable::CosmeticHat {
                display: 1234,
                mount_nudge: [0.0, 0.2, -0.1],
                offset: [0.0; 3],
                pitch: 0.0,
                scale: 1.0,
            }),
            pinned_trinket: None,
            build: "p1-0.2.0-abc1234".into(),
        };
        assert_eq!(round(&hello), hello);
        let cosmetics = Cosmetics {
            mount_display: 10718,
            mount_hat: hello.mount_hat,
        };
        assert_eq!(round(&cosmetics), cosmetics);
        let refused = Refused {
            reason: super::super::mismatch("Thrall"),
        };
        assert_eq!(round(&refused), refused);
        let mv = MoveInput {
            at: [1.0, 2.0, -3.5],
            yaw: 0.25,
            pace: 7.0,
            flags: 1,
        };
        assert_eq!(round(&mv), mv);
        assert_eq!(round(&FocusInput(true)), FocusInput(true));
        assert_eq!(round(&SelfRes), SelfRes);
        for d in [
            HandDecision::Pick(2),
            HandDecision::Reroll,
            HandDecision::Banish(1),
            HandDecision::Skip,
        ] {
            assert_eq!(round(&HandAction(WireDecision::of(d))).0.decision(), d);
        }
        let lobby = Lobby {
            players: vec![Member {
                player: 1,
                name: "Jaina".into(),
                class: 0,
                race: 1,
            }],
        };
        assert_eq!(round(&lobby), lobby);
        let start = StartRun {
            stage: 4,
            options: [1.0, 1.5, 2.0, 1.1],
        };
        assert_eq!(round(&start), start);
        for phase in [RunPhase::Over { won: true }, RunPhase::Lobby] {
            assert_eq!(round(&phase), phase);
        }
        let build = NetBuild {
            abilities: vec![("frostbolt".into(), 3)],
            passives: vec![("might".into(), 1)],
            bonuses,
            picks: 1,
            rerolls: 2,
            banishes: 0,
            banished: vec![WireStruck::Ability("blizzard".into())],
            overcharges: [0, 1, 0, 0, 3, 0],
            jewels: 1,
        };
        assert_eq!(round(&build), build);
        let hand = Hand {
            choices: vec![
                WireChoice::Heal,
                WireChoice::NewPassive("might".into()),
                WireChoice::Overcharge(2),
                WireChoice::Jewel(super::super::super::jewelry::pieces()[0].key.into()),
            ],
            build,
        };
        assert_eq!(round(&hand), hand);
        let run = NetRun {
            elapsed: 61.5,
            level: 4,
            xp: 12.0,
            gold: 30,
            banner: Some("Hogger approaches!".into()),
            ..Default::default()
        };
        assert_eq!(round(&run), run);
        // A camp's state rides the same record.
        let camp = NetRun {
            camp_left: 41.5,
            camp_ready: vec![0, 2],
            camp_fished: vec![1],
            fishing_dip: 2.9,
            camp_rolls: vec![(0, 73), (2, 12)],
            camp_roll_winner: Some(0),
            camp_fire: Some([10.0, 2.5, -7.0]),
            ..run
        };
        assert_eq!(round(&camp), camp);
        for action in [
            CampAction::Ready,
            CampAction::Fish,
            CampAction::Roll,
            CampAction::Buy(3),
        ] {
            assert_eq!(round(&action), action);
        }
        // The party UI's parts: lean, as they go to every client a few times a second.
        let auras = NetAuras(vec![1243, 1459, 19752]);
        assert_eq!(round(&auras), auras);
        assert!(size(&auras) <= 10);
        let buffs = NetBuffs(vec![(0, 1.0), (7, 1.4)]);
        assert_eq!(round(&buffs), buffs);
        assert!(size(&buffs) <= 11);
        let stats = NetStats {
            damage_done: 123_456.5,
            healing_done: 2_048.25,
            damage_taken: 900.0,
            revives: 2,
            gold: 310,
            downs: 1,
        };
        assert_eq!(round(&stats), stats);
        assert!(size(&stats) <= 30);
        let e = |i: u32| Ent(Entity::from_raw_u32(i).unwrap());
        let fx = NetFx::Go {
            caster: e(4),
            spell: 116,
            hits: vec![e(9), e(10)],
            dest: Some([1.0, 2.0, 3.0]),
        };
        assert_eq!(round(&fx), fx);
        assert_eq!(fx.entities().len(), 3);
        // A raid's forced move and yell, lean.
        let mut raid = HeroRaid::default();
        raid.knock(Vec3::NEG_Z, 12.0);
        let net = NetRaid::of(&raid);
        assert_eq!(round(&net), net);
        assert!(size(&net) <= 24);
        let yell = NetYell {
            text: "Lucifron goes into a berserker rage!".into(),
            sound: 0,
        };
        assert_eq!(round(&yell), yell);
    }

    #[test]
    fn fx_entities_map_to_the_clients() {
        struct Shift;
        impl EntityMapper for Shift {
            fn get_mapped(&mut self, e: Entity) -> Entity {
                Entity::from_raw_u32(e.index_u32() + 100).unwrap()
            }
            fn set_mapped(&mut self, _: Entity, _: Entity) {}
        }
        let e = |i: u32| Ent(Entity::from_raw_u32(i).unwrap());
        let mut fx = NetFx::Swing {
            attacker: e(1),
            victim: Some(e(2)),
            hit_info: 2,
            victim_state: 1,
            damage: 9,
        };
        fx.map_entities(&mut Shift);
        assert_eq!(
            fx.entities(),
            vec![
                Entity::from_raw_u32(101).unwrap(),
                Entity::from_raw_u32(102).unwrap()
            ]
        );
    }

    #[test]
    fn banished_keys_and_builds_resolve_on_the_other_side() {
        let s = Struck::Passive(data::PASSIVES[0].key);
        assert_eq!(round(&WireStruck::of(s)).struck(), Some(s));
        assert_eq!(WireStruck::Ability("gone".into()).struck(), None);
    }
}
