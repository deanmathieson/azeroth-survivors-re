//! The horde: spawning on a ring outside the view, the chase, contact damage, death, and the
//! experience gems and coins the dead leave.

use std::collections::HashMap;

use avian3d::prelude::Collider;
use benilla_world::collision::WorldCollision;
use benilla_world::rig_anim::GlobalSeqDrive;
use benilla_world::world_point::WorldPoint;
use bevy::math::Dir3;
use bevy::prelude::*;

use super::abilities::Hit;
use super::bonus::School;
use super::data::{self, EnemyDef};
use super::hero::{self, Hero, HERO_RADIUS};
use super::party;
use super::run_options::{self, RunOptions, XpCarry};
use super::synergy;
use super::treasure;
use super::units::{self, flat, yaw_toward, Fx, Marks, UnitSpec, COLOR_HURT, COLOR_SPELL};
use super::{HeroState, LocalHero, Phase, Rng, Run, Selection};
use crate::creature_anim::{move_flags, MovementState};
use crate::net::{FieldChanged, Guid, ObjectStore};

#[derive(Component)]
pub(super) struct Enemy {
    pub def: &'static EnemyDef,
    pub hp: f32,
    pub max_hp: f32,
    pub speed: f32,
    pub damage: f32,
    pub radius: f32,
    pub xp: u32,
    pub boss: bool,
    /// An elite or a guardian: tougher, marked, and worth a treasure cache.
    pub elite: bool,
    pub guardian: bool,
    /// A guardian holding its post until the hero comes near or hurts it.
    pub dormant: bool,
    pub attack_cd: f32,
    pub slow: f32,
    pub slow_t: f32,
    pub root_t: f32,
    pub stun_t: f32,
    pub fear_t: f32,
    pub weaken: f32,
    pub weaken_t: f32,
    pub dots: Vec<Dot>,
    pub dead: bool,
    /// The hero it must go for until the run second given, whatever else is near: a tank's taunt
    /// ([`super::synergy`]), over [`party::pick_target`]'s pick.
    pub taunted: Option<(Entity, f32)>,
    /// The party's shared debuffs on it, which raise what every hero's blows deal it.
    pub debuffs: synergy::Debuffs,
}

pub(super) struct Dot {
    pub dps: f32,
    pub left: f32,
    pub tick: f32,
    /// The hero whose spell it is, credited with its ticks.
    pub source: Entity,
    /// The spell's school, where known, for the debuffs that raise it.
    pub school: Option<School>,
}

/// A corpse playing its death before it fades.
#[derive(Component)]
pub(super) struct Dying(f32);

#[derive(Resource)]
pub(super) struct Spawner {
    timer: f32,
    /// What the Run Options density split owes the next spawns.
    pub xp_carry: XpCarry,
    /// Spawns made round a standing hero so far: the heroes take turns ([`party::spawn_anchor`]).
    pub turn: u32,
    /// The run second of the next host-authoritative micro-event.  It shares the horde's clock,
    /// so clients never independently roll an event.
    micro_next: f32,
    micro_kind: u8,
}

/// The run second the first micro-event comes at: the opening is the horde's alone.
pub(super) const MICRO_FIRST_SECS: f32 = 45.0;
/// Seconds from one micro-event to the next.
pub(super) const MICRO_EVERY_SECS: f32 = 45.0;
/// No micro-event this close before a roster boss walks out: it waits until the boss is out.
const MICRO_BOSS_CLEARANCE: f32 = 15.0;

/// The goblin Treasure Courier: a quick, frail goblin (`GoblinMale.mdx`) that flees the party,
/// dripping coins, and spills its purse when it is slain; it gets away after
/// [`COURIER_SECS`].
pub(super) static COURIER: EnemyDef = EnemyDef {
    name: "Treasure Courier",
    display: 7174,
    hp: 45.0,
    speed: 6.0,
    damage: 0.0,
    scale: 1.0,
    xp: 8,
    level: 60,
    big: false,
};
const COURIER_SECS: f32 = 22.0;
/// A coin every this many seconds as it runs, and the coins its purse spills: twice a cache's,
/// short of a boss chest's.
const COURIER_DRIP: f32 = 2.0;
const COURIER_PURSE: u32 = 12;

/// The Shrine of Overcharge: the Paladin shrine (`PaladinShrine.mdx`), standing this long; the
/// first hero within reach takes its blessing, a 30-second Overcharge
/// ([`SHRINE_BLESSING`]), under Power Infusion's visual.
pub(super) const SHRINE_DISPLAY: u32 = 6873;
const SHRINE_SECS: f32 = 25.0;
const SHRINE_REACH: f32 = 3.0;
pub(super) const SHRINE_SPELL: u32 = 10060;
const SHRINE_KEY: u32 = 0x5A1E_0001;
const SHRINE_BUFF_SECS: f32 = 30.0;
pub(super) const SHRINE_BLESSING: &[super::bonus::Bonus] = &[
    super::bonus::Bonus::new(super::bonus::StatKey::Damage, 0.15),
    super::bonus::Bonus::new(super::bonus::StatKey::Cooldown, 0.10),
    super::bonus::Bonus::new(super::bonus::StatKey::MoveSpeed, 0.10),
];

/// The named rares an Elite Pack's leader may be, 1.12's own: the silver dragon frame shows it
/// ([`super::moments`]).
pub(super) const RARE_NAMES: [&str; 8] = [
    "Mor'Ladim",
    "Rak'shiri",
    "Azzere the Skyblade",
    "Kurmokk",
    "Ruuzel",
    "Foreman Rigger",
    "Lord Sakrasis",
    "Brack",
];

/// Name parts of 1.12's undead and demons on the rosters: the Scourge's dead, ghosts and
/// abominations, and the Legion's fel beasts, felguard, infernals and satyr.
const BANE_PARTS: [&str; 26] = [
    "Skeletal",
    "Zombie",
    "Ghoul",
    "Corpse",
    " Dead",
    "Scourge",
    "Rattlecage",
    "Putrid",
    "Shambling",
    "Stitched",
    "Spirit",
    "Shade",
    "Bonecaster",
    "Death Lord",
    "Deathknight",
    "Crypt",
    "Screamer",
    "Fel",
    "Infernal",
    "Doomguard",
    "Manahound",
    "Manastalker",
    "Familiar",
    "Jadefire",
    "Hederine",
    "Nerubian",
];
/// The undead and demon lords the name parts miss.
const BANE_NAMES: [&str; 10] = [
    "Kel'Thuzad",
    "Patchwerk",
    "Gluth",
    "Thaddius",
    "Anub'Rekhan",
    "Lord Kazzak",
    "Lord Hel'nurath",
    "Lord Banehollow",
    "Lord Azrethoc",
    "Prince Xavalis",
];

/// Whether `def` is an undead or a demon, by its 1.12 creature: what Mark of the Champion's holy
/// wrath strikes ([`super::mechanics::Of::Bane`]).
pub(super) fn bane(def: &EnemyDef) -> bool {
    BANE_NAMES.contains(&def.name) || BANE_PARTS.iter().any(|part| def.name.contains(part))
}

/// A Treasure Courier on the run: seconds to its next coin and before it gets away, and whether
/// its purse has spilled.
#[derive(Component)]
pub(super) struct Courier {
    drip: f32,
    left: f32,
    paid: bool,
}

/// A Shrine of Overcharge standing, and the seconds it has left.
#[derive(Component)]
pub(super) struct Shrine {
    left: f32,
}

/// An Elite Pack's named rare, by its place in [`RARE_NAMES`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RareMob(pub u8);

impl RareMob {
    pub(super) fn name(self) -> &'static str {
        RARE_NAMES[usize::from(self.0) % RARE_NAMES.len()]
    }
}

/// Whether a micro-event due at `elapsed` waits: a roster boss walks out within
/// [`MICRO_BOSS_CLEARANCE`] seconds.
pub(super) fn micro_waits(elapsed: f32) -> bool {
    let to_boss = data::BOSS_EVERY_SECS - elapsed.rem_euclid(data::BOSS_EVERY_SECS);
    to_boss < MICRO_BOSS_CLEARANCE
}

/// How far a level's nova throws the horde back from each hero standing, in yards, and what it
/// deals each enemy it reaches at run level 1 (growing as the mechanics' flat numbers do).
pub(super) const LEVEL_NOVA_RADIUS: f32 = 7.0;
pub(super) const LEVEL_NOVA_DAMAGE: f32 = 10.0;

/// A Treasure Courier runs from the party, a coin dropping behind it every [`COURIER_DRIP`]
/// seconds; slain, its purse spills ([`COURIER_PURSE`] coins); left alone it gets away. The
/// host's.
pub(super) fn run_couriers(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    mut couriers: Query<(Entity, &Transform, &mut Enemy, &mut Courier)>,
) {
    let dt = time.delta_secs();
    for (e, tf, mut enemy, mut courier) in &mut couriers {
        let at = tf.translation;
        if enemy.dead {
            if !courier.paid {
                courier.paid = true;
                let coins = treasure::Coins::new(COURIER_PURSE, run.elapsed, rng.f32());
                spawn_gem(&mut commands, run.guid(), at, GemKind::Gold(coins));
                run.banner = Some(("The Treasure Courier spills its purse!".into(), 3.0));
            }
            continue;
        }
        // It never stops running.
        enemy.fear_t = enemy.fear_t.max(1.0);
        courier.drip -= dt;
        if courier.drip <= 0.0 {
            courier.drip += COURIER_DRIP;
            let coins = treasure::Coins::new(1, run.elapsed, rng.f32());
            spawn_gem(&mut commands, run.guid(), at, GemKind::Gold(coins));
        }
        courier.left -= dt;
        if courier.left <= 0.0 {
            commands.entity(e).try_despawn();
            run.banner = Some(("The Treasure Courier got away!".into(), 3.0));
        }
    }
}

/// A Shrine of Overcharge: the first hero standing within reach takes its 30-second blessing
/// ([`SHRINE_BLESSING`]) and the shrine is spent; untouched, it fades. The host's.
pub(super) fn touch_shrines(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut shrines: Query<(Entity, &Transform, &mut Shrine), Without<Hero>>,
    mut heroes: Query<(Entity, &Transform, &mut HeroState), With<Hero>>,
    mut fx: Fx,
) {
    let dt = time.delta_secs();
    for (e, tf, mut shrine) in &mut shrines {
        shrine.left -= dt;
        let at = tf.translation;
        let toucher = heroes.iter_mut().find(|(_, h, state)| {
            state.alive() && flat(h.translation - at).length() <= SHRINE_REACH
        });
        if let Some((hero_e, _, mut hero)) = toucher {
            super::mechanics::timed_buff(&mut hero, SHRINE_KEY, SHRINE_BUFF_SECS, SHRINE_BLESSING);
            fx.cast(hero_e, SHRINE_SPELL, &[hero_e], None);
            run.banner = Some((
                "Blessed by the Shrine: Overcharged for 30 seconds!".into(),
                3.0,
            ));
            commands
                .entity(e)
                .remove::<Shrine>()
                .insert(benilla_world::model_fade::DespawnFade::default());
        } else if shrine.left <= 0.0 {
            commands
                .entity(e)
                .remove::<Shrine>()
                .insert(benilla_world::model_fade::DespawnFade::default());
        }
    }
}

/// Each level the party makes, every hero standing throws a golden nova
/// ([`super::moments::LEVEL_RING`]): the horde near it is shoved out to [`LEVEL_NOVA_RADIUS`] (a
/// boss stands its ground) and takes [`LEVEL_NOVA_DAMAGE`]. The second of immunity is
/// [`collect_gems`]'. A new run's level falling back throws nothing.
pub(super) fn level_up_nova(
    run: Res<Run>,
    mut last: Local<Option<u32>>,
    heroes: Query<(Entity, &Transform, &HeroState), With<Hero>>,
    mut enemies: Query<(Entity, &mut Transform, &Enemy), Without<Hero>>,
    mut hits: MessageWriter<Hit>,
    mut fx: Fx,
) {
    let gained = last.is_some_and(|l| run.level > l);
    *last = Some(run.level);
    if !gained {
        return;
    }
    let damage = LEVEL_NOVA_DAMAGE * super::mechanics::level_scale(run.level);
    for (hero_e, htf, hero) in &heroes {
        if !hero.alive() {
            continue;
        }
        fx.cast(hero_e, super::moments::LEVEL_RING, &[hero_e], None);
        let at = htf.translation;
        for (e, mut tf, enemy) in &mut enemies {
            let away = flat(tf.translation - at);
            let dist = away.length();
            if enemy.dead || dist > LEVEL_NOVA_RADIUS {
                continue;
            }
            if !enemy.boss && dist > 0.01 {
                tf.translation += away / dist * (LEVEL_NOVA_RADIUS - dist);
            }
            hits.write(Hit::direct(hero_e, e, damage, false));
        }
    }
}

