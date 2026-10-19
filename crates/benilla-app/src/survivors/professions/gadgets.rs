//! Engineering's gadgets (docs/PROFESSIONS.md, 5.2, 7): bombs and grenades thrown at the thickest
//! pack, the Target Dummy the horde chases, the Goblin Rocket Boots, the Goblin Sapper Charge
//! that hurts its user too, the Mortar, the Dragon Gun, the Net-o-Matic, the Death Ray, the
//! Jumper Cables that shock a downed ally back up, and the pets that fight: the Gnomish Battle
//! Chicken, the three dragonlings and the Explosive Sheep.
//!
//! - **Using one** ([`use_gadget`], the bags or the satchel): in a run and away from camp, under
//!   its own cooldown and the shared [`ENGINEERING_LOCKOUT`]. A bomb, a dummy, a sheep or a sapper
//!   is spent; the devices are kept.
//! - **Firing** ([`fire_gadgets`]) happens where the run is simulated, so the host decides every
//!   gadget, a co-op client's included (its `UseItem`): it spawns the dummies and pets (ordinary
//!   units, replicated like any other), deals the damage and rolls the **malfunction** at the
//!   gadget's rate: the rocket boots blow up under you, the chicken and the older dragonlings turn
//!   on you, the Net-o-Matic nets you, the Death Ray finds you, the cables shock you.
//! - **The dummies and pets** run in [`run_gadget_units`]; the horde's chase turns to a dummy in
//!   reach ([`decoy_for`], `enemies::move_enemies`).
//!
//! **Real:** each gadget's use spell (whose visual plays), the summons' creature displays, the
//! Battle Squawk and the dragonlings' Flame Buffet, all checked against the client
//! ([`tests::gadget_spells_and_displays_resolve_in_the_client`]). **Authored:** every number, the
//! malfunction rates and which gadgets malfunction (the design doc's, 5.2).
//!
//! [`ENGINEERING_LOCKOUT`]: super::effects::ENGINEERING_LOCKOUT

use benilla_world::world_point::WorldPoint;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::super::abilities::Hit;
use super::super::bonus::{Bonus, School, StatKey};
use super::super::data::Effect;
use super::super::enemies::{self, Enemy};
use super::super::hero::{self, Hero};
use super::super::mechanics::{level_scale, replace_buff};
use super::super::party::{self, HeroState, LocalHero};
use super::super::units::{self, flat, yaw_toward, Fx, Marks, UnitSpec, COLOR_HURT, COLOR_SPELL};
use super::super::{Rng, Run};
use super::effects::{Category, UseCtx, Used};
use super::gather::{CastLine, UiLines};
use super::inventory::Bags;
use super::items::*;
use super::recipes::Spec;
use crate::creature_anim::{move_flags, MovementState};
use crate::net::{FieldChanged, Guid, ObjectStore};

/// How far a thrown gadget, a cone or a ray reaches for its target, in yards.
pub(crate) const THROW_RANGE: f32 = 25.0;
/// The horde within this of a decoy turns to it, bosses excepted.
pub(crate) const DECOY_PULL: f32 = 25.0;
/// A pet's reach for its foes around its owner, its melee reach, and its run.
const PET_HUNT: f32 = 25.0;
const PET_REACH: f32 = 2.5;
const PET_SPEED: f32 = 12.0;
/// How long a pet that turned on its owner stays hostile.
const TURNED_SECS: f32 = 8.0;
/// A cone's half-angle's cosine: 40 degrees either side.
const CONE_COS: f32 = 0.766;
/// How long a malfunction holds the user: the boots' knockdown, the Net-o-Matic's net.
const KNOCKDOWN_SECS: f32 = 3.0;
const NETTED_SECS: f32 = 5.0;
/// The Battle Squawk's reach, length and odds every [`SQUAWK_EVERY`] seconds.
const SQUAWK_REACH: f32 = 12.0;
const SQUAWK_SECS: f32 = 8.0;
const SQUAWK_EVERY: f32 = 12.0;
/// A dragonling's breath, every this many seconds, this wide.
const BREATH_EVERY: f32 = 4.0;
const BREATH_RADIUS: f32 = 5.0;
/// The timed buffs' keys: the boots' burst, the squawk.
const BOOTS_KEY: u32 = 0xCA4F_0201;
const SQUAWK_KEY: u32 = 0xCA4F_0202;

/// `Explosion` (12158): the boots blowing up.
pub(crate) const EXPLOSION_SPELL: u32 = 12158;
/// `Defibrillated!` (8338): the cables' shock on their user.
pub(crate) const SHOCK_SPELL: u32 = 8338;
/// `Battle Squawk` (23060): the chicken's cry.
pub(crate) const SQUAWK_SPELL: u32 = 23060;
/// `Flame Buffet` (9658): a dragonling's breath.
pub(crate) const BREATH_SPELL: u32 = 9658;
/// `Explosive Sheep` (4050): the sheep going off.
pub(crate) const SHEEP_BLAST_SPELL: u32 = 4050;

/// What kind of pet a gadget brings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PetKind {
    /// Pecks its foes, and squawks for the party.
    Chicken,
    /// Bites, and breathes fire on its foe's pack.
    Dragonling,
    /// Runs at the thickest pack and blows up.
    Sheep,
}

