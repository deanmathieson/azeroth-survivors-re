//! Raid bosses: the boss-side mechanics library of `docs/RAIDS.md` section 3, and the scripts
//! built on it ([`bosses`]). The hero toolkit ([`super::mechanics`]) belongs to heroes: a hero's
//! trigger sets off its own effect. Here the boss's script picks a time and a hero, warns
//! everyone, then lands.
//!
//! A boss is data: a [`Script`] of [`BossAbility`] rows, each with its timer or health threshold
//! ([`When`]), whom it picks ([`Pick`]) and what it does ([`Does`]). The blocks:
//!
//! - **Ground telegraphs** (circle, cone, line, donut): a warning shape on the ground for the
//!   wind-up, then a [`Blow`] on every hero inside: damage, fear, knockback, and optionally a
//!   lingering pool ([`Telegraph`], [`Pending`], [`Pool`]).
//! - **Debuffs** on heroes ([`HeroRaid`]): a curse or magic payload that a healer class in range
//!   dispels on its own ([`dispels_of`]); a delayed blow (Impending Doom); a bomb that explodes on
//!   its fuse and hurts allies near the carrier (Living Bomb); mind control (held, hurting allies
//!   near it); and tank-swap stacks added by the boss's swing on its target.
//! - **Fear and knockback** are forced movement ([`ForcedMove`]): the host decides it, and the
//!   machine that walks the hero plays it out ([`Walk`]).
//! - **Add waves**, **phases** at a health threshold or a time, a **Frenzy** that a Hunter's
//!   Tranquilizing Shot removes, the **hard enrage**, and **yells** with their sound ([`Yelled`]).
//! - **Riders on a blow** ([`After`]): a silence, a stun, a slow or a strip of buffs; and a blow
//!   can hurt the boss's own side too (an Eruption chaining through the Firesworn).
//! - **Casts and auras on the boss's side**: a blink to a hero's sigil ([`Does::Blink`]), a
//!   pulsing nova while the boss stands rooted ([`Does::Inferno`]), an interruptible heal
//!   ([`Does::Mend`]), a haste for its allies ([`Does::Inspire`]), and the detonation of an add.
//! - **Deaths**: an add that erupts where it fell or enrages its master, and an add that can't
//!   die while its master stands ([`Master`], [`Undying`]).
//!
//! The host simulates every script and rolls every die; a co-op client draws the replicated
//! telegraphs and plays the forced moves of its own hero ([`super::coop`]).

mod bosses;
mod draw;
#[cfg(test)]
mod tests;

use benilla_world::collision::WorldCollision;
use benilla_world::schedule::WorldStage;
use benilla_world::world_point::WorldPoint;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::abilities::Hit;
use super::bonus::School;
use super::enemies::{self, Enemy, Rank};
use super::hero::{self, Hero, HERO_RADIUS};
use super::party;
use super::run_options::RunOptions;
use super::units::{self, flat, yaw_toward, Fx, Marks, COLOR_HURT};
use super::{HeroState, Rng, Run, RunEntity, Selection};

pub(crate) use bosses::script_for;

/// A healer class's dispel pulses this often, removing one debuff it can from an ally in range.
pub(crate) const DISPEL_EVERY: f32 = 4.0;
/// How far a dispel pulse reaches, in yards.
pub(crate) const DISPEL_RANGE: f32 = 15.0;
/// How far a Hunter's Tranquilizing Shot reaches a frenzied boss.
pub(crate) const TRANQ_RANGE: f32 = 30.0;
/// A Frenzy stands this long before a Hunter's shot takes it off, so it reads.
pub(crate) const TRANQ_DELAY: f32 = 1.0;
/// A Shaman's Tremor Totem keeps fear off heroes within this many yards of the Shaman.
pub(crate) const TREMOR_RANGE: f32 = 20.0;
/// Pools and damage over time land in ticks this long, so armour's flat block counts once a tick.
pub(crate) const TICK: f32 = 0.5;
/// A feared hero runs at this pace, in yards a second.
pub(crate) const FLEE_SPEED: f32 = 6.0;
/// A knockback throws the hero over this many seconds.
pub(crate) const KNOCK_SECS: f32 = 0.35;
/// The reach a boss's swing adds over the two bodies, for its tank-swap stacks.
const MELEE_SLACK: f32 = 1.5;

/// The `SoundEntries` kits a yell can sound, by name; yells travel as their place here.
pub(crate) const SOUNDS: &[&str] = &["RaidWarning"];

// Real 1.12 spells the library plays on its own (checked against `Spell.dbc` in the tests).
/// Remove Lesser Curse, the Mage's decurse.
pub(crate) const REMOVE_LESSER_CURSE: u32 = 475;
/// Remove Curse, the Druid's.
pub(crate) const REMOVE_CURSE: u32 = 2782;
/// Dispel Magic, the Priest's.
pub(crate) const DISPEL_MAGIC: u32 = 527;
/// Cleanse, the Paladin's.
pub(crate) const CLEANSE: u32 = 4987;
/// Tranquilizing Shot, the Hunter's answer to a Frenzy.
pub(crate) const TRANQUILIZING_SHOT: u32 = 19801;
/// Tremor Totem, the Shaman's answer to fear.
pub(crate) const TREMOR_TOTEM: u32 = 8143;
/// The burning ground a pool shows: Flamestrike's flames on the ground (Lava Bomb's own visual is
/// only its missile, which the landing plays).
pub(crate) const POOL_VISUAL: u32 = 2120;

/// The purple a countdown over a doomed head is drawn in, 0xAARRGGBB.
const COLOR_DOOM: u32 = 0xFFC0_60FF;
/// The orange of a warning ("SWAP!").
const COLOR_WARN: u32 = 0xFFFF_9020;
/// How many of a party-scaled add a party this size meets (RAIDS.md section 7: the Firesworn).
const ADD_COUNTS: [u8; 5] = [3, 4, 5, 6, 8];
/// The sigil a blink marks, in yards.
const SIGIL_RADIUS: f32 = 4.0;
/// The wind-up of the blow a blink lands with (Arcane Explosion as Shazzrah arrives).
const BLINK_BLOW_WINDUP: f32 = 1.5;
/// Below this share of its health an add that can't die heals to full.
const UNDYING_FLOOR: f32 = 0.5;
/// How far past the bomb's ring a bot carrying it runs from its leader, in yards.
const BOMB_MARGIN: f32 = 6.0;

pub(super) fn plugin(app: &mut App) {
    app.add_message::<Yelled>()
        .add_systems(
            Update,
            (
                note_final_boss,
                attach_scripts,
                run_bosses,
                land_blinks,
                land_telegraphs,
                burn_pools,
                tick_heroes,
                follow_bombs,
                guard_undying,
                on_deaths,
            )
                .chain()
                .after(enemies::move_enemies)
                .before(hero::hero_vitals)
                .run_if(super::strip::world_runs)
                .run_if(party::simulates)
                .run_if(|l: Res<crate::loading_screen::LoadingScreen>| !l.covering()),
        )
        .add_systems(
            Update,
            (equip_walk, play_yells, draw::draw_telegraphs).after(WorldStage::Input),
        );
}

// ---------------------------------------------------------------------------------------------
// The data a script is made of
// ---------------------------------------------------------------------------------------------

/// When an ability fires, on the fight's own clock (seconds since the boss walked out).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum When {
    /// First at `first`, then every `every` seconds.
    Every { first: f32, every: f32 },
    /// Once, this many seconds in.
    At(f32),
    /// Once, as the boss's health first falls below this fraction.
    Health(f32),
}

/// Whom an ability picks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Pick {
    /// The boss itself: a shape round or ahead of it, or a buff on it.
    Boss,
    /// The boss's current target: its tank.
    Tank,
    /// Up to this many heroes standing, at random; 0 is the raid's curse count for the party's
    /// size ([`curse_targets`]).
    Random(u8),
    /// As [`Pick::Random`], never the tank while anyone else stands.
    RandomNonTank(u8),
    /// Every hero standing within this many yards of the boss.
    Within(f32),
}

/// A ground telegraph's shape. Circles and donuts sit on their anchor; cones and lines start at
/// the boss and point along the telegraph's facing.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) enum Shape {
    Circle {
        radius: f32,
    },
    /// `half_angle` in radians either side of the facing.
    Cone {
        radius: f32,
        half_angle: f32,
    },
    Line {
        length: f32,
        width: f32,
    },
    Donut {
        inner: f32,
        outer: f32,
    },
}

impl Shape {
    /// Whether `p` stands inside the shape laid at `origin`, facing `yaw` (benilla's yaw, as
    /// [`units::yaw_toward`] gives it). Only the ground plane counts.
    pub(crate) fn contains(self, origin: Vec3, yaw: f32, p: Vec3) -> bool {
        let off = flat(p - origin);
        let d = off.length();
        let ahead = forward(yaw);
        match self {
            Shape::Circle { radius } => d <= radius,
            Shape::Donut { inner, outer } => d >= inner && d <= outer,
            Shape::Cone { radius, half_angle } => {
                if d > radius {
                    return false;
                }
                // At the apex the hero is in, whichever way it faces.
                d < 0.5 || ahead.angle_between(off) <= half_angle
            }
            Shape::Line { length, width } => {
                let along = off.dot(ahead);
                let side = off - ahead * along;
                (0.0..=length).contains(&along) && side.length() <= width / 2.0
            }
        }
    }

    /// The wind-up a readable shape needs at least (RAIDS.md section 3).
    #[cfg(test)]
    pub(crate) fn min_windup(self) -> f32 {
        match self {
            Shape::Circle { .. } => 1.5,
            Shape::Cone { .. } | Shape::Line { .. } => 2.0,
            Shape::Donut { .. } => 2.5,
        }
    }
}

