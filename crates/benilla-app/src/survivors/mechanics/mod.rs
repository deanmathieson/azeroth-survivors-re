//! Mechanics: a data-driven library of effects a hero carries, set off by its own play. Abilities'
//! legendary transformations ([`data::LEGENDARIES`]) are built from it; talents and jewelry procs
//! grant theirs the same way.
//!
//! # A mechanic
//!
//! A [`MechanicSpec`] is one [`Trigger`], one [`Effect`], where it lands ([`Aim`]), an optional
//! real 1.12 spell whose visual plays there, and an internal cooldown:
//!
//! ```ignore
//! MechanicSpec::new(
//!     Trigger::OnHit { of: Of::Ability("frostbolt"), chance: 1.0 },
//!     Effect::Split { count: 3, spread: 8.0 },
//! )
//! .visual(116)
//! .cooldown(0.5)
//! ```
//!
//! - **Triggers.** [`Trigger::Periodic`], [`Trigger::OnCast`], [`Trigger::OnHit`],
//!   [`Trigger::OnCrit`], [`Trigger::OnKill`], [`Trigger::OnLethal`], [`Trigger::OnLowHealth`].
//!   The hit, crit, kill and cast triggers name the ability they listen to ([`Of`]); only an
//!   ability's own blows set them off, never a splash, a damage-over-time tick or a mechanic's
//!   blow, so procs never cascade.
//! - **Effects.** Area blows ([`Effect::Nova`], [`Effect::BurningGround`]), afflictions
//!   ([`Effect::Dot`], [`Effect::ApplyCc`], [`Effect::Execute`]), projectile shapes that carry the
//!   trigger's blow on ([`Effect::Chain`], [`Effect::Split`], [`Effect::Pierce`],
//!   [`Effect::Boomerang`], [`Effect::Orbit`]), re-casts ([`Effect::Echo`]), and the hero's own
//!   ([`Effect::Burst`], [`Effect::HealPulse`], [`Effect::Shield`], [`Effect::CheatDeath`],
//!   [`Effect::ResetCooldowns`]), the party's ([`Effect::PartyAura`], [`Effect::BestAllyAura`],
//!   [`Effect::ShareHeal`]), a next-cast guarantee ([`Effect::NextCast`]: Presence of Mind,
//!   Clearcasting), the burns' ([`Effect::Ignite`] fed by them, [`Effect::ConsumeDot`] consuming
//!   and spreading one), [`Effect::Shatter`] on frozen targets, and [`Effect::TotemReach`].
//! - **Where.** [`Aim::Trigger`] is where the trigger happened: the struck enemy for a hit, crit or
//!   kill, the hero for anything else. The hero's own effects always land on the hero.
//! - **Numbers.** Flat numbers (a nova's damage, a heal) are at run level 1: they grow with the
//!   run's level ([`level_scale`]) and damage with the owner's might, school and crits, so effects
//!   stay relevant late. The projectile shapes and [`Effect::Execute`] carry the trigger's blow
//!   instead: the hit that set them off, or for a cast the ability's own damage per target; set
//!   off by anything else they have nothing to carry and do nothing.
//! - **Credit.** Every blow and heal is the owner's: its damage and healing done.
//!
//! # Granting
//!
//! Mechanics live on each hero ([`HeroState::mechanics`]), so every hero in a party carries its own.
//! A source grants its set under a stable [`MechanicId`] and takes it back the same way:
//!
//! ```ignore
//! mechanics::grant(&mut hero, MechanicId::Talent(16757), [spec_a, spec_b]);
//! mechanics::revoke(&mut hero, MechanicId::Talent(16757));
//! hero.mechanics.revoke_where(|id| matches!(id, MechanicId::Jewelry(_)));
//! ```
//!
//! Granting replaces whatever the id held, so it is idempotent. A run begins with none
//! ([`HeroState::new`]): grant at run start, after the hero is made. Legendaries grant themselves
//! ([`sync_legendaries`]) once an ability reaches its top rank, for the rest of the run.
//!
//! # Runtime
//!
//! [`abilities`](super::abilities) records each cast and landed blow in [`Procs`]; once the frame's
//! blows have landed, [`run::tick`] advances every hero's clocks, matches the procs, and fires
//! what went off, before the heroes' vitals decide who falls (so [`Effect::CheatDeath`] can turn
//! a lethal blow away).

mod run;
mod target;
#[cfg(test)]
mod tests;

use bevy::prelude::*;

