//! The run's spoils: the gold the dead drop, the elites that walk out of the horde from the second
//! minute, the guardians holding posts around the battleground, and the treasure caches, each worth
//! a burst of gold and a level-up hand that needs no experience. Elites and guardians leave theirs
//! free; the chests set out at the start are locked, and cost more gold for every cache opened.

use benilla_protocol::field::FIELD_GAMEOBJECT_STATE;
use benilla_protocol::messages::{ObjectFields, ObjectType};
use benilla_world::model_fade::DespawnFade;
use benilla_world::world_point::WorldPoint;
use bevy::prelude::*;

use super::data;
use super::enemies::{self, Rank, Spawner};
use super::hero::{self, Hero, HERO_RADIUS};
use super::party;
use super::run_options::RunOptions;
use super::units::{self, flat, Fx};
use super::{HeroState, LocalHero, Phase, Rng, Run, Selection};
use crate::net::{FieldChanged, Guid, ObjectStore};

/// The first elite walks out at two minutes, then one every minute to a minute and a half.
pub(super) const ELITE_FIRST_SECS: f32 = 120.0;
const ELITE_GAP_MIN: f32 = 60.0;
const ELITE_GAP_MAX: f32 = 90.0;
/// An elite against its kind: four times the health, half again the damage, a third bigger.
pub(super) const ELITE_HP: f32 = 4.0;
pub(super) const ELITE_DAMAGE: f32 = 1.5;
pub(super) const ELITE_SCALE: f32 = 1.35;
/// Hunter's Mark: its state kit floats the red mark over an elite's head for as long as it lives.
pub(super) const ELITE_MARK: u32 = 1130;

/// A guardian wakes when the hero comes this near (yards); less in a small arena.
const GUARDIAN_WAKE: f32 = 25.0;
/// Guardians stand three quarters of the way to the leash, never further out than this.
const GUARDIAN_RING_MAX: f32 = 70.0;
/// A leash this wide holds three guardians; a smaller arena two.
const WIDE_FIELD: f32 = 60.0;
/// Locked chests stand between the guardians, nearer in: under half the leash, at most this far.
const CHEST_RING_MAX: f32 = 45.0;
/// A locked chest's price, and how much dearer each cache opened makes the next.
const CHEST_BASE_COST: u32 = 25;
const CHEST_COST_STEP: u32 = 15;

// Gold, tuned so a 10 minute run earns about 300 and a 15 minute win about 600: a few thousand
// kills at one in fifty, a pile per elite and boss, and a burst per cache, at 1-3 gold a coin.
/// The chance a horde kill drops a coin.
const COIN_CHANCE: f32 = 0.02;
/// A boss's purse and an elite's, in coins.
const BOSS_PILE_COINS: u32 = 10;
const ELITE_PILE_COINS: u32 = 6;
/// A cache's gold burst, in coins.
const CACHE_COINS: u32 = 6;

/// `GoldPilesmall01.mdx`, a coin or a pile by its scale.
const COIN_DISPLAY: u32 = 2614;
/// `TreasureChest01.mdx`, whose lid opens on `GO_STATE_ACTIVE`.
const CACHE_DISPLAY: u32 = 259;
const CACHE_SCALE: f32 = 1.5;
/// How near the hero's edge a cache opens.
const CACHE_REACH: f32 = 1.2;
/// Seconds an opened cache stands with its lid up before it fades.
const CACHE_LINGER: f32 = 2.5;
const GO_STATE_ACTIVE: u32 = 0;
const GO_STATE_READY: u32 = 1;

/// The combat-text colour of a gold burst, 0xAARRGGBB.
const COLOR_GOLD: u32 = 0xFFFF_D200;

/// A foe's health, damage and drawn scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Body {
    pub hp: f32,
    pub damage: f32,
    pub scale: f32,
}

/// The same foe as an elite or a guardian.
pub(super) fn elite_body(b: Body) -> Body {
    Body {
        hp: b.hp * ELITE_HP,
        damage: b.damage * ELITE_DAMAGE,
        scale: b.scale * ELITE_SCALE,
    }
}