/// The ground-plane direction a benilla yaw faces.
pub(crate) fn forward(yaw: f32) -> Vec3 {
    Quat::from_rotation_y(yaw) * Vec3::NEG_Z
}

/// A telegraph's colour, by what it does (RAIDS.md section 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Tone {
    /// Red: damage.
    Damage,
    /// Purple: a debuff or curse.
    Debuff,
    /// Yellow: knockback or fear.
    Control,
    /// Blue: a safe spot, or one to share.
    Safe,
    /// Orange: split damage.
    Split,
}

/// A pool a blow leaves burning on the ground.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PoolSpec {
    /// A fraction of a standing hero's maximum health each second.
    pub dps: f32,
    /// How long it burns; infinite never cools.
    pub secs: f32,
}

/// What else a blow leaves on each hero it lands on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum After {
    None,
    /// Its auto-casts stop for these seconds (Counterspell).
    Silence(f32),
    /// Held where it stands and silenced for these seconds (Hand of Ragnaros).
    Stun(f32),
    /// It walks at `rate` of its pace for `secs`, a magic debuff (Magma Shackles).
    Slow {
        rate: f32,
        secs: f32,
    },
    /// Its shields and timed buffs are stripped (Antimagic Pulse).
    Strip,
}

/// What lands on each hero inside a telegraph.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Blow {
    /// A fraction of the hero's maximum health.
    pub damage: f32,
    pub school: School,
    /// Seconds of fear: the hero runs away from the shape's origin.
    pub fear: f32,
    /// Yards the hero is thrown away from the shape's origin.
    pub knock: f32,
    pub pool: Option<PoolSpec>,
    pub after: After,
    /// A fraction of each enemy's own maximum health, to every foe of the heroes inside that is
    /// not a boss (an Eruption hurting the other Firesworn).
    pub foes: f32,
}

impl Blow {
    pub(crate) const fn hit(damage: f32, school: School) -> Self {
        Self {
            damage,
            school,
            fear: 0.0,
            knock: 0.0,
            pool: None,
            after: After::None,
            foes: 0.0,
        }
    }

    pub(crate) const fn then(self, after: After) -> Self {
        Self { after, ..self }
    }

    pub(crate) const fn foes(self, share: f32) -> Self {
        Self {
            foes: share,
            ..self
        }
    }

    pub(crate) const fn fear(secs: f32) -> Self {
        Self {
            fear: secs,
            ..Self::hit(0.0, School::Shadow)
        }
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "a library block the later Molten Core bosses use; tested here"
        )
    )]
    pub(crate) const fn knock(yards: f32, damage: f32) -> Self {
        Self {
            knock: yards,
            ..Self::hit(damage, School::Physical)
        }
    }

    pub(crate) const fn pool(self, dps: f32, secs: f32) -> Self {
        Self {
            pool: Some(PoolSpec { dps, secs }),
            ..self
        }
    }
}

/// Who may take a debuff off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Dispel {
    /// Nobody: it runs its course.
    None,
    /// The Mage's and the Druid's decurse.
    Curse,
    /// The Priest's and the Paladin's dispel.
    Magic,
}

/// What a debuff does while it lasts, or when it runs out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Payload {
    /// The hero's auto-casts come round at this rate (Lucifron's Curse: half).
    CooldownRate(f32),
    /// Run out undispelled, a blow of this fraction of maximum health (Impending Doom).
    Doom(f32),
    /// On the fuse, allies within `radius` of the carrier take `allies`, the carrier `carrier`,
    /// each a fraction of their own maximum health, and the foes there that are not bosses `foes`
    /// of theirs (Living Bomb: carry it into the horde).
    Bomb {
        radius: f32,
        allies: f32,
        carrier: f32,
        foes: f32,
    },
    /// Healing the hero receives is multiplied by this (Gehennas' Curse: a quarter).
    HealingTaken(f32),
    /// Magic damage the hero takes is multiplied by this (Shazzrah's Curse: double).
    DamageTaken(f32),
    /// Its auto-casts stop (Counterspell; a stun).
    Silence,
    /// It walks at this rate of its pace (Magma Shackles: half).
    Slow(f32),
    /// Mind control: the hero is held and casts nothing, and every second its allies within
    /// `radius` take `damage` of their maximum health (Dominate Mind).
    Dominate { radius: f32, damage: f32 },
    /// Tank-swap stacks: each burns `per` of maximum health a second; one falls off every `decay`
    /// seconds the boss's swing adds none; at `warn` the frame says swap (and, solo, they fall off).
    Stacks { per: f32, warn: u8, decay: f32 },
}

/// One ability of a boss.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Does {
    /// A ground telegraph per pick: `shape`, warned for `windup`, then `blow`.
    Ground {
        shape: Shape,
        windup: f32,
        blow: Blow,
        tone: Tone,
    },
    /// A debuff on each pick.
    Debuff {
        dispel: Dispel,
        secs: f32,
        payload: Payload,
    },
    /// `count` of the stage's add named `name`, round the boss; 0 is the party's count
    /// ([`add_count`]).
    Adds {
        name: &'static str,
        count: u8,
        radius: f32,
    },
    /// A sigil under each pick for `windup`; then the boss is there, its taunt dropped, and lands
    /// `blow` in a `radius` ring round itself as the spell `then` (Gate of Shazzrah).
    Blink {
        windup: f32,
        radius: f32,
        blow: Blow,
        then: u32,
    },
    /// Rooted, the boss glows for `windup`, then pulses `pulses` times, `every` seconds apart, in a
    /// `radius` ring round itself; each pulse hits for `damage` of maximum health, `growth` more
    /// for each pulse before it (Inferno).
    Inferno {
        radius: f32,
        windup: f32,
        pulses: u8,
        every: f32,
        damage: f32,
        growth: f32,
    },
    /// A heal cast for `cast` seconds, which any stun or fear on the caster interrupts; done, it
    /// heals the most hurt scripted ally within `range` (itself included) `share` of its maximum
    /// health (Dark Mending).
    Mend { share: f32, cast: f32, range: f32 },
    /// Every scripted ally within `range` (itself included) runs its script and its swing `haste`
    /// faster for `secs` (Inspire).
    Inspire { secs: f32, haste: f32, range: f32 },
    /// The living add of this name nearest the boss dies (Massive Eruption), setting off what its
    /// death does.
    Detonate { add: &'static str },
    /// A buff on the boss: `haste` more swings for `secs`; a Hunter's Tranquilizing Shot takes it
    /// off, and without a Hunter in the party it lasts half as long.
    Frenzy { secs: f32, haste: f32 },
    /// The hard enrage: damage and speed multiplied, for good.
    Berserk { damage: f32, speed: f32 },
    /// The fight moves to this phase: abilities of other phases stop, its own start.
    Phase(u8),
    /// Only the ability's yell.
    Yell,
}

/// A line a boss says, and its sound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Yell {
    /// `%s` is the boss's name, as in the cores' emote texts; empty for a sound alone.
    pub text: &'static str,
    /// A [`SOUNDS`] kit.
    pub sound: &'static str,
    /// A `SoundEntries` id played instead of `sound`, 0 for none: the creature's own voice from
    /// `CreatureSoundData` (its aggro roar, its death cry).
    pub kit: u32,
}

impl Yell {
    /// The creature's own voice kit, with `text` (empty for none).
    pub(crate) const fn voice(text: &'static str, kit: u32) -> Self {
        Self {
            text,
            sound: SOUNDS[0],
            kit,
        }
    }
}

/// One row of a boss script.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BossAbility {
    pub name: &'static str,
    /// The real 1.12 spell whose name, icon and visual it wears; 0 for none.
    pub spell: u32,
    pub when: When,
    pub pick: Pick,
    pub does: Does,
    pub yell: Option<Yell>,
    /// Off below this many heroes (RAIDS.md section 7).
    pub min_party: u8,
    /// The phases it runs in, phase `n` as bit `n - 1`; 0 is every phase.
    pub phases: u8,
}

impl BossAbility {
    pub(crate) const fn new(
        name: &'static str,
        spell: u32,
        when: When,
        pick: Pick,
        does: Does,
    ) -> Self {
        Self {
            name,
            spell,
            when,
            pick,
            does,
            yell: None,
            min_party: 1,
            phases: 0,
        }
    }

    pub(crate) const fn yell(self, text: &'static str, sound: &'static str) -> Self {
        Self {
            yell: Some(Yell {
                text,
                sound,
                kit: 0,
            }),
            ..self
        }
    }

    /// The creature's own voice kit as its yell ([`Yell::voice`]).
    pub(crate) const fn voice(self, text: &'static str, kit: u32) -> Self {
        Self {
            yell: Some(Yell::voice(text, kit)),
            ..self
        }
    }

    pub(crate) const fn min_party(self, n: u8) -> Self {
        Self {
            min_party: n,
            ..self
        }
    }

    pub(crate) const fn in_phases(self, mask: u8) -> Self {
        Self {
            phases: mask,
            ..self
        }
    }

    fn runs_in(&self, phase: u8) -> bool {
        self.phases == 0 || (phase >= 1 && self.phases & (1 << (phase - 1)) != 0)
    }
}

/// A boss, as data.
#[derive(Debug)]
pub(crate) struct Script {
    pub name: &'static str,
    pub abilities: &'static [BossAbility],
    /// Said as it falls.
    pub death: Option<Yell>,
    /// What its fall leaves where it fell (a Firesworn's Eruption).
    pub erupt: Option<Erupt>,
    /// The boss it serves, and how its fall or its life is tied to it.
    pub master: Option<Master>,
}