use super::abilities;
use super::bonus::{Bonus, School};
use super::data::{self, AbilityDef};
use super::HeroState;

/// How much a mechanic's flat numbers grow per run level past the first.
pub(crate) const LEVEL_GROWTH: f32 = 0.03;
/// Seconds between one echoed cast and the next.
pub(crate) const ECHO_GAP: f32 = 0.3;

/// The systems, between the frame's blows landing and the heroes' vitals.
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Procs>().add_systems(
        Update,
        (run::tick, run::follow_owners, run::spin_orbiters)
            .chain()
            .after(abilities::apply_hits)
            .before(super::hero::hero_vitals)
            .run_if(super::strip::world_runs)
            .run_if(super::party::simulates)
            .run_if(|l: Res<crate::loading_screen::LoadingScreen>| !l.covering()),
    );
}

/// The spell whose ground visual rings a totem's reach under Totemic Mastery
/// ([`Effect::TotemReach`]).
pub(crate) const TOTEM_RING: u32 = TRANQUIL_RING;
/// "Draw Ancient Glyphs", whose ground object is Tranquility's ring of green light
/// (`tranquility_area.mdx`): the one 1.12 area visual drawn as a ring.
const TRANQUIL_RING: u32 = 24815;

/// A ring on the ground at `pos`, `radius` across: a totem's reach shown.
pub(crate) fn totem_ring(
    commands: &mut Commands,
    run: &mut super::Run,
    pos: Vec3,
    radius: f32,
) -> Entity {
    let serial = run.guid();
    super::units::spawn_area_object(commands, serial, TOTEM_RING, radius, pos)
}

/// How much a mechanic's flat numbers have grown by run level `level`.
pub(crate) fn level_scale(level: u32) -> f32 {
    1.0 + LEVEL_GROWTH * level.saturating_sub(1) as f32
}

/// Which ability a trigger listens to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Of {
    /// Any of the hero's abilities.
    // Every legendary listens to its own ability; talents and jewelry will listen to any.
    #[allow(dead_code)]
    Any,
    /// Only this one, by its key in [`data::ABILITIES`].
    Ability(&'static str),
}

impl Of {
    pub(crate) fn matches(self, ability: &AbilityDef) -> bool {
        match self {
            Of::Any => true,
            Of::Ability(key) => ability.key == key,
        }
    }
}

/// What sets a mechanic off. Chances are 0 to 1.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Trigger {
    /// Every `every` seconds.
    Periodic { every: f32 },
    /// When the hero casts the ability.
    OnCast(Of),
    /// When one of the ability's blows lands.
    OnHit { of: Of, chance: f32 },
    /// When one of the ability's blows lands a critical strike.
    OnCrit { of: Of, chance: f32 },
    /// When one of the ability's blows kills.
    OnKill { of: Of, chance: f32 },
    /// When a blow would kill the hero.
    OnLethal,
    /// When the hero's health falls below `threshold` of its maximum; once each time it falls.
    OnLowHealth { threshold: f32 },
}

/// A crowd control, as [`Effect::ApplyCc`] and [`Effect::Nova`] land it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum CcKind {
    Stun,
    /// Slowed to this fraction of speed.
    Chill(f32),
    Root,
    Fear,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Cc {
    pub kind: CcKind,
    pub secs: f32,
}

impl Cc {
    /// The blow's effect that lands it ([`data::Effect`]); bosses shrug most of it off there.
    pub(crate) fn effect(self) -> data::Effect {
        match self.kind {
            CcKind::Stun => data::Effect::Stun(self.secs),
            CcKind::Chill(m) => data::Effect::Slow(m, self.secs),
            CcKind::Root => data::Effect::Root(self.secs),
            CcKind::Fear => data::Effect::Fear(self.secs),
        }
    }
}