/// What a gadget does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) enum Kind {
    /// Thrown at the thickest pack in reach: Fire damage in `radius`, and a stun.
    Bomb { damage: f32, radius: f32, stun: f32 },
    /// Goes off where its user stands: damage in `radius`, and `self_share` of the user's health.
    Sapper {
        damage: f32,
        radius: f32,
        self_share: f32,
    },
    /// A burst in a cone toward the thickest pack: damage, and a root of `root` seconds.
    Cone { damage: f32, range: f32, root: f32 },
    /// A Shadow beam at the healthiest foe in range, at `cost` of the user's health.
    Ray { damage: f32, range: f32, cost: f32 },
    /// A decoy the horde chases for `secs`, soaking `hp_share` of its user's max health.
    Dummy {
        secs: f32,
        hp_share: f32,
        display: u32,
    },
    /// These bonuses for `secs`.
    Boots {
        bonuses: &'static [Bonus],
        secs: f32,
    },
    /// A pet that fights for `secs`: `damage` a blow every `every` seconds, a dragonling's breath
    /// `breath` on the pack, a sheep's blast `breath` in [`BREATH_RADIUS`] and more.
    Pet {
        pet: PetKind,
        secs: f32,
        display: u32,
        scale: f32,
        damage: f32,
        every: f32,
        breath: f32,
    },
    /// Shocks a downed ally in `reach` back up.
    Cables { reach: f32 },
}

/// What goes wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Malfunction {
    /// The boots explode: Fire damage and a knockdown.
    Explode,
    /// The pet turns on its owner for [`TURNED_SECS`].
    TurnsOnYou,
    /// The net lands on its user.
    NetsYou,
    /// The ray hits its user.
    HitsYou,
    /// The cables shock their user, and nobody gets up.
    Shock,
}

impl Malfunction {
    /// What the user is told.
    pub(crate) const fn line(self) -> &'static str {
        match self {
            Malfunction::Explode => "Your rocket boots explode! You are on fire.",
            Malfunction::TurnsOnYou => "Your creation turns on you!",
            Malfunction::NetsYou => "The Net-o-Matic nets you instead!",
            Malfunction::HitsYou => "The Death Ray backfires!",
            Malfunction::Shock => "The jumper cables shock you instead!",
        }
    }
}

/// One gadget.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) struct GadgetDef {
    pub(crate) item: u32,
    pub(crate) kind: Kind,
    /// Its own cooldown, in seconds.
    pub(crate) cooldown: f32,
    /// Spent on use (a bomb), or kept (a device).
    pub(crate) consumed: bool,
    /// The use spell whose visual plays (`Spell.dbc`).
    pub(crate) spell: u32,
    /// What can go wrong, and how often in a hundred.
    pub(crate) malfunction: Option<(Malfunction, u32)>,
    /// The specialisation whose schematic it is, for its tooltip.
    pub(crate) spec: Option<Spec>,
}

const fn bomb(item: u32, spell: u32, damage: f32, radius: f32, stun: f32) -> GadgetDef {
    GadgetDef {
        item,
        kind: Kind::Bomb {
            damage,
            radius,
            stun,
        },
        cooldown: 30.0,
        consumed: true,
        spell,
        malfunction: None,
        spec: None,
    }
}

const fn pet(
    item: u32,
    spell: u32,
    pet: PetKind,
    display: u32,
    scale: f32,
    numbers: (f32, f32, f32, f32),
) -> GadgetDef {
    GadgetDef {
        item,
        kind: Kind::Pet {
            pet,
            secs: numbers.0,
            display,
            scale,
            damage: numbers.1,
            every: numbers.2,
            breath: numbers.3,
        },
        cooldown: 180.0,
        consumed: false,
        spell,
        malfunction: None,
        spec: None,
    }
}

impl GadgetDef {
    const fn kept(self, cooldown: f32) -> Self {
        Self {
            consumed: false,
            cooldown,
            ..self
        }
    }

    const fn misfires(self, m: Malfunction, pct: u32) -> Self {
        Self {
            malfunction: Some((m, pct)),
            ..self
        }
    }

    const fn of(self, spec: Spec) -> Self {
        Self {
            spec: Some(spec),
            ..self
        }
    }
}

/// The rocket boots' burst: 1.12's +70% run speed, softened.
const BOOTS: &[Bonus] = &[Bonus::new(StatKey::MoveSpeed, 0.6)];
/// The squawk: shorter cooldowns for the party.
const SQUAWK: &[Bonus] = &[Bonus::new(StatKey::Cooldown, 0.08)];

