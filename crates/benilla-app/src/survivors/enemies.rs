//! The horde: spawning on a ring outside the view, the chase, contact damage, death, and the
//! experience gems and coins the dead leave.

use std::collections::HashMap;

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

/// The clearance above a known stage or hero floor that a spawn probe starts from.  It stays under
/// a cave roof, unlike the old 64 yd outdoor probe, while still clearing a floor-plane roundoff.
const GROUND_PROBE_LIFT: f32 = 4.0;

fn collision_ground_y(collide: &WorldCollision, p: Vec3, fallback: f32) -> Option<f32> {
    let reference = p.y.max(fallback);
    let from = p.with_y(reference + GROUND_PROBE_LIFT);
    collide
        .ray_body(from, Dir3::NEG_Y, 128.0)
        .map(|hit| from.y - hit.distance)
        .filter(|&y| y <= reference + f32::EPSILON)
}

/// Ground a walking body on collision first, falling back to terrain while the collider streams.
pub(super) fn ground_on_collision(
    collide: Option<&WorldCollision>,
    world: &WorldPoint,
    mut p: Vec3,
    fallback: f32,
) -> Vec3 {
    // The reference point is already a floor (the hero or MapDef floor).  Beginning just above it
    // keeps a WMO ceiling out of the ray, so its nearest hit is the floor below the reference.
    p.y = collide
        .and_then(|collide| collision_ground_y(collide, p, fallback))
        .or_else(|| world.terrain_height_under(p))
        .unwrap_or(fallback);
    p
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
                    let pos = ground_on_collision(collide.as_ref(), &world, at, map.floor);
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
                    let pos = ground(&world, at, map.floor);
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
                        let pos = ground_on_collision(collide.as_ref(), &world, at, map.floor);
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
                        let pos = ground_on_collision(collide.as_ref(), &world, at, map.floor);
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
        let pos = ground_on_collision(collide.as_ref(), &world, at, map.floor);
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
            let pos = ground_on_collision(collide.as_ref(), &world, at, map.floor);
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
        let pos = ground_on_collision(collide.as_ref(), &world, at, map.floor);
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
            tf.translation = ground_on_collision(collide.as_ref(), &world, back, map.floor);
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
        if !held && !hero_down {
            let dir = if e.fear_t > 0.0 {
                -to_target.normalize_or_zero()
            } else {
                to_target.normalize_or_zero()
            };
            if !contact || e.fear_t > 0.0 {
                speed = e.speed * e.slow;
                step = dir * speed;
            }
            if dir != Vec3::ZERO {
                let target = Quat::from_rotation_y(yaw_toward(dir));
                tf.rotation = tf.rotation.slerp(target, (dt * 10.0).min(1.0));
            }
        }
        let shove = push.get(&ent).copied().unwrap_or_default();
        let mut next = pos + step * dt + shove * (dt * 8.0).min(0.5);
        next = ground_on_collision(collide.as_ref(), &world, next, map.floor);
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
