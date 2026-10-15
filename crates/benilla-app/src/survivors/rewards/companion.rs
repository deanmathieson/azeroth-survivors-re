//! The companion: a critter the Companion perk brings on every run, two with the Second Companion.
//! It keeps to its owner's heel and, every [`pet::fetch_every`] seconds, runs out for the nearest
//! experience gem or coin lying beyond its owner's pickup range but within [`pet::fetch_reach`]
//! yards of it. Reaching one sends the pickup to the owner along the magnet's own path
//! (`enemies::collect_gems`), so it is collected as any other; with Saddlebags it goes on to the
//! next before coming back. Healing is left where it fell and treasure caches are never touched.
//! The other skills ([`super::pet`]) bite, dig up coins and heal as it fetches.
//!
//! Each hero's companions are its own ([`Companion::owner`]): its player's profile dresses and
//! trains them. Only the local hero's come along today, the profile being the local player's.
//!
//! At the menu the companions sit beside the preview hero (the local hero, out of a run) and are
//! dressed anew as soon as a look is picked ([`menu_companions`]); the run then takes them over.
//! They are run entities: they go with the run, and the menu brings them back.

use benilla_world::collision::WorldCollision;
use benilla_world::schedule::WorldStage;
use benilla_world::world_point::WorldPoint;
use bevy::prelude::*;

use super::super::abilities::Hit;
use super::super::enemies::{self, Enemy, Gem, GemKind};
use super::super::hero::Hero;
use super::super::profile::Profile;
use super::super::units::{self, flat, yaw_toward, UnitSpec};
use super::super::{hud, party, treasure, HeroState, LocalHero, Phase, Run};
use super::looks::{look_for, Look};
use super::perks;
use super::pet::{self, PetSkills};
use super::save::RewardsSave;
use crate::creature_anim::{move_flags, MovementState};

/// Its speed out to a pickup, and its most when hurrying after the hero, in yards a second, before
/// Swiftness.
const DASH_SPEED: f32 = 16.0;
/// How near the hero it settles.
const HEEL: f32 = 2.5;
/// Farther than its fetch reach and this from the hero (after a Blink, say), it appears at the
/// hero's side.
const LEASH_SLACK: f32 = 15.0;
/// How near a pickup it must come to send it off.
const GRAB: f32 = 1.2;
/// Seconds before a fetch is given up.
const GIVE_UP: f32 = 4.0;
/// Where a companion sits at the menu, in the hero's own frame: to its side and a step ahead.
const MENU_SIDE: f32 = 1.5;
const MENU_AHEAD: f32 = 0.5;

/// One companion in the world.
#[derive(Component)]
pub(in crate::survivors) struct Companion {
    /// The hero it heels to, fetches for and fights beside: whose credit, gold and heals it earns.
    pub(in crate::survivors) owner: Entity,
    /// 0 the first pet, 1 the Second Companion's.
    slot: u8,
    /// The display it wears, so a look picked at the menu is seen.
    display: u32,
    /// The ranks it works with: its owner's training.
    skills: PetSkills,
    /// Seconds to the next fetch, bite and coin pile.
    next_fetch: f32,
    next_nip: f32,
    next_truffle: f32,
    /// The pickup it is running for, and the seconds spent on it.
    fetching: Option<(Entity, f32)>,
    /// Items sent on this trip, against [`pet::items_per_trip`].
    carried: u32,
}

impl Companion {
    fn new(owner: Entity, slot: u8, display: u32, skills: PetSkills) -> Self {
        let mut pet = Self {
            owner,
            slot,
            display,
            skills,
            next_fetch: 0.0,
            next_nip: 0.0,
            next_truffle: 0.0,
            fetching: None,
            carried: 0,
        };
        pet.train(skills);
        pet
    }

    /// Work with `skills` from now, its clocks wound for them.
    fn train(&mut self, skills: PetSkills) {
        self.skills = skills;
        self.next_fetch = pet::fetch_every(skills.swiftness);
        self.next_nip = 0.0;
        self.next_truffle = pet::truffle_every(skills.truffle_snout).unwrap_or(0.0);
    }
}