/// Every gadget. Damage is at run level 1 and grows with it ([`level_scale`]); the numbers are
/// ours, sized so a gadget is a burst or an escape, never a run's power (6.1).
pub(in crate::survivors) static GADGETS: &[GadgetDef] = &[
    bomb(ROUGH_COPPER_BOMB, 4064, 30.0, 6.0, 1.0),
    bomb(IRON_GRENADE, 4068, 45.0, 6.0, 3.0),
    bomb(BIG_IRON_BOMB, 4069, 60.0, 8.0, 2.0),
    bomb(THORIUM_GRENADE, 19769, 90.0, 8.0, 3.0),
    bomb(DENSE_DYNAMITE, 23063, 120.0, 7.0, 0.0),
    GadgetDef {
        kind: Kind::Sapper {
            damage: 200.0,
            radius: 10.0,
            self_share: 0.2,
        },
        cooldown: 60.0,
        ..bomb(GOBLIN_SAPPER_CHARGE, 13241, 0.0, 0.0, 0.0)
    }
    .of(Spec::Goblin),
    bomb(GOBLIN_MORTAR, 13237, 110.0, 8.0, 2.0)
        .kept(90.0)
        .of(Spec::Goblin),
    GadgetDef {
        kind: Kind::Cone {
            damage: 70.0,
            range: 12.0,
            root: 0.0,
        },
        ..bomb(GOBLIN_DRAGON_GUN, 13184, 0.0, 0.0, 0.0)
    }
    .kept(60.0)
    .of(Spec::Goblin),
    GadgetDef {
        kind: Kind::Boots {
            bonuses: BOOTS,
            secs: 20.0,
        },
        ..bomb(GOBLIN_ROCKET_BOOTS, 8892, 0.0, 0.0, 0.0)
    }
    .kept(120.0)
    .misfires(Malfunction::Explode, 5)
    .of(Spec::Goblin),
    GadgetDef {
        kind: Kind::Cone {
            damage: 0.0,
            range: 12.0,
            root: 10.0,
        },
        ..bomb(GNOMISH_NET_O_MATIC_PROJECTOR, 13099, 0.0, 0.0, 0.0)
    }
    .kept(60.0)
    .misfires(Malfunction::NetsYou, 10)
    .of(Spec::Gnomish),
    GadgetDef {
        kind: Kind::Ray {
            damage: 400.0,
            range: 30.0,
            cost: 0.1,
        },
        ..bomb(GNOMISH_DEATH_RAY, 13278, 0.0, 0.0, 0.0)
    }
    .kept(120.0)
    .misfires(Malfunction::HitsYou, 10)
    .of(Spec::Gnomish),
    GadgetDef {
        kind: Kind::Dummy {
            secs: 10.0,
            hp_share: 0.5,
            display: 3019,
        },
        cooldown: 60.0,
        ..bomb(TARGET_DUMMY, 4071, 0.0, 0.0, 0.0)
    },
    GadgetDef {
        kind: Kind::Dummy {
            secs: 15.0,
            hp_share: 1.5,
            display: 16074,
        },
        cooldown: 60.0,
        ..bomb(MASTERWORK_TARGET_DUMMY, 19805, 0.0, 0.0, 0.0)
    },
    pet(
        GNOMISH_BATTLE_CHICKEN,
        13166,
        PetKind::Chicken,
        6909,
        1.0,
        (90.0, 18.0, 1.5, 0.0),
    )
    .misfires(Malfunction::TurnsOnYou, 10)
    .of(Spec::Gnomish),
    pet(
        MECHANICAL_DRAGONLING,
        4073,
        PetKind::Dragonling,
        4465,
        1.0,
        (45.0, 14.0, 1.5, 20.0),
    )
    .misfires(Malfunction::TurnsOnYou, 5),
    pet(
        MITHRIL_MECHANICAL_DRAGONLING,
        12749,
        PetKind::Dragonling,
        7908,
        1.15,
        (60.0, 22.0, 1.5, 35.0),
    )
    .misfires(Malfunction::TurnsOnYou, 3),
    pet(
        ARCANITE_DRAGONLING,
        19804,
        PetKind::Dragonling,
        12489,
        1.25,
        (60.0, 32.0, 1.5, 55.0),
    ),
    GadgetDef {
        consumed: true,
        cooldown: 30.0,
        ..pet(
            EXPLOSIVE_SHEEP,
            4074,
            PetKind::Sheep,
            3886,
            1.0,
            (8.0, 0.0, 0.0, 80.0),
        )
    },
    GadgetDef {
        kind: Kind::Cables { reach: 15.0 },
        ..bomb(GOBLIN_JUMPER_CABLES, 8342, 0.0, 0.0, 0.0)
    }
    .kept(60.0)
    .misfires(Malfunction::Shock, 33),
];

/// What `item` does as a gadget, if it is one.
pub(in crate::survivors) fn gadget(item: u32) -> Option<&'static GadgetDef> {
    GADGETS.iter().find(|g| g.item == item)
}

/// The tooltip's green `Use:` line, from what the gadget does.
pub(in crate::survivors) fn use_line(g: &GadgetDef) -> String {
    let what = match g.kind {
        Kind::Bomb {
            damage,
            radius,
            stun,
        } if stun > 0.0 => format!(
            "Hurls an explosive at the thickest pack: {damage:.0} Fire damage in {radius:.0} \
             yards, stunning for {stun:.0} sec."
        ),
        Kind::Bomb { damage, radius, .. } => format!(
            "Hurls an explosive at the thickest pack: {damage:.0} Fire damage in {radius:.0} \
             yards."
        ),
        Kind::Sapper {
            damage,
            radius,
            self_share,
        } => format!(
            "Explodes where you stand: {damage:.0} Fire damage in {radius:.0} yards, and {:.0}% \
             of your own health.",
            self_share * 100.0
        ),
        Kind::Cone { damage, root, .. } if root > 0.0 => format!(
            "Fires a net at the thickest pack before you, rooting it for {root:.0} sec.{}",
            if damage > 0.0 {
                format!(" {damage:.0} damage.")
            } else {
                String::new()
            }
        ),
        Kind::Cone { damage, .. } => {
            format!("Breathes fire at the pack before you: {damage:.0} Fire damage.")
        }
        Kind::Ray { damage, cost, .. } => format!(
            "A beam of {damage:.0} Shadow damage at the healthiest foe in sight, drawn from {:.0}% \
             of your own health.",
            cost * 100.0
        ),
        Kind::Dummy { secs, .. } => format!("The horde chases a target dummy for {secs:.0} sec."),
        Kind::Boots { bonuses, secs } => format!(
            "Rocket-powered: {} for {secs:.0} sec.",
            bonuses
                .iter()
                .map(Bonus::describe)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Kind::Pet {
            pet: PetKind::Sheep,
            breath,
            ..
        } => format!("A sheep runs at the thickest pack and explodes for {breath:.0} damage."),
        Kind::Pet { pet, secs, .. } => format!(
            "Summons a {} that fights beside you for {secs:.0} sec.",
            match pet {
                PetKind::Chicken => "battle chicken",
                _ => "dragonling",
            }
        ),
        Kind::Cables { .. } => "Shocks a fallen ally back to their feet.".to_string(),
    };
    let wrong = g
        .malfunction
        .map(|(_, pct)| format!(" It malfunctions {pct}% of the time."))
        .unwrap_or_default();
    let spec = g
        .spec
        .map(|s| format!(" ({} schematic.)", s.name()))
        .unwrap_or_default();
    format!("{what}{wrong}{spec}")
}

/// A gadget used from the bags at `at`: in a run, away from camp, off its cooldowns; spent if it
/// is spent; its own cooldown and the engineering lockout started.
pub(in crate::survivors) fn use_gadget(
    bags: &mut Bags,
    g: &'static GadgetDef,
    at: (i64, u32),
    ctx: UseCtx,
) -> Used {
    if !ctx.in_run {
        return Used::Refused("ERR_USE_CANT_IMMUNE", "You can't do that right now.");
    }
    if ctx.at_camp {
        return Used::Refused("SPELL_FAILED_NOT_HERE", "Not at camp.");
    }
    let own = bags
        .own_cooldown(g.item)
        .is_some_and(|start| ctx.now < start + f64::from(g.cooldown));
    if own || bags.cooldown_running(Category::Engineering, ctx.now) {
        return Used::NotReady;
    }
    if g.consumed && bags.destroy(at.0, at.1, 1).is_err() {
        return Used::Nothing;
    }
    bags.start_own_cooldown(g.item, ctx.now);
    bags.start_cooldown(Category::Engineering, ctx.now);
    Used::Gadget(g)
}

/// Whether this use misfires: `roll` in 0..100 against the gadget's rate.
pub(in crate::survivors) fn misfires(g: &GadgetDef, roll: usize) -> Option<Malfunction> {
    g.malfunction
        .filter(|&(_, pct)| roll < pct as usize)
        .map(|(m, _)| m)
}

/// The gadgets fired this frame, `(user's hero, item)`: the local player's, and (on a co-op host)
/// the clients' `UseItem`s.
#[derive(Resource, Default)]
pub(crate) struct GadgetQueue(pub(crate) Vec<(Entity, u32)>);

/// A target dummy the horde chases: its health, the seconds it has left, and its user.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct Decoy {
    pub(crate) hp: f32,
    pub(crate) left: f32,
    pub(crate) owner: Entity,
}