impl Default for Spawner {
    fn default() -> Self {
        Self {
            timer: 0.0,
            xp_carry: XpCarry::default(),
            turn: 0,
            micro_next: MICRO_FIRST_SECS,
            micro_kind: 0,
        }
    }
}

/// The small rewards that punctuate a quiet stretch between boss and elite moments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MicroEvent {
    GoldRush,
    Shrine,
    Swarm,
    ElitePack,
}

fn micro_event(n: u8) -> MicroEvent {
    match n % 4 {
        0 => MicroEvent::GoldRush,
        1 => MicroEvent::Shrine,
        2 => MicroEvent::Swarm,
        _ => MicroEvent::ElitePack,
    }
}

#[cfg(test)]
mod micro_tests {
    use super::*;

    fn foe(world: &mut World, x: f32, boss: bool) -> Entity {
        let def = &data::roster(0).enemies[0];
        world
            .spawn((
                Transform::from_xyz(x, 0.0, 0.0),
                Enemy {
                    def,
                    hp: 1000.0,
                    max_hp: 1000.0,
                    speed: 0.0,
                    damage: 0.0,
                    radius: 0.6,
                    xp: 1,
                    boss,
                    elite: false,
                    guardian: false,
                    dormant: false,
                    attack_cd: 0.0,
                    slow: 1.0,
                    slow_t: 0.0,
                    root_t: 0.0,
                    stun_t: 0.0,
                    fear_t: 0.0,
                    weaken: 1.0,
                    weaken_t: 0.0,
                    dots: Vec::new(),
                    dead: false,
                    taunted: None,
                    debuffs: Default::default(),
                },
            ))
            .id()
    }

    #[test]
    fn a_level_throws_the_horde_back_from_every_hero_standing() {
        let mut world = World::new();
        world.init_resource::<Run>();
        world.init_resource::<Messages<Hit>>();
        world.init_resource::<crate::creature_anim::PlaySeq>();
        world.init_resource::<Messages<crate::creature_anim::CastEvent>>();
        world.init_resource::<Messages<crate::creature_anim::SpellGoTargets>>();
        world.init_resource::<Messages<crate::creature_anim::SwingMessage>>();
        world.init_resource::<Messages<crate::combat_text::CombatTextSpawn>>();
        world.spawn((Hero::default(), Transform::default(), HeroState::default()));
        let near = foe(&mut world, 3.0, false);
        let boss = foe(&mut world, -2.0, true);
        let far = foe(&mut world, 20.0, false);
        let nova = world.register_system(level_up_nova);
        let run_once = |world: &mut World| {
            world.run_system(nova).unwrap();
            world
                .resource_mut::<Messages<Hit>>()
                .drain()
                .collect::<Vec<_>>()
        };
        // The first frame only learns the level.
        assert!(run_once(&mut world).is_empty());
        world.resource_mut::<Run>().level = 2;
        let hits = run_once(&mut world);
        assert_eq!(hits.len(), 2, "the two in reach");
        assert!(hits.iter().all(|h| h.damage > 0.0 && h.target != far));
        let x = |world: &World, e: Entity| world.get::<Transform>(e).unwrap().translation.x;
        assert!(
            (x(&world, near) - LEVEL_NOVA_RADIUS).abs() < 1e-3,
            "shoved out"
        );
        assert_eq!(x(&world, boss), -2.0, "a boss stands its ground");
        assert_eq!(x(&world, far), 20.0);
        // No level, no nova; a new run's level falling back throws none either.
        assert!(run_once(&mut world).is_empty());
        world.resource_mut::<Run>().level = 1;
        assert!(run_once(&mut world).is_empty());
    }

    fn fx_world() -> World {
        let mut world = World::new();
        world.init_resource::<Run>();
        world.insert_resource(Rng(8));
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(500));
        world.insert_resource(time);
        world.init_resource::<Messages<Hit>>();
        world.init_resource::<crate::creature_anim::PlaySeq>();
        world.init_resource::<Messages<crate::creature_anim::CastEvent>>();
        world.init_resource::<Messages<crate::creature_anim::SpellGoTargets>>();
        world.init_resource::<Messages<crate::creature_anim::SwingMessage>>();
        world.init_resource::<Messages<crate::combat_text::CombatTextSpawn>>();
        world
    }

    fn coins(world: &mut World) -> usize {
        world
            .query::<&Gem>()
            .iter(world)
            .filter(|g| matches!(g.kind, GemKind::Gold(_)))
            .count()
    }

    #[test]
    fn a_courier_drips_coins_flees_and_spills_its_purse() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = fx_world();
        let e = foe(&mut world, 5.0, false);
        world.entity_mut(e).insert(Courier {
            drip: 0.2,
            left: 10.0,
            paid: false,
        });
        world.run_system_once(run_couriers).unwrap();
        assert_eq!(coins(&mut world), 1, "a coin as it runs");
        assert!(world.get::<Enemy>(e).unwrap().fear_t >= 1.0, "it flees");
        world.get_mut::<Enemy>(e).unwrap().dead = true;
        world.run_system_once(run_couriers).unwrap();
        world.run_system_once(run_couriers).unwrap();
        assert_eq!(coins(&mut world), 2, "its purse spills once");
        // One left alone gets away.
        let gone = foe(&mut world, 5.0, false);
        world.entity_mut(gone).insert(Courier {
            drip: 5.0,
            left: 0.1,
            paid: false,
        });
        world.run_system_once(run_couriers).unwrap();
        assert!(world.get_entity(gone).is_err());
    }

    #[test]
    fn the_first_hero_to_touch_a_shrine_is_overcharged() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = fx_world();
        let near = world
            .spawn((Hero::default(), Transform::default(), HeroState::default()))
            .id();
        let far = world
            .spawn((
                Hero::default(),
                Transform::from_xyz(30.0, 0.0, 0.0),
                HeroState::default(),
            ))
            .id();
        let might = world.get::<HeroState>(near).unwrap().stats().might;
        let shrine = world
            .spawn((
                Transform::from_xyz(1.0, 0.0, 0.0),
                Shrine { left: SHRINE_SECS },
            ))
            .id();
        world.run_system_once(touch_shrines).unwrap();
        assert!(world.get::<Shrine>(shrine).is_none(), "spent");
        let blessed = world.get::<HeroState>(near).unwrap();
        assert!(blessed.stats().might > might, "Overcharged");
        assert_eq!(
            super::super::mechanics::buff_left(blessed, SHRINE_KEY),
            Some(SHRINE_BUFF_SECS)
        );
        let other = world.get::<HeroState>(far).unwrap();
        assert_eq!(super::super::mechanics::buff_left(other, SHRINE_KEY), None);
    }

    #[test]
    fn only_undead_and_demons_are_a_banes_foes() {
        let named = |name: &'static str| EnemyDef { name, ..COURIER };
        for name in [
            "Skeletal Warrior",
            "Rotting Dead",
            "Felhound",
            "Lesser Infernal",
            "Kel'Thuzad",
            "Jadefire Felsworn",
        ] {
            assert!(bane(&named(name)), "{name}");
        }
        for name in [
            "Hogger",
            "Defias Thug",
            "Prairie Wolf",
            "Lucifron",
            "Murloc",
        ] {
            assert!(!bane(&named(name)), "{name}");
        }
    }

    #[test]
    fn micro_events_wait_out_a_boss_walking_out() {
        assert!(!micro_waits(MICRO_FIRST_SECS));
        assert!(micro_waits(data::BOSS_EVERY_SECS - 5.0));
        assert!(!micro_waits(data::BOSS_EVERY_SECS + 1.0));
        assert_eq!(RareMob(1).name(), RARE_NAMES[1]);
        assert_eq!(RareMob(200).name(), RARE_NAMES[200 % RARE_NAMES.len()]);
    }

    #[test]
    fn no_micro_event_comes_before_its_first_time() {
        let spawner = Spawner::default();
        assert_eq!(spawner.micro_next, MICRO_FIRST_SECS);
        assert!(spawner.micro_next >= 30.0, "nothing at 0:00");
    }

    #[test]
    fn micro_events_rotate_through_all_rewards() {
        assert_eq!(micro_event(0), MicroEvent::GoldRush);
        assert_eq!(micro_event(1), MicroEvent::Shrine);
        assert_eq!(micro_event(2), MicroEvent::Swarm);
        assert_eq!(micro_event(3), MicroEvent::ElitePack);
        assert_eq!(micro_event(4), MicroEvent::GoldRush);
    }
}

/// An enemy died here, worth this much.
#[derive(Message, Clone, Copy)]
pub(super) struct Loot {
    pub pos: Vec3,
    pub xp: u32,
    pub boss: bool,
    pub elite: bool,
    /// What died: a boss's name is its place in the roster, which gear drops by.
    pub def: &'static EnemyDef,
}

/// What a spawn walks out as.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Rank {
    Horde,
    Boss,
    /// Tougher and marked ([`treasure::elite_body`]).
    Elite,
    /// An elite holding a post until the hero comes near.
    Guardian,
}

#[derive(Clone, Copy)]
pub(super) enum GemKind {
    Xp(u32),
    Heal(f32),
    /// Coins, paying under the gold multiplier of whoever picks them up.
    Gold(treasure::Coins),
}

#[derive(Component)]
pub(super) struct Gem {
    /// Read by the rewards companion, which fetches only experience and coins.
    pub(super) kind: GemKind,
    /// The hero it is flying to; the companion sets it to send a fetched pickup on its way.
    pub(super) pulled: Option<Entity>,
    speed: f32,
}

/// The `GameObjectDisplayInfo` row and scale a pickup wears: a wisp per experience tier, the
/// bigger tiers rarer, a turkey leg for health and a heap of gold for coins.
fn look_for(kind: GemKind) -> (u32, f32) {
    match kind {
        // `TurkeyLeg.mdx`.
        GemKind::Heal(_) => (114, 2.5),
        GemKind::Gold(coins) => treasure::coin_look(coins.base),
        // `BFD_WispSmallPurple.mdx`.
        GemKind::Xp(xp) if xp >= 20 => (1307, 1.0),
        // `BFD_WispSmallGreen.mdx`.
        GemKind::Xp(xp) if xp >= 5 => (1267, 1.0),
        // `BFD_WispSmall.mdx`.
        GemKind::Xp(_) => (426, 1.0),
    }
}

/// Body radius: the big beasts need more room.
fn radius_for(def: &EnemyDef) -> f32 {
    (if def.big { 1.3 } else { 0.6 }) * def.scale
}

/// A roster boss walks out half again its kind's size over the horde's; its body radius, and so
/// the reach it swings from ([`move_enemies`]' contact range) and the room hits find, grow with it.
pub(super) const BOSS_SCALE: f32 = 1.5;

/// A spawn's body under its rank: an elite's or a guardian's ([`treasure::elite_body`]), a roster
/// boss's [`BOSS_SCALE`] bigger, the horde's as tuned.
fn ranked_body(body: treasure::Body, rank: Rank) -> treasure::Body {
    match rank {
        Rank::Elite | Rank::Guardian => treasure::elite_body(body),
        Rank::Boss => treasure::Body {
            scale: body.scale * BOSS_SCALE,
            ..body
        },
        Rank::Horde => body,
    }
}

/// The body radius of `def` drawn at `scale`.
fn body_radius(def: &EnemyDef, scale: f32) -> f32 {
    radius_for(def) * scale / def.scale
}

