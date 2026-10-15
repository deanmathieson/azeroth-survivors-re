//! The run's spoils: the gold the dead drop, the elites that walk out of the horde from the second
//! minute, the guardians holding posts around the battleground, and the treasure caches, each worth
//! a burst of gold and a jewelry choice ([`super::jewelry`]). Elites and guardians leave theirs
//! free; the chests set out at the start are locked, and cost more gold for every cache opened.
//!
//! A roster boss leaves a chest of its own where it dies ([`ChestTier::Boss`], the last boss's
//! grander still), glowing gold and marked on the minimap: its gear rolls as it opens, the reveal
//! ([`super::gear`]), with a bigger burst of gold and a hand. A run that ends with one still shut
//! opens it for the results ([`open_leftover_chests`]).

use benilla_protocol::field::FIELD_GAMEOBJECT_STATE;
use benilla_protocol::messages::{ObjectFields, ObjectType};
use benilla_world::collision::WorldCollision;
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
use crate::sound::SoundCue;

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
/// A boss chest's burst, and the last boss's.
const BOSS_CHEST_COINS: u32 = 15;
const FINAL_CHEST_COINS: u32 = 25;

/// `GoldPilesmall01.mdx`, a coin or a pile by its scale.
const COIN_DISPLAY: u32 = 2614;
/// How near the hero's edge a cache opens; a boss's bigger chest from further out.
const CACHE_REACH: f32 = 1.2;
const BOSS_CHEST_REACH: f32 = 2.4;
/// Consecration's golden ground under a boss chest until it opens, this wide (yards).
const CHEST_GLOW: u32 = 26573;
const CHEST_GLOW_RADIUS: f32 = 2.5;
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
    Coins::new(coins, elapsed, dither).pays(gold)
}

/// Coins on the ground, their worth not yet reckoned: they pay under the gold multiplier of the
/// hero who picks them up ([`Coins::pays`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Coins {
    /// What they are worth before any multiplier: the coins at what a coin was worth as they fell.
    pub base: u32,
    /// The rounding's dither, uniform in `[0, 1)` ([`worth`]).
    pub dither: f32,
}

impl Coins {
    /// `coins` coins falling `elapsed` seconds in.
    pub(super) fn new(coins: u32, elapsed: f32, dither: f32) -> Self {
        Self {
            base: coins * coin_value(elapsed),
            dither,
        }
    }

    /// What they pay a picker whose gold multiplier is `gold`, rounded once; never less than one.
    pub(super) fn pays(self, gold: f32) -> u32 {
        let exact = self.base as f32 * gold.max(0.0);
        ((exact + self.dither).floor() as u32).max(1)
    }
}

/// The gold a kill drops, if any: a pile from a boss or an elite, now and then a coin from the
/// horde, its worth reckoned by whoever picks it up. `roll` is uniform in `[0, 1)`.
pub(super) fn purse(boss: bool, elite: bool, elapsed: f32, roll: f32) -> Option<Coins> {
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
    Some(Coins::new(coins, elapsed, dither))
}

/// A cache's burst of gold under the gold multiplier `gold`, rounded to the nearest coin.
pub(super) fn cache_gold(elapsed: f32, gold: f32) -> u32 {
    worth(CACHE_COINS, elapsed, gold, 0.5)
}

/// A boss chest's burst of gold, bigger than a cache's, the last boss's biggest.
pub(super) fn boss_chest_gold(tier: ChestTier, elapsed: f32, gold: f32) -> u32 {
    let coins = if tier == ChestTier::Final {
        FINAL_CHEST_COINS
    } else {
        BOSS_CHEST_COINS
    };
    worth(coins, elapsed, gold, 0.5)
}

/// The `GameObjectDisplayInfo` row and scale of `gold` on the ground: the heap grows with it.
pub(super) fn coin_look(gold: u32) -> (u32, f32) {
    (COIN_DISPLAY, (0.6 + 0.04 * gold as f32).min(1.4))
}