impl Decoy {
    pub(crate) fn standing(&self) -> bool {
        self.hp > 0.0 && self.left > 0.0
    }
}

/// The decoy an enemy at `pos` turns to: the nearest standing one within [`DECOY_PULL`], unless
/// it is a boss.
pub(crate) fn decoy_for(
    pos: Vec3,
    boss: bool,
    decoys: &[(Entity, Vec3)],
) -> Option<(Entity, Vec3)> {
    if boss {
        return None;
    }
    decoys
        .iter()
        .map(|&(e, at)| (e, at, flat(at - pos).length()))
        .filter(|&(.., d)| d <= DECOY_PULL)
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(e, at, _)| (e, at))
}

/// A gadget's pet in the world.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct GadgetPet {
    pub(crate) owner: Entity,
    pub(crate) kind: PetKind,
    /// Seconds it has left.
    pub(crate) left: f32,
    /// Seconds to its next blow, breath and squawk.
    next: f32,
    next_breath: f32,
    next_squawk: f32,
    damage: f32,
    every: f32,
    breath: f32,
    /// Seconds it stays turned on its owner.
    pub(crate) turned: f32,
}

/// The thickest pack among `foes` within `range` of `from`: the place with the most others within
/// `radius` of it.
pub(crate) fn thickest(foes: &[Vec3], from: Vec3, range: f32, radius: f32) -> Option<Vec3> {
    let near: Vec<Vec3> = foes
        .iter()
        .copied()
        .filter(|p| flat(*p - from).length() <= range)
        .collect();
    near.iter()
        .map(|&p| {
            let n = near
                .iter()
                .filter(|o| flat(**o - p).length() <= radius)
                .count();
            (p, n)
        })
        .max_by_key(|&(_, n)| n)
        .map(|(p, _)| p)
}

/// What firing a gadget reaches.
#[derive(SystemParam)]
pub(in crate::survivors) struct Firing<'w, 's> {
    commands: Commands<'w, 's>,
    run: ResMut<'w, Run>,
    rng: ResMut<'w, Rng>,
    fx: Fx<'w>,
    hits: MessageWriter<'w, Hit>,
    fields: MessageWriter<'w, FieldChanged>,
    ui: ResMut<'w, UiLines>,
    queue: ResMut<'w, GadgetQueue>,
}

/// A hero as a gadget sees it.
type HeroRow = (
    Entity,
    &'static Guid,
    &'static Transform,
    &'static mut HeroState,
    &'static mut ObjectStore,
    &'static mut Marks,
    Has<LocalHero>,
);

/// `raw` damage to a gadget's own user, shown over it.
fn hurt_user(heroes: &mut Query<HeroRow, With<Hero>>, fx: &mut Fx, user: Entity, raw: f32) {
    if let Ok((_, _, _, mut h, ..)) = heroes.get_mut(user) {
        let taken = hero::hurt_hero(&mut h, raw);
        if taken > 0.0 {
            fx.number(user, taken, false, COLOR_HURT);
        }
    }
}