pub(super) fn spawn_enemy(
    commands: &mut Commands,
    run: &mut Run,
    def: &'static EnemyDef,
    pos: Vec3,
    hero: Vec3,
    rank: Rank,
    opts: &RunOptions,
    xp_carry: &mut XpCarry,
) -> Entity {
    spawn_enemy_with(commands, run, def, pos, hero, rank, opts, xp_carry, 1)
}

/// [`spawn_enemy`], worth `xp_mult` times its experience: a Swarm's prey.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_enemy_with(
    commands: &mut Commands,
    run: &mut Run,
    def: &'static EnemyDef,
    pos: Vec3,
    hero: Vec3,
    rank: Rank,
    opts: &RunOptions,
    xp_carry: &mut XpCarry,
    xp_mult: u32,
) -> Entity {
    // The Run Options first, then the run clock's growth and the party's size, then an elite's own
    // multipliers. Only the horde splits its experience under density: neither density nor party
    // size adds bosses, elites or guardians.
    let tuned = run_options::scaled(
        def,
        opts,
        rank != Rank::Horde,
        party::scale(run.party),
        xp_carry,
    );
    let body = ranked_body(
        treasure::Body {
            hp: tuned.hp * data::hp_scale(run.elapsed) * party::scale(run.party),
            damage: tuned.damage * data::damage_scale(run.elapsed),
            scale: def.scale,
        },
        rank,
    );
    let elite = matches!(rank, Rank::Elite | Rank::Guardian);
    let mut marks = Marks::debuffs();
    if elite {
        // Held for life: the mark floats over the elite's head.
        marks.add(treasure::ELITE_MARK, f32::INFINITY);
    }
    let serial = run.guid();
    let e = units::spawn_unit(
        commands,
        &UnitSpec {
            serial,
            display: def.display,
            scale: body.scale,
            level: def.level,
            faction: units::FACTION_MONSTER,
            bytes0: 1 << 8,
            weapons: &[],
            sheath: 0,
        },
        pos,
        yaw_toward(hero - pos),
    );
    commands.entity(e).insert((
        Enemy {
            def,
            hp: body.hp,
            max_hp: body.hp,
            speed: tuned.speed,
            damage: body.damage,
            radius: body_radius(def, body.scale),
            xp: tuned.xp.saturating_mul(xp_mult),
            boss: rank == Rank::Boss,
            elite,
            guardian: rank == Rank::Guardian,
            dormant: rank == Rank::Guardian,
            attack_cd: 0.5,
            slow: 1.0,
            slow_t: 0.0,
            root_t: 0.0,
            stun_t: 0.0,
            fear_t: 0.0,
            weaken: 1.0,
            weaken_t: 0.0,
            dots: Vec::new(),
            dead: false,
            taunted: None,
            debuffs: Default::default(),
        },
        marks,
    ));
    e
}

/// Ground an object on the terrain heightfield where it has no walking body.
pub(super) fn ground(world: &WorldPoint, mut p: Vec3, fallback: f32) -> Vec3 {
    p.y = world.terrain_height_under(p).unwrap_or(fallback);
    p
}

/// What grounding asks of the world besides its colliders: the ADT heightfield and whether a point
/// is inside a building. [`WorldPoint`] in the game; a synthetic plane in the tests.
pub(super) trait Heightfield {
    /// The MCNK heightfield's height under `p`, `None` off the streamed set or on a map without
    /// one (a WMO-only instance such as Molten Core).
    fn height_under(&self, p: Vec3) -> Option<f32>;
    /// Whether `p` is inside a WMO group.
    fn indoors(&self, p: Vec3) -> bool;
}

impl Heightfield for WorldPoint<'_, '_> {
    fn height_under(&self, p: Vec3) -> Option<f32> {
        self.terrain_height_under(p)
    }

    fn indoors(&self, p: Vec3) -> bool {
        self.indoors_at(p)
    }
}

/// The clearance above a known stage or hero floor that a spawn probe starts from.  It stays under
/// a cave roof, unlike the old 64 yd outdoor probe, while still clearing a floor-plane roundoff.
const GROUND_PROBE_LIFT: f32 = 4.0;
/// A walking body may step this far up in a frame.  A higher collision hit is a roof, shelf, or
/// streamed-in interior and must not replace the floor it was already walking on.
const STEP_UP: f32 = 1.5;
/// How far a certified outdoor point may sit from the ADT heightfield, and how far under it a
/// walking body counts as fallen into a cellar or through the world.
pub(super) const OUTDOOR_SLACK: f32 = 2.0;
/// How high over an outdoor point a roof or ceiling disqualifies it.
const ROOF_CLEARANCE: f32 = 30.0;
/// How far round a spawn candidate the outdoor search looks before it settles for the candidate.
const SPAWN_REACH: f32 = 6.0;
/// The ring spacing of the outdoor search, yards.
const RING_STEP: f32 = 3.0;

/// The floor under `p` from collision: the first face under a probe from just over the reference
/// floor, unless it is more than [`STEP_UP`] over it (a roof, a shelf, a wall top), in which case
/// a second probe from step height looks under it.
fn collision_ground_y(collide: &WorldCollision, p: Vec3, fallback: f32) -> Option<f32> {
    let reference = p.y.max(fallback);
    let limit = reference + STEP_UP;
    let probe = |lift: f32| {
        let from = p.with_y(reference + lift);
        collide
            .ray_body(from, Dir3::NEG_Y, 128.0)
            .map(|hit| from.y - hit.distance)
    };
    match probe(GROUND_PROBE_LIFT) {
        Some(y) if y <= limit => Some(y),
        Some(_) => probe(STEP_UP).filter(|&y| y <= limit),
        None => None,
    }
}

/// A point outdoor content may stand on: a collision floor (the heightfield while there is no
/// collision at all) within [`OUTDOOR_SLACK`] of the ADT heightfield, no roof or ceiling within
/// [`ROOF_CLEARANCE`] over it, and not inside a WMO group. `None` without a heightfield: a map
/// with no ADT has no outdoors to certify.
pub(super) fn valid_outdoor_point(
    collide: Option<&WorldCollision>,
    world: &impl Heightfield,
    p: Vec3,
) -> Option<Vec3> {
    let terrain = world.height_under(p)?;
    let y = match collide {
        Some(c) => collision_ground_y(c, p.with_y(terrain), terrain)?,
        None => terrain,
    };
    if (y - terrain).abs() > OUTDOOR_SLACK {
        return None;
    }
    let grounded = p.with_y(y);
    let lifted = grounded + Vec3::Y * 0.05;
    // A body ray is one-sided: going up it meets a roof's or ceiling's downward faces only, never
    // the ground it starts on.
    if collide.is_some_and(|c| c.ray_body(lifted, Dir3::Y, ROOF_CLEARANCE).is_some()) {
        return None;
    }
    if world.indoors(lifted) {
        return None;
    }
    Some(grounded)
}

/// Candidates round `p` out to `reach`, nearest ring first, eight to a ring.
fn rings(p: Vec3, from: f32, reach: f32) -> impl Iterator<Item = Vec3> {
    let n = (reach / RING_STEP).floor().max(0.0) as u32;
    let first = (from / RING_STEP).ceil().max(1.0) as u32;
    (first..=n).flat_map(move |k| {
        let r = k as f32 * RING_STEP;
        (0..8).map(move |i| {
            let a = i as f32 * std::f32::consts::TAU / 8.0;
            p + Vec3::new(a.cos(), 0.0, a.sin()) * r
        })
    })
}

/// The nearest certified outdoor point to `p` that `keep` allows, searching rings from `from` out
/// to `reach` yards (`p` itself first when `from` is zero).
pub(super) fn nearest_outdoor(
    collide: Option<&WorldCollision>,
    world: &impl Heightfield,
    p: Vec3,
    from: f32,
    reach: f32,
    keep: impl Fn(Vec3) -> bool,
) -> Option<Vec3> {
    let here = (from <= 0.0).then_some(p);
    here.into_iter()
        .chain(rings(p, from, reach))
        .filter(|&q| keep(q))
        .find_map(|q| valid_outdoor_point(collide, world, q))
}

/// Ground a spawn candidate on the nearest certified outdoor point round it, else as a walking
/// body: a cave stage has no outdoors, and an arena still starts while its collision streams.
pub(super) fn outdoor_ground(
    collide: Option<&WorldCollision>,
    world: &impl Heightfield,
    p: Vec3,
    fallback: f32,
) -> Vec3 {
    nearest_outdoor(collide, world, p, 0.0, SPAWN_REACH, |_| true)
        .unwrap_or_else(|| ground_on_collision(collide, world, p, fallback))
}

/// Enemies this near a hero sweep their body every frame; farther ones feel ahead with a ray.
const SWEEP_RANGE: f32 = 50.0;
/// The swept body rides this far over the feet, so the ground, a slope and a kerb the ground probe
/// steps onto are not walls.
const STEP_CLEAR: f32 = 0.6;
/// The swept sphere's radius bounds: a small body still sweeps a real one, and a giant's does not
/// wedge in every gap its model would clip through anyway.
const PROBE_MIN: f32 = 0.3;
const PROBE_MAX: f32 = 1.0;
/// The sweep's skin.
const SKIN: f32 = 0.02;

/// Where an enemy at `pos` that wants to move `movement` (flat) ends up against the world's walls,
/// before grounding, and the flat normal of the last wall it met. Near a hero it sweeps a sphere
/// ([`WorldCollision::cast_body`]); farther away a ray feeler at the same height does. Either
/// slides along what it meets, at most twice.
fn enemy_step(
    collide: Option<&WorldCollision>,
    pos: Vec3,
    movement: Vec3,
    radius: f32,
    near_hero: bool,
) -> (Vec3, Option<Vec3>) {
    let movement = flat(movement);
    let Some(c) = collide else {
        return (pos + movement, None);
    };
    if movement.length_squared() <= 1e-10 {
        return (pos, None);
    }
    let r = radius.clamp(PROBE_MIN, PROBE_MAX);
    let center = pos + Vec3::Y * (STEP_CLEAR + r);
    let shape = near_hero.then(|| Collider::sphere(r));
    let mut moved = Vec3::ZERO;
    let mut left = movement;
    let mut wall = None;
    for _ in 0..2 {
        let Ok((dir, len)) = Dir3::new_and_length(left) else {
            break;
        };
        // How far along `left` it gets, and the wall's normal there.
        let hit = match &shape {
            Some(shape) => c
                .cast_body(shape, center + moved, left, SKIN)
                .map(|h| (h.distance.min(len), h.normal1)),
            None => c
                .ray_body(center + moved, dir, len + r)
                .map(|h| ((h.distance - r).clamp(0.0, len), h.normal)),
        };
        let Some((reach, normal)) = hit else {
            return (pos + moved + left, wall);
        };
        let along = *dir * reach;
        moved += along;
        left -= along;
        let n = flat(normal).normalize_or_zero();
        if n == Vec3::ZERO {
            break;
        }
        wall = Some(n);
        // Keep what runs along the wall, drop what runs into it.
        left -= n * left.dot(n).min(0.0);
    }
    (pos + moved, wall)
}

/// How long an enemy a wall stopped keeps walking along it before it turns to its prey again.
const DETOUR_SECS: f32 = 1.0;
/// How long after a detour ends the next one still keeps its way round, so a horde member does
/// not swing back and forth at the same wall.
const DETOUR_MEMORY: f32 = 3.0;

/// A wall-follow: the way along the wall that stopped the chase, until when, and that wall's flat
/// normal.
#[derive(Clone, Copy, Debug)]
pub(super) struct Detour {
    along: Vec3,
    wall: Vec3,
    until: f32,
}

/// One enemy's walk this frame: where it stands, the chase's and the crowd's flat displacement,
/// its body radius, whether a hero is near enough for the full sweep, and the way round (`±1`)
/// it takes when a wall faces its prey square on.
#[derive(Clone, Copy)]
pub(super) struct Stride {
    pub(super) pos: Vec3,
    pub(super) chase: Vec3,
    pub(super) shove: Vec3,
    pub(super) radius: f32,
    pub(super) near: bool,
    pub(super) side: f32,
}

impl Detour {
    fn active(&self, now: f32) -> bool {
        self.until > now
    }
}