impl Script {
    /// A script of these abilities and nothing more.
    pub(crate) const fn of(name: &'static str, abilities: &'static [BossAbility]) -> Self {
        Self {
            name,
            abilities,
            death: None,
            erupt: None,
            master: None,
        }
    }
}

/// A ground blow an add's fall sets off where it fell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Erupt {
    pub spell: u32,
    pub shape: Shape,
    pub windup: f32,
    pub blow: Blow,
    pub tone: Tone,
}

/// An add's tie to its master.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Master {
    pub name: &'static str,
    /// As the add falls its master gains a stack of this spell: `per` more damage each.
    pub enrage: Option<(u32, f32)>,
    /// While its master stands the add can't die: below half health it heals to full, and it
    /// falls with its master ([`Undying`]).
    pub undying: bool,
    /// The emote as it refuses to die, `%s` its name.
    pub refuses: Option<Yell>,
}

/// How many heroes a curse picks for a party this size (RAIDS.md section 7).
pub(crate) fn curse_targets(party: u8) -> usize {
    match party {
        0..=2 => 1,
        3 | 4 => 2,
        _ => 3,
    }
}

/// How many of a party-scaled add come for a party this size (RAIDS.md section 7: Firesworn 3, 4,
/// 5, 6, 8).
pub(crate) fn add_count(party: u8) -> u8 {
    ADD_COUNTS[usize::from(party.clamp(1, 5)) - 1]
}

// ---------------------------------------------------------------------------------------------
// The boss's brain
// ---------------------------------------------------------------------------------------------

/// A scripted boss's fight: its clock, each ability's next time, its phase and its buffs.
#[derive(Component, Debug)]
pub(crate) struct RaidBoss {
    pub script: &'static Script,
    /// Seconds since it walked out.
    pub clock: f32,
    next: Vec<f32>,
    done: Vec<bool>,
    pub phase: u8,
    /// Seconds of Frenzy left, its haste, and how long it has stood.
    pub frenzy_t: f32,
    pub frenzy_haste: f32,
    pub frenzy_age: f32,
    pub berserk: bool,
    /// The heal it is casting, if any.
    pub cast: Option<Casting>,
    /// Seconds of an ally's Inspire left, and its haste.
    pub inspired_t: f32,
    pub inspire_haste: f32,
}

/// A heal on its cast bar ([`Does::Mend`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Casting {
    pub spell: u32,
    /// The run second it is done.
    pub done: f32,
    pub share: f32,
    pub range: f32,
    /// Its ring on the ground, which fills as the cast does.
    pub ring: Entity,
}

/// Where a cast stands this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CastState {
    Casting,
    /// A stun or fear on the caster broke it.
    Interrupted,
    Done,
}

/// The cast `c` at run second `now`, its caster `stunned` (or feared) or not.
pub(crate) fn cast_state(c: &Casting, now: f32, stunned: bool) -> CastState {
    if stunned {
        CastState::Interrupted
    } else if now >= c.done {
        CastState::Done
    } else {
        CastState::Casting
    }
}

impl RaidBoss {
    pub(crate) fn new(script: &'static Script) -> Self {
        Self {
            script,
            clock: 0.0,
            next: script
                .abilities
                .iter()
                .map(|a| match a.when {
                    When::Every { first, .. } => first,
                    When::At(secs) => secs,
                    When::Health(_) => 0.0,
                })
                .collect(),
            done: vec![false; script.abilities.len()],
            phase: 1,
            frenzy_t: 0.0,
            frenzy_haste: 0.0,
            frenzy_age: 0.0,
            berserk: false,
            cast: None,
            inspired_t: 0.0,
            inspire_haste: 0.0,
        }
    }

    /// The clock on by `dt` with the boss at `health` (a fraction) and `party` heroes: the
    /// abilities due now, in script order. A phase change takes effect at once, for the rows after
    /// it; a timed row whose phase is off still keeps its beat.
    pub(crate) fn due(&mut self, dt: f32, health: f32, party: u8) -> Vec<&'static BossAbility> {
        self.clock += dt;
        let mut out = Vec::new();
        for (i, a) in self.script.abilities.iter().enumerate() {
            let ready = match a.when {
                When::Every { every, .. } => {
                    let ready = self.clock >= self.next[i];
                    if ready {
                        self.next[i] += every.max(0.1);
                    }
                    ready
                }
                When::At(secs) => {
                    let ready = !self.done[i] && self.clock >= secs;
                    self.done[i] |= ready;
                    ready
                }
                When::Health(below) => {
                    let ready = !self.done[i] && health < below;
                    self.done[i] |= ready;
                    ready
                }
            };
            if ready && a.runs_in(self.phase) && party >= a.min_party {
                if let Does::Phase(p) = a.does {
                    self.phase = p;
                }
                out.push(a);
            }
        }
        out
    }
}

/// One hero as a boss sees it this frame.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Seen {
    pub e: Entity,
    pub pos: Vec3,
    pub up: bool,
    pub threat: f32,
}

/// The boss's target: its taunter while the taunt holds, else [`party::pick_target`]'s pick.
pub(crate) fn boss_target(
    pos: Vec3,
    taunted: Option<(Entity, f32)>,
    now: f32,
    seen: &[Seen],
) -> Option<usize> {
    let up = |h: &&Seen| h.up;
    let forced = taunted
        .filter(|&(_, until)| until > now)
        .and_then(|(hero, _)| seen.iter().position(|h| h.e == hero && h.up));
    forced.or_else(|| {
        let e = party::pick_target(pos, seen.iter().filter(up).map(|h| (h.e, h.pos, h.threat)))?;
        seen.iter().position(|h| h.e == e)
    })
}

/// The heroes `pick` chooses, as places in `seen`.
pub(crate) fn pick(
    pick: Pick,
    boss: Vec3,
    tank: Option<usize>,
    seen: &[Seen],
    party: u8,
    rng: &mut Rng,
) -> Vec<usize> {
    let up: Vec<usize> = (0..seen.len()).filter(|&i| seen[i].up).collect();
    let random = |mut pool: Vec<usize>, n: usize, rng: &mut Rng| {
        let mut out = Vec::new();
        while out.len() < n && !pool.is_empty() {
            out.push(pool.swap_remove(rng.index(pool.len())));
        }
        out
    };
    match pick {
        Pick::Boss => Vec::new(),
        Pick::Tank => tank.into_iter().collect(),
        Pick::Random(n) => {
            let n = if n == 0 {
                curse_targets(party)
            } else {
                usize::from(n)
            };
            random(up, n, rng)
        }
        Pick::RandomNonTank(n) => {
            let others: Vec<usize> = up.iter().copied().filter(|&i| Some(i) != tank).collect();
            let pool = if others.is_empty() { up } else { others };
            random(pool, usize::from(n.max(1)), rng)
        }
        Pick::Within(r) => up
            .into_iter()
            .filter(|&i| hero::reach(boss, seen[i].pos) <= r)
            .collect(),
    }
}

// ---------------------------------------------------------------------------------------------
// Telegraphs and pools
// ---------------------------------------------------------------------------------------------

/// A warning on the ground, as every machine draws it: replicated in co-op. Times are on the
/// run's clock ([`Run::elapsed`]).
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Telegraph {
    pub shape: Shape,
    pub at: [f32; 3],
    pub yaw: f32,
    pub tone: Tone,
    pub start: f32,
    /// When the blow lands; the fill has grown to the edge.
    pub land: f32,
    /// When it is gone: the landing for a plain blow, later for a pool.
    pub until: f32,
}

impl Telegraph {
    pub(crate) fn origin(&self) -> Vec3 {
        Vec3::from_array(self.at)
    }

    /// The fill's share of the wind-up at run second `now`, 0 to 1.
    pub(crate) fn progress(&self, now: f32) -> f32 {
        let span = (self.land - self.start).max(1e-3);
        ((now - self.start) / span).clamp(0.0, 1.0)
    }

    pub(crate) fn contains(&self, p: Vec3) -> bool {
        self.shape.contains(self.origin(), self.yaw, p)
    }
}

/// On a telegraph still winding up: what lands, from whom (host only).
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct Pending {
    pub source: Entity,
    pub spell: u32,
    pub blow: Blow,
}

/// On a telegraph that left burning ground: the boss that left it (its pools cool as it falls), its
/// burn, the heroes' next ticks, and the spell's area visual (host only).
#[derive(Component, Debug)]
pub(crate) struct Pool {
    pub source: Entity,
    pub dps: f32,
    pub tick: f32,
    pub visual: Option<Entity>,
}

/// On a blink's sigil: the boss that arrives on it, and the blow it lands with (host only).
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct BlinkTo {
    pub boss: Entity,
    pub spell: u32,
    pub radius: f32,
    pub blow: Blow,
    pub then: u32,
}

/// On a bomb's ring: the hero carrying it, which it follows until the fuse (host only; its moves
/// replicate as the telegraph's).
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct Carried {
    pub hero: Entity,
    pub spell: u32,
}

/// On an add that can't die while its master stands ([`Master::undying`]): a killing blow leaves
/// it standing ([`super::abilities::apply_hits`]).
#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct Undying;

// ---------------------------------------------------------------------------------------------
// Heroes: debuffs and forced movement
// ---------------------------------------------------------------------------------------------

/// A debuff on a hero.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Debuff {
    pub spell: u32,
    pub dispel: Dispel,
    pub left: f32,
    pub payload: Payload,
    pub stacks: u8,
    /// Seconds since its last tick, and since its last stack was added.
    tick: f32,
    idle: f32,
    /// The stack count its last swap warning was for.
    warned: u8,
}

