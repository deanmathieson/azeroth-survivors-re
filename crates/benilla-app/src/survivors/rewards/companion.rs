//! The companion: a critter the Companion perk brings on every run. It keeps to the local hero's
//! heel and, every [`FETCH_EVERY`] seconds, runs out for the nearest experience gem or coin lying
//! beyond the hero's pickup range but within [`FETCH_REACH`] yards of the hero. Reaching it sends
//! the pickup to the hero along the magnet's own path (`enemies::collect_gems`), so it is collected
//! as any other. Healing is left where it fell and treasure caches are never touched. The critter
//! is a run entity: it goes with the run.

use benilla_world::schedule::WorldStage;
use benilla_world::world_point::WorldPoint;
use bevy::prelude::*;

use super::super::enemies::{self, Gem, GemKind};
use super::super::hero::Hero;
use super::super::profile::Profile;
use super::super::units::{self, flat, yaw_toward, UnitSpec};
use super::super::{HeroState, LocalHero, Phase, Run};
use super::perks;
use super::save::RewardsSave;
use crate::creature_anim::{move_flags, MovementState};

/// One look the companion can wear.
pub(in crate::survivors) struct Look {
    /// The key a save stores.
    pub(in crate::survivors) key: &'static str,
    pub(in crate::survivors) name: &'static str,
    /// The `CreatureDisplayInfo.dbc` row.
    pub(in crate::survivors) display: u32,
    /// `OBJECT_FIELD_SCALE_X`, which alone sizes a unit: about 1.8 times the display's own
    /// scale, so the critter reads under the raised camera.
    pub(in crate::survivors) scale: f32,
}

/// The looks, each checked against the 1.12 client's `CreatureDisplayInfo.dbc` and
/// `CreatureModelData.dbc`: the model is named beside it.
pub(in crate::survivors) static LOOKS: &[Look] = &[
    // `Creature\Squirrel\Squirrel.mdx`, display scale 1.3.
    Look {
        key: "squirrel",
        name: "Squirrel",
        display: 134,
        scale: 2.3,
    },
    // `Creature\Rabbit\Rabbit.mdx`.
    Look {
        key: "rabbit",
        name: "Rabbit",
        display: 328,
        scale: 1.8,
    },
    // `Creature\Frog\Frog.mdx`, brown spots.
    Look {
        key: "frog",
        name: "Frog",
        display: 901,
        scale: 1.8,
    },
    // `Creature\Cat\Cat.mdx`, the orange tabby, display scale 0.5.
    Look {
        key: "cat",
        name: "Tabby Cat",
        display: 5554,
        scale: 0.9,
    },
    // `Creature\Chicken\Chicken.mdx`.
    Look {
        key: "chicken",
        name: "Chicken",
        display: 304,
        scale: 1.8,
    },
];

/// Looks earned elsewhere, offered once their display is in [`RewardsSave::unlocked_looks`]: the
/// quest chains' rewards ([`crate::survivors::quests`]). Checked as [`LOOKS`] are.
pub(in crate::survivors) static EARNED_LOOKS: &[Look] = &[
    // `Creature\snake\snake.mdx`, black, the Black Kingsnake's.
    Look {
        key: "black_kingsnake",
        name: "Black Kingsnake",
        display: 1206,
        scale: 1.8,
    },
    // `Creature\cockroach\Cockroach.mdx`.
    Look {
        key: "undercity_cockroach",
        name: "Undercity Cockroach",
        display: 2177,
        scale: 2.4,
    },
    // `Creature\DragonWhelp\DragonWhelp.mdx`, green, display scale 0.5.
    Look {
        key: "emerald_whelpling",
        name: "Emerald Whelpling",
        display: 6291,
        scale: 0.9,
    },
];

/// Every look, [`LOOKS`] then [`EARNED_LOOKS`]: what [`super::RewardsAction::Look`] indexes.
pub(in crate::survivors) fn all_looks() -> impl Iterator<Item = &'static Look> {
    LOOKS.iter().chain(EARNED_LOOKS)
}

/// Whether `save` may dress the companion in `look`: a starting look always, an earned one once
/// unlocked.
pub(in crate::survivors) fn offered(look: &Look, save: &RewardsSave) -> bool {
    LOOKS.iter().any(|l| l.key == look.key) || save.unlocked_looks.contains(&look.display)
}