/// One frame of an enemy's walk over the world at time `now`: the chase (or the wall-follow a
/// wall started, [`Detour`]) and the crowd's shove, slid along walls ([`enemy_step`]), then
/// grounded. A chase a wall stops nearly dead turns along the wall, the way that leads towards
/// its prey, and keeps on round it for [`DETOUR_SECS`]: enemies go round a house, not into it.
pub(super) fn walk_step(
    collide: Option<&WorldCollision>,
    world: &impl Heightfield,
    fallback: f32,
    s: Stride,
    detour: &mut Option<Detour>,
    now: f32,
) -> Vec3 {
    if detour.is_some_and(|d| d.until + DETOUR_MEMORY <= now) {
        *detour = None;
    }
    let len = s.chase.length();
    let chase_dir = flat(s.chase).normalize_or_zero();
    let chase = match *detour {
        Some(d) if d.active(now) => d.along * len,
        _ => s.chase,
    };
    let (next, wall) = enemy_step(collide, s.pos, chase + s.shove, s.radius, s.near);
    if let Some(n) = wall {
        let progress = flat(next - s.pos).dot(flat(chase).normalize_or_zero());
        if len > 1e-4 && progress < 0.5 * len {
            let tangent = Vec3::new(-n.z, 0.0, n.x);
            let sign = match *detour {
                // Round the same wall the same way; at a corner, away from the wall left.
                Some(d) => {
                    let keep = tangent.dot(d.along);
                    if keep.abs() > 0.1 {
                        keep.signum()
                    } else {
                        tangent.dot(d.wall).signum()
                    }
                }
                None => {
                    let toward = tangent.dot(chase_dir);
                    if toward.abs() > 0.05 {
                        toward.signum()
                    } else {
                        s.side
                    }
                }
            };
            *detour = Some(Detour {
                along: tangent * sign,
                wall: n,
                until: now + DETOUR_SECS,
            });
        }
    }
    ground_on_collision(collide, world, next, fallback)
}

/// Ground a walking body on collision first, falling back to terrain while the collider streams.
pub(super) fn ground_on_collision(
    collide: Option<&WorldCollision>,
    world: &impl Heightfield,
    mut p: Vec3,
    fallback: f32,
) -> Vec3 {
    // The reference point is already a floor (the hero or MapDef floor).  Beginning just above it
    // keeps a WMO ceiling out of the ray, so its nearest hit is the floor below the reference.
    p.y = collide
        .and_then(|collide| collision_ground_y(collide, p, fallback))
        .or_else(|| world.height_under(p))
        .unwrap_or(fallback);
    p
}

/// How far along its path to the hero a lost enemy is looked for a way out, yards.
const RESCUE_REACH: f32 = 24.0;

/// An enemy whose step left it under the heightfield (in a cellar, under a WMO floor, through the
/// world) while the hero it chases, at `hero`, is not: the first outdoor point along its path to
/// that hero, `None` while it is where it should be or no way out is found.
fn rescue_enemy(
    collide: Option<&WorldCollision>,
    world: &impl Heightfield,
    at: Vec3,
    hero: Vec3,
) -> Option<Vec3> {
    let under = |p: Vec3| {
        world
            .height_under(p)
            .is_some_and(|t| p.y < t - OUTDOOR_SLACK)
    };
    if !under(at) || under(hero) {
        return None;
    }
    let dir = flat(hero - at).normalize_or_zero();
    let gap = flat(hero - at).length();
    (1..=(RESCUE_REACH / RING_STEP) as u32)
        .map(|k| at + dir * (k as f32 * RING_STEP).min(gap))
        .find_map(|p| valid_outdoor_point(collide, world, p))
}

/// A horde ring never steps beyond the battle circle, even when its target is already at the edge.
fn spawn_site(map: &data::MapDef, hero_pos: Vec3, angle: f32, radius: f32) -> Vec3 {
    let mut site = hero_pos + Vec3::new(angle.cos(), 0.0, angle.sin()) * radius;
    let center = hero::arena(map);
    let offset = flat(site - center);
    let limit = (map.leash - 1.0).max(0.0);
    if offset.length() > limit {
        site = center + offset.normalize_or_zero() * limit;
        site.y = hero_pos.y;
    }
    site
}

/// Where the heroes stand: those standing (whom spawns ring round) and everyone (whom spawns keep
/// clear of).
pub(super) fn hero_spots(
    heroes: &Query<(&Transform, &HeroState), With<Hero>>,
) -> (Vec<Vec3>, Vec<Vec3>) {
    let standing = heroes
        .iter()
        .filter(|(_, h)| h.alive())
        .map(|(tf, _)| tf.translation)
        .collect();
    let everyone = heroes.iter().map(|(tf, _)| tf.translation).collect();
    (standing, everyone)
}

/// Keep the horde topped up on a ring outside the view, round one standing hero at a time and
/// clear of every hero, with the bosses and the encircling waves on the clock. The party's size
/// swells the crowd ([`party::crowd`]).
pub(super) fn spawn_enemies(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    mut spawner: ResMut<Spawner>,
    selection: Res<Selection>,
    opts: Res<RunOptions>,
    heroes: Query<(&Transform, &HeroState), With<Hero>>,
    enemies: Query<&Enemy>,
    collide: Option<WorldCollision>,
    world: WorldPoint,
) {
    let (standing, everyone) = hero_spots(&heroes);
    // A camp after a boss: the horde waits ([`super::campfire`]).
    if standing.is_empty() || !super::strip::may_spawn(&run) {
        return;
    }
    let leash = party::Leash::of(selection.stage_def());
    let heroes_in = run.party;
    let elapsed = run.elapsed;
    let map = selection.stage_def();
    let roster = data::roster(selection.map);
    let pool = data::wave_pool(elapsed, roster.enemies.len());

    // A host event every [`MICRO_EVERY_SECS`] from [`MICRO_FIRST_SECS`], held while a boss is
    // about to walk out. It makes ordinary replicated enemies, pickups and objects, and counts in
    // `Run::events`, which sounds the raid warning on every screen ([`super::sfx`]).
    if elapsed >= spawner.micro_next {
        if micro_waits(elapsed) {
            spawner.micro_next = elapsed + MICRO_BOSS_CLEARANCE;
        } else {
            let event = micro_event(spawner.micro_kind);
            spawner.micro_kind = spawner.micro_kind.wrapping_add(1);
            spawner.micro_next = elapsed + MICRO_EVERY_SECS;
            run.events += 1;
            let hpos = party::spawn_anchor(&standing, &mut spawner.turn).unwrap_or(standing[0]);
            let def = &roster.enemies[pool[0]];
            match event {
                MicroEvent::GoldRush => {
                    let at = party::spawn_point(
                        hpos,
                        rng.range(0.0, std::f32::consts::TAU),
                        20.0,
                        &everyone,
                        leash,
                    );
                    let pos = outdoor_ground(collide.as_ref(), &world, at, map.floor);
                    let e = spawn_enemy(
                        &mut commands,
                        &mut run,
                        &COURIER,
                        pos,
                        hpos,
                        Rank::Horde,
                        &opts,
                        &mut spawner.xp_carry,
                    );
                    commands.entity(e).insert(Courier {
                        drip: COURIER_DRIP,
                        left: COURIER_SECS,
                        paid: false,
                    });
                    run.banner = Some((
                        "Gold Rush! A Treasure Courier is loose: catch it!".into(),
                        4.0,
                    ));
                }
                MicroEvent::Shrine => {
                    let at = party::spawn_point(
                        hpos,
                        rng.range(0.0, std::f32::consts::TAU),
                        10.0,
                        &everyone,
                        leash,
                    );
                    let pos = outdoor_ground(collide.as_ref(), &world, at, map.floor);
                    let e =
                        units::spawn_object(&mut commands, run.guid(), SHRINE_DISPLAY, 1.0, pos);
                    commands.entity(e).insert(Shrine { left: SHRINE_SECS });
                    run.banner = Some((
                        "A Shrine of Overcharge rises! Touch it for its blessing!".into(),
                        4.0,
                    ));
                }
                MicroEvent::Swarm => {
                    for i in 0..12 {
                        let at = party::spawn_point(
                            hpos,
                            i as f32 / 12.0 * std::f32::consts::TAU,
                            22.0,
                            &everyone,
                            leash,
                        );
                        let pos = outdoor_ground(collide.as_ref(), &world, at, map.floor);
                        spawn_enemy_with(
                            &mut commands,
                            &mut run,
                            def,
                            pos,
                            hpos,
                            Rank::Horde,
                            &opts,
                            &mut spawner.xp_carry,
                            2,
                        );
                    }
                    run.banner = Some(("Swarm! Double experience prey!".into(), 4.0));
                }
                MicroEvent::ElitePack => {
                    let rare = RareMob(rng.index(RARE_NAMES.len()) as u8);
                    for i in 0..3 {
                        let at = party::spawn_point(
                            hpos,
                            i as f32 / 3.0 * std::f32::consts::TAU,
                            24.0,
                            &everyone,
                            leash,
                        );
                        let pos = outdoor_ground(collide.as_ref(), &world, at, map.floor);
                        let e = spawn_enemy(
                            &mut commands,
                            &mut run,
                            def,
                            pos,
                            hpos,
                            Rank::Elite,
                            &opts,
                            &mut spawner.xp_carry,
                        );
                        // The first leads: the named rare.
                        if i == 0 {
                            commands.entity(e).insert(rare);
                        }
                    }
                    run.banner = Some((format!("Rare pack! {} leads the hunt!", rare.name()), 4.0));
                }
            }
        }
    }

    // On a raid stage the last boss walks out after the trash ([`data::next_boss`]).
    while let Some(next) = data::next_boss(map, elapsed, run.bosses, roster.bosses.len()) {
        let def = &roster.bosses[next];
        let Some(hpos) = party::spawn_anchor(&standing, &mut spawner.turn) else {
            break;
        };
        let a = rng.range(0.0, std::f32::consts::TAU);
        let at = party::spawn_point(hpos, a, 30.0, &everyone, leash);
        let pos = outdoor_ground(collide.as_ref(), &world, at, map.floor);
        spawn_enemy(
            &mut commands,
            &mut run,
            def,
            pos,
            hpos,
            Rank::Boss,
            &opts,
            &mut spawner.xp_carry,
        );
        run.bosses = next as u32 + 1;
        run.banner = Some((format!("{} approaches!", def.name), 4.0));
    }

    // Half way between bosses, a closing ring of the moment's weakest.
    let due_rings = ((elapsed + data::BOSS_EVERY_SECS / 2.0) / data::BOSS_EVERY_SECS) as u32;
    if run.rings < due_rings {
        run.rings = due_rings;
        let def = &roster.enemies[pool[0]];
        let n = party::crowd(
            opts.crowd(28 + (data::progress(elapsed) * 60.0) as usize),
            heroes_in,
        );
        // The whole ring closes round one hero.
        let hpos = party::spawn_anchor(&standing, &mut spawner.turn).unwrap_or(standing[0]);
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            let at = party::spawn_point(hpos, a, 26.0, &everyone, leash);
            let pos = outdoor_ground(collide.as_ref(), &world, at, map.floor);
            spawn_enemy(
                &mut commands,
                &mut run,
                def,
                pos,
                hpos,
                Rank::Horde,
                &opts,
                &mut spawner.xp_carry,
            );
        }
        run.banner = Some(("They're surrounding you!".into(), 3.0));
    }

    spawner.timer -= time.delta_secs();
    if spawner.timer > 0.0 {
        return;
    }
    spawner.timer = 0.12;
    // Guardians at their posts are no part of the horde.
    let alive = enemies.iter().filter(|e| !e.dead && !e.dormant).count();
    let target = party::crowd(opts.crowd(data::target_alive(elapsed)), heroes_in);
    if alive >= target {
        return;
    }
    let n = (((target - alive) as f32) / 6.0)
        .ceil()
        .min(4.0 * opts.density * party::scale(heroes_in)) as usize;
    for _ in 0..n {
        let def = &roster.enemies[pool[rng.index(pool.len())]];
        let a = rng.range(0.0, std::f32::consts::TAU);
        let r = rng.range(34.0, 42.0);
        let Some(hpos) = party::spawn_anchor(&standing, &mut spawner.turn) else {
            break;
        };
        let at = party::spawn_point(hpos, a, r, &everyone, leash);
        let pos = outdoor_ground(collide.as_ref(), &world, at, map.floor);
        spawn_enemy(
            &mut commands,
            &mut run,
            def,
            pos,
            hpos,
            Rank::Horde,
            &opts,
            &mut spawner.xp_carry,
        );
    }
}