/// What a mechanic does when it goes off. Radii and ranges are in yards, times in seconds.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Effect {
    /// A blast around where it lands: `damage` to all within `radius`, thrown `knockback` yards
    /// clear (bosses stand), with an optional crowd control.
    Nova {
        school: School,
        radius: f32,
        damage: f32,
        knockback: f32,
        cc: Option<Cc>,
    },
    /// A buff on the hero: `bonuses` for `duration`; going off again while it runs refreshes it.
    Burst {
        duration: f32,
        bonuses: &'static [Bonus],
    },
    /// Ground that burns whatever stands in it, `dps` a second for `duration`; aimed
    /// [`Aim::Follow`], it moves with the hero.
    BurningGround {
        school: School,
        radius: f32,
        dps: f32,
        duration: f32,
    },
    /// The blow jumps on from its target, `jumps` times, each `falloff` of the last.
    Chain { jumps: u8, falloff: f32 },
    /// The blow shatters into `count` shards at the nearest others within `spread`, half as hard.
    Split { count: u8, spread: f32 },
    /// The blow passes through its target into up to `n` behind it.
    Pierce { n: u8 },
    /// The blow flies back to the hero, striking all it passes.
    Boomerang,
    /// `n` motes circle the hero at `radius` for `duration`, each striking what it touches with the
    /// blow.
    Orbit { n: u8, radius: f32, duration: f32 },
    /// The ability casts `extra_casts` more times, [`ECHO_GAP`] apart.
    Echo { extra_casts: u8 },
    /// A target the blow leaves below `threshold` of its health takes it `mult` times over.
    Execute { threshold: f32, mult: f32 },
    /// Heals every hero within `radius` for `amount`, through [`super::party::heal`].
    HealPulse { radius: f32, amount: f32 },
    /// A temporary aura on every living party member within `radius`. Talent auras live here,
    /// rather than in the class-buff picker, because their owner and duration matter.
    PartyAura {
        radius: f32,
        duration: f32,
        bonuses: &'static [Bonus],
    },
    /// A temporary buff on the party member in range who has dealt the most damage this run.
    /// This keeps support casts deterministic on the host without requiring client targeting UI.
    BestAllyAura {
        radius: f32,
        duration: f32,
        bonuses: &'static [Bonus],
    },
    /// A shield on the hero, through [`super::party::shield`].
    Shield { amount: f32, duration: f32 },
    /// Back up at `heal_frac` of max health and untouchable for `immune` seconds: with
    /// [`Trigger::OnLethal`], death turned away.
    CheatDeath { immune: f32, heal_frac: f32 },
    /// Every one of the hero's abilities is ready at once.
    ResetCooldowns,
    /// A crowd control on the target.
    ApplyCc { kind: CcKind, secs: f32 },
    /// Damage over time on the target, `dps` a second for `secs`.
    Dot { school: School, dps: f32, secs: f32 },
    /// A next-cast guarantee, held on the hero until its next cast spends it ([`Armed`]): that
    /// cast's blows all crit with `crit`, and its cooldown is `cooldown` of its own (0.5 halves
    /// it). With `instant`, the ability furthest from ready is ready at once, as Presence of
    /// Mind's instant cast. `aura` is the buff spell the hero shows while it holds, 0 for none.
    NextCast {
        instant: bool,
        crit: bool,
        cooldown: f32,
        aura: u32,
    },
    /// Ignite fed by the burns already on its target: a Fire damage over time of `dps` a second
    /// for `secs`, `per_burn` more of it for each Fire burn the target carries (up to
    /// [`MAX_FEED`]).
    Ignite { dps: f32, secs: f32, per_burn: f32 },
    /// Shatter: a blow on a frozen target (rooted, or chilled to half speed or slower) strikes it
    /// again for `bonus` of itself.
    Shatter { bonus: f32 },
    /// A damage over time of `school` the hero has on the nearest enemy carrying one, consumed:
    /// its remaining damage strikes at once, `burst` times over, and a fresh one of the same
    /// strength spreads to up to `spread` enemies within `radius` of it for `secs`. Conflagrate
    /// on Immolate.
    ConsumeDot {
        school: School,
        burst: f32,
        spread: u8,
        radius: f32,
        secs: f32,
    },
    /// Vampiric Embrace's party heal: `share` of the `school` damage the hero dealt since it last
    /// went off heals every living party member within `radius`, the hero too.
    ShareHeal {
        school: School,
        share: f32,
        radius: f32,
    },
    /// The hero's totems (the party totems, its attack totems and a Healing Stream Totem) reach
    /// `mult` times as far, their reach ringed on the ground. Nothing to do when it goes off.
    TotemReach { mult: f32 },
}

/// A chill this deep (this fraction of speed or slower) freezes its target for Shatter.
pub(crate) const SHATTER_CHILL: f32 = 0.5;
/// A Healthstone reaches the allies this near its taker, healing this share of their max health,
/// with the `Spell.dbc` row of the Minor Healthstone's heal as its visual.
pub(crate) const HEALTHSTONE_REACH: f32 = 10.0;
pub(crate) const HEALTHSTONE_SHARE: f32 = 0.3;
pub(crate) const HEALTHSTONE_SPELL: u32 = 6262;