/// What a cache holds: a burst of gold (under the gold multiplier of the hero who opened it, into
/// the party's purse) and a jewelry choice for every hero, banked until play allows
/// ([`super::jewelry`]; a level-up hand when no jewelry is left to offer). The one place a cache's
/// reward is decided. Returns the gold given.
pub(super) fn open_cache(run: &mut Run, opener: &HeroState) -> u32 {
    let gold = cache_gold(run.elapsed, opener.stats().gold);
    run.gold += gold;
    run.caches += 1;
    run.pending_jewelry += 1;
    gold
}

/// What a boss chest holds besides its boss's drop: a bigger burst of gold than a cache's (under
/// the opener's gold multiplier) and one level-up hand. Returns the gold given.
pub(super) fn open_boss_chest(run: &mut Run, opener: &HeroState, tier: ChestTier) -> u32 {
    let gold = boss_chest_gold(tier, run.elapsed, opener.stats().gold);
    run.gold += gold;
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

/// A chest's look and worth: the cache an elite or guardian leaves, a locked one set out at the
/// start (it opens only for gold, [`chest_cost`]), a roster boss's, and the last boss's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ChestTier {
    Standard,
    Locked,
    Boss,
    Final,
}

impl ChestTier {
    /// Its `GameObjectDisplayInfo` row and drawn scale.
    pub(super) const fn look(self) -> (u32, f32) {
        match self {
            // `TreasureChest01.mdx`, the small wooden chest, whose lid opens on
            // `GO_STATE_ACTIVE`.
            Self::Standard => (259, 1.5),
            // `Chest03.mdx`, iron-bound; small as authored.
            Self::Locked => (41, 2.2),
            // `G_ChestTitan.mdx`, Uldaman's ornate titan chest, big as authored.
            Self::Boss => (1387, 1.0),
            // `G_ChestAhnQiraj.mdx`, the Ahn'Qiraj raid cache.
            Self::Final => (6502, 2.6),
        }
    }

    pub(super) const fn is_boss(self) -> bool {
        matches!(self, Self::Boss | Self::Final)
    }

    /// The sound a chest of this tier opens with when its model has none: `Chest03` authors no
    /// open event, where the others ring their own display slot through the GameObject sounds
    /// (`ChestOpen`, `G_ChestTitanOpen`, `G_ChestAhnQirajOpen`).
    const fn open_cue(self) -> Option<&'static str> {
        match self {
            Self::Locked => Some("ChestOpen"),
            _ => None,
        }
    }

    fn reach(self) -> f32 {
        if self.is_boss() {
            BOSS_CHEST_REACH
        } else {
            CACHE_REACH
        }
    }
}

/// A roster boss's chest tier: the last boss's is the grandest.
pub(super) fn boss_tier(boss: usize, bosses: usize) -> ChestTier {
    if boss + 1 >= bosses {
        ChestTier::Final
    } else {
        ChestTier::Boss
    }
}

/// A treasure cache on the ground; `opened` counts the seconds since its lid lifted.
#[derive(Component)]
pub(super) struct Cache {
    opened: Option<f32>,
    tier: ChestTier,
    /// A boss chest's roster boss, whose drop rolls as it opens.
    boss: Option<usize>,
    /// The golden ground under a boss chest, gone as it opens.
    glow: Option<Entity>,
    /// The hero has been told the price and has not stepped away since.
    refused: bool,
}

impl Cache {
    pub(super) fn is_open(&self) -> bool {
        self.opened.is_some()
    }

    pub(super) fn tier(&self) -> ChestTier {
        self.tier
    }

    /// Lift the lid, once: whether it was shut.
    fn open(&mut self) -> bool {
        let was_shut = self.opened.is_none();
        if was_shut {
            self.opened = Some(0.0);
        }
        was_shut
    }
}