/// One hero as the horde sees it this frame.
#[derive(Clone, Copy)]
struct Seen {
    e: Entity,
    pos: Vec3,
    /// Standing, not down.
    up: bool,
    /// Slipped the horde's sight (Fade): only bosses still see it.
    hidden: bool,
    threat: f32,
}

/// The hero an enemy at `pos` goes for, and where it stands: its taunter's while `taunted` lasts
/// (to run second `now`), else [`party::pick_target`]'s among the heroes standing that it can see.
/// `None` with the party down or, for all but a boss, every hero standing hidden.
fn chase_target(
    pos: Vec3,
    boss: bool,
    taunted: Option<(Entity, f32)>,
    now: f32,
    seen: &[Seen],
) -> Option<(Entity, Vec3)> {
    let prey = seen.iter().filter(|h| h.up && (boss || !h.hidden));
    let forced = taunted
        .filter(|&(_, until)| until > now)
        .and_then(|(hero, _)| prey.clone().find(|h| h.e == hero));
    let picked = match forced {
        Some(h) => Some(h.e),
        None => party::pick_target(pos, prey.map(|h| (h.e, h.pos, h.threat))),
    };
    picked
        .and_then(|hero| seen.iter().find(|h| h.e == hero))
        .map(|h| (h.e, h.pos))
}

/// The chase: straight at the nearest hero standing, pushed apart from each other, stopped by
/// crowd control, and swinging at that hero when in reach.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn move_enemies(
    time: Res<Time>,
    camp: Res<super::campfire::Camp>,
    mut run: ResMut<Run>,
    selection: Res<Selection>,
    collide: Option<WorldCollision>,
    world: WorldPoint,
    mut heroes: Query<(Entity, &Transform, &mut HeroState), (With<Hero>, Without<Enemy>)>,
    mut enemies: Query<(Entity, &mut Transform, &mut Enemy, &mut MovementState), Without<Hero>>,
    mut fx: Fx,
    mut hits: MessageWriter<Hit>,
    mut detours: Local<HashMap<Entity, Detour>>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    if camp.active(&run) {
        for (_, _, _, mut mv) in &mut enemies {
            mv.speed = 0.0;
            mv.flags = 0;
        }
        return;
    }
    let seen: Vec<Seen> = heroes
        .iter()
        .map(|(e, tf, h)| Seen {
            e,
            pos: tf.translation,
            up: h.alive(),
            hidden: h.mobile.vanish_t > 0.0,
            threat: h.threat,
        })
        .collect();
    if seen.is_empty() {
        return;
    }
    let map = selection.stage_def();
    let hero_down = seen.iter().all(|h| !h.up);
    let wake = treasure::wake_radius(map.leash);

    // Separation over a coarse grid: only neighbours in the same or adjacent cells push.
    const CELL: f32 = 3.0;
    let bodies: Vec<(Entity, Vec3, f32)> = enemies
        .iter()
        .filter(|(_, _, e, _)| !e.dead)
        .map(|(ent, tf, e, _)| (ent, tf.translation, e.radius))
        .collect();
    let key = |p: Vec3| ((p.x / CELL).floor() as i32, (p.z / CELL).floor() as i32);
    let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for (i, b) in bodies.iter().enumerate() {
        grid.entry(key(b.1)).or_default().push(i);
    }
    let mut push: HashMap<Entity, Vec3> = HashMap::with_capacity(bodies.len());
    for (i, &(ent, pos, r)) in bodies.iter().enumerate() {
        let (cx, cz) = key(pos);
        let mut acc = Vec3::ZERO;
        for dx in -1..=1 {
            for dz in -1..=1 {
                let Some(cell) = grid.get(&(cx + dx, cz + dz)) else {
                    continue;
                };
                for &j in cell {
                    if j == i {
                        continue;
                    }
                    let (_, other, or) = bodies[j];
                    let d = flat(pos - other);
                    let len = d.length();
                    let min = r + or;
                    if len < min && len > 1e-4 {
                        acc += d / len * (min - len);
                    } else if len <= 1e-4 {
                        acc += Vec3::new((i as f32).sin(), 0.0, (i as f32).cos()) * 0.1;
                    }
                }
            }
        }
        push.insert(ent, acc);
    }
    // Forgotten detours go, and so do any from an earlier run's clock.
    let now = run.elapsed;
    detours.retain(|_, d| d.until + DETOUR_MEMORY > now && d.until <= now + DETOUR_SECS);

    for (ent, mut tf, mut e, mut mv) in &mut enemies {
        let e = &mut *e;
        if e.dead {
            if mv.speed != 0.0 || mv.flags != 0 {
                mv.speed = 0.0;
                mv.flags = 0;
            }
            continue;
        }
        e.attack_cd -= dt;
        for t in [
            &mut e.slow_t,
            &mut e.root_t,
            &mut e.stun_t,
            &mut e.fear_t,
            &mut e.weaken_t,
        ] {
            *t = (*t - dt).max(0.0);
        }
        if e.slow_t <= 0.0 {
            e.slow = 1.0;
        }
        if e.weaken_t <= 0.0 {
            e.weaken = 1.0;
        }
        let pos = tf.translation;
        // The nearest hero standing (the nearest at all once the party is down) wakes guardians
        // and calls strays back.
        let near = |up_only: bool| {
            let heroes = seen.iter().filter(|h| h.up || !up_only);
            party::nearest(heroes.map(|h| (h.e, h.pos)), pos)
        };
        let Some((_, hpos)) = near(true).or_else(|| near(false)) else {
            continue;
        };
        let to_hero = flat(hpos - pos);
        let dist = to_hero.length();

        // A guardian holds its post, out of the chase, until the hero comes near or hurts it.
        if e.dormant {
            if dist > wake && e.hp >= e.max_hp {
                continue;
            }
            e.dormant = false;
            run.banner = Some((format!("{} leaves its post!", e.def.name), 3.0));
        }

        // Strays far behind are brought back in front, as the reference genre does.
        if dist > 75.0 {
            let back = spawn_site(map, hpos, to_hero.z.atan2(to_hero.x), 38.0);
            tf.translation = outdoor_ground(collide.as_ref(), &world, back, map.floor);
            continue;
        }

        // Under Fade a hero is lost to the horde (bosses still see it): with every hero standing
        // lost, it mills where it stands.
        let chase = chase_target(pos, e.boss, e.taunted, run.elapsed, &seen);
        let lost = !hero_down && chase.is_none();
        let held = e.stun_t > 0.0 || e.root_t > 0.0 || lost;
        let to_target = chase.map_or(to_hero, |(_, at)| flat(at - pos));
        let contact = chase.is_some() && to_target.length() <= HERO_RADIUS + e.radius + 0.35;
        let mut step = Vec3::ZERO;
        let mut speed = 0.0;
        let mut face = Vec3::ZERO;
        if !held && !hero_down {
            face = if e.fear_t > 0.0 {
                -to_target.normalize_or_zero()
            } else {
                to_target.normalize_or_zero()
            };
            if !contact || e.fear_t > 0.0 {
                speed = e.speed * e.slow;
                step = face * speed;
            }
        }
        let shove = push.get(&ent).copied().unwrap_or_default();
        let mut detour = detours.get(&ent).copied();
        let stride = Stride {
            pos,
            chase: step * dt,
            shove: shove * (dt * 8.0).min(0.5),
            radius: e.radius,
            near: dist <= SWEEP_RANGE,
            side: if ent.to_bits() & 1 == 0 { 1.0 } else { -1.0 },
        };
        let mut next = walk_step(
            collide.as_ref(),
            &world,
            map.floor,
            stride,
            &mut detour,
            run.elapsed,
        );
        match detour {
            Some(d) => {
                // Going round a wall, it looks the way it walks.
                if step != Vec3::ZERO && d.active(run.elapsed) {
                    face = d.along;
                }
                detours.insert(ent, d);
            }
            None => {
                detours.remove(&ent);
            }
        }
        if face != Vec3::ZERO {
            let target = Quat::from_rotation_y(yaw_toward(face));
            tf.rotation = tf.rotation.slerp(target, (dt * 10.0).min(1.0));
        }
        // An enemy found under the heightfield (a cellar, under a floor) is put back outdoors
        // along its path: each one is looked at every eighth frame, staggered over the horde.
        let phase = ((run.elapsed * 60.0) as u32) ^ (ent.to_bits() as u32);
        if let Some(out) = phase
            .is_multiple_of(8)
            .then(|| rescue_enemy(collide.as_ref(), &world, next, hpos))
            .flatten()
        {
            next = out;
        }
        tf.translation = next;
        if speed > 0.0 {
            mv.speed = speed;
            mv.flags = move_flags::FORWARD;
        } else if mv.speed != 0.0 || mv.flags != 0 {
            mv.speed = 0.0;
            mv.flags = 0;
        }

        let Some((target, _)) = chase else { continue };
        if contact
            && !lost
            && e.stun_t <= 0.0
            && e.fear_t <= 0.0
            && e.attack_cd <= 0.0
            && !hero_down
        {
            let Ok((_, _, mut struck)) = heroes.get_mut(target) else {
                continue;
            };
            e.attack_cd = if e.boss { 1.0 } else { 1.3 };
            let raw = e.damage * e.weaken;
            let taken = hero::hurt_hero(&mut struck, raw);
            fx.swing(ent, target, taken);
            if taken > 0.0 {
                fx.number(target, taken, false, COLOR_HURT);
            }
            if struck.thorns_t > 0.0 {
                let thorns = struck.thorns * struck.stats().might;
                hits.write(Hit::direct(target, ent, thorns, false));
            }
            // A full Battlegear of Might strikes a share of the blow back.
            if struck.reflect > 0.0 {
                hits.write(Hit::direct(target, ent, raw * struck.reflect, false));
            }
            // A paladin's Retribution Aura answers the blow with holy damage, the paladin's own.
            if let Some((paladin, reflect)) = synergy::reflect(&struck, run.level) {
                hits.write(Hit {
                    color: COLOR_SPELL,
                    school: Some(School::Holy),
                    ..Hit::direct(paladin, ent, reflect, false)
                });
            }
        }
    }
}

/// Mark timers run down; the state kits reap with them.
pub(super) fn tick_marks(
    time: Res<Time>,
    mut units: Query<(Entity, &Guid, &mut ObjectStore, &mut Marks), With<Enemy>>,
    mut fields: MessageWriter<FieldChanged>,
) {
    let dt = time.delta_secs();
    for (entity, guid, mut store, mut marks) in &mut units {
        marks.tick(dt);
        marks.flush(entity, guid.0, &mut store, &mut fields);
    }
}

/// Corpses fade once their death has played.
pub(super) fn reap_dead(
    mut commands: Commands,
    time: Res<Time>,
    mut dying: Query<(Entity, &mut Dying)>,
) {
    for (e, mut d) in &mut dying {
        d.0 += time.delta_secs();
        if d.0 > 2.5 {
            commands
                .entity(e)
                .remove::<Dying>()
                .insert(benilla_world::model_fade::DespawnFade::default());
        }
    }
}

pub(super) fn mark_dying(commands: &mut Commands, e: Entity) {
    commands.entity(e).insert(Dying(0.0));
}

/// Most experience-gem bodies on the ground at once. Past it, a dense horde's XP joins an
/// existing gem instead of turning one frame of deaths into thousands of models or entities.
const MAX_GEMS: usize = 350;

fn merge_pickup(into: &mut Gem, from: GemKind) {
    match (&mut into.kind, from) {
        (GemKind::Xp(total), GemKind::Xp(value)) => *total = total.saturating_add(value),
        _ => unreachable!("only experience gems merge: each coin keeps its own rounding"),
    }
}