/// The earned look wearing `display`.
pub(in crate::survivors) fn earned_look(display: u32) -> Option<&'static Look> {
    EARNED_LOOKS.iter().find(|l| l.display == display)
}

/// The look `key` names; empty or unknown is the first.
pub(in crate::survivors) fn look_for(key: &str) -> &'static Look {
    all_looks().find(|l| l.key == key).unwrap_or(&LOOKS[0])
}

/// Seconds between fetches.
pub(in crate::survivors) const FETCH_EVERY: f32 = 2.0;
/// The farthest from the hero a pickup is fetched, in yards.
pub(in crate::survivors) const FETCH_REACH: f32 = 25.0;
/// Its speed out to a pickup, and its most when hurrying after the hero, in yards a second.
const DASH_SPEED: f32 = 16.0;
/// How near the hero it settles.
const HEEL: f32 = 2.5;
/// Farther than this from the hero (after a Blink, say), it appears at the hero's side.
const LEASH: f32 = 40.0;
/// How near a pickup it must come to send it off.
const GRAB: f32 = 1.2;
/// Seconds before a fetch is given up.
const GIVE_UP: f32 = 4.0;

#[derive(Component)]
pub(in crate::survivors) struct Companion {
    /// Seconds to the next fetch.
    next_fetch: f32,
    /// The pickup it is running for, and the seconds spent on it.
    fetching: Option<(Entity, f32)>,
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (spawn_companion, run_companion)
            .chain()
            .after(WorldStage::Input)
            .run_if(in_state(Phase::Playing))
            .run_if(|l: Res<crate::loading_screen::LoadingScreen>| !l.covering()),
    );
}

/// The pickup to fetch: of `pickups` (a key and a position each), the nearest to `hero` on the
/// ground plane that lies beyond `magnet`, which takes the nearer ones itself, and within
/// [`FETCH_REACH`].
pub(in crate::survivors) fn fetch_target<T>(
    hero: Vec3,
    magnet: f32,
    pickups: impl IntoIterator<Item = (T, Vec3)>,
) -> Option<T> {
    pickups
        .into_iter()
        .map(|(key, at)| (key, flat(at - hero).length()))
        .filter(|&(_, d)| d > magnet && d <= FETCH_REACH)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(key, _)| key)
}

/// Experience and coins not already flying to a hero.
fn fetchable(gem: &Gem) -> bool {
    gem.pulled.is_none() && matches!(gem.kind, GemKind::Xp(_) | GemKind::Gold(_))
}

/// Once a run is under way, the companion joins it at the local hero's side, if the perk is bought:
/// the perk is the local player's.
fn spawn_companion(
    mut commands: Commands,
    profile: Res<Profile>,
    mut run: ResMut<Run>,
    hero: Query<&Transform, With<LocalHero>>,
    companions: Query<(), With<Companion>>,
) {
    if !companions.is_empty() || !perks::companion_unlocked(&profile.rewards) {
        return;
    }
    let Ok(h) = hero.single() else { return };
    let look = look_for(&profile.rewards.companion);
    let pos = h.translation + Vec3::X * HEEL;
    let serial = run.guid();
    let e = units::spawn_unit(
        &mut commands,
        &UnitSpec {
            serial,
            display: look.display,
            scale: look.scale,
            level: 1,
            faction: units::FACTION_FRIENDLY,
            bytes0: 1 << 8,
            weapons: &[],
            sheath: 0,
        },
        pos,
        yaw_toward(h.translation - pos),
    );
    commands.entity(e).insert(Companion {
        next_fetch: FETCH_EVERY,
        fetching: None,
    });
}