/// The local player's companions' haul this run: the items they fetched, which the pet's
/// experience counts as the run is recorded ([`super::award_pet_xp`]).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::survivors) struct PetRun {
    pub(in crate::survivors) fetched: u32,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<PetRun>()
        .add_systems(
            OnTransition {
                exited: Phase::Menu,
                entered: Phase::Playing,
            },
            |mut haul: ResMut<PetRun>| *haul = PetRun::default(),
        )
        .add_systems(
            Update,
            (spawn_companions, run_companions)
                .chain()
                .after(WorldStage::Input)
                .run_if(in_state(Phase::Playing))
                .run_if(|l: Res<crate::loading_screen::LoadingScreen>| !l.covering()),
        )
        .add_systems(
            Update,
            menu_companions
                .after(hud::menu_preview)
                .run_if(in_state(Phase::Menu)),
        );
}

/// The look `slot`'s companion wears.
pub(in crate::survivors) fn slot_look(save: &RewardsSave, slot: u8) -> &'static Look {
    look_for(if slot == 0 {
        &save.companion
    } else {
        &save.second_look
    })
}

/// The side of the hero `slot`'s companion keeps to: the first on the right, the second on the
/// left.
fn side(slot: u8) -> Vec3 {
    if slot == 0 {
        Vec3::X
    } else {
        Vec3::NEG_X
    }
}

/// The pickup to fetch: of `pickups` (a key and a position each), the nearest to `from` on the
/// ground plane that lies beyond `magnet` of `hero`, which takes the nearer ones itself, and
/// within `reach` of it. A trip's first pickup is the nearest to the hero; the next ones, with
/// Saddlebags, the nearest to the pet.
pub(in crate::survivors) fn fetch_target<T>(
    from: Vec3,
    hero: Vec3,
    magnet: f32,
    reach: f32,
    pickups: impl IntoIterator<Item = (T, Vec3)>,
) -> Option<T> {
    pickups
        .into_iter()
        .filter(|&(_, at)| {
            let d = flat(at - hero).length();
            d > magnet && d <= reach
        })
        .map(|(key, at)| (key, flat(at - from).length()))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(key, _)| key)
}

/// Experience and coins not already flying to a hero.
fn fetchable(gem: &Gem) -> bool {
    gem.pulled.is_none() && matches!(gem.kind, GemKind::Xp(_) | GemKind::Gold(_))
}

/// Spawn `slot`'s companion for `owner`, dressed in `look` at its pet level, at `pos` facing
/// `yaw`.
#[allow(clippy::too_many_arguments)]
fn spawn_pet(
    commands: &mut Commands,
    serial: u64,
    owner: Entity,
    slot: u8,
    look: &Look,
    level: u32,
    skills: PetSkills,
    pos: Vec3,
    yaw: f32,
) -> Entity {
    let e = units::spawn_unit(
        commands,
        &UnitSpec {
            serial,
            display: look.display,
            scale: look.scale,
            level,
            faction: units::FACTION_FRIENDLY,
            bytes0: 1 << 8,
            weapons: &[],
            sheath: 0,
        },
        pos,
        yaw,
    );
    commands
        .entity(e)
        .insert(Companion::new(owner, slot, look.display, skills));
    e
}

/// Once a run is under way, the local hero's companions join it at its side if the perk is bought
/// (the perk is the local player's), unless the menu's are already with it.
fn spawn_companions(
    mut commands: Commands,
    profile: Res<Profile>,
    mut run: ResMut<Run>,
    hero: Query<(Entity, &Transform), With<LocalHero>>,
    companions: Query<&Companion>,
) {
    let save = &profile.rewards;
    if !perks::companion_unlocked(save) {
        return;
    }
    let Ok((owner, h)) = hero.single() else {
        return;
    };
    let skills = PetSkills::of(save);
    for slot in 0..skills.pets() {
        if companions
            .iter()
            .any(|c| c.owner == owner && c.slot == slot)
        {
            continue;
        }
        let pos = h.translation + side(slot) * HEEL;
        spawn_pet(
            &mut commands,
            run.guid(),
            owner,
            slot,
            slot_look(save, slot),
            pet::level(save.pet_xp),
            skills,
            pos,
            yaw_toward(h.translation - pos),
        );
    }
}