/// Spawn `kind`, or merge an overflowing XP gem into this frame's overflow body. The first
/// overflow finds a body once; the rest avoid repeatedly scanning the capped query.
fn add_pickup(
    commands: &mut Commands,
    gems: &mut Query<(Entity, &mut Gem)>,
    overflow: &mut Option<Entity>,
    count: &mut usize,
    serial: u64,
    pos: Vec3,
    kind: GemKind,
) {
    if !matches!(kind, GemKind::Xp(_)) || *count < MAX_GEMS {
        spawn_gem(commands, serial, pos, kind);
        *count += 1;
        return;
    }
    let target = (*overflow).or_else(|| {
        gems.iter_mut()
            .find_map(|(e, gem)| matches!(gem.kind, GemKind::Xp(_)).then_some(e))
    });
    if let Some(e) = target {
        if let Ok((_, mut gem)) = gems.get_mut(e) {
            merge_pickup(&mut gem, kind);
            *overflow = Some(e);
            return;
        }
    }
    // The cap was reached by non-XP pickups, so no XP body can receive this first overflow.
    // Keep one representative; later XP overflow joins it.
    *overflow = Some(spawn_gem(commands, serial, pos, kind));
    *count += 1;
}

pub(super) fn drop_loot(
    mut commands: Commands,
    mut drops: MessageReader<Loot>,
    mut rng: ResMut<Rng>,
    mut run: ResMut<Run>,
    mut gems: Query<(Entity, &mut Gem)>,
) {
    let mut count = gems.iter().count();
    let mut overflow = None;
    for d in drops.read() {
        if d.boss || rng.f32() < 0.015 {
            let heal = GemKind::Heal(if d.boss { 1000.0 } else { 30.0 });
            add_pickup(
                &mut commands,
                &mut gems,
                &mut overflow,
                &mut count,
                run.guid(),
                d.pos + Vec3::X * 0.8,
                heal,
            );
        }
        // Their worth is reckoned as they are picked up, by the picker's gold multiplier.
        if let Some(coins) = treasure::purse(d.boss, d.elite, run.elapsed, rng.f32()) {
            add_pickup(
                &mut commands,
                &mut gems,
                &mut overflow,
                &mut count,
                run.guid(),
                d.pos - Vec3::X * 0.8,
                GemKind::Gold(coins),
            );
        }
        if d.elite {
            run.elites += 1;
            treasure::spawn_cache(&mut commands, run.guid(), d.pos + Vec3::Z * 1.2, false);
        }
        // A Run Options density share can round to nothing: no empty wisp.
        if d.xp == 0 {
            continue;
        }
        add_pickup(
            &mut commands,
            &mut gems,
            &mut overflow,
            &mut count,
            run.guid(),
            d.pos,
            GemKind::Xp(d.xp),
        );
    }
}

pub(super) fn spawn_gem(commands: &mut Commands, serial: u64, pos: Vec3, kind: GemKind) -> Entity {
    let (display, scale) = look_for(kind);
    let e = units::spawn_object(commands, serial, display, scale, pos + Vec3::Y * 0.5);
    commands.entity(e).insert(Gem {
        kind,
        pulled: None,
        speed: 6.0,
    });
    e
}

/// Gems in a magnet's reach fly to the nearest hero standing whose magnet reaches them; experience
/// (under the picker's growth) and gold (under the picker's gold multiplier) go to the party's
/// pool, healing to the picker, and levels bank as they land.
#[allow(clippy::type_complexity)]
pub(super) fn collect_gems(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut next: ResMut<NextState<Phase>>,
    mut heroes: Query<
        (Entity, &Transform, &mut HeroState, Has<LocalHero>),
        (With<Hero>, Without<Gem>),
    >,
    mut gems: Query<(Entity, &mut Transform, &mut Gem)>,
) {
    // Each hero standing: where gems fly to, its magnet, its growth and its gold multiplier.
    let standing: Vec<(Entity, Vec3, f32, f32, f32)> = heroes
        .iter()
        .filter(|(_, _, h, _)| h.alive())
        .map(|(e, tf, h, _)| {
            let s = h.stats();
            (
                e,
                tf.translation + Vec3::Y * 1.0,
                s.magnet,
                s.growth,
                s.gold,
            )
        })
        .collect();
    if standing.is_empty() {
        return;
    }
    let dt = time.delta_secs();
    for (e, mut tf, mut g) in &mut gems {
        let at = tf.translation;
        if g.pulled.is_none() {
            let reached = standing
                .iter()
                .filter(|h| flat(h.1 - at).length() <= h.2)
                .map(|h| (h.0, h.1));
            g.pulled = party::nearest(reached, at).map(|(hero, _)| hero);
        }
        let Some(to) = g.pulled else { continue };
        // Its hero fell on the way: it flies on to the nearest one standing.
        let Some(picker) = standing.iter().find(|h| h.0 == to).or_else(|| {
            standing
                .iter()
                .min_by(|a, b| hero::reach(at, a.1).total_cmp(&hero::reach(at, b.1)))
        }) else {
            continue;
        };
        g.pulled = Some(picker.0);
        let d = picker.1 - at;
        let dist = d.length();
        if dist < 0.9 {
            match g.kind {
                GemKind::Xp(xp) => run.xp += xp as f32 * data::XP_RATE * picker.3,
                GemKind::Heal(hp) => {
                    let target = party::support_target(picker.0);
                    if let Ok((.., mut hero, _)) = heroes.get_mut(target) {
                        party::heal(&mut hero, hp);
                    }
                }
                GemKind::Gold(coins) => {
                    let paid = coins.pays(picker.4);
                    run.gold += paid;
                    if let Ok((.., mut hero, _)) = heroes.get_mut(picker.0) {
                        hero.gold_collected += paid;
                        hero.gold += paid;
                    }
                }
            }
            commands.entity(e).despawn();
            continue;
        }
        g.speed += 30.0 * dt;
        tf.translation += d / dist * (g.speed * dt).min(dist);
    }
    let learned = run.level;
    while run.xp >= data::xp_to_next(run.level) {
        run.xp -= data::xp_to_next(run.level);
        run.level += 1;
        run.pending_levels += 1;
    }
    if run.level > learned {
        // A level is a breather even when Quick Picks keeps the horde moving.  The short
        // immunity is deliberately per hero, rather than a shared party shield.
        for (_, _, mut hero, _) in &mut heroes {
            if hero.alive() {
                hero.immune_t = hero.immune_t.max(1.0);
            }
        }
        run.banner = Some(("Level up!".into(), 1.0));
    }
    // The local hero is told of its mobility as the party reaches the level that teaches it.
    if learned < super::mobility::LEARN_LEVEL && run.level >= super::mobility::LEARN_LEVEL {
        let class = heroes
            .iter()
            .find(|(.., local)| *local)
            .map(|(_, _, h, _)| h.class.id);
        if let Some(m) = class.and_then(super::mobility::for_class) {
            run.banner = Some((format!("You have learned {}! Press F.", m.name), 4.0));
        }
    }
    if run.pending_levels > 0 {
        next.set(Phase::LevelUp);
    }
}

/// Pickups hang still: a wisp's own loop (its circling motes and their trails) stops on its first
/// frame, clip and global sequences alike, as soon as the model attaches.
pub(super) fn still_gems(
    mut players: Query<&mut AnimationPlayer, (With<Gem>, Added<AnimationPlayer>)>,
    mut drives: Query<&mut GlobalSeqDrive, (With<Gem>, Added<GlobalSeqDrive>)>,
) {
    for mut player in &mut players {
        for (_, active) in player.playing_animations_mut() {
            active.set_speed(0.0);
        }
    }
    for mut drive in &mut drives {
        drive.set_paused(true);
    }
}

#[cfg(test)]
mod tests {
    use avian3d::prelude::{Collider, PhysicsPlugins, RigidBody};
    use bevy::ecs::system::RunSystemOnce;

    use super::super::bonus::{Bonus, StatKey};
    use super::*;

    fn id(i: u32) -> Entity {
        Entity::from_raw_u32(i).unwrap()
    }

    fn seen(i: u32, x: f32) -> Seen {
        Seen {
            e: id(i),
            pos: Vec3::new(x, 0.0, 0.0),
            up: true,
            hidden: false,
            threat: 1.0,
        }
    }