impl Debuff {
    pub(crate) fn new(spell: u32, dispel: Dispel, secs: f32, payload: Payload) -> Self {
        Self {
            spell,
            dispel,
            left: secs,
            payload,
            stacks: 1,
            tick: 0.0,
            idle: 0.0,
            warned: 0,
        }
    }
}

/// How a hero is moved for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum MoveKind {
    /// Running in fear.
    Flee,
    /// Thrown.
    Knock,
    /// Held where it stands.
    Hold,
}

/// A forced movement: the host decides it, the machine that walks the hero plays it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ForcedMove {
    pub kind: MoveKind,
    /// The ground direction, `[x, z]`, unit length (zero for a hold).
    pub dir: [f32; 2],
    pub speed: f32,
    pub secs: f32,
}

impl ForcedMove {
    pub(crate) fn velocity(&self) -> Vec3 {
        Vec3::new(self.dir[0], 0.0, self.dir[1]) * self.speed
    }
}

/// Something a hero's debuffs set off this tick, for the system to apply.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum RaidEvent {
    /// The hero itself takes this fraction of its maximum health.
    Hit { spell: u32, damage: f32 },
    /// Its allies within `radius` take this fraction of theirs.
    Splash {
        spell: u32,
        radius: f32,
        damage: f32,
    },
    /// The foes within `radius` that are not bosses take this fraction of theirs.
    Blast {
        spell: u32,
        radius: f32,
        damage: f32,
    },
    /// A fuse's whole seconds left, shown over the head.
    Count(u32),
    /// Tank-swap stacks reached their warning.
    Swap,
}

/// A hero's part in a raid fight: its debuffs, fear, and the forced moves the host sent it.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct HeroRaid {
    pub debuffs: Vec<Debuff>,
    pub fear_t: f32,
    /// Bumped by every forced move, so the walking machine plays each once.
    pub moves: u16,
    pub last_move: Option<ForcedMove>,
    /// Seconds of fear or mind control left: the walker stops early once this reaches 0.
    pub forced_left: f32,
    /// This hero's own dispel pulse, if its class has one.
    pub dispel_cd: f32,
    /// The share of its pace a slow takes off, 0 for none: kept from its debuffs on the host and
    /// sent to the machine that walks it ([`HeroRaid::move_rate`]).
    pub slow: f32,
}

impl HeroRaid {
    /// Put `d` on the hero: a debuff already there is refreshed, and stacks add one.
    pub(crate) fn add(&mut self, d: Debuff) {
        if let Some(have) = self.debuffs.iter_mut().find(|h| h.spell == d.spell) {
            have.left = have.left.max(d.left);
            if let Payload::Stacks { .. } = have.payload {
                have.stacks = have.stacks.saturating_add(1);
                have.idle = 0.0;
            }
            return;
        }
        if let Payload::Dominate { .. } = d.payload {
            self.hold(d.left);
        }
        self.debuffs.push(d);
        self.slow = self.slow_of();
    }

    /// The slowest slow on it, as the share of its pace it takes off.
    fn slow_of(&self) -> f32 {
        self.debuffs
            .iter()
            .filter_map(|d| match d.payload {
                Payload::Slow(rate) => Some(1.0 - rate),
                _ => None,
            })
            .fold(0.0, f32::max)
            .clamp(0.0, 0.9)
    }

    /// The rate the hero walks at: 1 unslowed.
    pub(crate) fn move_rate(&self) -> f32 {
        1.0 - self.slow.clamp(0.0, 0.9)
    }

    /// The rate the hero receives healing at: the harshest curse on it.
    pub(crate) fn healing_rate(&self) -> f32 {
        self.debuffs
            .iter()
            .filter_map(|d| match d.payload {
                Payload::HealingTaken(r) => Some(r),
                _ => None,
            })
            .fold(1.0, f32::min)
    }

    /// What magic damage it takes is multiplied by: the harshest curse on it.
    pub(crate) fn magic_taken(&self) -> f32 {
        self.debuffs
            .iter()
            .filter_map(|d| match d.payload {
                Payload::DamageTaken(m) => Some(m),
                _ => None,
            })
            .fold(1.0, f32::max)
    }

    /// Counterspelled or stunned: its auto-casts wait.
    pub(crate) fn silenced(&self) -> bool {
        self.debuffs
            .iter()
            .any(|d| matches!(d.payload, Payload::Silence))
    }

    /// A bomb it carries: its fuse's seconds left and its ring's radius.
    pub(crate) fn bomb(&self) -> Option<(f32, f32)> {
        self.debuffs.iter().find_map(|d| match d.payload {
            Payload::Bomb { radius, .. } => Some((d.left, radius)),
            _ => None,
        })
    }

    /// The rate the hero's auto-cast cooldowns come round at: the slowest curse on it.
    pub(crate) fn cooldown_rate(&self) -> f32 {
        self.debuffs
            .iter()
            .filter_map(|d| match d.payload {
                Payload::CooldownRate(r) => Some(r),
                _ => None,
            })
            .fold(1.0, f32::min)
    }

    pub(crate) fn dominated(&self) -> bool {
        self.debuffs
            .iter()
            .any(|d| matches!(d.payload, Payload::Dominate { .. }))
    }

    /// Feared, mind-controlled or silenced, a hero's auto-casts wait.
    pub(crate) fn can_cast(&self) -> bool {
        self.fear_t <= 0.0 && !self.dominated() && !self.silenced()
    }

    fn push_move(&mut self, m: ForcedMove) {
        self.moves = self.moves.wrapping_add(1);
        self.last_move = Some(m);
    }

    /// Run in fear along `dir` for `secs`.
    pub(crate) fn fear(&mut self, dir: Vec3, secs: f32) {
        let dir = flat(dir).normalize_or(Vec3::X);
        self.fear_t = self.fear_t.max(secs);
        self.forced_left = self.forced_left.max(secs);
        self.push_move(ForcedMove {
            kind: MoveKind::Flee,
            dir: [dir.x, dir.z],
            speed: FLEE_SPEED,
            secs,
        });
    }

    /// Thrown `yards` along `dir`.
    pub(crate) fn knock(&mut self, dir: Vec3, yards: f32) {
        let dir = flat(dir).normalize_or(Vec3::X);
        self.push_move(ForcedMove {
            kind: MoveKind::Knock,
            dir: [dir.x, dir.z],
            speed: yards / KNOCK_SECS,
            secs: KNOCK_SECS,
        });
    }

    /// Held where it stands for `secs`.
    pub(crate) fn hold(&mut self, secs: f32) {
        self.forced_left = self.forced_left.max(secs);
        self.push_move(ForcedMove {
            kind: MoveKind::Hold,
            dir: [0.0, 0.0],
            speed: 0.0,
            secs,
        });
    }

    /// Take off the most urgent debuff `kind` may (the soonest to run out); what came off.
    pub(crate) fn dispel(&mut self, kind: Dispel) -> Option<Debuff> {
        if kind == Dispel::None {
            return None;
        }
        let i = self
            .debuffs
            .iter()
            .enumerate()
            .filter(|(_, d)| d.dispel == kind)
            .min_by(|a, b| a.1.left.total_cmp(&b.1.left))
            .map(|(i, _)| i)?;
        let gone = self.debuffs.remove(i);
        if let Payload::Dominate { .. } = gone.payload {
            self.forced_left = self.fear_t.max(0.0);
        }
        self.slow = self.slow_of();
        Some(gone)
    }

    /// The soonest-ending debuff `kind` may take off, if any: how urgent this hero is.
    pub(crate) fn urgency(&self, kind: Dispel) -> Option<f32> {
        self.debuffs
            .iter()
            .filter(|d| kind != Dispel::None && d.dispel == kind)
            .map(|d| d.left)
            .min_by(f32::total_cmp)
    }

    /// Everything off, as the hero falls or the run ends.
    pub(crate) fn clear(&mut self) {
        self.debuffs.clear();
        self.fear_t = 0.0;
        self.slow = 0.0;
        if self.forced_left > 0.0 {
            self.forced_left = 0.0;
        }
    }