/// What one coin is worth `elapsed` seconds in: 1 early, 2 mid-run, 3 at the end, before the
/// hero's gold multiplier ([`worth`] applies it).
pub(super) fn coin_value(elapsed: f32) -> u32 {
    (1.0 + 2.0 * data::progress(elapsed).min(1.0)).round() as u32
}

/// `coins` coins' worth `elapsed` seconds in under the gold multiplier `gold` ([`Stats::gold`],
/// 1.0 with no Gold bonus): the one place a coin's worth is reckoned. The drop is rounded once,
/// its fraction up when `dither` (uniform in `[0, 1)`) falls under it, so even a lone coin pays
/// the bonus in full on average; never less than one.
///
/// [`Stats::gold`]: super::Stats::gold
pub(super) fn worth(coins: u32, elapsed: f32, gold: f32, dither: f32) -> u32 {
    let exact = (coins * coin_value(elapsed)) as f32 * gold.max(0.0);
    ((exact + dither).floor() as u32).max(1)
}

/// The gold a kill drops, if any: a pile from a boss or an elite, now and then a coin from the
/// horde, under the gold multiplier `gold`. `roll` is uniform in `[0, 1)`.
pub(super) fn purse(boss: bool, elite: bool, elapsed: f32, gold: f32, roll: f32) -> Option<u32> {
    let (coins, dither) = if boss {
        (BOSS_PILE_COINS, roll)
    } else if elite {
        (ELITE_PILE_COINS, roll)
    } else if roll < COIN_CHANCE {
        // The roll that let the coin drop is uniform under the chance: stretched, it dithers.
        (1, roll / COIN_CHANCE)
    } else {
        return None;
    };
    Some(worth(coins, elapsed, gold, dither))
}

/// A cache's burst of gold under the gold multiplier `gold`, rounded to the nearest coin.
pub(super) fn cache_gold(elapsed: f32, gold: f32) -> u32 {
    worth(CACHE_COINS, elapsed, gold, 0.5)
}

/// The `GameObjectDisplayInfo` row and scale of `gold` on the ground: the heap grows with it.
pub(super) fn coin_look(gold: u32) -> (u32, f32) {
    (COIN_DISPLAY, (0.6 + 0.04 * gold as f32).min(1.4))
}

/// What a cache holds: a burst of gold (under the gold multiplier of the hero who opened it, into
/// the party's purse) and a level-up hand that needs no experience. The one place a cache's reward
/// is decided. Returns the gold given.
pub(super) fn open_cache(run: &mut Run, opener: &HeroState) -> u32 {
    let gold = cache_gold(run.elapsed, opener.stats().gold);
    run.gold += gold;
    run.caches += 1;
    run.pending_levels += 1;
    gold
}

/// How near the hero comes before a guardian wakes: a quarter of the way across a big field, less
/// in a small arena so the start stays clear of it.
pub(super) fn wake_radius(leash: f32) -> f32 {
    GUARDIAN_WAKE.min(leash * 0.4)
}

/// Where the guardians stand, as offsets in yards from the battleground's centre: three on an open
/// field, two in a small arena, spread on a ring inside the leash and out of wake reach of the
/// start.
pub(super) fn guardian_posts(leash: f32) -> Vec<Vec3> {
    ring_posts(leash, 0.0, (leash * 0.75).min(GUARDIAN_RING_MAX))
}

/// Where the locked chests stand: as many as the guardians, midway between their posts and nearer
/// the centre, so taking one wakes no guardian.
pub(super) fn chest_posts(leash: f32) -> Vec<Vec3> {
    ring_posts(leash, 0.5, (leash * 0.45).min(CHEST_RING_MAX))
}

/// Evenly round a ring of radius `r`, turned `half` of a gap on from the guardians' first post.
fn ring_posts(leash: f32, half: f32, r: f32) -> Vec<Vec3> {
    let (n, turn) = if leash >= WIDE_FIELD {
        (3, 0.5)
    } else {
        (2, 0.8)
    };
    (0..n)
        .map(|i| {
            let a = turn + (i as f32 + half) / n as f32 * std::f32::consts::TAU;
            Vec3::new(a.cos(), 0.0, a.sin()) * r
        })
        .collect()
}