    #[test]
    fn a_cave_probe_starts_under_the_ceiling_and_chooses_the_floor() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::transform::TransformPlugin,
            bevy::asset::AssetPlugin::default(),
            bevy::scene::ScenePlugin,
            PhysicsPlugins::new(bevy::app::PostUpdate),
        ));
        app.init_asset::<Mesh>();
        app.init_resource::<benilla_world::collision::MoverTraceExclusions>();
        let quad = |y| {
            (
                vec![
                    Vec3::new(-5.0, y, -5.0),
                    Vec3::new(5.0, y, -5.0),
                    Vec3::new(5.0, y, 5.0),
                    Vec3::new(-5.0, y, 5.0),
                ],
                vec![[0, 2, 1], [0, 3, 2]],
            )
        };
        for y in [0.0, 32.0] {
            let (verts, tris) = quad(y);
            app.world_mut().spawn((
                RigidBody::Static,
                Collider::trimesh(verts, tris),
                Transform::default(),
            ));
        }
        app.finish();
        app.cleanup();
        app.update();

        let ground = app
            .world_mut()
            .run_system_once(|collide: WorldCollision| {
                collision_ground_y(&collide, Vec3::ZERO, 0.0)
            })
            .expect("ground probe runs");
        assert_eq!(ground, Some(0.0), "the 32 yd roof is never considered");
    }

    type Mesh3 = (Vec<Vec3>, Vec<[u32; 3]>);

    /// A heightfield flat at y = 0 everywhere and indoors nowhere, so the colliders alone decide.
    struct OpenField;

    impl Heightfield for OpenField {
        fn height_under(&self, _: Vec3) -> Option<f32> {
            Some(0.0)
        }

        fn indoors(&self, _: Vec3) -> bool {
            false
        }
    }

    /// A map with no heightfield at all, as Molten Core's.
    struct NoField;

    impl Heightfield for NoField {
        fn height_under(&self, _: Vec3) -> Option<f32> {
            None
        }

        fn indoors(&self, _: Vec3) -> bool {
            false
        }
    }

    /// The flat heightfield, inside a building everywhere: a cellar.
    struct Cellar;

    impl Heightfield for Cellar {
        fn height_under(&self, _: Vec3) -> Option<f32> {
            Some(0.0)
        }

        fn indoors(&self, _: Vec3) -> bool {
            true
        }
    }

    /// A planar quad `c` (corners in order round it), wound to face `out`.
    fn quad(mesh: &mut Mesh3, c: [Vec3; 4], out: Vec3) {
        let i = mesh.0.len() as u32;
        mesh.0.extend(c);
        if (c[1] - c[0]).cross(c[2] - c[0]).dot(out) >= 0.0 {
            mesh.1.extend([[i, i + 1, i + 2], [i, i + 2, i + 3]]);
        } else {
            mesh.1.extend([[i, i + 2, i + 1], [i, i + 3, i + 2]]);
        }
    }

    /// A box's four walls, and its top when `top`, every face wound outwards (`facing` 1) or
    /// inwards (−1).
    fn shell(mesh: &mut Mesh3, min: Vec3, max: Vec3, facing: f32, top: bool) {
        let v = Vec3::new;
        let ([x0, y0, z0], [x1, y1, z1]) = (min.to_array(), max.to_array());
        let walls = [
            (
                [v(x0, y0, z0), v(x0, y1, z0), v(x0, y1, z1), v(x0, y0, z1)],
                Vec3::NEG_X,
            ),
            (
                [v(x1, y0, z0), v(x1, y1, z0), v(x1, y1, z1), v(x1, y0, z1)],
                Vec3::X,
            ),
            (
                [v(x0, y0, z0), v(x1, y0, z0), v(x1, y1, z0), v(x0, y1, z0)],
                Vec3::NEG_Z,
            ),
            (
                [v(x0, y0, z1), v(x1, y0, z1), v(x1, y1, z1), v(x0, y1, z1)],
                Vec3::Z,
            ),
        ];
        for (c, out) in walls {
            quad(mesh, c, out * facing);
        }
        if top {
            let roof = [v(x0, y1, z0), v(x1, y1, z0), v(x1, y1, z1), v(x0, y1, z1)];
            quad(mesh, roof, Vec3::Y * facing);
        }
    }

    /// Up-facing ground at y = 0 over x and z in `[min, max]`.
    fn ground_patch(mesh: &mut Mesh3, min: Vec2, max: Vec2) {
        let v = |x, z| Vec3::new(x, 0.0, z);
        let c = [
            v(min.x, min.y),
            v(max.x, min.y),
            v(max.x, max.y),
            v(min.x, max.y),
        ];
        quad(mesh, c, Vec3::Y);
    }

    /// The test house's footprint: a WMO-like building 10 × 10 yd and 6 high.
    const HOUSE_MIN: Vec3 = Vec3::new(10.0, 0.0, -5.0);
    const HOUSE_MAX: Vec3 = Vec3::new(20.0, 6.0, 5.0);

    fn in_house(p: Vec3) -> bool {
        p.x > HOUSE_MIN.x && p.x < HOUSE_MAX.x && p.z > HOUSE_MIN.z && p.z < HOUSE_MAX.z
    }

    /// Open ground with the house on it: outer walls and roof facing out, inner walls and
    /// ceiling facing in, as a building's two skins are.
    fn house_world() -> App {
        let mut ground = Mesh3::default();
        ground_patch(&mut ground, Vec2::splat(-60.0), Vec2::splat(60.0));
        let mut house = Mesh3::default();
        shell(&mut house, HOUSE_MIN, HOUSE_MAX, 1.0, true);
        let skin = Vec3::new(0.3, 0.0, 0.3);
        shell(
            &mut house,
            HOUSE_MIN + skin,
            HOUSE_MAX - skin - Vec3::Y * 0.3,
            -1.0,
            true,
        );
        physics_world(vec![ground, house])
    }

    fn physics_world(meshes: Vec<Mesh3>) -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::transform::TransformPlugin,
            bevy::asset::AssetPlugin::default(),
            bevy::scene::ScenePlugin,
            PhysicsPlugins::new(bevy::app::PostUpdate),
        ));
        app.init_asset::<Mesh>();
        app.init_resource::<benilla_world::collision::MoverTraceExclusions>();
        for (verts, tris) in meshes {
            app.world_mut().spawn((
                RigidBody::Static,
                Collider::trimesh(verts, tris),
                Transform::default(),
            ));
        }
        app.finish();
        app.cleanup();
        app.update();
        app
    }

    /// An enemy chasing prey straight through the house, 20 s of frames: how many frames it spent
    /// inside the footprint, how high it ever stood, and where it ended.
    fn chase_past_the_house(c: &WorldCollision, near: bool) -> (usize, f32, Vec3) {
        let prey = Vec3::new(30.0, 0.0, 1.0);
        let mut pos = Vec3::new(0.0, 0.0, 1.0);
        let mut detour = None;
        let (mut inside, mut highest) = (0, f32::MIN);
        let dt = 1.0 / 60.0;
        for frame in 0..60 * 20 {
            let chase = flat(prey - pos);
            if chase.length() < 1.0 {
                break;
            }
            let stride = Stride {
                pos,
                chase: chase.normalize() * 5.0 * dt,
                shove: Vec3::ZERO,
                radius: 0.5,
                near,
                side: 1.0,
            };
            pos = walk_step(
                Some(c),
                &OpenField,
                0.0,
                stride,
                &mut detour,
                frame as f32 * dt,
            );
            inside += usize::from(in_house(pos));
            highest = highest.max(pos.y);
        }
        (inside, highest, pos)
    }

    #[test]
    fn an_enemy_goes_round_a_house_not_through_it() {
        let mut app = house_world();
        for near in [true, false] {
            let (inside, highest, end) = app
                .world_mut()
                .run_system_once(move |c: WorldCollision| chase_past_the_house(&c, near))
                .expect("the chase runs");
            let how = if near { "swept" } else { "feeler" };
            assert_eq!(inside, 0, "{how}: frames spent inside the house's walls");
            assert!(
                highest < 0.1,
                "{how}: it walked the ground, never onto the roof, but stood {highest} yd up"
            );
            assert!(
                flat(end - Vec3::new(30.0, 0.0, 1.0)).length() < 1.5,
                "{how}: it found its way round to the prey, but ended at {end}"
            );
        }
    }

    #[test]
    fn a_wall_stops_an_enemy_and_it_slides_along() {
        let mut app = house_world();
        let (square, glancing) = app
            .world_mut()
            .run_system_once(|c: WorldCollision| {
                // Square on, 2 yd of step from 1 yd off the wall: it stops at its body's width.
                let square =
                    enemy_step(Some(&c), Vec3::new(9.0, 0.0, 0.0), Vec3::X * 2.0, 0.5, true);
                // At 45 degrees, it keeps what runs along the wall.
                let glancing = enemy_step(
                    Some(&c),
                    Vec3::new(9.0, 0.0, 0.0),
                    Vec3::new(2.0, 0.0, 2.0),
                    0.5,
                    true,
                );
                (square, glancing)
            })
            .expect("the steps run");
        let (at, wall) = square;
        assert!(
            at.x <= HOUSE_MIN.x - 0.45,
            "stopped short of the wall, not at {at}"
        );
        assert!(at.x > 9.3, "but it did walk up to it, to {at}");
        assert_eq!(wall.map(|n| n.x.round()), Some(-1.0), "the wall faces it");
        let (at, _) = glancing;
        assert!(
            at.x <= HOUSE_MIN.x - 0.45,
            "the glancing step stays outside, at {at}"
        );
        assert!(at.z > 1.5, "and slides along the wall, to {at}");
    }

    #[test]
    fn a_kerb_is_stepped_onto_and_a_roof_is_not() {
        let mut ground = Mesh3::default();
        ground_patch(&mut ground, Vec2::splat(-20.0), Vec2::splat(20.0));
        let mut blocks = Mesh3::default();
        // A 1 yd kerb, and a shed whose roof is 2.5 yd up.
        shell(
            &mut blocks,
            Vec3::new(0.0, 0.0, -2.0),
            Vec3::new(4.0, 1.0, 2.0),
            1.0,
            true,
        );
        shell(
            &mut blocks,
            Vec3::new(8.0, 0.0, -2.0),
            Vec3::new(12.0, 2.5, 2.0),
            1.0,
            true,
        );
        let mut app = physics_world(vec![ground, blocks]);
        let (kerb, shed) = app
            .world_mut()
            .run_system_once(|c: WorldCollision| {
                let at = |x| ground_on_collision(Some(&c), &OpenField, Vec3::new(x, 0.0, 0.0), 0.0);
                (at(2.0).y, at(10.0).y)
            })
            .expect("the probes run");
        assert!((kerb - 1.0).abs() < 1e-3, "a kerb is a step up, got {kerb}");
        assert!(
            shed.abs() < 1e-3,
            "a roof over the feet is never the floor, got {shed}"
        );
    }

    #[test]
    fn spawns_avoid_the_inside_of_a_house() {
        let mut app = house_world();
        let (open, under_roof, moved, indoors) = app
            .world_mut()
            .run_system_once(|c: WorldCollision| {
                let c = Some(&c);
                (
                    valid_outdoor_point(c, &OpenField, Vec3::new(0.0, 0.0, 0.0)),
                    valid_outdoor_point(c, &OpenField, Vec3::new(15.0, 0.0, 0.0)),
                    outdoor_ground(c, &OpenField, Vec3::new(15.0, 0.0, 0.0), 0.0),
                    valid_outdoor_point(c, &Cellar, Vec3::new(0.0, 0.0, 0.0)),
                )
            })
            .expect("the checks run");
        assert_eq!(
            open.map(|p| p.y.abs() < 1e-3),
            Some(true),
            "open ground is fine"
        );
        assert_eq!(
            under_roof, None,
            "under the house's ceiling is no spawn point"
        );
        assert!(
            !in_house(moved),
            "a spawn asked for inside is moved out, to {moved}"
        );
        assert!(moved.y.abs() < 1e-3, "onto the ground, at {moved}");
        assert_eq!(indoors, None, "inside a WMO group is no spawn point either");
    }

    #[test]
    fn a_hero_in_a_pit_is_lifted_out() {
        // Ground at y = 0 with a 4 × 4 yd hole in it, and a 6 yd pit under the hole that the
        // heightfield knows nothing of.
        let mut ground = Mesh3::default();
        ground_patch(&mut ground, Vec2::new(-40.0, -40.0), Vec2::new(-2.0, 40.0));
        ground_patch(&mut ground, Vec2::new(2.0, -40.0), Vec2::new(40.0, 40.0));
        ground_patch(&mut ground, Vec2::new(-2.0, -40.0), Vec2::new(2.0, -2.0));
        ground_patch(&mut ground, Vec2::new(-2.0, 2.0), Vec2::new(2.0, 40.0));
        let mut pit = Mesh3::default();
        let (lo, hi) = (Vec3::new(-2.0, -6.0, -2.0), Vec3::new(2.0, 0.0, 2.0));
        shell(&mut pit, lo, hi, -1.0, false);
        let v = |x, z| Vec3::new(x, -6.0, z);
        quad(
            &mut pit,
            [v(-2.0, -2.0), v(2.0, -2.0), v(2.0, 2.0), v(-2.0, 2.0)],
            Vec3::Y,
        );
        let mut app = physics_world(vec![ground, pit]);

        let feet = Vec3::new(0.0, -6.0, 0.0);
        let arena = Vec3::new(10.0, 0.0, 0.0);
        assert!(super::super::hero::fallen(&OpenField, feet, arena, false));
        // Not on a cave battleground, nor on a map with no heightfield, nor in a cellar until
        // the player is held against its walls.
        assert!(!super::super::hero::fallen(&OpenField, feet, feet, true));
        assert!(!super::super::hero::fallen(&NoField, feet, arena, true));
        assert!(!super::super::hero::fallen(&Cellar, feet, arena, false));
        assert!(super::super::hero::fallen(&Cellar, feet, arena, true));
        // Standing on open ground is never a fall.
        assert!(!super::super::hero::fallen(
            &OpenField,
            Vec3::ZERO,
            arena,
            true
        ));

        let (lifted, no_field) = app
            .world_mut()
            .run_system_once(move |c: WorldCollision| {
                use super::super::hero::escape_point;
                (
                    escape_point(Some(&c), &OpenField, feet, 0.0, arena, 40.0, 0.0),
                    escape_point(Some(&c), &NoField, feet, 3.0, arena, 40.0, 0.0),
                )
            })
            .expect("the escapes run");
        assert!(lifted.y.abs() < 1e-3, "lifted onto the ground, to {lifted}");
        assert!(
            lifted.x.abs() > 2.0 || lifted.z.abs() > 2.0,
            "beside the pit, not over it: {lifted}"
        );
        assert!(
            flat(lifted - feet).length() <= 6.0,
            "and to the nearest ground, not far off: {lifted}"
        );
        assert!(
            no_field.distance(arena) < 1e-3,
            "with no outdoors anywhere the battleground's centre is the way out, not {no_field}"
        );
    }

    /// Six hundred enemies, the horde's cap and then some, all within sweep range of the hero and
    /// pressing on the house: a frame of their walk must stay a small share of a frame.
    #[test]
    fn six_hundred_enemies_walk_the_world_in_a_frame() {
        let mut app = house_world();
        let elapsed = app
            .world_mut()
            .run_system_once(|c: WorldCollision| {
                let prey = Vec3::new(30.0, 0.0, 0.0);
                let mut horde: Vec<(Vec3, Option<Detour>)> = (0..600)
                    .map(|i| {
                        let (row, col) = ((i / 30) as f32, (i % 30) as f32);
                        (Vec3::new(-20.0 + row * 1.4, 0.0, -21.0 + col * 1.4), None)
                    })
                    .collect();
                let dt = 1.0 / 60.0;
                let frames = 10;
                let start = std::time::Instant::now();
                for frame in 0..frames {
                    for (i, (pos, detour)) in horde.iter_mut().enumerate() {
                        let stride = Stride {
                            pos: *pos,
                            chase: flat(prey - *pos).normalize_or_zero() * 5.0 * dt,
                            shove: Vec3::ZERO,
                            radius: 0.5,
                            near: !i.is_multiple_of(3),
                            side: 1.0,
                        };
                        *pos =
                            walk_step(Some(&c), &OpenField, 0.0, stride, detour, frame as f32 * dt);
                    }
                }
                start.elapsed() / frames
            })
            .expect("the horde walks");
        println!("600 enemies: {elapsed:?} a frame");
        // Generous for an unoptimised test build on a busy machine; a shipped frame is far less.
        assert!(
            elapsed < std::time::Duration::from_millis(100),
            "600 enemies took {elapsed:?} a frame"
        );
    }

    #[test]
    fn the_horde_goes_for_the_nearest_hero_standing() {
        let target = |party: &[Seen], boss: bool| {
            chase_target(Vec3::ZERO, boss, None, 0.0, party).map(|(e, _)| e)
        };
        let mut party = [seen(1, 10.0), seen(2, -3.0), seen(3, 6.0)];
        assert_eq!(
            chase_target(Vec3::ZERO, false, None, 0.0, &party),
            Some((id(2), party[1].pos))
        );
        // A hero down is passed over.
        party[1].up = false;
        assert_eq!(target(&party, false), Some(id(3)));
        // So is one under Fade, except by a boss.
        party[2].hidden = true;
        assert_eq!(target(&party, false), Some(id(1)));
        assert_eq!(target(&party, true), Some(id(3)));
        // Every hero standing hidden: the horde has lost them.
        party[0].hidden = true;
        assert_eq!(target(&party, false), None);
        // The whole party down: no one to chase.
        for h in &mut party {
            h.up = false;
        }
        assert_eq!(target(&party, true), None);
    }

    #[test]
    fn a_taunt_holds_the_enemy_while_it_lasts() {
        let party = [seen(1, 10.0), seen(2, -3.0)];
        let taunt = Some((id(1), 5.0));
        let target = |now: f32| chase_target(Vec3::ZERO, false, taunt, now, &party).map(|t| t.0);
        assert_eq!(target(4.0), Some(id(1)));
        assert_eq!(target(5.0), Some(id(2)), "over: back to the nearest");
        // A taunt from a hero who is down holds nothing.
        let mut down = party;
        down[0].up = false;
        assert_eq!(
            chase_target(Vec3::ZERO, false, taunt, 0.0, &down).map(|t| t.0),
            Some(id(2))
        );
    }

    #[test]
    fn a_tank_s_taunt_forces_the_enemy_onto_it() {
        use super::super::synergy::testing;
        let mut w = testing::world(1.0);
        w.resource_mut::<Run>().elapsed = 20.0;
        // The mage stands on the enemy; the warrior, a tank, 11 yd off.
        let hs = testing::heroes(&mut w, &[("Mage", 0.0), ("Warrior", 10.0)]);
        let foe = Enemy {
            def: &data::roster(0).enemies[0],
            hp: 100.0,
            max_hp: 100.0,
            speed: 0.0,
            damage: 0.0,
            radius: 0.6,
            xp: 1,
            boss: false,
            elite: false,
            guardian: false,
            dormant: false,
            attack_cd: 0.0,
            slow: 1.0,
            slow_t: 0.0,
            root_t: 0.0,
            stun_t: 0.0,
            fear_t: 0.0,
            weaken: 1.0,
            weaken_t: 0.0,
            dots: Vec::new(),
            dead: false,
            taunted: None,
            debuffs: Default::default(),
        };
        let foe = w.spawn((foe, Transform::from_xyz(-1.0, 0.0, 0.0))).id();
        let party = [
            Seen {
                e: hs[0],
                ..seen(0, 0.0)
            },
            Seen {
                e: hs[1],
                threat: 3.0,
                ..seen(0, 10.0)
            },
        ];
        let target = |w: &World, now: f32| {
            let taunt = w.get::<Enemy>(foe).unwrap().taunted;
            chase_target(Vec3::new(-1.0, 0.0, 0.0), false, taunt, now, &party).map(|t| t.0)
        };
        assert_eq!(
            target(&w, 20.0),
            Some(hs[0]),
            "the nearer mage, threat or not"
        );
        w.run_system_once(synergy::taunts).unwrap();
        let until = 20.0 + synergy::TAUNT_SECS;
        assert_eq!(w.get::<Enemy>(foe).unwrap().taunted, Some((hs[1], until)));
        assert_eq!(target(&w, 21.0), Some(hs[1]), "taunted onto the warrior");
        assert_eq!(target(&w, until), Some(hs[0]), "and back once it ends");
    }

    #[test]
    fn a_blow_hurts_only_the_hero_it_lands_on() {
        let party = [seen(1, 10.0), seen(2, -3.0)];
        let mut heroes = [HeroState::default(), HeroState::default()];
        let full = heroes[0].hp;
        let (struck, _) = chase_target(Vec3::ZERO, false, None, 0.0, &party).unwrap();
        let i = party.iter().position(|h| h.e == struck).unwrap();
        let taken = hero::hurt_hero(&mut heroes[i], 30.0);
        assert!(taken > 0.0);
        assert_eq!(heroes[1].hp, full - taken);
        assert_eq!(heroes[1].damage_taken, f64::from(taken));
        assert_eq!((heroes[0].hp, heroes[0].damage_taken), (full, 0.0));
    }

    fn hero_at(world: &mut World, x: f32, state: HeroState) -> Entity {
        let local = state.player == super::party::LOCAL_PLAYER;
        let hero = Hero::default();
        let mut e = world.spawn((hero, Transform::from_xyz(x, 0.0, 0.0), state));
        if local {
            e.insert(LocalHero);
        }
        e.id()
    }

    /// Coins worth `base` before the picker's multiplier, rounded down.
    fn coins(base: u32) -> GemKind {
        GemKind::Gold(treasure::Coins { base, dither: 0.0 })
    }

    fn gem_at(world: &mut World, at: Vec3, kind: GemKind) -> Entity {
        let gem = Gem {
            kind,
            pulled: None,
            speed: 6.0,
        };
        world.spawn((Transform::from_translation(at), gem)).id()
    }

    /// Two heroes, ten yards apart: the local one plain (a 4.5 yd magnet), the other with three
    /// times the reach and half again the growth.
    fn two_heroes(world: &mut World) -> (Entity, Entity) {
        world.init_resource::<Time>();
        world.init_resource::<Run>();
        world.init_resource::<NextState<Phase>>();
        let a = hero_at(world, 0.0, HeroState::default());
        let mut wide = HeroState {
            player: 1,
            ..HeroState::default()
        };
        wide.bonuses.add(&Bonus::new(StatKey::Pickup, 2.0));
        wide.bonuses.add(&Bonus::new(StatKey::Experience, 0.5));
        let b = hero_at(world, 10.0, wide);
        (a, b)
    }

    #[test]
    fn pickups_fly_to_the_nearest_hero_whose_magnet_reaches_them() {
        let mut world = World::new();
        let (a, b) = two_heroes(&mut world);
        // In both magnets: the nearer hero's.
        let both = gem_at(&mut world, Vec3::new(3.0, 0.0, 0.0), GemKind::Xp(1));
        // Nearer `a` but past its magnet: `b`'s, whose magnet reaches it.
        let past = gem_at(&mut world, Vec3::new(4.8, 0.0, 0.0), coins(1));
        // In no magnet: it lies where it fell.
        let none = gem_at(&mut world, Vec3::new(-20.0, 0.0, 0.0), GemKind::Xp(1));
        world.run_system_once(collect_gems).unwrap();
        let pulled = |g: Entity| world.get::<Gem>(g).unwrap().pulled;
        assert_eq!(pulled(both), Some(a));
        assert_eq!(pulled(past), Some(b));
        assert_eq!(pulled(none), None);
        // The hero it flies to falls: it flies on to the one still standing.
        world.get_mut::<HeroState>(b).unwrap().dead_t = Some(0.0);
        world.run_system_once(collect_gems).unwrap();
        assert_eq!(world.get::<Gem>(past).unwrap().pulled, Some(a));
    }

    #[test]
    fn experience_and_gold_go_to_the_pool_and_a_heal_to_its_picker() {
        let mut world = World::new();
        let (a, b) = two_heroes(&mut world);
        for hero in [a, b] {
            world.get_mut::<HeroState>(hero).unwrap().hp = 50.0;
        }
        // Each lands on its hero this frame.
        let on = |x: f32| Vec3::new(x, 1.0, 0.0);
        let gems = [
            gem_at(&mut world, on(10.0), GemKind::Xp(1)),
            gem_at(&mut world, on(0.0), coins(7)),
            gem_at(&mut world, on(0.0), GemKind::Heal(30.0)),
        ];
        world.run_system_once(collect_gems).unwrap();
        let run = world.resource::<Run>();
        // `b`'s growth on `b`'s pickup.
        assert_eq!(run.xp, data::XP_RATE * 1.5);
        assert_eq!(run.gold, 7);
        assert!(gems.iter().all(|&g| world.get_entity(g).is_err()), "taken");
        assert_eq!(world.get::<HeroState>(a).unwrap().hp, 80.0);
        assert_eq!(world.get::<HeroState>(b).unwrap().hp, 50.0);
        // A level's worth banks one hand for the party, which deals one pick to each hero.
        gem_at(&mut world, on(0.0), GemKind::Xp(5));
        world.run_system_once(collect_gems).unwrap();
        assert_eq!(world.resource::<Run>().level, 2);
        assert_eq!(world.resource::<Run>().pending_levels, 1);
        world
            .run_system_once(|mut run: ResMut<Run>, mut heroes: Query<&mut HeroState>| {
                party::deal_hands(&mut run, heroes.iter_mut());
            })
            .unwrap();
        let picks: Vec<u32> = [a, b]
            .iter()
            .map(|&h| world.get::<HeroState>(h).unwrap().picks)
            .collect();
        assert_eq!(picks, [1, 1]);
    }

    #[test]
    fn coins_pay_under_the_gold_multiplier_of_whoever_picks_them_up() {
        let mut world = World::new();
        let (a, b) = two_heroes(&mut world);
        world
            .get_mut::<HeroState>(b)
            .unwrap()
            .bonuses
            .add(&Bonus::new(StatKey::Gold, 1.0));
        let on = |x: f32| Vec3::new(x, 1.0, 0.0);
        // `b` (doubling its gold) picks up ten; `a` (no bonus) picks up ten.
        gem_at(&mut world, on(10.0), coins(10));
        world.run_system_once(collect_gems).unwrap();
        assert_eq!(world.resource::<Run>().gold, 20, "the picker's multiplier");
        gem_at(&mut world, on(0.0), coins(10));
        world.run_system_once(collect_gems).unwrap();
        assert_eq!(
            world.resource::<Run>().gold,
            30,
            "the local hero's is no one else's"
        );
        // Each picker is credited with what it picked up, for the results' Gold Goblin.
        let collected = |h: Entity| world.get::<HeroState>(h).unwrap().gold_collected;
        assert_eq!((collected(a), collected(b)), (10, 20));
    }

    #[test]
    fn an_overflow_bucket_absorbs_two_thousand_pickups() {
        let mut gem = Gem {
            kind: GemKind::Xp(1),
            pulled: None,
            speed: 6.0,
        };
        for _ in 1..2_000 {
            merge_pickup(&mut gem, GemKind::Xp(1));
        }
        assert!(matches!(gem.kind, GemKind::Xp(2_000)));
    }

    #[test]
    fn only_roster_bosses_grow_and_their_reach_with_them() {
        let def = &data::roster(0).bosses[0];
        let base = treasure::Body {
            hp: 100.0,
            damage: 10.0,
            scale: def.scale,
        };
        let boss = ranked_body(base, Rank::Boss);
        assert!((boss.scale - def.scale * BOSS_SCALE).abs() < 1e-5);
        assert_eq!((boss.hp, boss.damage), (base.hp, base.damage));
        assert_eq!(ranked_body(base, Rank::Horde), base);
        // Elites and guardians keep their own growth.
        for rank in [Rank::Elite, Rank::Guardian] {
            assert_eq!(ranked_body(base, rank), treasure::elite_body(base));
        }
        // The body radius, and so the swing's contact range, grows by the same factor.
        let r = body_radius(def, boss.scale);
        assert!((r - radius_for(def) * BOSS_SCALE).abs() < 1e-5);
        assert!(r + HERO_RADIUS > radius_for(def) + HERO_RADIUS);
    }
}