/// The most Fire burns an [`Effect::Ignite`] feeds on.
pub(crate) const MAX_FEED: u8 = 5;

/// A next-cast guarantee held on a hero ([`Effect::NextCast`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Armed {
    pub crit: bool,
    /// The fraction of its own cooldown the empowered cast keeps.
    pub cooldown: f32,
    /// The buff spell shown while it holds, 0 for none.
    pub aura: u32,
}

/// Where a mechanic lands.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Aim {
    /// Where the trigger happened: the struck enemy for a hit, crit or kill, else the hero.
    Trigger,
    /// The hero, where it stands.
    Hero,
    /// On the hero and moving with it: ground follows; anything else lands as [`Aim::Hero`].
    Follow,
    /// The `n` nearest other enemies within `radius` of the trigger, each its own landing.
    Spread { radius: f32, n: u8 },
    /// A random enemy within `range` of the hero.
    Random { range: f32 },
    /// The thickest pack within `range` of the hero.
    Densest { range: f32 },
}

/// One mechanic: its trigger, its effect, where it lands, its visual and its cooldown.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct MechanicSpec {
    pub trigger: Trigger,
    pub effect: Effect,
    pub at: Aim,
    /// A real 1.12 spell whose visual plays where the effect lands: cast on the hero for the
    /// hero's own effects and areas around it, at the point for areas away from it (the visual's
    /// ground model), as an impact on each enemy struck or afflicted, and as a ground's own area.
    pub visual: Option<u32>,
    /// Seconds before it can go off again; 0 for every time its trigger fires.
    pub cooldown: f32,
}

impl MechanicSpec {
    /// Landing where its trigger happened, with no visual or cooldown.
    pub const fn new(trigger: Trigger, effect: Effect) -> Self {
        Self {
            trigger,
            effect,
            at: Aim::Trigger,
            visual: None,
            cooldown: 0.0,
        }
    }

    pub const fn at(self, at: Aim) -> Self {
        Self { at, ..self }
    }

    pub const fn visual(self, spell: u32) -> Self {
        Self {
            visual: Some(spell),
            ..self
        }
    }

    pub const fn cooldown(self, secs: f32) -> Self {
        Self {
            cooldown: secs,
            ..self
        }
    }