/// What a locked chest costs with `opened` caches opened so far this run.
pub(super) fn chest_cost(opened: u32) -> u32 {
    CHEST_BASE_COST + CHEST_COST_STEP * opened
}

/// Which of a `len`-long roster guardian `i` of `n` is: from a third of the way up to three
/// quarters, so each post is a harder fight than the last.
fn guardian_tier(i: usize, n: usize, len: usize) -> usize {
    let t = 0.3 + 0.45 * i as f32 / n.saturating_sub(1).max(1) as f32;
    ((t * len as f32) as usize).min(len.saturating_sub(1))
}

/// A treasure cache on the ground; `opened` counts the seconds since its lid lifted.
#[derive(Component)]
pub(super) struct Cache {
    opened: Option<f32>,
    /// Opens only for gold ([`chest_cost`]).
    locked: bool,
    /// The hero has been told the price and has not stepped away since.
    refused: bool,
}

impl Cache {
    pub(super) fn is_open(&self) -> bool {
        self.opened.is_some()
    }
}

/// A cache, free or `locked` for gold.
pub(super) fn spawn_cache(commands: &mut Commands, serial: u64, pos: Vec3, locked: bool) {
    let e = units::spawn_object(commands, serial, CACHE_DISPLAY, CACHE_SCALE, pos);
    commands.entity(e).insert(Cache {
        opened: None,
        locked,
        refused: false,
    });
}

/// The HUD's gold counter.
#[derive(Component)]
pub(super) struct GoldText;

/// At the run's start the guardians take their posts around the battleground's centre, and the
/// locked chests are set out between them.
pub(super) fn place_guardians(
    mut commands: Commands,
    mut run: ResMut<Run>,
    selection: Res<Selection>,
    opts: Res<RunOptions>,
    mut spawner: ResMut<Spawner>,
    hero: Query<&Transform, With<LocalHero>>,
    world: WorldPoint,
) {
    if run.guarded {
        return;
    }
    let Ok(h) = hero.single() else { return };
    run.guarded = true;
    let map = selection.stage_def();
    let center = hero::arena(map);
    let roster = data::roster(selection.map);
    let posts = guardian_posts(map.leash);
    for (i, off) in posts.iter().enumerate() {
        let def = &roster.enemies[guardian_tier(i, posts.len(), roster.enemies.len())];
        let pos = enemies::ground(&world, center + *off, h.translation.y);
        enemies::spawn_enemy(
            &mut commands,
            &mut run,
            def,
            pos,
            center,
            Rank::Guardian,
            &opts,
            &mut spawner.xp_carry,
        );
    }
    for off in chest_posts(map.leash) {
        let pos = enemies::ground(&world, center + off, h.translation.y);
        spawn_cache(&mut commands, run.guid(), pos, true);
    }
}

/// From the second minute, every minute or so one of the moment's foes walks out as an elite, near
/// the heroes still standing.
pub(super) fn spawn_elites(
    mut commands: Commands,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    selection: Res<Selection>,
    opts: Res<RunOptions>,
    mut spawner: ResMut<Spawner>,
    heroes: Query<(&Transform, &HeroState), With<Hero>>,
    world: WorldPoint,
) {
    let standing = heroes.iter().filter(|(_, h)| h.alive());
    let Some(hpos) = party::centre(standing.map(|(tf, _)| tf.translation)) else {
        return;
    };
    if run.elapsed < run.next_elite {
        return;
    }
    run.next_elite = run.elapsed + rng.range(ELITE_GAP_MIN, ELITE_GAP_MAX);
    let roster = data::roster(selection.map);
    let pool = data::wave_pool(run.elapsed, roster.enemies.len());
    let def = &roster.enemies[pool[rng.index(pool.len())]];
    let a = rng.range(0.0, std::f32::consts::TAU);
    let pos = enemies::ground(
        &world,
        hpos + Vec3::new(a.cos(), 0.0, a.sin()) * 30.0,
        hpos.y,
    );
    enemies::spawn_enemy(
        &mut commands,
        &mut run,
        def,
        pos,
        hpos,
        Rank::Elite,
        &opts,
        &mut spawner.xp_carry,
    );
    run.banner = Some((format!("Elite {} approaches!", def.name), 3.0));
}