    /// The debuffs' spells, for the hero's aura slots.
    pub(crate) fn spells(&self) -> impl Iterator<Item = u32> + '_ {
        self.debuffs.iter().map(|d| d.spell).filter(|&s| s != 0)
    }

    /// The tank-swap stacks of `spell` on the hero.
    #[cfg(test)]
    pub(crate) fn stacks(&self, spell: u32) -> u8 {
        self.debuffs
            .iter()
            .find(|d| d.spell == spell)
            .map_or(0, |d| d.stacks)
    }

    /// The timers on by `dt`: fuses run out, mind control pulses, stacks burn and fall off.
    /// `solo` drops tank-swap stacks at their warning, as RAIDS.md section 7 has it for one hero.
    pub(crate) fn tick(&mut self, dt: f32, solo: bool) -> Vec<RaidEvent> {
        let mut out = Vec::new();
        self.fear_t = (self.fear_t - dt).max(0.0);
        let mut i = 0;
        while i < self.debuffs.len() {
            let d = &mut self.debuffs[i];
            let before = d.left;
            d.left -= dt;
            d.tick += dt;
            d.idle += dt;
            let mut gone = d.left <= 0.0;
            match d.payload {
                Payload::Doom(damage) => {
                    if gone {
                        out.push(RaidEvent::Hit {
                            spell: d.spell,
                            damage,
                        });
                    } else if before.ceil() > d.left.ceil() {
                        out.push(RaidEvent::Count(d.left.ceil() as u32));
                    }
                }
                Payload::Bomb {
                    radius,
                    allies,
                    carrier,
                    foes,
                } => {
                    if gone {
                        out.push(RaidEvent::Hit {
                            spell: d.spell,
                            damage: carrier,
                        });
                        out.push(RaidEvent::Splash {
                            spell: d.spell,
                            radius,
                            damage: allies,
                        });
                        if foes > 0.0 {
                            out.push(RaidEvent::Blast {
                                spell: d.spell,
                                radius,
                                damage: foes,
                            });
                        }
                    } else if before.ceil() > d.left.ceil() {
                        out.push(RaidEvent::Count(d.left.ceil() as u32));
                    }
                }
                Payload::Dominate { radius, damage } => {
                    while d.tick >= 1.0 {
                        d.tick -= 1.0;
                        out.push(RaidEvent::Splash {
                            spell: d.spell,
                            radius,
                            damage,
                        });
                    }
                }
                Payload::Stacks { per, warn, decay } => {
                    while d.tick >= TICK {
                        d.tick -= TICK;
                        out.push(RaidEvent::Hit {
                            spell: d.spell,
                            damage: per * f32::from(d.stacks) * TICK,
                        });
                    }
                    if d.idle >= decay {
                        d.idle -= decay;
                        d.stacks = d.stacks.saturating_sub(1);
                    }
                    if d.stacks < warn {
                        d.warned = 0;
                    } else if d.stacks != d.warned {
                        d.warned = d.stacks;
                        out.push(RaidEvent::Swap);
                        if solo {
                            d.stacks = 0;
                        }
                    }
                    gone |= d.stacks == 0;
                }
                Payload::CooldownRate(_)
                | Payload::HealingTaken(_)
                | Payload::DamageTaken(_)
                | Payload::Silence
                | Payload::Slow(_) => {}
            }
            if gone {
                self.debuffs.remove(i);
            } else {
                i += 1;
            }
        }
        self.slow = self.slow_of();
        let held = self
            .debuffs
            .iter()
            .filter(|d| matches!(d.payload, Payload::Dominate { .. }))
            .map(|d| d.left)
            .fold(0.0, f32::max);
        self.forced_left = self.fear_t.max(held);
        out
    }
}

/// On every hero: the forced move this machine is playing for it. Only the machine that walks the
/// hero reads it: its own player's, or the host's bots.
#[derive(Component, Default, Debug)]
pub(crate) struct Walk {
    seen: u16,
    now: Option<(ForcedMove, f32)>,
}

impl Walk {
    /// The velocity a forced move holds the hero to this frame, `None` when it walks freely. A new
    /// move in `raid` starts at once; one that `raid` ended early (a dispelled mind control) stops
    /// once it has had time to hear of it.
    pub(crate) fn step(&mut self, raid: &HeroRaid, dt: f32) -> Option<Vec3> {
        if raid.moves != self.seen {
            self.seen = raid.moves;
            self.now = raid.last_move.map(|m| (m, 0.0));
        }
        let (m, t) = self.now.as_mut()?;
        *t += dt;
        let early = *t > 0.5 && m.kind != MoveKind::Knock && raid.forced_left <= 0.0;
        if *t > m.secs || early {
            self.now = None;
            return None;
        }
        Some(m.velocity())
    }

    #[cfg(test)]
    pub(crate) fn kind(&self) -> Option<MoveKind> {
        self.now.map(|(m, _)| m.kind)
    }
}

/// Every hero gets its [`Walk`].
fn equip_walk(mut commands: Commands, heroes: Query<Entity, (With<Hero>, Without<Walk>)>) {
    for e in &heroes {
        commands.entity(e).try_insert(Walk::default());
    }
}

/// Which debuffs a class takes off its allies, and the spell it shows doing it (RAIDS.md section
/// 1: curses are the Mage's and Druid's, magic the Priest's and Paladin's).
pub(crate) fn dispels_of(class: u8) -> Option<(Dispel, u32)> {
    match class {
        8 => Some((Dispel::Curse, REMOVE_LESSER_CURSE)),
        11 => Some((Dispel::Curse, REMOVE_CURSE)),
        5 => Some((Dispel::Magic, DISPEL_MAGIC)),
        2 => Some((Dispel::Magic, CLEANSE)),
        _ => None,
    }
}

/// Whether a hero shrugs off fear: a Warrior's Berserker Rage, an Undead's Will of the Forsaken,
/// or a Shaman's Tremor Totem within [`TREMOR_RANGE`] (`shamans` are where Shamans stand).
pub(crate) fn fear_immune(class: u8, race: u8, pos: Vec3, shamans: &[Vec3]) -> bool {
    class == 1 || race == 5 || shamans.iter().any(|&s| hero::reach(pos, s) <= TREMOR_RANGE)
}

/// Among `afflicted` (each hero's position and its soonest dispellable debuff, if any), the one a
/// dispeller at `at` takes: the most urgent within [`DISPEL_RANGE`].
pub(crate) fn dispel_target(at: Vec3, afflicted: &[(Vec3, Option<f32>)]) -> Option<usize> {
    afflicted
        .iter()
        .enumerate()
        .filter(|(_, (p, left))| left.is_some() && hero::reach(at, *p) <= DISPEL_RANGE)
        .min_by(|a, b| a.1 .1.unwrap().total_cmp(&b.1 .1.unwrap()))
        .map(|(i, _)| i)
}

/// Where a bot carrying a bomb of `radius` heads: straight away from its party's `leader`, past the
/// ring's edge, so its fuse catches nobody (the iconic Living Bomb run).
pub(crate) fn bomb_escape(me: Vec3, leader: Vec3, radius: f32) -> Vec3 {
    let away = flat(me - leader).normalize_or(Vec3::X);
    leader + away * (radius + BOMB_MARGIN)
}

/// Among `allies` (each one's entity, position and share of its health), the one a heal from `at`
/// reaching `range` takes: the most hurt in reach, if any is hurt.
pub(crate) fn mend_target(at: Vec3, range: f32, allies: &[(Entity, Vec3, f32)]) -> Option<Entity> {
    allies
        .iter()
        .filter(|(_, p, health)| *health < 1.0 && hero::reach(at, *p) <= range)
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(e, ..)| *e)
}