    /// Whether its numbers make sense: positive radii, times and counts, chances and thresholds
    /// within 0 to 1, a visual that names a spell. The reason when not.
    pub(crate) fn validate(&self) -> Result<(), String> {
        let positive = |what: &str, v: f32| {
            if v > 0.0 && v.is_finite() {
                Ok(())
            } else {
                Err(format!("{what} {v} is not positive"))
            }
        };
        let fraction = |what: &str, v: f32| {
            if v > 0.0 && v <= 1.0 {
                Ok(())
            } else {
                Err(format!("{what} {v} is outside (0, 1]"))
            }
        };
        let count = |what: &str, n: u8| {
            if n > 0 {
                Ok(())
            } else {
                Err(format!("{what} is 0"))
            }
        };
        match self.trigger {
            Trigger::Periodic { every } => positive("period", every)?,
            Trigger::OnCast(_) | Trigger::OnLethal => {}
            Trigger::OnHit { chance, .. }
            | Trigger::OnCrit { chance, .. }
            | Trigger::OnKill { chance, .. } => fraction("chance", chance)?,
            Trigger::OnLowHealth { threshold } => {
                if threshold <= 0.0 || threshold >= 1.0 || threshold.is_nan() {
                    return Err(format!(
                        "low-health threshold {threshold} is outside (0, 1)"
                    ));
                }
            }
        }
        match self.effect {
            Effect::Nova {
                radius,
                damage,
                knockback,
                cc,
                ..
            } => {
                positive("radius", radius)?;
                if damage < 0.0 || knockback < 0.0 {
                    return Err("negative nova damage or knockback".into());
                }
                if damage == 0.0 && knockback == 0.0 && cc.is_none() {
                    return Err("a nova that does nothing".into());
                }
                if let Some(cc) = cc {
                    positive("crowd-control time", cc.secs)?;
                }
            }
            Effect::Burst { duration, bonuses } => {
                positive("duration", duration)?;
                if bonuses.is_empty() {
                    return Err("a burst with no bonuses".into());
                }
            }
            Effect::BurningGround {
                radius,
                dps,
                duration,
                ..
            } => {
                positive("radius", radius)?;
                positive("damage per second", dps)?;
                positive("duration", duration)?;
            }
            Effect::Chain { jumps, falloff } => {
                count("jumps", jumps)?;
                fraction("falloff", falloff)?;
            }
            Effect::Split { count: n, spread } => {
                count("shards", n)?;
                positive("spread", spread)?;
            }
            Effect::Pierce { n } => count("pierce", n)?,
            Effect::Boomerang | Effect::ResetCooldowns => {}
            Effect::Orbit {
                n,
                radius,
                duration,
            } => {
                count("motes", n)?;
                positive("radius", radius)?;
                positive("duration", duration)?;
            }
            Effect::Echo { extra_casts } => count("extra casts", extra_casts)?,
            Effect::Execute { threshold, mult } => {
                fraction("execute threshold", threshold)?;
                if mult <= 1.0 {
                    return Err(format!("execute multiplier {mult} adds nothing"));
                }
            }
            Effect::HealPulse { radius, amount } => {
                positive("radius", radius)?;
                positive("heal", amount)?;
            }
            Effect::PartyAura {
                radius,
                duration,
                bonuses,
            } => {
                positive("party aura radius", radius)?;
                positive("party aura duration", duration)?;
                if bonuses.is_empty() {
                    return Err("a party aura with no bonuses".into());
                }
            }
            Effect::BestAllyAura {
                radius,
                duration,
                bonuses,
            } => {
                positive("ally aura radius", radius)?;
                positive("ally aura duration", duration)?;
                if bonuses.is_empty() {
                    return Err("an ally aura with no bonuses".into());
                }
            }
            Effect::Shield { amount, duration } => {
                positive("absorb", amount)?;
                positive("duration", duration)?;
            }
            Effect::CheatDeath { immune, heal_frac } => {
                positive("immunity", immune)?;
                fraction("heal fraction", heal_frac)?;
            }
            Effect::ApplyCc { kind, secs } => {
                positive("crowd-control time", secs)?;
                if let CcKind::Chill(m) = kind {
                    fraction("chill speed", m)?;
                }
            }
            Effect::Dot { dps, secs, .. } => {
                positive("damage per second", dps)?;
                positive("duration", secs)?;
            }
            Effect::NextCast {
                instant,
                crit,
                cooldown,
                ..
            } => {
                fraction("kept cooldown", cooldown)?;
                if !instant && !crit && cooldown >= 1.0 {
                    return Err("a next-cast guarantee that guarantees nothing".into());
                }
            }
            Effect::Ignite {
                dps,
                secs,
                per_burn,
            } => {
                positive("damage per second", dps)?;
                positive("duration", secs)?;
                if per_burn < 0.0 || per_burn.is_nan() {
                    return Err(format!("ignite feed {per_burn}"));
                }
            }
            Effect::Shatter { bonus } => positive("shatter bonus", bonus)?,
            Effect::ConsumeDot {
                burst,
                radius,
                secs,
                ..
            } => {
                positive("burst", burst)?;
                positive("spread radius", radius)?;
                positive("spread duration", secs)?;
            }
            Effect::ShareHeal { share, radius, .. } => {
                fraction("heal share", share)?;
                positive("radius", radius)?;
            }
            Effect::TotemReach { mult } => {
                if mult <= 1.0 || !mult.is_finite() {
                    return Err(format!("totem reach {mult} adds nothing"));
                }
            }
        }
        match self.at {
            Aim::Trigger | Aim::Hero | Aim::Follow => {}
            Aim::Spread { radius, n } => {
                positive("spread radius", radius)?;
                count("spread targets", n)?;
            }
            Aim::Random { range } | Aim::Densest { range } => positive("range", range)?,
        }
        if self.visual == Some(0) {
            return Err("visual spell 0".into());
        }
        if self.cooldown < 0.0 || self.cooldown.is_nan() {
            return Err(format!("cooldown {}", self.cooldown));
        }
        Ok(())
    }
}