/// A cache, free or `locked` for gold.
pub(super) fn spawn_cache(commands: &mut Commands, serial: u64, pos: Vec3, locked: bool) {
    let tier = if locked {
        ChestTier::Locked
    } else {
        ChestTier::Standard
    };
    spawn_chest(commands, serial, pos, tier, None, None);
}

fn spawn_chest(
    commands: &mut Commands,
    serial: u64,
    pos: Vec3,
    tier: ChestTier,
    boss: Option<usize>,
    glow: Option<Entity>,
) -> Entity {
    let (display, scale) = tier.look();
    let e = units::spawn_object(commands, serial, display, scale, pos);
    commands.entity(e).insert(Cache {
        opened: None,
        tier,
        boss,
        glow,
        refused: false,
    });
    e
}

/// Roster boss `boss`'s chest at `pos`, on its golden ground.
fn spawn_boss_chest(
    commands: &mut Commands,
    run: &mut Run,
    pos: Vec3,
    tier: ChestTier,
    boss: usize,
) -> Entity {
    let glow = units::spawn_area_object(commands, run.guid(), CHEST_GLOW, CHEST_GLOW_RADIUS, pos);
    spawn_chest(commands, run.guid(), pos, tier, Some(boss), Some(glow))
}

/// A boss chest opened: its roster boss's drop, which [`super::gear`] rolls for the heroes it
/// names, and the chest its finds are shown at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BossChestOpened {
    pub map: usize,
    pub boss: usize,
    pub chest: Entity,
    /// Opened in the run: only heroes still standing roll. A run's end rolls for everyone.
    pub living_only: bool,
}

/// Boss chests opened and not yet rolled. A queue, not a message, so the results' roll (ahead of
/// the run's record) and the run's own each take an opening once.
#[derive(Resource, Default)]
pub(super) struct BossRolls(pub Vec<BossChestOpened>);

/// A roster boss falls: its chest stands where it died, glowing, its drop inside until it opens.
pub(super) fn drop_boss_chests(
    mut commands: Commands,
    mut kills: MessageReader<enemies::Loot>,
    mut run: ResMut<Run>,
    selection: Res<Selection>,
) {
    for kill in kills.read() {
        if !kill.boss {
            continue;
        }
        let roster = data::roster(selection.map);
        let Some(boss) = roster.bosses.iter().position(|b| b.name == kill.def.name) else {
            continue;
        };
        let tier = boss_tier(boss, roster.bosses.len());
        spawn_boss_chest(
            &mut commands,
            &mut run,
            kill.pos + Vec3::Z * 1.5,
            tier,
            boss,
        );
    }
}