/// Where the run is simulated: this frame's gadgets go off (see the module doc).
pub(in crate::survivors) fn fire_gadgets(
    mut f: Firing,
    mut heroes: Query<HeroRow, With<Hero>>,
    foes: Query<(Entity, &Transform, &Enemy), Without<Hero>>,
) {
    if f.queue.0.is_empty() {
        return;
    }
    let level = f.run.level;
    let scale = level_scale(level);
    let standing: Vec<(Entity, Vec3, f32, f32)> = foes
        .iter()
        .filter(|(.., e)| !e.dead)
        .map(|(e, t, en)| (e, t.translation, en.hp, en.radius))
        .collect();
    let places: Vec<Vec3> = standing.iter().map(|s| s.1).collect();
    for (user, item) in std::mem::take(&mut f.queue.0) {
        let Some(g) = gadget(item) else {
            continue;
        };
        let Ok((_, _, tf, hero, _, _, local)) = heroes.get(user) else {
            continue;
        };
        if !hero.alive() {
            continue;
        }
        let (at, max_hp) = (tf.translation, hero.stats().max_hp);
        let misfire = misfires(g, f.rng.index(100));
        if let Some(m) = misfire {
            if local {
                f.ui.0.push(CastLine::Malfunction(m.line()));
            }
            f.fx.text(user, "Malfunction!".to_string(), COLOR_HURT);
        }
        // The foes within `radius` of `centre`.
        let within = |centre: Vec3, radius: f32| -> Vec<Entity> {
            standing
                .iter()
                .filter(|s| flat(s.1 - centre).length() <= radius + s.3 * 0.5)
                .map(|s| s.0)
                .collect()
        };
        match g.kind {
            Kind::Bomb {
                damage,
                radius,
                stun,
            } => {
                let target = thickest(&places, at, THROW_RANGE, radius).unwrap_or(at);
                let struck = within(target, radius);
                f.fx.cast(user, g.spell, &struck, Some(target));
                for &foe in &struck {
                    let mut hit = Hit {
                        color: COLOR_SPELL,
                        school: Some(School::Fire),
                        ..Hit::direct(user, foe, damage * scale, false)
                    };
                    if stun > 0.0 {
                        hit.effect = Effect::Stun(stun);
                        hit.mark = g.spell;
                        hit.mark_dur = stun;
                    }
                    f.hits.write(hit);
                }
            }
            Kind::Sapper {
                damage,
                radius,
                self_share,
            } => {
                let struck = within(at, radius);
                f.fx.cast(user, g.spell, &struck, Some(at));
                for &foe in &struck {
                    f.hits.write(Hit {
                        color: COLOR_SPELL,
                        school: Some(School::Fire),
                        ..Hit::direct(user, foe, damage * scale, false)
                    });
                }
                hurt_user(&mut heroes, &mut f.fx, user, max_hp * self_share);
            }
            Kind::Cone {
                damage,
                range,
                root,
            } => {
                if misfire == Some(Malfunction::NetsYou) {
                    if let Ok((_, _, _, mut h, ..)) = heroes.get_mut(user) {
                        h.raid.hold(NETTED_SECS);
                    }
                    f.fx.impact(user, g.spell);
                    continue;
                }
                let Some(aim) = thickest(&places, at, range, 6.0) else {
                    continue;
                };
                let dir = flat(aim - at).normalize_or(Vec3::X);
                let struck: Vec<Entity> = standing
                    .iter()
                    .filter(|s| {
                        let to = flat(s.1 - at);
                        to.length() <= range && to.normalize_or(dir).dot(dir) >= CONE_COS
                    })
                    .map(|s| s.0)
                    .collect();
                f.fx.cast(user, g.spell, &struck, Some(aim));
                for &foe in &struck {
                    let mut hit = Hit {
                        color: COLOR_SPELL,
                        school: Some(School::Fire),
                        ..Hit::direct(user, foe, damage * scale, false)
                    };
                    if root > 0.0 {
                        hit.effect = Effect::Root(root);
                        hit.mark = g.spell;
                        hit.mark_dur = root;
                    }
                    f.hits.write(hit);
                }
            }
            Kind::Ray {
                damage,
                range,
                cost,
            } => {
                if misfire == Some(Malfunction::HitsYou) {
                    f.fx.impact(user, g.spell);
                    hurt_user(&mut heroes, &mut f.fx, user, max_hp * 0.25);
                    continue;
                }
                let target = standing
                    .iter()
                    .filter(|s| flat(s.1 - at).length() <= range)
                    .max_by(|a, b| a.2.total_cmp(&b.2));
                if let Some(&(foe, ..)) = target {
                    f.fx.cast(user, g.spell, &[foe], None);
                    f.hits.write(Hit {
                        color: COLOR_SPELL,
                        school: Some(School::Shadow),
                        ..Hit::direct(user, foe, damage * scale, false)
                    });
                    hurt_user(&mut heroes, &mut f.fx, user, max_hp * cost);
                }
            }
            Kind::Dummy {
                secs,
                hp_share,
                display,
            } => {
                let pos = at + Vec3::new(2.0, 0.0, 0.0);
                let e = spawn_unit(&mut f.commands, &mut f.run, display, 1.0, pos, at);
                f.commands.entity(e).insert(Decoy {
                    hp: max_hp * hp_share,
                    left: secs,
                    owner: user,
                });
                f.fx.cast(user, g.spell, &[], Some(pos));
            }
            Kind::Boots { bonuses, secs } => {
                if misfire == Some(Malfunction::Explode) {
                    if let Ok((_, _, _, mut h, ..)) = heroes.get_mut(user) {
                        h.raid.hold(KNOCKDOWN_SECS);
                    }
                    f.fx.impact(user, EXPLOSION_SPELL);
                    hurt_user(&mut heroes, &mut f.fx, user, max_hp * 0.15);
                    continue;
                }
                if let Ok((_, _, _, mut h, ..)) = heroes.get_mut(user) {
                    replace_buff(&mut h, BOOTS_KEY, secs, bonuses);
                }
                f.fx.cast(user, g.spell, &[], None);
            }
            Kind::Pet {
                pet,
                secs,
                display,
                scale: size,
                damage,
                every,
                breath,
            } => {
                let pos = at + Vec3::new(-1.5, 0.0, 1.5);
                let e = spawn_unit(&mut f.commands, &mut f.run, display, size, pos, at);
                f.commands.entity(e).insert(GadgetPet {
                    owner: user,
                    kind: pet,
                    left: secs,
                    next: every,
                    next_breath: BREATH_EVERY,
                    next_squawk: SQUAWK_EVERY,
                    damage: damage * scale,
                    every,
                    breath: breath * scale,
                    turned: if misfire == Some(Malfunction::TurnsOnYou) {
                        TURNED_SECS
                    } else {
                        0.0
                    },
                });
                f.fx.cast(user, g.spell, &[], Some(pos));
            }
            Kind::Cables { reach } => {
                let fallen = heroes
                    .iter()
                    .filter(|r| r.0 != user && !r.3.alive())
                    .map(|r| (r.0, flat(r.2.translation - at).length()))
                    .filter(|&(_, d)| d <= reach)
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(e, _)| e);
                let Some(ally) = fallen else {
                    continue;
                };
                if misfire == Some(Malfunction::Shock) {
                    f.fx.impact(user, SHOCK_SPELL);
                    hurt_user(&mut heroes, &mut f.fx, user, max_hp * 0.1);
                    continue;
                }
                if let Ok((e, guid, _, mut h, mut store, mut marks, _)) = heroes.get_mut(ally) {
                    party::get_up(&mut h);
                    units::raise_unit(&mut store);
                    marks.clear();
                    marks.flush(e, guid.0, &mut store, &mut f.fields);
                }
                f.fx.cast(user, g.spell, &[ally], None);
            }
        }
    }
}