/// Heel, fetch and come back, bite and dig: see the module doc.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn run_companions(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut haul: ResMut<PetRun>,
    world: WorldPoint,
    mut heroes: Query<
        (&Transform, &mut HeroState, Has<LocalHero>),
        (With<Hero>, Without<Companion>),
    >,
    mut companions: Query<
        (&mut Transform, &mut MovementState, &mut Companion),
        (Without<Hero>, Without<Gem>, Without<Enemy>),
    >,
    mut gems: Query<(Entity, &Transform, &mut Gem), (Without<Hero>, Without<Companion>)>,
    foes: Query<(Entity, &Transform, &Enemy), (Without<Hero>, Without<Companion>, Without<Gem>)>,
    mut hits: MessageWriter<Hit>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    for (mut tf, mut mv, mut pet) in &mut companions {
        let Ok((h, mut me, local)) = heroes.get_mut(pet.owner) else {
            continue;
        };
        let hpos = h.translation;
        let stats = me.stats();
        let down = me.dead_t.is_some();
        let skills = pet.skills;
        let reach = pet::fetch_reach(skills.wide_patrol);
        let pos = tf.translation;
        if flat(hpos - pos).length() > reach + LEASH_SLACK {
            tf.translation = enemies::ground(&world, hpos + side(pet.slot) * HEEL, hpos.y);
            pet.fetching = None;
            continue;
        }
        // Out after a pickup: there, send it on and go for the next if the bags have room; gone,
        // already flying or chased too long, drop it.
        let mut chase = None;
        if let Some((target, t)) = pet.fetching.take() {
            let mut grabbed = false;
            if let Ok((_, at, mut gem)) = gems.get_mut(target) {
                if gem.pulled.is_none() && t < GIVE_UP && !down {
                    if flat(at.translation - pos).length() <= GRAB {
                        gem.pulled = Some(pet.owner);
                        grabbed = true;
                    } else {
                        pet.fetching = Some((target, t + dt));
                        chase = Some(at.translation);
                    }
                }
            }
            if grabbed {
                pet.carried += 1;
                if local {
                    haul.fetched = haul.fetched.saturating_add(1);
                }
                let heal = pet::loyal_heal(skills.loyal) * stats.max_hp;
                if heal > 0.0 {
                    party::heal(&mut me, heal);
                }
                if pet.carried < pet::items_per_trip(skills.saddlebags) {
                    let pickups = gems
                        .iter()
                        .filter(|(_, _, gem)| fetchable(gem))
                        .map(|(e, at, _)| (e, at.translation));
                    if let Some(next) = fetch_target(pos, hpos, stats.magnet, reach, pickups) {
                        pet.fetching = Some((next, 0.0));
                        chase = gems.get(next).ok().map(|(_, at, _)| at.translation);
                    }
                }
            }
        }
        pet.next_fetch -= dt;
        if pet.fetching.is_none() && pet.next_fetch <= 0.0 && !down {
            pet.next_fetch = pet::fetch_every(skills.swiftness);
            pet.carried = 0;
            let pickups = gems
                .iter()
                .filter(|(_, _, gem)| fetchable(gem))
                .map(|(e, at, _)| (e, at.translation));
            pet.fetching = fetch_target(hpos, hpos, stats.magnet, reach, pickups).map(|e| (e, 0.0));
        }
        // Nip: the nearest enemy in reach, as soon as the last bite is done; the blow is the
        // owner's, its damage and any kill credited to that hero.
        if skills.nip > 0 && !down {
            pet.next_nip = (pet.next_nip - dt).max(0.0);
            if pet.next_nip <= 0.0 {
                let bitten = foes
                    .iter()
                    .filter(|(_, _, en)| !en.dead)
                    .map(|(e, at, en)| (e, flat(at.translation - pos).length() - en.radius))
                    .filter(|&(_, d)| d <= pet::NIP_RANGE)
                    .min_by(|a, b| a.1.total_cmp(&b.1));
                if let Some((foe, _)) = bitten {
                    let damage = pet::nip_damage(skills.nip, run.level);
                    hits.write(Hit::direct(pet.owner, foe, damage, false));
                    pet.next_nip = pet::NIP_EVERY;
                }
            }
        }
        // Truffle Snout: a small pile of coins at its feet, worth what the gold bonus of the hero
        // who picks it up makes it, as every coin is.
        if let Some(every) = pet::truffle_every(skills.truffle_snout) {
            pet.next_truffle -= dt;
            if pet.next_truffle <= 0.0 && !down {
                pet.next_truffle = every;
                let coins = treasure::Coins::new(pet::TRUFFLE_COINS, run.elapsed, 0.5);
                let serial = run.guid();
                enemies::spawn_gem(&mut commands, serial, pos, GemKind::Gold(coins));
            }
        }
        // To the pickup, else to a spot at the hero's heel on its own side.
        let quick = pet::run_speed(skills.swiftness);
        let dash = DASH_SPEED * quick;
        let (dest, speed) = match chase {
            Some(at) => (at, dash),
            None => {
                let spot = hpos + flat(pos - hpos).normalize_or(side(pet.slot)) * HEEL;
                let gap = flat(spot - pos).length();
                (spot, (stats.speed * 1.2 * quick).max(gap * 2.0).min(dash))
            }
        };
        let step = flat(dest - pos);
        let dist = step.length();
        if dist > 0.3 {
            let dir = step / dist;
            let next = pos + dir * (speed * dt).min(dist);
            tf.translation = enemies::ground(&world, next, pos.y);
            let face = Quat::from_rotation_y(yaw_toward(dir));
            tf.rotation = tf.rotation.slerp(face, (dt * 10.0).min(1.0));
            mv.speed = speed;
            mv.flags = move_flags::FORWARD;
        } else if mv.speed != 0.0 || mv.flags != 0 {
            mv.speed = 0.0;
            mv.flags = 0;
        }
    }
}