/// The run is over with boss chests still shut: they open for the results, so no drop is lost.
/// Their gold counts in the run's record (this runs ahead of it); there is no hand left to deal.
pub(super) fn open_leftover_chests(
    mut run: ResMut<Run>,
    selection: Res<Selection>,
    local: Query<&HeroState, With<LocalHero>>,
    mut caches: Query<(Entity, &mut Cache)>,
    mut rolls: ResMut<BossRolls>,
) {
    let gold = local.single().map_or(1.0, |h| h.stats().gold);
    for (e, mut cache) in &mut caches {
        let Some(boss) = cache.boss else { continue };
        if !cache.open() {
            continue;
        }
        run.gold += boss_chest_gold(cache.tier, run.elapsed, gold);
        rolls.0.push(BossChestOpened {
            map: selection.map,
            boss,
            chest: e,
            living_only: false,
        });
    }
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
    collide: Option<WorldCollision>,
    world: WorldPoint,
) {
    if run.guarded {
        return;
    }
    if hero.single().is_err() {
        return;
    }
    run.guarded = true;
    let map = selection.stage_def();
    let center = hero::arena(map);
    let roster = data::roster(selection.map);
    let posts = guardian_posts(map.leash);
    for (i, off) in posts.iter().enumerate() {
        let def = &roster.enemies[guardian_tier(i, posts.len(), roster.enemies.len())];
        let pos = enemies::ground_on_collision(collide.as_ref(), &world, center + *off, map.floor);
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
        let pos = enemies::ground_on_collision(collide.as_ref(), &world, center + off, map.floor);
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
    collide: Option<WorldCollision>,
    world: WorldPoint,
) {
    if run.elapsed < run.next_elite || !heroes.iter().any(|(_, h)| h.alive()) {
        return;
    }
    let (standing, everyone) = enemies::hero_spots(&heroes);
    let Some(hpos) = party::spawn_anchor(&standing, &mut spawner.turn) else {
        return;
    };
    run.next_elite = run.elapsed + rng.range(ELITE_GAP_MIN, ELITE_GAP_MAX);
    let roster = data::roster(selection.map);
    let pool = data::wave_pool(run.elapsed, roster.enemies.len());
    let def = &roster.enemies[pool[rng.index(pool.len())]];
    let a = rng.range(0.0, std::f32::consts::TAU);
    let leash = party::Leash::of(selection.stage_def());
    let at = party::spawn_point(hpos, a, 30.0, &everyone, leash);
    let pos =
        enemies::ground_on_collision(collide.as_ref(), &world, at, selection.stage_def().floor);
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
/// moment later. A boss chest opens for whoever reaches it: [`open_boss_chest`]'s reward, and its
/// boss's drop queued for the roll ([`BossRolls`]).
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
    selection: Res<Selection>,
    mut rolls: ResMut<BossRolls>,
    mut cues: MessageWriter<SoundCue>,
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
        let within = cache.tier.reach() + HERO_RADIUS;
        let reach = heroes
            .iter()
            .filter(|(_, h, state)| state.alive() && flat(at - h.translation).length() <= within)
            .map(|(e, h, _)| (e, h.translation));
        let Some((hero_e, _)) = party::nearest(reach, at) else {
            cache.refused = false;
            continue;
        };
        let Ok((.., opener)) = heroes.get(hero_e) else {
            continue;
        };
        let cost = (cache.tier == ChestTier::Locked).then(|| chest_cost(run.caches));
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
        cache.open();
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
        if let Some(kit) = cache.tier.open_cue() {
            cues.write(SoundCue {
                kit,
                at: Some(at),
                volume: 1.0,
            });
        }
        if let Some(boss) = cache.boss {
            if let Some(glow) = cache.glow.take() {
                commands.entity(glow).try_despawn();
            }
            let gold = open_boss_chest(&mut run, opener, cache.tier);
            fx.number(hero_e, gold as f32, false, COLOR_GOLD);
            rolls.0.push(BossChestOpened {
                map: selection.map,
                boss,
                chest: e,
                living_only: true,
            });
            let name = data::roster(selection.map)
                .bosses
                .get(boss)
                .map_or("The boss", |b| b.name);
            run.banner = Some((format!("{name}'s hoard! +{gold} gold"), 3.0));
            continue;
        }
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

    /// What a kill's purse pays a picker whose gold multiplier is `gold`.
    fn purse(boss: bool, elite: bool, elapsed: f32, gold: f32, roll: f32) -> Option<u32> {
        super::purse(boss, elite, elapsed, roll).map(|c| c.pays(gold))
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
    fn a_cache_gives_gold_and_a_jewelry_choice() {
        let mut run = Run::default();
        let gold = open_cache(&mut run, &HeroState::default());
        assert_eq!(gold, cache_gold(0.0, 1.0));
        assert_eq!(run.gold, gold);
        assert_eq!(run.caches, 1);
        assert_eq!(run.pending_jewelry, 1);
        assert_eq!(run.pending_levels, 0, "jewelry instead of the free hand");
        assert_eq!(run.level, 1);
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

    #[test]
    fn every_chest_tier_wears_its_own_chest() {
        let tiers = [
            ChestTier::Standard,
            ChestTier::Locked,
            ChestTier::Boss,
            ChestTier::Final,
        ];
        for (i, a) in tiers.iter().enumerate() {
            for b in &tiers[i + 1..] {
                assert_ne!(a.look().0, b.look().0, "{a:?} and {b:?}");
            }
        }
        assert_eq!(
            ChestTier::Standard.look(),
            (259, 1.5),
            "the cache keeps its chest"
        );
        assert_eq!(ChestTier::Locked.look().0, 41);
        assert_eq!(ChestTier::Boss.look().0, 1387);
        assert_eq!(ChestTier::Final.look().0, 6502);
        assert!(ChestTier::Boss.is_boss() && ChestTier::Final.is_boss());
        assert!(!ChestTier::Standard.is_boss() && !ChestTier::Locked.is_boss());
        // The last roster boss's chest is the grandest.
        assert_eq!(boss_tier(0, 3), ChestTier::Boss);
        assert_eq!(boss_tier(1, 3), ChestTier::Boss);
        assert_eq!(boss_tier(2, 3), ChestTier::Final);
        assert_eq!(boss_tier(0, 1), ChestTier::Final);
    }

    /// Every tier's display is a `GameObjectDisplayInfo` row whose model the client data holds.
    #[test]
    fn every_chest_tier_display_is_in_the_client_data() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).unwrap();
        let catalog = benilla_formats::load_gameobject_catalog(&mut chain).unwrap();
        for tier in [
            ChestTier::Standard,
            ChestTier::Locked,
            ChestTier::Boss,
            ChestTier::Final,
        ] {
            let (display, _) = tier.look();
            let model = catalog
                .model_path(display)
                .unwrap_or_else(|| panic!("{tier:?}: no display {display}"));
            assert!(
                model.to_ascii_lowercase().contains("chest"),
                "{tier:?}: {model}"
            );
            let m2 = format!(
                "{}.m2",
                model.trim_end_matches(".mdx").trim_end_matches(".MDX")
            );
            assert!(
                chain.read(&m2).is_ok() || chain.read(model).is_ok(),
                "{tier:?}: {model} is not in the data"
            );
        }
    }

    #[test]
    fn a_boss_chest_pays_more_than_a_cache() {
        for t in [0.0, 300.0, data::VICTORY_SECS] {
            assert!(boss_chest_gold(ChestTier::Boss, t, 1.0) > cache_gold(t, 1.0));
            assert!(
                boss_chest_gold(ChestTier::Final, t, 1.0)
                    > boss_chest_gold(ChestTier::Boss, t, 1.0)
            );
        }
        let mut run = Run::default();
        let gold = open_boss_chest(&mut run, &HeroState::default(), ChestTier::Boss);
        assert_eq!(gold, boss_chest_gold(ChestTier::Boss, 0.0, 1.0));
        assert_eq!((run.gold, run.pending_levels), (gold, 1), "gold and a hand");
        assert_eq!(run.caches, 0, "no dearer locked chests for it");
    }

    /// The map, the roster's boss count, and a world holding what the boss-chest systems read.
    fn chest_world() -> (World, usize) {
        let map = data::MAPS
            .iter()
            .position(|m| m.name == "Elwynn Forest")
            .unwrap();
        let mut world = World::new();
        world.init_resource::<Messages<enemies::Loot>>();
        world.init_resource::<BossRolls>();
        world.insert_resource(Run::default());
        world.insert_resource(Selection { map, ..default() });
        (world, map)
    }

    fn chests(world: &mut World) -> Vec<(ChestTier, Option<usize>, bool)> {
        world
            .query::<&Cache>()
            .iter(world)
            .map(|c| (c.tier(), c.boss, c.is_open()))
            .collect()
    }

    #[test]
    fn a_roster_boss_leaves_its_chest_where_it_dies() {
        use bevy::ecs::system::RunSystemOnce;
        let (mut world, map) = chest_world();
        let bosses = &data::roster(map).bosses;
        let last = bosses.len() - 1;
        for b in [0, last] {
            world.write_message(enemies::Loot {
                pos: Vec3::new(10.0, 0.0, 5.0),
                xp: 1,
                boss: true,
                elite: false,
                def: &bosses[b],
            });
        }
        // An elite leaves no boss chest.
        world.write_message(enemies::Loot {
            pos: Vec3::ZERO,
            xp: 1,
            boss: false,
            elite: true,
            def: &bosses[0],
        });
        world.run_system_once(drop_boss_chests).unwrap();
        let mut got = chests(&mut world);
        got.sort_by_key(|c| c.1);
        assert_eq!(
            got,
            [
                (ChestTier::Boss, Some(0), false),
                (ChestTier::Final, Some(last), false)
            ]
        );
    }

    #[test]
    fn opening_a_boss_chest_queues_its_roll_once() {
        use crate::combat_text::CombatTextSpawn;
        use crate::creature_anim::{CastEvent, PlaySeq, SpellGoTargets, SwingMessage};
        use bevy::ecs::system::RunSystemOnce;
        let (mut world, map) = chest_world();
        world.init_resource::<Time>();
        world.init_resource::<NextState<Phase>>();
        world.init_resource::<PlaySeq>();
        world.init_resource::<Messages<CastEvent>>();
        world.init_resource::<Messages<SpellGoTargets>>();
        world.init_resource::<Messages<SwingMessage>>();
        world.init_resource::<Messages<CombatTextSpawn>>();
        world.init_resource::<Messages<FieldChanged>>();
        world.init_resource::<Messages<SoundCue>>();
        world.spawn((Hero::default(), Transform::default(), HeroState::default()));
        let at = Vec3::new(BOSS_CHEST_REACH, 0.0, 0.0);
        let chest = world
            .spawn((
                Guid(1),
                Transform::from_translation(at),
                ObjectStore(ObjectFields::default()),
                Cache {
                    opened: None,
                    tier: ChestTier::Boss,
                    boss: Some(1),
                    glow: None,
                    refused: false,
                },
            ))
            .id();
        for _ in 0..3 {
            world.run_system_once(open_caches).unwrap();
        }
        assert_eq!(
            world.resource::<BossRolls>().0,
            [BossChestOpened {
                map,
                boss: 1,
                chest,
                living_only: true,
            }],
            "rolled once, for the heroes standing"
        );
        let run = world.resource::<Run>();
        assert_eq!(run.gold, boss_chest_gold(ChestTier::Boss, 0.0, 1.0));
        assert_eq!(run.pending_levels, 1);
        assert!(matches!(
            *world.resource::<NextState<Phase>>(),
            NextState::Pending(Phase::LevelUp)
        ));
    }

    #[test]
    fn a_boss_chest_left_shut_opens_for_the_results() {
        use bevy::ecs::system::RunSystemOnce;
        let (mut world, map) = chest_world();
        world.spawn((HeroState::default(), LocalHero));
        let spawn = |world: &mut World, tier, boss, opened| {
            world
                .spawn(Cache {
                    opened,
                    tier,
                    boss,
                    glow: None,
                    refused: false,
                })
                .id()
        };
        let shut = spawn(&mut world, ChestTier::Final, Some(2), None);
        spawn(&mut world, ChestTier::Boss, Some(0), Some(1.0));
        spawn(&mut world, ChestTier::Standard, None, None);
        for _ in 0..2 {
            world.run_system_once(open_leftover_chests).unwrap();
        }
        assert_eq!(
            world.resource::<BossRolls>().0,
            [BossChestOpened {
                map,
                boss: 2,
                chest: shut,
                living_only: false,
            }],
            "only the shut boss chest, once, for every hero"
        );
        let run = world.resource::<Run>();
        assert_eq!(run.gold, boss_chest_gold(ChestTier::Final, 0.0, 1.0));
        assert_eq!(run.pending_levels, 0, "no hand at the run's end");
        assert!(chests(&mut world)
            .iter()
            .all(|&(tier, _, open)| open || tier == ChestTier::Standard));
    }
}
