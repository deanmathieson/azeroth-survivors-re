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

pub(super) fn plugin(app: &mut App) {
    app.add_message::<Yelled>()
        .add_systems(
            Update,
            (
                note_final_boss,
                attach_scripts,
                run_bosses,
                land_telegraphs,
                burn_pools,
                tick_heroes,
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
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "a library block the later Molten Core bosses use; tested here"
        )
    )]
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
}

impl Blow {
    pub(crate) const fn hit(damage: f32, school: School) -> Self {
        Self {
            damage,
            school,
            fear: 0.0,
            knock: 0.0,
            pool: None,
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
    /// each a fraction of their own maximum health (Living Bomb).
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "a library block the later Molten Core bosses use; tested here"
        )
    )]
    Bomb {
        radius: f32,
        allies: f32,
        carrier: f32,
    },
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
    /// `count` of the stage's add named `name`, round the boss.
    Adds {
        name: &'static str,
        count: u8,
        radius: f32,
    },
    /// A buff on the boss: `haste` more swings for `secs`; a Hunter's Tranquilizing Shot takes it
    /// off, and without a Hunter in the party it lasts half as long.
    Frenzy { secs: f32, haste: f32 },
    /// The hard enrage: damage and speed multiplied, for good.
    Berserk { damage: f32, speed: f32 },
    /// The fight moves to this phase: abilities of other phases stop, its own start.
    Phase(u8),
    /// Only the ability's yell.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "a library block the later Molten Core bosses use; tested here"
        )
    )]
    Yell,
}

/// A line a boss says, and its sound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Yell {
    /// `%s` is the boss's name, as in the cores' emote texts.
    pub text: &'static str,
    /// A [`SOUNDS`] kit.
    pub sound: &'static str,
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
            yell: Some(Yell { text, sound }),
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
}

/// How many heroes a curse picks for a party this size (RAIDS.md section 7).
pub(crate) fn curse_targets(party: u8) -> usize {
    match party {
        0..=2 => 1,
        3 | 4 => 2,
        _ => 3,
    }
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

    /// Feared or mind-controlled, a hero's auto-casts wait.
    pub(crate) fn can_cast(&self) -> bool {
        self.fear_t <= 0.0 && !self.dominated()
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
                Payload::CooldownRate(_) => {}
            }
            if gone {
                self.debuffs.remove(i);
            } else {
                i += 1;
            }
        }
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

// ---------------------------------------------------------------------------------------------
// Yells
// ---------------------------------------------------------------------------------------------

/// A boss said something: every machine shows it and sounds its kit.
#[derive(Message, Clone, Debug, PartialEq)]
pub(crate) struct Yelled {
    pub text: String,
    /// A place in [`SOUNDS`].
    pub sound: u8,
}

impl Yelled {
    pub(crate) fn of(yell: &Yell, boss: &str) -> Self {
        Self {
            text: yell.text.replace("%s", boss),
            sound: SOUNDS.iter().position(|s| *s == yell.sound).unwrap_or(0) as u8,
        }
    }
}

fn play_yells(
    mut yells: MessageReader<Yelled>,
    mut run: ResMut<Run>,
    mut cues: MessageWriter<crate::sound::SoundCue>,
) {
    for y in yells.read() {
        run.banner = Some((y.text.clone(), 4.0));
        if let Some(kit) = SOUNDS.get(usize::from(y.sound)) {
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

/// Every scripted boss's clock: what is due fires, Frenzy runs and Hunters shoot it off.
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
    let mut adds = Vec::new();
    for (boss, tf, mut enemy, mut brain, mut marks) in &mut bosses {
        if enemy.dead {
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

        let health = enemy.hp / enemy.max_hp.max(1.0);
        let tank = boss_target(pos, enemy.taunted, now, &seen);
        for a in brain.due(dt, health, party) {
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
                    }
                    if !hit.is_empty() {
                        fx.cast(boss, a.spell, &hit, None);
                    }
                }
                Does::Adds {
                    name: add,
                    count,
                    radius,
                } => adds.push((pos, add, count, radius)),
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
                Does::Phase(_) | Does::Yell => {}
            }
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

/// The Frenzy spell of a script, for taking its mark off.
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

/// The blow of each telegraph whose wind-up is over: every hero inside takes it. One that leaves a
/// pool stays on as the pool.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn land_telegraphs(
    mut commands: Commands,
    mut run: ResMut<Run>,
    mut telegraphs: Query<(Entity, &mut Telegraph, &Pending)>,
    mut heroes: Query<(Entity, &Transform, &mut HeroState), With<Hero>>,
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
        let mut hits = Vec::new();
        for (he, tf, mut hero) in &mut heroes {
            if !hero.alive() || !t.contains(tf.translation) {
                continue;
            }
            hits.push(he);
            land_blow(
                he,
                tf.translation,
                origin,
                &blow,
                &mut hero,
                &shamans,
                &mut fx,
            );
        }
        fx.cast(pending.source, pending.spell, &hits, Some(origin));
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

/// One telegraph's blow on one hero inside it: damage, then fear (unless it is immune) or a
/// knockback, away from the shape's origin.
pub(crate) fn land_blow(
    he: Entity,
    at: Vec3,
    origin: Vec3,
    blow: &Blow,
    hero: &mut HeroState,
    shamans: &[Vec3],
    fx: &mut Fx,
) {
    if blow.damage > 0.0 {
        let taken = hurt_share(hero, blow.damage);
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
                let taken = hurt_share(&mut hero, pool.dps * TICK);
                if taken > 0.0 {
                    fx.number(he, taken, false, COLOR_HURT);
                }
            }
        }
    }
}

/// Every hero's raid debuffs on the clock, then each healer class's dispel pulse.
fn tick_heroes(
    time: Res<Time>,
    run: Res<Run>,
    mut heroes: Query<(Entity, &Transform, &mut HeroState), With<Hero>>,
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
                    let taken = hurt_share(&mut hero, damage);
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
            let taken = hurt_share(&mut hero, damage);
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

/// A blow of `share` of the hero's own maximum health, through its armour, shield and immunity;
/// what got through.
pub(crate) fn hurt_share(hero: &mut HeroState, share: f32) -> f32 {
    let raw = share * hero.stats().max_hp;
    hero::hurt_hero(hero, raw)
}

/// Put `hero`'s raid debuffs into the auras it shows, after its own.
pub(super) fn show_debuffs(hero: &HeroState, shown: &mut Vec<u32>) {
    shown.extend(hero.raid.spells());
}