/// A gadget's unit, friendly to the heroes, facing `toward`.
fn spawn_unit(
    commands: &mut Commands,
    run: &mut Run,
    display: u32,
    scale: f32,
    pos: Vec3,
    toward: Vec3,
) -> Entity {
    units::spawn_unit(
        commands,
        &UnitSpec {
            serial: run.guid(),
            display,
            scale,
            level: 60,
            faction: units::FACTION_FRIENDLY,
            bytes0: 1 << 8,
            weapons: &[],
            sheath: 0,
        },
        pos,
        yaw_toward(toward - pos),
    )
}

/// What the gadgets' units read and write as they run.
#[derive(SystemParam)]
pub(super) struct Units<'w, 's> {
    commands: Commands<'w, 's>,
    time: Res<'w, Time>,
    rng: ResMut<'w, Rng>,
    fx: Fx<'w>,
    hits: MessageWriter<'w, Hit>,
    world: WorldPoint<'w, 's>,
}

/// Where the run is simulated: the pets hunt, bite, breathe, squawk and turn; the sheep runs and
/// blows up; the dummies and pets go when their time or health is up.
#[allow(clippy::type_complexity)]
pub(super) fn run_gadget_units(
    mut u: Units,
    mut heroes: Query<(Entity, &Transform, &mut HeroState), (With<Hero>, Without<GadgetPet>)>,
    mut pets: Query<
        (
            Entity,
            &mut Transform,
            Option<&mut MovementState>,
            &mut GadgetPet,
        ),
        (Without<Hero>, Without<Enemy>, Without<Decoy>),
    >,
    mut decoys: Query<(Entity, &mut Decoy, &mut ObjectStore), (Without<GadgetPet>, Without<Hero>)>,
    foes: Query<(Entity, &Transform, &Enemy), (Without<Hero>, Without<GadgetPet>)>,
) {
    let dt = u.time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    for (e, mut d, mut store) in &mut decoys {
        d.left -= dt;
        if !d.standing() {
            units::kill_unit(&mut store);
            u.commands
                .entity(e)
                .remove::<Decoy>()
                .insert(benilla_world::model_fade::DespawnFade::default());
        }
    }
    let standing: Vec<(Entity, Vec3, f32)> = foes
        .iter()
        .filter(|(.., en)| !en.dead)
        .map(|(e, t, en)| (e, t.translation, en.radius))
        .collect();
    let places: Vec<Vec3> = standing.iter().map(|s| s.1).collect();
    for (me, mut tf, mv, mut p) in &mut pets {
        p.left -= dt;
        p.next -= dt;
        p.next_breath -= dt;
        p.next_squawk -= dt;
        p.turned = (p.turned - dt).max(0.0);
        let owner_at = heroes.get(p.owner).ok().map(|(_, t, _)| t.translation);
        let pos = tf.translation;
        let gone = p.left <= 0.0 || owner_at.is_none();
        // The sheep: at the thickest pack, or out of time, it goes off.
        if p.kind == PetKind::Sheep {
            let aim = thickest(&places, pos, PET_HUNT, BREATH_RADIUS);
            let arrived = aim.is_some_and(|a| flat(a - pos).length() <= 2.0);
            if arrived || gone {
                let struck: Vec<Entity> = standing
                    .iter()
                    .filter(|s| flat(s.1 - pos).length() <= BREATH_RADIUS + 3.0)
                    .map(|s| s.0)
                    .collect();
                u.fx.cast(me, SHEEP_BLAST_SPELL, &struck, None);
                for &foe in &struck {
                    u.hits.write(Hit {
                        color: COLOR_SPELL,
                        school: Some(School::Fire),
                        ..Hit::direct(p.owner, foe, p.breath, false)
                    });
                }
                u.commands
                    .entity(me)
                    .remove::<GadgetPet>()
                    .insert(benilla_world::model_fade::DespawnFade::default());
                continue;
            }
            step(&mut tf, mv, &u.world, aim.unwrap_or(pos), PET_SPEED, dt);
            continue;
        }
        if gone {
            u.commands
                .entity(me)
                .remove::<GadgetPet>()
                .insert(benilla_world::model_fade::DespawnFade::default());
            continue;
        }
        let owner_at = owner_at.unwrap_or(pos);
        // Turned on its owner: it runs at them and pecks or bites them.
        if p.turned > 0.0 {
            if flat(owner_at - pos).length() <= PET_REACH + 1.0 {
                if p.next <= 0.0 {
                    p.next = p.every;
                    u.fx.swing(me, p.owner, p.damage * 0.5);
                    if let Ok((.., mut h)) = heroes.get_mut(p.owner) {
                        hero::hurt_hero(&mut h, p.damage * 0.5);
                    }
                }
                step(&mut tf, mv, &u.world, pos, 0.0, dt);
            } else {
                step(&mut tf, mv, &u.world, owner_at, PET_SPEED, dt);
            }
            continue;
        }
        // The nearest foe near its owner.
        let prey = standing
            .iter()
            .filter(|s| flat(s.1 - owner_at).length() <= PET_HUNT)
            .min_by(|a, b| {
                flat(a.1 - pos)
                    .length()
                    .total_cmp(&flat(b.1 - pos).length())
            })
            .copied();
        let Some((foe, foe_at, radius)) = prey else {
            let heel = owner_at + Vec3::new(-2.0, 0.0, 2.0);
            step(&mut tf, mv, &u.world, heel, PET_SPEED, dt);
            continue;
        };
        if flat(foe_at - pos).length() > PET_REACH + radius {
            step(&mut tf, mv, &u.world, foe_at, PET_SPEED, dt);
            continue;
        }
        step(&mut tf, mv, &u.world, pos, 0.0, dt);
        let face = Quat::from_rotation_y(yaw_toward(foe_at - pos));
        tf.rotation = tf.rotation.slerp(face, (dt * 10.0).min(1.0));
        if p.next <= 0.0 {
            p.next = p.every;
            u.fx.swing(me, foe, p.damage);
            u.hits.write(Hit::direct(p.owner, foe, p.damage, false));
        }
        if p.kind == PetKind::Dragonling && p.next_breath <= 0.0 {
            p.next_breath = BREATH_EVERY;
            let struck: Vec<Entity> = standing
                .iter()
                .filter(|s| flat(s.1 - foe_at).length() <= BREATH_RADIUS)
                .map(|s| s.0)
                .collect();
            u.fx.cast(me, BREATH_SPELL, &struck, None);
            for &hit in &struck {
                u.hits.write(Hit {
                    color: COLOR_SPELL,
                    school: Some(School::Fire),
                    ..Hit::direct(p.owner, hit, p.breath, false)
                });
            }
        }
        if p.kind == PetKind::Chicken && p.next_squawk <= 0.0 {
            p.next_squawk = SQUAWK_EVERY;
            if u.rng.index(2) == 0 {
                u.fx.cast(me, SQUAWK_SPELL, &[], None);
                for (_, at, mut h) in &mut heroes {
                    if h.alive() && flat(at.translation - pos).length() <= SQUAWK_REACH {
                        replace_buff(&mut h, SQUAWK_KEY, SQUAWK_SECS, SQUAWK);
                    }
                }
            }
        }
    }
}