/// An Inferno's pulses as seconds from its start: each one's warning, its landing, and its share of
/// maximum health. The first warns over the glow, each later one over the beat before it.
pub(crate) fn inferno_beats(
    windup: f32,
    pulses: u8,
    every: f32,
    damage: f32,
    growth: f32,
) -> Vec<(f32, f32, f32)> {
    (0..pulses)
        .map(|k| {
            let k = f32::from(k);
            let land = windup + every * k;
            let warn = if k == 0.0 { 0.0 } else { land - every };
            (warn, land, damage * (1.0 + growth * k))
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Yells
// ---------------------------------------------------------------------------------------------

/// A boss said something: every machine shows it and sounds its kit.
#[derive(Message, Clone, Debug, PartialEq)]
pub(crate) struct Yelled {
    pub text: String,
    /// A place in [`SOUNDS`].
    pub sound: u8,
    /// A `SoundEntries` id sounded instead, 0 for none ([`Yell::kit`]).
    pub kit: u32,
}

impl Yelled {
    pub(crate) fn of(yell: &Yell, boss: &str) -> Self {
        Self {
            text: yell.text.replace("%s", boss),
            sound: SOUNDS.iter().position(|s| *s == yell.sound).unwrap_or(0) as u8,
            kit: yell.kit,
        }
    }
}

fn play_yells(
    mut yells: MessageReader<Yelled>,
    mut run: ResMut<Run>,
    mut cues: MessageWriter<crate::sound::SoundCue>,
    mut voices: Option<MessageWriter<crate::sound::KitCue>>,
) {
    for y in yells.read() {
        if !y.text.is_empty() {
            run.banner = Some((y.text.clone(), 4.0));
        }
        if y.kit != 0 {
            if let Some(voices) = voices.as_mut() {
                voices.write(crate::sound::KitCue {
                    kit: y.kit,
                    at: None,
                });
            }
        } else if let Some(kit) = SOUNDS.get(usize::from(y.sound)) {
            cues.write(crate::sound::SoundCue::flat(kit));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The host's systems
// ---------------------------------------------------------------------------------------------

/// The stage's last roster boss falling is noted on the run: on a raid stage that wins it
/// ([`super::data::won`], read by [`hero::hero_vitals`]).
fn note_final_boss(
    mut kills: MessageReader<enemies::Loot>,
    mut run: ResMut<Run>,
    selection: Res<Selection>,
) {
    let last = super::data::roster(selection.stage)
        .bosses
        .last()
        .map(|b| b.name);
    for kill in kills.read() {
        if kill.boss && Some(kill.def.name) == last {
            run.final_boss = true;
        }
    }
}

/// A boss or add whose name has a script gets its brain.
fn attach_scripts(mut commands: Commands, fresh: Query<(Entity, &Enemy), Without<RaidBoss>>) {
    for (e, enemy) in &fresh {
        if enemy.dead {
            continue;
        }
        if let Some(script) = script_for(enemy.def.name) {
            commands.entity(e).try_insert(RaidBoss::new(script));
        }
    }
}

fn seen_of(heroes: &Query<(Entity, &Transform, &mut HeroState), With<Hero>>) -> Vec<Seen> {
    heroes
        .iter()
        .map(|(e, tf, h)| Seen {
            e,
            pos: tf.translation,
            up: h.alive(),
            threat: h.threat,
        })
        .collect()
}

/// One scripted foe as the others see it this frame: for heals, Inspire and detonations.
#[derive(Clone, Copy, Debug)]
struct Ally {
    e: Entity,
    pos: Vec3,
    hp: f32,
    max_hp: f32,
    name: &'static str,
}

/// Every scripted boss's clock: what is due fires, Frenzy runs and Hunters shoot it off, heals
/// finish or are interrupted.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn run_bosses(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    selection: Res<Selection>,
    opts: Res<RunOptions>,
    mut spawner: ResMut<enemies::Spawner>,
    collide: Option<WorldCollision>,
    world: WorldPoint,
    mut bosses: Query<(Entity, &Transform, &mut Enemy, &mut RaidBoss, &mut Marks), Without<Hero>>,
    mut heroes: Query<(Entity, &Transform, &mut HeroState), With<Hero>>,
    mut fx: Fx,
    mut yells: MessageWriter<Yelled>,
    mut hits: MessageWriter<Hit>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let seen = seen_of(&heroes);
    if seen.iter().all(|h| !h.up) {
        return;
    }
    let now = run.elapsed;
    let party = run.party.max(1);
    let hunters: Vec<(Entity, Vec3)> = heroes
        .iter()
        .filter(|(_, _, h)| h.alive() && h.class.id == 3)
        .map(|(e, tf, _)| (e, tf.translation))
        .collect();
    let allies: Vec<Ally> = bosses
        .iter()
        .filter(|(_, _, en, ..)| !en.dead)
        .map(|(e, tf, en, ..)| Ally {
            e,
            pos: tf.translation,
            hp: en.hp,
            max_hp: en.max_hp.max(1.0),
            name: en.def.name,
        })
        .collect();
    let mut adds = Vec::new();
    let mut heals: Vec<(Entity, f32)> = Vec::new();
    let mut inspires: Vec<(Vec3, u32, f32, f32, f32)> = Vec::new();
    let mut detonations: Vec<(Entity, Vec3, &'static str)> = Vec::new();
    for (boss, tf, mut enemy, mut brain, mut marks) in &mut bosses {
        if enemy.dead {
            if let Some(c) = brain.cast.take() {
                commands.entity(c.ring).try_despawn();
            }
            continue;
        }
        let pos = tf.translation;
        let name = brain.script.name;

        // A Frenzy hastens the swing; a Hunter in range shoots it off once it has stood a moment.
        if brain.frenzy_t > 0.0 {
            brain.frenzy_t -= dt;
            brain.frenzy_age += dt;
            enemy.attack_cd -= dt * brain.frenzy_haste;
            let shot = hunters
                .iter()
                .find(|(_, at)| hero::reach(pos, *at) <= TRANQ_RANGE)
                .filter(|_| brain.frenzy_age >= TRANQ_DELAY);
            if let Some(&(hunter, _)) = shot {
                brain.frenzy_t = 0.0;
                fx.cast(hunter, TRANQUILIZING_SHOT, &[boss], None);
                fx.text(boss, "Tranquilized".into(), COLOR_WARN);
            }
            if brain.frenzy_t <= 0.0 {
                brain.frenzy_t = 0.0;
                marks.remove(frenzy_spell(brain.script));
            }
        }

        // An ally's Inspire runs its script and its swing faster.
        let mut beat = dt;
        if brain.inspired_t > 0.0 {
            brain.inspired_t -= dt;
            beat *= 1.0 + brain.inspire_haste;
            enemy.attack_cd -= dt * brain.inspire_haste;
        }

        // A heal on the cast bar: a stun or fear breaks it; done, it lands on the most hurt.
        if let Some(c) = brain.cast {
            let stunned = enemy.stun_t > 0.0 || enemy.fear_t > 0.0;
            let state = cast_state(&c, now, stunned);
            if state == CastState::Interrupted {
                brain.cast = None;
                commands.entity(c.ring).try_despawn();
                fx.fail(boss, c.spell);
                fx.text(boss, "Interrupted".into(), COLOR_WARN);
            } else if state == CastState::Done {
                brain.cast = None;
                commands.entity(c.ring).try_despawn();
                let near: Vec<(Entity, Vec3, f32)> = allies
                    .iter()
                    .map(|a| (a.e, a.pos, a.hp / a.max_hp))
                    .collect();
                if let Some(target) = mend_target(pos, c.range, &near) {
                    heals.push((target, c.share));
                    fx.cast(boss, c.spell, &[target], None);
                }
            }
        }

        let health = enemy.hp / enemy.max_hp.max(1.0);
        let tank = boss_target(pos, enemy.taunted, now, &seen);
        for a in brain.due(beat, health, party) {
            if let Some(y) = &a.yell {
                yells.write(Yelled::of(y, name));
            }
            let picked = pick(a.pick, pos, tank, &seen, party, &mut rng);
            match a.does {
                Does::Ground {
                    shape,
                    windup,
                    blow,
                    tone,
                } => {
                    fx.start(boss, a.spell);
                    for (at, yaw) in anchors(shape, a.pick, pos, &picked, &seen, tf) {
                        commands.spawn((
                            Telegraph {
                                shape,
                                at: at.to_array(),
                                yaw,
                                tone,
                                start: now,
                                land: now + windup,
                                until: now + windup,
                            },
                            Pending {
                                source: boss,
                                spell: a.spell,
                                blow,
                            },
                            RunEntity,
                        ));
                    }
                }
                Does::Debuff {
                    dispel,
                    secs,
                    payload,
                } => {
                    let mut hit = Vec::new();
                    for &i in &picked {
                        // Tank-swap stacks come only on a swing that reaches.
                        if let Payload::Stacks { .. } = payload {
                            let reach = enemy.radius + HERO_RADIUS + MELEE_SLACK;
                            if hero::reach(pos, seen[i].pos) > reach {
                                continue;
                            }
                        }
                        if let Ok((_, _, mut h)) = heroes.get_mut(seen[i].e) {
                            h.raid.add(Debuff::new(a.spell, dispel, secs, payload));
                            hit.push(seen[i].e);
                        }
                        // A bomb's ring rides on its carrier, filling as the fuse burns.
                        if let Payload::Bomb { radius, .. } = payload {
                            commands.spawn((
                                Telegraph {
                                    shape: Shape::Circle { radius },
                                    at: seen[i].pos.to_array(),
                                    yaw: 0.0,
                                    tone: Tone::Damage,
                                    start: now,
                                    land: now + secs,
                                    until: now + secs,
                                },
                                Carried {
                                    hero: seen[i].e,
                                    spell: a.spell,
                                },
                                RunEntity,
                            ));
                        }
                    }
                    if !hit.is_empty() {
                        fx.cast(boss, a.spell, &hit, None);
                    }
                }
                Does::Adds {
                    name: add,
                    count,
                    radius,
                } => {
                    let count = if count == 0 { add_count(party) } else { count };
                    adds.push((pos, add, count, radius));
                }
                Does::Frenzy { secs, haste } => {
                    let secs = if hunters.is_empty() { secs / 2.0 } else { secs };
                    brain.frenzy_t = secs;
                    brain.frenzy_haste = haste;
                    brain.frenzy_age = 0.0;
                    marks.add(a.spell, secs);
                    fx.cast(boss, a.spell, &[boss], None);
                }
                Does::Berserk { damage, speed } => {
                    if !brain.berserk {
                        brain.berserk = true;
                        enemy.damage *= damage;
                        enemy.speed *= speed;
                        marks.add(a.spell, f32::INFINITY);
                        fx.cast(boss, a.spell, &[boss], None);
                    }
                }
                Does::Blink {
                    windup,
                    radius,
                    blow,
                    then,
                } => {
                    for &i in &picked {
                        commands.spawn((
                            Telegraph {
                                shape: Shape::Circle {
                                    radius: SIGIL_RADIUS,
                                },
                                at: seen[i].pos.to_array(),
                                yaw: 0.0,
                                tone: Tone::Debuff,
                                start: now,
                                land: now + windup,
                                until: now + windup,
                            },
                            BlinkTo {
                                boss,
                                spell: a.spell,
                                radius,
                                blow,
                                then,
                            },
                            RunEntity,
                        ));
                    }
                }
                Does::Inferno {
                    radius,
                    windup,
                    pulses,
                    every,
                    damage,
                    growth,
                } => {
                    enemy.root_t = enemy.root_t.max(windup + every * f32::from(pulses));
                    fx.cast(boss, a.spell, &[boss], None);
                    let yaw = hero::yaw_of(tf.rotation);
                    for (warn, land, share) in inferno_beats(windup, pulses, every, damage, growth)
                    {
                        commands.spawn((
                            Telegraph {
                                shape: Shape::Circle { radius },
                                at: pos.to_array(),
                                yaw,
                                tone: Tone::Damage,
                                start: now + warn,
                                land: now + land,
                                until: now + land,
                            },
                            Pending {
                                source: boss,
                                spell: a.spell,
                                blow: Blow::hit(share, School::Fire),
                            },
                            RunEntity,
                        ));
                    }
                }
                Does::Mend { share, cast, range } => {
                    if brain.cast.is_none() {
                        fx.start(boss, a.spell);
                        let ring = commands
                            .spawn((
                                Telegraph {
                                    shape: Shape::Circle {
                                        radius: enemy.radius + 1.5,
                                    },
                                    at: pos.to_array(),
                                    yaw: 0.0,
                                    tone: Tone::Safe,
                                    start: now,
                                    land: now + cast,
                                    until: now + cast,
                                },
                                RunEntity,
                            ))
                            .id();
                        brain.cast = Some(Casting {
                            spell: a.spell,
                            done: now + cast,
                            share,
                            range,
                            ring,
                        });
                    }
                }
                Does::Inspire { secs, haste, range } => {
                    fx.cast(boss, a.spell, &[boss], None);
                    inspires.push((pos, a.spell, secs, haste, range));
                }
                Does::Detonate { add } => detonations.push((boss, pos, add)),
                Does::Phase(_) | Does::Yell => {}
            }
        }
    }

    // Heals and Inspire land on the allies; a detonation kills the add nearest its boss.
    for (target, share) in heals {
        if let Ok((_, _, mut en, ..)) = bosses.get_mut(target) {
            if !en.dead {
                en.hp = (en.hp + share * en.max_hp).min(en.max_hp);
            }
        }
    }
    for (at, spell, secs, haste, range) in inspires {
        for (_, tf, en, mut brain, mut marks) in &mut bosses {
            if !en.dead && hero::reach(at, tf.translation) <= range {
                brain.inspired_t = secs;
                brain.inspire_haste = haste;
                marks.add(spell, secs);
            }
        }
    }
    for (boss, at, add) in detonations {
        let doomed = allies
            .iter()
            .filter(|a| a.name == add)
            .min_by(|a, b| hero::reach(at, a.pos).total_cmp(&hero::reach(at, b.pos)));
        if let Some(a) = doomed {
            hits.write(Hit {
                school: Some(School::Fire),
                ..Hit::direct(boss, a.e, a.hp + a.max_hp, false)
            });
        }
    }

    let map = selection.stage_def();
    let roster = super::data::roster(selection.stage);
    for (pos, add, count, radius) in adds {
        let Some(def) = roster.adds.iter().find(|d| d.name == add) else {
            continue;
        };
        for i in 0..count {
            let a = f32::from(i) / f32::from(count.max(1)) * std::f32::consts::TAU;
            let at = pos + Vec3::new(a.cos(), 0.0, a.sin()) * radius;
            let at = enemies::ground_on_collision(collide.as_ref(), &world, at, map.floor);
            enemies::spawn_enemy(
                &mut commands,
                &mut run,
                def,
                at,
                pos,
                Rank::Elite,
                &opts,
                &mut spawner.xp_carry,
            );
        }
    }
}

// The Frenzy spell of a script, for taking its mark off.
fn frenzy_spell(script: &Script) -> u32 {
    script
        .abilities
        .iter()
        .find(|a| matches!(a.does, Does::Frenzy { .. }))
        .map_or(0, |a| a.spell)
}

/// Where each telegraph of a ground ability lies, and its facing. Circles and donuts sit under
/// each pick (round the boss for [`Pick::Boss`] and [`Pick::Within`]); cones and lines start at
/// the boss, one toward each pick, or along its facing.
pub(crate) fn anchors(
    shape: Shape,
    pick: Pick,
    boss: Vec3,
    picked: &[usize],
    seen: &[Seen],
    tf: &Transform,
) -> Vec<(Vec3, f32)> {
    let facing = hero::yaw_of(tf.rotation);
    let round_boss = matches!(pick, Pick::Boss | Pick::Within(_));
    match shape {
        Shape::Circle { .. } | Shape::Donut { .. } => {
            if round_boss {
                vec![(boss, facing)]
            } else {
                picked.iter().map(|&i| (seen[i].pos, facing)).collect()
            }
        }
        Shape::Cone { .. } | Shape::Line { .. } => {
            if picked.is_empty() || round_boss {
                vec![(boss, facing)]
            } else {
                picked
                    .iter()
                    .map(|&i| (boss, yaw_toward(flat(seen[i].pos - boss))))
                    .collect()
            }
        }
    }
}

/// A blink's sigil whose wind-up is over: its boss stands there, its taunt dropped, and lands its
/// blow round itself after a short warning.
fn land_blinks(
    mut commands: Commands,
    run: Res<Run>,
    sigils: Query<(Entity, &Telegraph, &BlinkTo)>,
    mut bosses: Query<(&mut Transform, &mut Enemy), Without<Hero>>,
    mut fx: Fx,
) {
    let now = run.elapsed;
    for (e, t, blink) in &sigils {
        if now < t.land {
            continue;
        }
        commands.entity(e).despawn();
        let Ok((mut tf, mut enemy)) = bosses.get_mut(blink.boss) else {
            continue;
        };
        if enemy.dead {
            continue;
        }
        let at = t.origin();
        tf.translation = at;
        enemy.taunted = None;
        fx.cast(blink.boss, blink.spell, &[], Some(at));
        commands.spawn((
            Telegraph {
                shape: Shape::Circle {
                    radius: blink.radius,
                },
                at: at.to_array(),
                yaw: t.yaw,
                tone: Tone::Damage,
                start: now,
                land: now + BLINK_BLOW_WINDUP,
                until: now + BLINK_BLOW_WINDUP,
            },
            Pending {
                source: blink.boss,
                spell: blink.then,
                blow: blink.blow,
            },
            RunEntity,
        ));
    }
}

/// The blow of each telegraph whose wind-up is over: every hero inside takes it, and the foes
/// inside that are not bosses take its share for foes. One that leaves a pool stays on as the pool.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn land_telegraphs(
    mut commands: Commands,
    mut run: ResMut<Run>,
    mut telegraphs: Query<(Entity, &mut Telegraph, &Pending)>,
    mut heroes: Query<(Entity, &Transform, &mut HeroState), With<Hero>>,
    foes: Query<(Entity, &Transform, &Enemy), Without<Hero>>,
    mut hits: MessageWriter<Hit>,
    mut fx: Fx,
) {
    let now = run.elapsed;
    let shamans: Vec<Vec3> = heroes
        .iter()
        .filter(|(_, _, h)| h.alive() && h.class.id == 7)
        .map(|(_, tf, _)| tf.translation)
        .collect();
    for (e, mut t, pending) in &mut telegraphs {
        if now < t.land {
            continue;
        }
        let origin = t.origin();
        let blow = pending.blow;
        let mut struck = Vec::new();
        for (he, tf, mut hero) in &mut heroes {
            if !hero.alive() || !t.contains(tf.translation) {
                continue;
            }
            struck.push(he);
            land_blow(
                he,
                tf.translation,
                origin,
                pending.spell,
                &blow,
                &mut hero,
                &shamans,
                &mut fx,
            );
        }
        if blow.foes > 0.0 {
            for (fe, tf, en) in &foes {
                if !en.dead && !en.boss && t.contains(tf.translation) {
                    hits.write(Hit {
                        school: Some(blow.school),
                        ..Hit::direct(pending.source, fe, blow.foes * en.max_hp, false)
                    });
                }
            }
        }
        fx.cast(pending.source, pending.spell, &struck, Some(origin));
        match blow.pool {
            Some(pool) => {
                t.until = now + pool.secs;
                let radius = match t.shape {
                    Shape::Circle { radius } => radius,
                    Shape::Donut { outer, .. } => outer,
                    Shape::Cone { radius, .. } => radius,
                    Shape::Line { length, .. } => length,
                };
                let visual = units::spawn_area_object(
                    &mut commands,
                    run.guid(),
                    POOL_VISUAL,
                    radius,
                    origin,
                );
                commands.entity(e).remove::<Pending>().insert(Pool {
                    source: pending.source,
                    dps: pool.dps,
                    tick: 0.0,
                    visual: Some(visual),
                });
            }
            None => commands.entity(e).despawn(),
        }
    }
}

/// One telegraph's blow (of `spell`) on one hero inside it: damage, then fear (unless it is
/// immune) or a knockback away from the shape's origin, then what it leaves ([`After`]).
#[allow(clippy::too_many_arguments)]
pub(crate) fn land_blow(
    he: Entity,
    at: Vec3,
    origin: Vec3,
    spell: u32,
    blow: &Blow,
    hero: &mut HeroState,
    shamans: &[Vec3],
    fx: &mut Fx,
) {
    if blow.damage > 0.0 {
        let taken = hurt_school(hero, blow.damage, blow.school);
        if taken > 0.0 {
            fx.number(he, taken, false, COLOR_HURT);
        }
    }
    let away = flat(at - origin);
    if blow.fear > 0.0 {
        if fear_immune(hero.class.id, hero.race, at, shamans) {
            if hero.class.id == 7 || !shamans.is_empty() {
                fx.impact(he, TREMOR_TOTEM);
            }
        } else {
            hero.raid.fear(away, blow.fear);
        }
    }
    if blow.knock > 0.0 {
        hero.raid.knock(away, blow.knock);
    }
    after_blow(hero, spell, blow.after);
}

/// What a blow of `spell` leaves on a hero it lands on.
pub(crate) fn after_blow(hero: &mut HeroState, spell: u32, after: After) {
    match after {
        After::None => {}
        After::Silence(secs) => {
            hero.raid
                .add(Debuff::new(spell, Dispel::None, secs, Payload::Silence));
        }
        After::Stun(secs) => {
            hero.raid.hold(secs);
            hero.raid
                .add(Debuff::new(spell, Dispel::None, secs, Payload::Silence));
        }
        After::Slow { rate, secs } => {
            hero.raid
                .add(Debuff::new(spell, Dispel::Magic, secs, Payload::Slow(rate)));
        }
        After::Strip => {
            hero.shield = 0.0;
            hero.shield_t = 0.0;
            hero.thorns_t = 0.0;
            hero.heal_t = 0.0;
        }
    }
}

/// Burning ground: a hero standing in a pool takes its burn every [`TICK`]; a pool goes as it cools
/// or as the boss that left it falls.
fn burn_pools(
    mut commands: Commands,
    time: Res<Time>,
    run: Res<Run>,
    mut pools: Query<(Entity, &Telegraph, &mut Pool)>,
    bosses: Query<&Enemy>,
    mut heroes: Query<(Entity, &Transform, &mut HeroState), With<Hero>>,
    mut fx: Fx,
) {
    let now = run.elapsed;
    for (e, t, mut pool) in &mut pools {
        let fallen = !matches!(bosses.get(pool.source), Ok(b) if !b.dead);
        if now >= t.until || fallen {
            if let Some(v) = pool.visual {
                commands.entity(v).try_despawn();
            }
            commands.entity(e).despawn();
            continue;
        }
        pool.tick += time.delta_secs();
        if pool.tick < TICK {
            continue;
        }
        pool.tick -= TICK;
        for (he, tf, mut hero) in &mut heroes {
            if hero.alive() && t.contains(tf.translation) {
                let taken = hurt_school(&mut hero, pool.dps * TICK, School::Fire);
                if taken > 0.0 {
                    fx.number(he, taken, false, COLOR_HURT);
                }
            }
        }
    }
}

/// Every hero's raid debuffs on the clock, then each healer class's dispel pulse.
#[allow(clippy::type_complexity)]
fn tick_heroes(
    time: Res<Time>,
    run: Res<Run>,
    mut heroes: Query<(Entity, &Transform, &mut HeroState), With<Hero>>,
    foes: Query<(Entity, &Transform, &Enemy), Without<Hero>>,
    mut hits: MessageWriter<Hit>,
    mut fx: Fx,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let solo = run.party <= 1;
    let mut splashes = Vec::new();
    for (he, tf, mut hero) in &mut heroes {
        if !hero.alive() {
            if !hero.raid.debuffs.is_empty() || hero.raid.fear_t > 0.0 {
                hero.raid.clear();
            }
            continue;
        }
        for event in hero.raid.tick(dt, solo) {
            match event {
                RaidEvent::Hit { spell, damage } => {
                    let taken = hurt_school(&mut hero, damage, School::Fire);
                    if taken > 0.0 {
                        fx.impact(he, spell);
                        fx.number(he, taken, false, COLOR_HURT);
                    }
                }
                RaidEvent::Splash {
                    spell,
                    radius,
                    damage,
                } => splashes.push((he, tf.translation, spell, radius, damage)),
                RaidEvent::Blast {
                    spell: _,
                    radius,
                    damage,
                } => {
                    let at = tf.translation;
                    for (fe, ftf, en) in &foes {
                        if !en.dead && !en.boss && hero::reach(at, ftf.translation) <= radius {
                            hits.write(Hit {
                                school: Some(School::Fire),
                                ..Hit::direct(he, fe, damage * en.max_hp, false)
                            });
                        }
                    }
                }
                RaidEvent::Count(n) => fx.text(he, n.to_string(), COLOR_DOOM),
                RaidEvent::Swap => fx.text(he, "SWAP!".into(), COLOR_WARN),
            }
        }
    }
    for (from, at, spell, radius, damage) in splashes {
        let mut hit = Vec::new();
        for (he, tf, mut hero) in &mut heroes {
            if he == from || !hero.alive() || hero::reach(at, tf.translation) > radius {
                continue;
            }
            let taken = hurt_school(&mut hero, damage, School::Fire);
            if taken > 0.0 {
                fx.number(he, taken, false, COLOR_HURT);
            }
            hit.push(he);
        }
        fx.cast(from, spell, &hit, Some(at));
    }

    // The dispel pulses: each healer class takes one debuff off the most urgent ally in range.
    let afflicted: Vec<(Entity, Vec3, [Option<f32>; 2])> = heroes
        .iter()
        .map(|(e, tf, h)| {
            let alive = h.alive();
            let urgent = |k| if alive { h.raid.urgency(k) } else { None };
            (
                e,
                tf.translation,
                [urgent(Dispel::Curse), urgent(Dispel::Magic)],
            )
        })
        .collect();
    let mut dispels = Vec::new();
    for (he, tf, mut hero) in &mut heroes {
        let Some((kind, spell)) = dispels_of(hero.class.id).filter(|_| hero.alive()) else {
            continue;
        };
        hero.raid.dispel_cd = (hero.raid.dispel_cd - dt).max(0.0);
        if hero.raid.dispel_cd > 0.0 {
            continue;
        }
        let slot = usize::from(kind == Dispel::Magic);
        let near: Vec<(Vec3, Option<f32>)> = afflicted.iter().map(|a| (a.1, a.2[slot])).collect();
        if let Some(i) = dispel_target(tf.translation, &near) {
            hero.raid.dispel_cd = DISPEL_EVERY;
            dispels.push((he, afflicted[i].0, kind, spell));
        }
    }
    for (healer, target, kind, spell) in dispels {
        if let Ok((_, _, mut hero)) = heroes.get_mut(target) {
            if hero.raid.dispel(kind).is_some() {
                fx.cast(healer, spell, &[target], None);
            }
        }
    }
}

/// A bomb's ring follows its carrier until the fuse; it goes with the bomb (the carrier fell, or
/// its debuff is gone).
fn follow_bombs(
    mut commands: Commands,
    run: Res<Run>,
    mut rings: Query<(Entity, &mut Telegraph, &Carried)>,
    heroes: Query<(&Transform, &HeroState), With<Hero>>,
) {
    let now = run.elapsed;
    for (e, mut t, carried) in &mut rings {
        let carrier = heroes
            .get(carried.hero)
            .ok()
            .filter(|(_, h)| h.alive() && h.raid.spells().any(|s| s == carried.spell));
        let Some((tf, _)) = carrier.filter(|_| now <= t.until) else {
            commands.entity(e).despawn();
            continue;
        };
        // Moved on only when it has walked a little, so the wire carries no jitter.
        if tf.translation.distance(t.origin()) > 0.1 {
            t.at = tf.translation.to_array();
        }
    }
}

/// An add that can't die while its master stands: tied while the master lives (a killing blow
/// leaves it at a sliver, [`Undying`]), healed to full below half health with its emote, and
/// fallen with its master.
#[allow(clippy::type_complexity)]
fn guard_undying(
    mut commands: Commands,
    mut adds: Query<(Entity, &mut Enemy, &RaidBoss, Has<Undying>)>,
    mut hits: MessageWriter<Hit>,
    mut yells: MessageWriter<Yelled>,
    mut fx: Fx,
) {
    let living: Vec<&'static str> = adds
        .iter()
        .filter(|(_, en, ..)| !en.dead)
        .map(|(_, en, ..)| en.def.name)
        .collect();
    for (e, mut en, brain, tied) in &mut adds {
        let Some(master) = brain.script.master.filter(|m| m.undying) else {
            continue;
        };
        if en.dead {
            continue;
        }
        if living.contains(&master.name) {
            if !tied {
                commands.entity(e).try_insert(Undying);
            }
            if let Some(healed) = undying_heal(en.hp, en.max_hp) {
                en.hp = healed;
                fx.text(e, "Refuses to die".into(), COLOR_WARN);
                if let Some(y) = &master.refuses {
                    yells.write(Yelled::of(y, brain.script.name));
                }
            }
        } else if tied {
            // Its master fell: it falls with him.
            commands.entity(e).remove::<Undying>();
            hits.write(Hit::direct(e, e, en.hp + en.max_hp, false));
        }
    }
}

/// An undying add's health after the guard: full once it has dropped below [`UNDYING_FLOOR`].
pub(crate) fn undying_heal(hp: f32, max_hp: f32) -> Option<f32> {
    (hp < max_hp * UNDYING_FLOOR).then_some(max_hp)
}

/// What a scripted foe's fall sets off: its death cry, an eruption where it fell, and a stack of its
/// master's enrage.
#[allow(clippy::type_complexity)]
fn on_deaths(
    mut commands: Commands,
    run: Res<Run>,
    mut kills: MessageReader<enemies::Loot>,
    mut masters: Query<(Entity, &mut Enemy, &mut Marks), (With<RaidBoss>, Without<Hero>)>,
    mut yells: MessageWriter<Yelled>,
    mut fx: Fx,
) {
    let now = run.elapsed;
    for kill in kills.read() {
        let Some(script) = script_for(kill.def.name) else {
            continue;
        };
        if let Some(y) = &script.death {
            yells.write(Yelled::of(y, script.name));
        }
        let master = script.master.and_then(|m| {
            masters
                .iter()
                .find(|(_, en, _)| !en.dead && en.def.name == m.name)
                .map(|(e, ..)| (e, m))
        });
        if let (Some(erupt), Some((source, _))) = (script.erupt, master) {
            commands.spawn((
                Telegraph {
                    shape: erupt.shape,
                    at: kill.pos.to_array(),
                    yaw: 0.0,
                    tone: erupt.tone,
                    start: now,
                    land: now + erupt.windup,
                    until: now + erupt.windup,
                },
                Pending {
                    source,
                    spell: erupt.spell,
                    blow: erupt.blow,
                },
                RunEntity,
            ));
        }
        if let Some((
            e,
            Master {
                enrage: Some((spell, per)),
                ..
            },
        )) = master
        {
            if let Ok((_, mut en, mut marks)) = masters.get_mut(e) {
                en.damage *= 1.0 + per;
                marks.add(spell, f32::INFINITY);
                fx.cast(e, spell, &[e], None);
            }
        }
    }
}

/// A blow of `share` of the hero's own maximum health, through its armour, shield and immunity;
/// what got through.
pub(crate) fn hurt_share(hero: &mut HeroState, share: f32) -> f32 {
    let raw = share * hero.stats().max_hp;
    hero::hurt_hero(hero, raw)
}

/// As [`hurt_share`], a blow of `school`: magic is raised by the curses on the hero
/// ([`HeroRaid::magic_taken`]).
pub(crate) fn hurt_school(hero: &mut HeroState, share: f32, school: School) -> f32 {
    let share = if school == School::Physical {
        share
    } else {
        share * hero.raid.magic_taken()
    };
    hurt_share(hero, share)
}

/// Put `hero`'s raid debuffs into the auras it shows, after its own.
pub(super) fn show_debuffs(hero: &HeroState, shown: &mut Vec<u32>) {
    shown.extend(hero.raid.spells());
}