/// Who granted a mechanic, stably, so the source can take it back.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum MechanicId {
    /// An ability's legendary transformation, by the ability's key.
    Legendary(&'static str),
    // The sources still to come: talents, then jewelry, grant under these.
    /// A talent's, by its `Talent.dbc` id.
    #[allow(dead_code)]
    Talent(u32),
    /// A piece of jewelry's, by its own id.
    #[allow(dead_code)]
    Jewelry(u32),
    /// Any other source, by a stable key.
    #[allow(dead_code)]
    Other(&'static str),
}

/// One granted mechanic and its clocks.
#[derive(Clone, Debug)]
struct Granted {
    id: MechanicId,
    /// Unique on this hero, so a burst it set going is refreshed rather than stacked.
    uid: u32,
    spec: MechanicSpec,
    /// Seconds before it can go off again.
    cd: f32,
    /// A periodic trigger's time into its period.
    clock: f32,
    /// A low-health trigger is armed while health stands above its threshold.
    armed: bool,
}

/// A [`Effect::Burst`] running on the hero: its bonuses come off when it ends.
#[derive(Clone, Debug)]
struct Running {
    uid: u32,
    left: f32,
    bonuses: &'static [Bonus],
}

/// Re-casts of one ability queued by [`Effect::Echo`].
#[derive(Clone, Debug)]
struct Echo {
    key: &'static str,
    /// Run seconds the next is due.
    due: f32,
    left: u8,
}

/// Every mechanic one hero carries, and their running state.
#[derive(Default, Clone, Debug)]
pub(crate) struct Mechanics {
    granted: Vec<Granted>,
    bursts: Vec<Running>,
    echoes: Vec<Echo>,
    /// Transformations just taken, whose visual plays on the hero next frame.
    flourish: Vec<u32>,
    next_uid: u32,
    /// The spells and passives learned talents teach, by key: offered to this hero, and taken
    /// at rank 2 ([`super::talents`]).
    taught: Vec<&'static str>,
    /// A next-cast guarantee waiting on the hero's next cast ([`Effect::NextCast`]).
    armed: Option<Armed>,
    /// Damage dealt by school since an [`Effect::ShareHeal`] last went off, kept only for the
    /// schools one listens to.
    pooled: [f32; 7],
    /// A Healthstone just taken, whose heal reaches the allies near ([`run::tick`]).
    healthstone: bool,
}

impl Mechanics {
    /// `specs` under `id`, replacing whatever `id` held.
    pub(crate) fn grant(&mut self, id: MechanicId, specs: impl IntoIterator<Item = MechanicSpec>) {
        self.revoke(id);
        for spec in specs {
            debug_assert!(spec.validate().is_ok(), "{id:?}: {:?}", spec.validate());
            self.next_uid += 1;
            self.granted.push(Granted {
                id,
                uid: self.next_uid,
                spec,
                cd: 0.0,
                clock: 0.0,
                armed: true,
            });
        }
    }

    /// Everything `id` granted taken back; how many mechanics went. A burst it set going runs out.
    pub(crate) fn revoke(&mut self, id: MechanicId) -> usize {
        self.revoke_where(|g| g == id)
    }

    /// Everything granted under an id `pick` chooses taken back: a whole source at once.
    pub(crate) fn revoke_where(&mut self, pick: impl Fn(MechanicId) -> bool) -> usize {
        let before = self.granted.len();
        self.granted.retain(|g| !pick(g.id));
        before - self.granted.len()
    }

    pub(crate) fn has(&self, id: MechanicId) -> bool {
        self.granted.iter().any(|g| g.id == id)
    }

    /// A learned talent teaches the spell or passive `key` for the run.
    pub(crate) fn teach(&mut self, key: &'static str) {
        if !self.taught(key) {
            self.taught.push(key);
        }
    }

    /// Whether a learned talent teaches the spell or passive `key`.
    pub(crate) fn taught(&self, key: &str) -> bool {
        self.taught.contains(&key)
    }

    /// Every granted mechanic, with its id, in the order granted.
    // For the panels that will list them; the tests read it today.
    #[allow(dead_code)]
    pub(crate) fn specs(&self) -> impl Iterator<Item = (MechanicId, &MechanicSpec)> {
        self.granted.iter().map(|g| (g.id, &g.spec))
    }

    /// Whether any mechanic waits on casts or blows, so they are worth recording.
    pub(crate) fn listens(&self) -> bool {
        self.granted.iter().any(|g| {
            matches!(
                g.spec.trigger,
                Trigger::OnCast(_)
                    | Trigger::OnHit { .. }
                    | Trigger::OnCrit { .. }
                    | Trigger::OnKill { .. }
            )
        })
    }

    /// The next-cast guarantee waiting on the hero's next cast, if one is.
    pub(crate) fn armed(&self) -> Option<Armed> {
        self.armed
    }

    /// A next-cast guarantee armed; one already held keeps the better of each part.
    pub(crate) fn arm(&mut self, armed: Armed) {
        self.armed = Some(match self.armed {
            Some(held) => Armed {
                crit: held.crit || armed.crit,
                cooldown: held.cooldown.min(armed.cooldown),
                aura: if armed.aura != 0 {
                    armed.aura
                } else {
                    held.aura
                },
            },
            None => armed,
        });
    }

    /// The next-cast guarantee spent on a cast.
    pub(crate) fn spend_armed(&mut self) -> Option<Armed> {
        self.armed.take()
    }

    /// How many times as far the hero's totems reach: 1 without [`Effect::TotemReach`].
    pub(crate) fn totem_reach(&self) -> f32 {
        self.granted
            .iter()
            .filter_map(|g| match g.spec.effect {
                Effect::TotemReach { mult } => Some(mult),
                _ => None,
            })
            .fold(1.0, f32::max)
    }

    /// `amount` of `school` damage the hero dealt, pooled for an [`Effect::ShareHeal`] that
    /// listens to the school; dropped otherwise.
    pub(crate) fn pool(&mut self, school: School, amount: f32) {
        let listens = self
            .granted
            .iter()
            .any(|g| matches!(g.spec.effect, Effect::ShareHeal { school: s, .. } if s == school));
        if listens && amount > 0.0 {
            self.pooled[school as usize] += amount;
        }
    }

    /// The `school` damage pooled so far.
    #[cfg(test)]
    pub(crate) fn pooled(&self, school: School) -> f32 {
        self.pooled[school as usize]
    }

    /// The `school` damage pooled since last asked, emptied.
    fn drain_pool(&mut self, school: School) -> f32 {
        std::mem::take(&mut self.pooled[school as usize])
    }

    /// A Healthstone was taken: its heal reaches the allies near on the next tick.
    pub(crate) fn share_healthstone(&mut self) {
        self.healthstone = true;
    }

    /// `extra` more casts of ability `key` queued, the first at run second `due`.
    fn echo(&mut self, key: &'static str, due: f32, extra: u8) {
        match self.echoes.iter_mut().find(|e| e.key == key) {
            Some(e) => e.left = e.left.max(extra),
            None => self.echoes.push(Echo {
                key,
                due,
                left: extra,
            }),
        }
    }

    /// Whether an echoed cast of `key` is due at run second `now`, spending it if so.
    pub(crate) fn take_echo(&mut self, key: &str, now: f32) -> bool {
        let Some(i) = self
            .echoes
            .iter()
            .position(|e| e.key == key && e.due <= now)
        else {
            return false;
        };
        let e = &mut self.echoes[i];
        e.left = e.left.saturating_sub(1);
        if e.left == 0 {
            self.echoes.remove(i);
        } else {
            e.due = now + ECHO_GAP;
        }
        true
    }
}

/// `specs` granted on `hero` under `id`, replacing whatever `id` held.
pub(crate) fn grant(
    hero: &mut HeroState,
    id: MechanicId,
    specs: impl IntoIterator<Item = MechanicSpec>,
) {
    hero.mechanics.grant(id, specs);
}

/// What `id` granted on `hero` taken back; how many mechanics went.
// Legendaries last the run; talents and jewelry will take theirs back.
#[allow(dead_code)]
pub(crate) fn revoke(hero: &mut HeroState, id: MechanicId) -> usize {
    hero.mechanics.revoke(id)
}

pub(crate) fn has(hero: &HeroState, id: MechanicId) -> bool {
    hero.mechanics.has(id)
}

/// A timed buff from outside the mechanics (a camp elixir, a meal): `bonuses` on `hero` for
/// `secs`, run and worn off as an [`Effect::Burst`] is. `key` must be unique to the source and
/// clear of the granted mechanics' own (small) numbers; a second application refreshes it.
pub(crate) fn timed_buff(hero: &mut HeroState, key: u32, secs: f32, bonuses: &'static [Bonus]) {
    run::burst(hero, key, secs, bonuses);
}

/// The seconds left on the buff under `key`, if it runs.
#[cfg(test)]
pub(crate) fn buff_left(hero: &HeroState, key: u32) -> Option<f32> {
    hero.mechanics
        .bursts
        .iter()
        .find(|b| b.uid == key)
        .map(|b| b.left)
}

/// `dt` on the hero's running buffs alone, as [`run::tick`] advances them.
#[cfg(test)]
pub(crate) fn tick_buffs(hero: &mut HeroState, dt: f32) {
    run::wear_bursts(hero, dt);
}

/// Every transformation `hero` has earned and lacks, granted: an ability at its top rank carries
/// its legendary for the rest of the run, however it got there. Its visual plays on the hero as
/// the hero next fights. How many were new.
pub(crate) fn sync_legendaries(hero: &mut HeroState) -> usize {
    let earned: Vec<&'static data::LegendaryDef> = hero
        .abilities
        .iter()
        .filter(|o| o.level >= data::MAX_ABILITY_LEVEL)
        .filter_map(|o| data::legendary(o.def.key))
        .filter(|l| !has(hero, MechanicId::Legendary(l.ability)))
        .collect();
    for legend in &earned {
        let id = MechanicId::Legendary(legend.ability);
        grant(hero, id, legend.mechanics.iter().copied());
        hero.mechanics.flourish.push(legend.visual_spell);
    }
    earned.len()
}

/// What happened, for the triggers to match.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum ProcKind {
    Cast,
    Hit { crit: bool, killed: bool },
}