/// Walking into a cache opens it, a locked one only for its price (from the party's purse): its lid
/// lifts, [`open_cache`] gives the reward to the nearest hero standing in reach, and it fades a
/// moment later.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn open_caches(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut next: ResMut<NextState<Phase>>,
    heroes: Query<(Entity, &Transform, &HeroState), With<Hero>>,
    mut caches: Query<(Entity, &Guid, &Transform, &mut Cache, &mut ObjectStore), Without<Hero>>,
    mut fields: MessageWriter<FieldChanged>,
    mut fx: Fx,
) {
    if heroes.is_empty() {
        return;
    }
    let dt = time.delta_secs();
    for (e, guid, tf, mut cache, mut store) in &mut caches {
        if let Some(t) = cache.opened.as_mut() {
            *t += dt;
            if *t > CACHE_LINGER {
                commands
                    .entity(e)
                    .remove::<Cache>()
                    .insert(DespawnFade::default());
            }
            continue;
        }
        let at = tf.translation;
        let reach = heroes
            .iter()
            .filter(|(_, h, state)| {
                state.alive() && flat(at - h.translation).length() <= CACHE_REACH + HERO_RADIUS
            })
            .map(|(e, h, _)| (e, h.translation));
        let Some((hero_e, _)) = party::nearest(reach, at) else {
            cache.refused = false;
            continue;
        };
        let Ok((.., opener)) = heroes.get(hero_e) else {
            continue;
        };
        let cost = cache.locked.then(|| chest_cost(run.caches));
        if let Some(cost) = cost {
            if run.gold < cost {
                // Told once per visit, not every frame the hero stands there.
                if !cache.refused {
                    cache.refused = true;
                    run.banner = Some((format!("This chest needs {cost} gold"), 2.5));
                }
                continue;
            }
            run.gold -= cost;
        }
        cache.opened = Some(0.0);
        // The lid lifts as a looted chest's does ([`crate::go_anim`]).
        store.0.merge(ObjectFields::from_pairs(&[(
            FIELD_GAMEOBJECT_STATE,
            GO_STATE_ACTIVE,
        )]));
        fields.write(FieldChanged {
            entity: e,
            guid: guid.0,
            kind: ObjectType::GameObject,
            index: FIELD_GAMEOBJECT_STATE,
            old: GO_STATE_READY,
            new: GO_STATE_ACTIVE,
        });
        let gold = open_cache(&mut run, opener);
        fx.number(hero_e, gold as f32, false, COLOR_GOLD);
        let line = match cost {
            Some(cost) => format!("Chest opened for {cost} gold!"),
            None => format!("Treasure! +{gold} gold"),
        };
        run.banner = Some((line, 3.0));
    }
    if run.pending_levels > 0 {
        next.set(Phase::LevelUp);
    }
}