/// One frame of a gadget unit's walk toward `to` at `speed` (0 stands).
fn step(
    tf: &mut Transform,
    mv: Option<Mut<MovementState>>,
    world: &WorldPoint,
    to: Vec3,
    speed: f32,
    dt: f32,
) {
    let pos = tf.translation;
    let way = flat(to - pos);
    let dist = way.length();
    let moving = speed > 0.0 && dist > 0.3;
    if moving {
        let dir = way / dist;
        let next = pos + dir * (speed * dt).min(dist);
        tf.translation = enemies::ground(world, next, pos.y);
        let face = Quat::from_rotation_y(yaw_toward(dir));
        tf.rotation = tf.rotation.slerp(face, (dt * 10.0).min(1.0));
    }
    if let Some(mut mv) = mv {
        if moving {
            mv.speed = speed;
            mv.flags = move_flags::FORWARD;
        } else if mv.speed != 0.0 || mv.flags != 0 {
            mv.speed = 0.0;
            mv.flags = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every gadget is an authored item, made by a recipe; the devices are kept and the bombs
    /// spent; the specialisations' gadgets are their schematics'.
    #[test]
    fn survivors_gadgets_are_authored_and_made() {
        use super::super::recipes;
        let mut seen = std::collections::HashSet::new();
        for g in GADGETS {
            assert!(seen.insert(g.item), "{} twice", g.item);
            let def = template(g.item).expect("authored");
            let recipe = recipes::all()
                .find(|r| r.product == g.item)
                .unwrap_or_else(|| panic!("no recipe makes {}", def.name));
            assert_eq!(recipe.spec, g.spec, "{}", def.name);
            assert!(g.cooldown >= 30.0, "{}", def.name);
            if let Some((_, pct)) = g.malfunction {
                assert!((1..=33).contains(&pct), "{}", def.name);
            }
            assert!(super::super::effects::usable(g.item), "{}", def.name);
            assert!(!use_line(g).is_empty());
        }
        assert!(gadget(SILVER_BAR).is_none());
        assert!(use_line(gadget(GOBLIN_SAPPER_CHARGE).unwrap()).contains("20% of your own"));
    }

    /// Each malfunction rolls at its gadget's rate over 10,000 seeded uses, give or take.
    #[test]
    fn survivors_gadgets_malfunction_at_their_rates() {
        let mut rng = Rng(77);
        for g in GADGETS {
            let mut wrong = 0;
            for _ in 0..10_000 {
                if misfires(g, rng.index(100)).is_some() {
                    wrong += 1;
                }
            }
            let want = g.malfunction.map_or(0, |(_, pct)| pct * 100);
            assert!(
                (wrong as i64 - want as i64).abs() <= 250,
                "{}: {wrong} of 10,000, want about {want}",
                g.item
            );
        }
        let chicken = gadget(GNOMISH_BATTLE_CHICKEN).unwrap();
        assert_eq!(misfires(chicken, 9), Some(Malfunction::TurnsOnYou));
        assert_eq!(misfires(chicken, 10), None);
        assert_eq!(
            misfires(gadget(ARCANITE_DRAGONLING).unwrap(), 0),
            None,
            "honest"
        );
    }

    /// Bombs are spent and devices kept; each waits on its own cooldown and on the engineering
    /// lockout; none goes off out of a run or at camp.
    #[test]
    fn survivors_gadgets_use_their_cooldowns() {
        let mut bags = Bags::default();
        bags.add(ROUGH_COPPER_BOMB, 2);
        bags.add(GOBLIN_ROCKET_BOOTS, 1);
        let run = UseCtx {
            now: 100.0,
            in_run: true,
            at_camp: false,
            run_secs: 200.0,
        };
        let bomb = gadget(ROUGH_COPPER_BOMB).unwrap();
        let boots = gadget(GOBLIN_ROCKET_BOOTS).unwrap();
        assert!(matches!(
            use_gadget(
                &mut bags,
                bomb,
                (0, 1),
                UseCtx {
                    at_camp: true,
                    ..run
                }
            ),
            Used::Refused(..)
        ));
        assert!(matches!(
            use_gadget(&mut bags, bomb, (0, 1), UseCtx::default()),
            Used::Refused(..)
        ));
        assert_eq!(use_gadget(&mut bags, bomb, (0, 1), run), Used::Gadget(bomb));
        assert_eq!(bags.count(ROUGH_COPPER_BOMB), 1, "spent");
        let later = |now: f64| UseCtx { now, ..run };
        assert_eq!(
            use_gadget(&mut bags, boots, (0, 2), later(105.0)),
            Used::NotReady,
            "the engineering lockout"
        );
        assert_eq!(
            use_gadget(&mut bags, boots, (0, 2), later(111.0)),
            Used::Gadget(boots)
        );
        assert_eq!(bags.count(GOBLIN_ROCKET_BOOTS), 1, "kept");
        assert_eq!(
            use_gadget(&mut bags, boots, (0, 2), later(200.0)),
            Used::NotReady,
            "its own two minutes"
        );
        assert_eq!(
            super::super::effects::cooldown_of(&bags, GOBLIN_ROCKET_BOOTS),
            Some((111.0, 120.0))
        );
        assert_eq!(
            use_gadget(&mut bags, boots, (0, 2), later(232.0)),
            Used::Gadget(boots)
        );
    }

    /// A target dummy pulls the horde within its reach, never a boss, and the nearest one wins.
    #[test]
    fn survivors_gadgets_the_target_dummy_redirects_the_chase() {
        let mut world = World::new();
        let far = world.spawn_empty().id();
        let near = world.spawn_empty().id();
        let decoys = [
            (far, Vec3::new(20.0, 0.0, 0.0)),
            (near, Vec3::new(5.0, 0.0, 0.0)),
        ];
        assert_eq!(decoy_for(Vec3::ZERO, false, &decoys), Some(decoys[1]));
        assert_eq!(
            decoy_for(Vec3::ZERO, true, &decoys),
            None,
            "bosses ignore it"
        );
        assert_eq!(
            decoy_for(Vec3::new(-30.0, 0.0, 0.0), false, &decoys),
            None,
            "out of its pull"
        );
        assert!(Decoy {
            hp: 1.0,
            left: 1.0,
            owner: near
        }
        .standing());
        assert!(!Decoy {
            hp: 0.0,
            left: 1.0,
            owner: near
        }
        .standing());
    }

    #[test]
    fn survivors_gadgets_aim_at_the_thickest_pack() {
        let foes = [
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(-10.0, 0.0, 0.0),
            Vec3::new(-11.0, 0.0, 0.0),
            Vec3::new(-10.0, 0.0, 1.0),
            Vec3::new(100.0, 0.0, 0.0),
        ];
        let at = thickest(&foes, Vec3::ZERO, THROW_RANGE, 3.0).unwrap();
        assert!(at.x < 0.0, "{at}");
        assert!(thickest(&foes[4..], Vec3::ZERO, THROW_RANGE, 3.0).is_none());
    }

    /// The gadgets are real: each use spell is named for its gadget (or its effect), the summons'
    /// displays are the right creatures, and the squawk, the breath and the malfunctions' spells
    /// are what they say. Every mismatch is listed.
    #[test]
    fn gadget_spells_and_displays_resolve_in_the_client() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open the MPQ chain");
        let spells = benilla_formats::load_spell_catalog(&mut chain).expect("Spell.dbc");
        let creatures = benilla_formats::load_creature_catalog(&mut chain).expect("creatures");
        let mut bad = Vec::new();
        let name = |id: u32| spells.get(id).map(|s| s.name.clone());
        for g in GADGETS {
            let def = template(g.item).unwrap();
            let want = match g.item {
                GNOMISH_NET_O_MATIC_PROJECTOR => "Net-o-Matic",
                GOBLIN_JUMPER_CABLES => "Defibrillate",
                GNOMISH_BATTLE_CHICKEN => "Battle Chicken",
                _ => def.name,
            };
            if name(g.spell).as_deref() != Some(want) {
                bad.push(format!(
                    "{}: spell {} is {:?}",
                    def.name,
                    g.spell,
                    name(g.spell)
                ));
            }
            let display = match g.kind {
                Kind::Dummy { display, .. } | Kind::Pet { display, .. } => display,
                _ => continue,
            };
            let model = creatures
                .model(display)
                .map(|m| (m.model_path.to_ascii_lowercase(), m.textures[0].clone()));
            let ok = match model {
                Some((path, tex)) => match g.kind {
                    Kind::Dummy { .. } => path.contains("woodendummy"),
                    Kind::Pet {
                        pet: PetKind::Chicken,
                        ..
                    } => path.contains("chicken") && tex.as_deref() == Some("ChickenSkinRobot"),
                    Kind::Pet {
                        pet: PetKind::Sheep,
                        ..
                    } => path.contains("sheep") && tex.as_deref() == Some("SheepRobo"),
                    _ => {
                        path.contains("dragonwhelp")
                            && tex.as_deref() == Some("DragonWhelpSkinMechanical")
                    }
                },
                None => false,
            };
            if !ok {
                bad.push(format!("{}: display {display}", def.name));
            }
        }
        for (id, want) in [
            (EXPLOSION_SPELL, "Explosion"),
            (SHOCK_SPELL, "Defibrillated!"),
            (SQUAWK_SPELL, "Battle Squawk"),
            (BREATH_SPELL, "Flame Buffet"),
            (SHEEP_BLAST_SPELL, "Explosive Sheep"),
            (super::super::effects::STONE_SPELL, "Sharpen Blade"),
            (super::super::effects::KIT_SPELL, "Armor +16"),
        ] {
            if name(id).as_deref() != Some(want) {
                bad.push(format!("spell {id} is {:?}, not {want}", name(id)));
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }
}