/// Where `slot`'s companion sits at the menu beside a hero standing at `hero`: on its side and a
/// step ahead, facing as it faces.
fn menu_spot(hero: &Transform, slot: u8) -> Vec3 {
    let along = side(slot) * MENU_SIDE + Vec3::NEG_Z * MENU_AHEAD;
    hero.translation + hero.rotation * along
}

/// At the menu, the companions sit beside the preview hero once the perk is bought: the first in
/// its picked look, and the second in its own with the Second Companion. A new pick, a new hero
/// (another class or look) or a refund replaces or removes them at once; new training is theirs
/// as it is made.
#[allow(clippy::type_complexity)]
fn menu_companions(
    mut commands: Commands,
    profile: Res<Profile>,
    mut run: ResMut<Run>,
    collide: Option<WorldCollision>,
    world: WorldPoint,
    hero: Query<(Entity, &Transform), (With<LocalHero>, Without<Companion>)>,
    mut pets: Query<(Entity, &mut Transform, &mut Companion), Without<LocalHero>>,
) {
    let save = &profile.rewards;
    let hero = hero.single().ok();
    let skills = PetSkills::of(save);
    let want = match hero {
        Some(_) if perks::companion_unlocked(save) => skills.pets(),
        _ => 0,
    };
    let mut seated = [false; 2];
    for (e, mut tf, mut pet) in &mut pets {
        let keep = hero.is_some_and(|(owner, _)| owner == pet.owner)
            && pet.slot < want
            && !seated[pet.slot as usize]
            && pet.display == slot_look(save, pet.slot).display;
        let Some((_, h)) = hero.filter(|_| keep) else {
            commands.entity(e).despawn();
            continue;
        };
        seated[pet.slot as usize] = true;
        if pet.skills != skills {
            pet.train(skills);
        }
        let at = enemies::ground_on_collision(
            collide.as_ref(),
            &world,
            menu_spot(h, pet.slot),
            h.translation.y,
        );
        if tf.translation.distance_squared(at) > 1e-6 {
            tf.translation = at;
        }
        if tf.rotation != h.rotation {
            tf.rotation = h.rotation;
        }
    }
    let Some((owner, h)) = hero else {
        return;
    };
    for slot in (0..want).filter(|&s| !seated[s as usize]) {
        let at = enemies::ground_on_collision(
            collide.as_ref(),
            &world,
            menu_spot(h, slot),
            h.translation.y,
        );
        let e = spawn_pet(
            &mut commands,
            run.guid(),
            owner,
            slot,
            slot_look(save, slot),
            pet::level(save.pet_xp),
            skills,
            at,
            0.0,
        );
        commands
            .entity(e)
            .insert(Transform::from_translation(at).with_rotation(h.rotation));
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::net::NetEntity;

    #[test]
    fn fetches_the_nearest_pickup_past_the_magnet_and_within_reach() {
        let hero = Vec3::new(10.0, 5.0, -3.0);
        let at = |x: f32, y: f32, z: f32| hero + Vec3::new(x, y, z);
        let reach = pet::BASE_REACH;
        let pickups = [
            ("inside the magnet", at(3.0, 0.0, 0.0)),
            ("far", at(0.0, 0.0, 20.0)),
            // A hillside: the height between does not count.
            ("near, uphill", at(6.0, 9.0, 6.0)),
            ("out of reach", at(26.0, 0.0, 0.0)),
        ];
        assert_eq!(
            fetch_target(hero, hero, 4.5, reach, pickups),
            Some("near, uphill")
        );
        // With a bigger magnet the near one is the magnet's.
        assert_eq!(fetch_target(hero, hero, 9.0, reach, pickups), Some("far"));
        assert_eq!(
            fetch_target(hero, hero, 4.5, reach, [("edge", at(reach, 0.0, 0.0))]),
            Some("edge")
        );
        assert_eq!(
            fetch_target(hero, hero, 4.5, reach, [pickups[0], pickups[3]]),
            None,
            "nothing between the magnet and the reach"
        );
        assert_eq!(fetch_target::<u8>(hero, hero, 4.5, reach, []), None);
        // Wide Patrol reaches the far one.
        assert_eq!(
            fetch_target(hero, hero, 4.5, pet::fetch_reach(1), [pickups[3]]),
            Some("out of reach")
        );
        // On a trip, the next is the nearest to the pet, still within the hero's reach.
        let from = at(0.0, 0.0, 18.0);
        assert_eq!(
            fetch_target(from, hero, 4.5, reach, [pickups[1], pickups[2]]),
            Some("far")
        );
    }

    /// A menu with the preview hero, and a profile with the companion bought.
    fn menu() -> (World, Entity) {
        let mut world = World::new();
        benilla_world::world_point::init_world_point_resources(&mut world);
        world.init_resource::<Run>();
        let mut profile = Profile::default();
        profile.rewards.perks.insert("companion".into(), 1);
        world.insert_resource(profile);
        let hero = world
            .spawn((
                Transform::from_xyz(5.0, 2.0, -4.0),
                HeroState::default(),
                LocalHero,
            ))
            .id();
        (world, hero)
    }

    /// The companions in the world, by slot: the display each wears, its owner, and the display
    /// its unit shows.
    fn seated(world: &mut World) -> Vec<(u8, u32, Entity, Option<u32>)> {
        let mut q = world.query::<(&Companion, &NetEntity)>();
        let mut out: Vec<_> = q
            .iter(world)
            .map(|(c, n)| (c.slot, c.display, c.owner, n.display_id))
            .collect();
        out.sort_unstable_by_key(|s| s.0);
        out
    }

    #[test]
    fn the_menu_companion_follows_the_chosen_look() {
        let (mut world, hero) = menu();
        world.run_system_once(menu_companions).unwrap();
        let squirrel = look_for("").display;
        assert_eq!(seated(&mut world), [(0, squirrel, hero, Some(squirrel))]);
        // Beside the hero, on the ground it stands on.
        let mut q = world.query_filtered::<&Transform, With<Companion>>();
        let at = q.single(&world).unwrap().translation;
        let beside = Vec3::new(5.0 + MENU_SIDE, 2.0, -4.0 - MENU_AHEAD);
        assert!((at - beside).length() < 1e-4, "{at}");
        // Nothing changed: the same one stays.
        world.run_system_once(menu_companions).unwrap();
        assert_eq!(seated(&mut world).len(), 1);

        // A new look is worn at once.
        world.resource_mut::<Profile>().rewards.companion = "frog".into();
        world.run_system_once(menu_companions).unwrap();
        assert_eq!(seated(&mut world), [(0, 901, hero, Some(901))]);

        // The Second Companion sits on the other side, in its own look.
        {
            let mut profile = world.resource_mut::<Profile>();
            profile.rewards.pet_xp = pet::xp_for_level(pet::MAX_LEVEL);
            profile
                .rewards
                .pet_skills
                .insert("second_companion".into(), 1);
            profile.rewards.second_look = "cat".into();
        }
        world.run_system_once(menu_companions).unwrap();
        assert_eq!(
            seated(&mut world),
            [(0, 901, hero, Some(901)), (1, 5554, hero, Some(5554))]
        );

        // A new preview hero takes them over; a refund sends them off.
        world.entity_mut(hero).despawn();
        let next = world
            .spawn((Transform::default(), HeroState::default(), LocalHero))
            .id();
        world.run_system_once(menu_companions).unwrap();
        let now = seated(&mut world);
        assert_eq!(now.len(), 2);
        assert!(now.iter().all(|s| s.2 == next));
        world.resource_mut::<Profile>().rewards.perks.clear();
        world.run_system_once(menu_companions).unwrap();
        assert!(seated(&mut world).is_empty());
    }

    #[test]
    fn menu_training_reaches_the_seated_companion() {
        let (mut world, _) = menu();
        world.run_system_once(menu_companions).unwrap();
        world
            .resource_mut::<Profile>()
            .rewards
            .pet_skills
            .insert("truffle_snout".into(), 2);
        world.run_system_once(menu_companions).unwrap();
        let mut q = world.query::<&Companion>();
        let pet = q.single(&world).unwrap();
        assert_eq!(pet.skills.truffle_snout, 2);
        assert_eq!(pet.next_truffle, 40.0, "its clock wound for the new rank");
    }
}