pub(super) fn refresh_gold(run: Res<Run>, mut texts: Query<&mut Text, With<GoldText>>) {
    let want = format!("Gold {}", run.gold);
    for mut t in &mut texts {
        if **t != want {
            **t = want.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coins_grow_through_the_run() {
        assert_eq!(coin_value(0.0), 1);
        assert_eq!(coin_value(data::VICTORY_SECS / 2.0), 2);
        assert_eq!(coin_value(data::VICTORY_SECS), 3);
        assert_eq!(coin_value(data::VICTORY_SECS * 3.0), 3);
        let mut last = 0;
        for s in 0..=900 {
            let v = coin_value(s as f32);
            assert!(v >= last, "a coin never loses worth");
            last = v;
        }
    }

    #[test]
    fn purses_by_rank() {
        let t = data::VICTORY_SECS;
        assert_eq!(purse(true, false, t, 1.0, 0.99), Some(BOSS_PILE_COINS * 3));
        assert_eq!(purse(false, true, 0.0, 1.0, 0.99), Some(ELITE_PILE_COINS));
        assert_eq!(purse(false, false, 0.0, 1.0, 0.0), Some(1));
        assert_eq!(purse(false, false, t, 1.0, COIN_CHANCE / 2.0), Some(3));
        assert_eq!(purse(false, false, 0.0, 1.0, COIN_CHANCE), None);
        assert_eq!(cache_gold(0.0, 1.0), CACHE_COINS);
        assert!(purse(true, false, 0.0, 1.0, 0.5) > purse(false, true, 0.0, 1.0, 0.5));
    }

    #[test]
    fn a_gold_bonus_multiplies_every_coin() {
        let t = data::VICTORY_SECS;
        let pile = |coins: u32| (coins * 3) as f32 * 1.5;
        assert_eq!(
            purse(true, false, t, 1.5, 0.0),
            Some(pile(BOSS_PILE_COINS) as u32)
        );
        assert_eq!(cache_gold(t, 1.5), pile(CACHE_COINS).round() as u32);
        // A lone early coin is worth 1.4 on average: the fraction pays out that often.
        let n = 1000;
        let total: u32 = (0..n)
            .map(|i| purse(false, false, 0.0, 1.4, COIN_CHANCE * i as f32 / n as f32).unwrap())
            .sum();
        assert!((total as i64 - 1400).abs() <= 1, "{total}");
        // A negative multiplier still drops a coin.
        assert_eq!(purse(false, false, 0.0, -1.0, 0.0), Some(1));
    }

    #[test]
    fn a_cache_gives_gold_and_a_hand() {
        let mut run = Run::default();
        let gold = open_cache(&mut run, &HeroState::default());
        assert_eq!(gold, cache_gold(0.0, 1.0));
        assert_eq!(run.gold, gold);
        assert_eq!(run.caches, 1);
        assert_eq!(run.pending_levels, 1);
        assert_eq!(run.level, 1, "a cache's hand needs no experience");
    }

    #[test]
    fn elites_scale_their_kind() {
        let b = Body {
            hp: 100.0,
            damage: 10.0,
            scale: 2.0,
        };
        let e = elite_body(b);
        assert_eq!(e.hp, 400.0);
        assert_eq!(e.damage, 15.0);
        assert!((e.scale - 2.7).abs() < 1e-5);
    }

    #[test]
    fn guardian_posts_stay_in_every_leash() {
        for map in data::MAPS {
            let posts = guardian_posts(map.leash);
            assert!((2..=3).contains(&posts.len()), "{}", map.name);
            for p in &posts {
                let r = flat(*p).length();
                assert!(
                    r < map.leash,
                    "{}: post {r} yd out of {}",
                    map.name,
                    map.leash
                );
                assert!(
                    r > wake_radius(map.leash),
                    "{}: a guardian wakes at the start",
                    map.name
                );
            }
            for (i, a) in posts.iter().enumerate() {
                for b in &posts[i + 1..] {
                    assert!((*a - *b).length() > wake_radius(map.leash), "{}", map.name);
                }
            }
        }
    }

    #[test]
    fn locked_chests_stand_clear_of_the_guardians() {
        for map in data::MAPS {
            let wake = wake_radius(map.leash);
            let guards = guardian_posts(map.leash);
            for c in chest_posts(map.leash) {
                assert!(flat(c).length() < map.leash, "{}", map.name);
                for g in &guards {
                    assert!(
                        (c - *g).length() > wake,
                        "{}: a chest wakes a guardian",
                        map.name
                    );
                }
            }
        }
    }

    #[test]
    fn chests_cost_more_with_every_cache() {
        assert_eq!(chest_cost(0), CHEST_BASE_COST);
        assert!((0..20).all(|n| chest_cost(n + 1) > chest_cost(n)));
    }

    #[test]
    fn guardian_tiers_climb_the_roster() {
        for n in 2..=3 {
            let tiers: Vec<usize> = (0..n).map(|i| guardian_tier(i, n, 30)).collect();
            assert!(tiers.windows(2).all(|w| w[0] < w[1]), "{tiers:?}");
            assert!(tiers.iter().all(|&t| t < 30));
        }
        assert_eq!(guardian_tier(0, 1, 1), 0);
    }
}