/// Heel, fetch and come back: see the module doc.
#[allow(clippy::type_complexity)]
fn run_companion(
    time: Res<Time>,
    world: WorldPoint,
    hero: Query<(Entity, &Transform, &HeroState), (With<LocalHero>, Without<Companion>)>,
    mut companions: Query<
        (&mut Transform, &mut MovementState, &mut Companion),
        (Without<Hero>, Without<Gem>),
    >,
    mut gems: Query<(Entity, &Transform, &mut Gem), (Without<Hero>, Without<Companion>)>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let Ok((owner, h, me)) = hero.single() else {
        return;
    };
    let hpos = h.translation;
    let stats = me.stats();
    let down = me.dead_t.is_some();
    for (mut tf, mut mv, mut pet) in &mut companions {
        let pos = tf.translation;
        if flat(hpos - pos).length() > LEASH {
            tf.translation = enemies::ground(&world, hpos + Vec3::X * HEEL, hpos.y);
            pet.fetching = None;
            continue;
        }
        // Out after a pickup: there, send it on; gone, already flying or chased too long, drop it.
        let mut chase = None;
        if let Some((target, t)) = pet.fetching.take() {
            if let Ok((_, at, mut gem)) = gems.get_mut(target) {
                if gem.pulled.is_none() && t < GIVE_UP && !down {
                    if flat(at.translation - pos).length() <= GRAB {
                        gem.pulled = Some(owner);
                    } else {
                        pet.fetching = Some((target, t + dt));
                        chase = Some(at.translation);
                    }
                }
            }
        }
        pet.next_fetch -= dt;
        if pet.fetching.is_none() && pet.next_fetch <= 0.0 && !down {
            pet.next_fetch = FETCH_EVERY;
            let pickups = gems
                .iter()
                .filter(|(_, _, gem)| fetchable(gem))
                .map(|(e, at, _)| (e, at.translation));
            pet.fetching = fetch_target(hpos, stats.magnet, pickups).map(|e| (e, 0.0));
        }
        // To the pickup, else to a spot at the hero's heel on its own side.
        let (dest, speed) = match chase {
            Some(at) => (at, DASH_SPEED),
            None => {
                let spot = hpos + flat(pos - hpos).normalize_or(Vec3::X) * HEEL;
                let gap = flat(spot - pos).length();
                (spot, (stats.speed * 1.2).max(gap * 2.0).min(DASH_SPEED))
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

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;

    #[test]
    fn fetches_the_nearest_pickup_past_the_magnet_and_within_reach() {
        let hero = Vec3::new(10.0, 5.0, -3.0);
        let at = |x: f32, y: f32, z: f32| hero + Vec3::new(x, y, z);
        let pickups = [
            ("inside the magnet", at(3.0, 0.0, 0.0)),
            ("far", at(0.0, 0.0, 20.0)),
            // A hillside: the height between does not count.
            ("near, uphill", at(6.0, 9.0, 6.0)),
            ("out of reach", at(26.0, 0.0, 0.0)),
        ];
        assert_eq!(fetch_target(hero, 4.5, pickups), Some("near, uphill"));
        // With a bigger magnet the near one is the magnet's.
        assert_eq!(fetch_target(hero, 9.0, pickups), Some("far"));
        assert_eq!(
            fetch_target(hero, 4.5, [("edge", at(FETCH_REACH, 0.0, 0.0))]),
            Some("edge")
        );
        assert_eq!(
            fetch_target(hero, 4.5, [pickups[0], pickups[3]]),
            None,
            "nothing between the magnet and the reach"
        );
        assert_eq!(fetch_target::<u8>(hero, 4.5, []), None);
    }

    #[test]
    fn looks_have_their_own_keys_and_a_default() {
        let keys: HashSet<&str> = LOOKS.iter().map(|l| l.key).collect();
        assert_eq!(keys.len(), LOOKS.len());
        assert_eq!(look_for("").key, LOOKS[0].key);
        assert_eq!(look_for("dragon").key, LOOKS[0].key);
        assert_eq!(look_for("frog").display, 901);
    }

    #[test]
    fn looks_are_the_named_critters_in_the_client_data() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open chain");
        let catalog = benilla_formats::load_creature_catalog(&mut chain).expect("creatures");
        let models: HashMap<u32, String> = catalog
            .sized_models()
            .map(|(id, path, _)| (id, path.to_ascii_lowercase().replace('/', "\\")))
            .collect();
        let displays: HashMap<u32, u32> = catalog.display_models().collect();
        for (key, model) in [
            ("squirrel", "creature\\squirrel\\squirrel."),
            ("rabbit", "creature\\rabbit\\rabbit."),
            ("frog", "creature\\frog\\frog."),
            ("cat", "creature\\cat\\cat."),
            ("chicken", "creature\\chicken\\chicken."),
        ] {
            let look = look_for(key);
            assert_eq!(look.key, key);
            let path = displays
                .get(&look.display)
                .and_then(|m| models.get(m))
                .unwrap_or_else(|| panic!("display {} has no model", look.display));
            assert!(
                path.starts_with(model),
                "{key}: display {} is {path}",
                look.display
            );
        }
    }
}