/// A cast or a landed blow of a hero's ability, recorded for its mechanics.
#[derive(Clone, Copy)]
pub(crate) struct Proc {
    pub owner: Entity,
    pub ability: &'static AbilityDef,
    pub kind: ProcKind,
    /// The enemy struck, for a blow.
    pub target: Option<Entity>,
    /// The enemy's place for a blow, the hero's for a cast.
    pub pos: Vec3,
    /// The blow's damage; a cast's per target.
    pub damage: f32,
    /// The blow's own effect, which the projectile shapes carry on.
    pub effect: data::Effect,
}

/// This frame's casts and blows, for [`run::tick`].
#[derive(Resource, Default)]
pub(crate) struct Procs(pub Vec<Proc>);

/// Which triggers a proc of `kind` from `ability` sets off, as its chance.
pub(crate) fn fires_on(trigger: Trigger, kind: ProcKind, ability: &AbilityDef) -> Option<f32> {
    match (trigger, kind) {
        (Trigger::OnCast(of), ProcKind::Cast) if of.matches(ability) => Some(1.0),
        (Trigger::OnHit { of, chance }, ProcKind::Hit { .. }) if of.matches(ability) => {
            Some(chance)
        }
        (Trigger::OnCrit { of, chance }, ProcKind::Hit { crit: true, .. })
            if of.matches(ability) =>
        {
            Some(chance)
        }
        (Trigger::OnKill { of, chance }, ProcKind::Hit { killed: true, .. })
            if of.matches(ability) =>
        {
            Some(chance)
        }
        _ => None,
    }
}

/// Advances a periodic clock by `dt`: whether its period came round.
pub(crate) fn periodic(clock: &mut f32, every: f32, dt: f32) -> bool {
    *clock += dt;
    if *clock >= every {
        // A long frame fires once, not a burst of catch-ups.
        *clock = (*clock - every).min(every);
        true
    } else {
        false
    }
}

/// Whether health at `frac` of max just fell below `threshold`: once per fall, re-armed once it
/// stands above again.
pub(crate) fn low_health(armed: &mut bool, frac: f32, threshold: f32) -> bool {
    if frac >= threshold {
        *armed = true;
        false
    } else if *armed {
        *armed = false;
        true
    } else {
        false
    }
}

/// A roll at `p`: 1 always passes, 0 never.
pub(crate) fn chance(rng: &mut super::Rng, p: f32) -> bool {
    p >= 1.0 || (p > 0.0 && rng.f32() < p)
}

/// The extra damage [`Effect::Execute`] adds to a blow of `base` that left its target at `hp` of
/// `max_hp`: the blow `mult` times over below `threshold`, nothing above it or on the dead.
pub(crate) fn execute_bonus(hp: f32, max_hp: f32, base: f32, threshold: f32, mult: f32) -> f32 {
    if hp <= 0.0 || max_hp <= 0.0 || hp / max_hp > threshold {
        return 0.0;
    }
    base * (mult - 1.0).max(0.0)
}

/// Death turned away: back to `heal_frac` of max health (never below where it stands) and
/// untouchable for `immune` seconds.
pub(crate) fn cheat_death(hero: &mut HeroState, immune: f32, heal_frac: f32) {
    let max = hero.stats().max_hp;
    hero.hp = hero.hp.max(max * heal_frac).max(1.0);
    hero.immune_t = hero.immune_t.max(immune);
}
