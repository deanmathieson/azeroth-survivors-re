//! The quests on the battleground. A giver walks out near the party on [`super::schedule`]'s
//! rhythm wearing the engine's overhead `!` ([`crate::quest_markers`], fed through
//! [`QuestGiver`]); walking into it accepts its quest without a pause, and the [`Tracker`] runs.
//! Kills and pickups count for the whole party. A Collect's items drop from its foe and fly to a
//! hero as gems do; a Defend marks a spot and sends the horde at it; a named foe's quest keeps a
//! few of that foe walking in so the quest can always be done. Done, the giver wears a gold `?` and
//! walking back to it turns the quest in for a cache's worth ([`treasure::open_cache`]: gold, and a
//! hand for every hero) and a bonus. Failed or ignored, the giver walks off and fades.

use benilla_world::model_fade::DespawnFade;
use benilla_world::world_point::WorldPoint;
use bevy::prelude::*;

use super::super::data;
use super::super::enemies::{self, Enemy, Loot, Rank, Spawner};
use super::super::hero::{self, Hero, HERO_RADIUS};
use super::super::hud::{NoteKind, ResultsNotes};
use super::super::party;
use super::super::run_options::RunOptions;
use super::super::treasure;
use super::super::units::{self, flat, yaw_toward, Fx, UnitSpec};
use super::super::{HeroState, Phase, Rng, Run, Selection};
use super::catalog::{for_map, Goal, ItemLook, QuestDef};
use super::objective::{Status, Tracker};
use super::schedule::{self, FIRST_GIVER_SECS, GIVER_EVERY_SECS, MIN_EVERY_SECS};
use crate::creature_anim::{move_flags, MovementState};
use crate::net::{Guid, GuidIndex};
use crate::ui_quest::QuestGiver;

/// A hero this near a giver's edge accepts or turns in its quest.
const GIVER_REACH: f32 = 2.2;
/// Seconds a giver waits for the party before it moves on.
pub(in crate::survivors) const OFFER_SECS: f32 = 45.0;
/// A giver leaving walks this long, at this pace, before it fades.
const LEAVE_SECS: f32 = 4.0;
const WALK_SPEED: f32 = 2.5;
/// A Defend's spot: a hero within this many yards holds it.
pub(in crate::survivors) const SPOT_RADIUS: f32 = 10.0;
/// While a hero is this near the spot the horde comes at it, one foe every [`SPOT_SPAWN_SECS`]
/// (sooner under a denser Run Options horde) from a ring this wide round it.
const SPOT_PULL: f32 = 30.0;
const SPOT_SPAWN_SECS: f32 = 1.0;
const SPOT_RING: (f32, f32) = (20.0, 26.0);
/// A named foe's quest sends one more of it every [`FEED_SECS`] while fewer than [`FEED_ALIVE`]
/// stand, from beyond the view, as the spawner's ring does.
const FEED_SECS: f32 = 2.5;
const FEED_ALIVE: usize = 5;
const FEED_RING: (f32, f32) = (30.0, 36.0);
/// The spot's marks: the dueling flag (`DuelingFlag.mdx`) in its middle and small braziers
/// (`SmallBrazier01.mdx`) round its edge, as `GameObjectDisplayInfo` rows and scales.
const SPOT_FLAG: (u32, f32) = (787, 1.3);
const SPOT_BRAZIER: (u32, f32) = (201, 0.6);
const SPOT_BRAZIERS: usize = 8;
#[cfg(test)]
pub(in crate::survivors) const SPOT_DISPLAYS: [u32; 2] = [SPOT_FLAG.0, SPOT_BRAZIER.0];
/// A quest item's starting pace, a gem's.
const ITEM_SPEED: f32 = 6.0;
/// A quest turned in pays a cache, then this many coins more and this share of a level.
const BONUS_COINS: u32 = 8;
const BONUS_LEVEL_SHARE: f32 = 0.5;
/// The combat-text colour of the gold paid, 0xAARRGGBB, as a cache's.
const COLOR_GOLD: u32 = 0xFFFF_D200;

/// `SMSG_QUESTGIVER_STATUS` values the engine's overhead markers draw: the gold `!`, the grey `?`
/// and the gold `?` (`quest_markers::marker_model`).
const STATUS_AVAILABLE: u32 = 5;
const STATUS_INCOMPLETE: u32 = 3;
const STATUS_REWARD: u32 = 6;

/// A quest giver standing on the battleground.
#[derive(Component)]
pub(in crate::survivors) struct QuestNpc;

/// A quest item on the ground, flying to a hero once a magnet reaches it.
#[derive(Component)]
pub(in crate::survivors) struct QuestItem {
    pulled: Option<Entity>,
    speed: f32,
}

/// What the survivors minimap marks for the quests: a giver, pinned to the rim while out of view,
/// and a Defend's spot as a ring of its true size.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) enum MapPin {
    Giver,
    Spot { radius: f32 },
}

/// Where the run's quest giver is with its quest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) enum Stage {
    /// Waiting for a hero to walk in, for this many seconds more.
    Offered(f32),
    /// Accepted: its [`Tracker`] runs.
    Accepted,
    /// Walking off the field, this many seconds so far.
    Leaving(f32),
}

/// The giver on the field and the quest it brought.
#[derive(Debug)]
pub(in crate::survivors) struct Active {
    pub(in crate::survivors) quest: &'static QuestDef,
    pub(in crate::survivors) giver_name: &'static str,
    pub(in crate::survivors) stage: Stage,
    pub(in crate::survivors) tracker: Tracker,
    giver: Entity,
    /// The giver's guid, once its [`Guid`] is readable: the marker's key.
    guid: Option<u64>,
    /// Where it stands.
    post: Vec3,
    /// What the tracker last announced, so completion is told once.
    announced: Status,
    /// A Defend's spot once accepted, and the props marking it.
    spot: Option<Vec3>,
    props: Vec<Entity>,
    /// Seconds to the next foe the quest sends.
    feed: f32,
}

impl Active {
    fn offered(
        quest: &'static QuestDef,
        giver_name: &'static str,
        giver: Entity,
        post: Vec3,
    ) -> Self {
        Self {
            quest,
            giver_name,
            stage: Stage::Offered(OFFER_SECS),
            tracker: Tracker::new(quest),
            giver,
            guid: None,
            post,
            announced: Status::Active,
            spot: None,
            props: Vec::new(),
            feed: 0.0,
        }
    }

    /// An accepted quest still under way.
    fn running(&self) -> bool {
        self.stage == Stage::Accepted && self.tracker.status == Status::Active
    }
}

/// The env aids, read once: `WOW_SURVIVORS_QUEST_EVERY=<secs>` and `WOW_SURVIVORS_QUEST=<id>`.
#[derive(Resource, Clone, Debug, PartialEq)]
pub(in crate::survivors) struct QuestAids {
    pub(in crate::survivors) every: f32,
    pub(in crate::survivors) forced: Option<String>,
}

impl Default for QuestAids {
    fn default() -> Self {
        Self {
            every: GIVER_EVERY_SECS,
            forced: None,
        }
    }
}

impl QuestAids {
    pub(super) fn from_env() -> Self {
        let every = every_from(crate::webenv::var("WOW_SURVIVORS_QUEST_EVERY").as_deref());
        let forced = crate::webenv::var("WOW_SURVIVORS_QUEST")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        if let Some(id) = &forced {
            match super::catalog::quest(id) {
                Some((zone, _)) => info!(
                    "survivors quests: the first giver brings {id} ({})",
                    zone.zone
                ),
                None => warn!("survivors quests: WOW_SURVIVORS_QUEST={id:?} is no quest"),
            }
        }
        Self { every, forced }
    }
}

/// The giver interval `WOW_SURVIVORS_QUEST_EVERY` asks for: seconds, at least
/// [`MIN_EVERY_SECS`]; unset or malformed is the default.
pub(in crate::survivors) fn every_from(text: Option<&str>) -> f32 {
    let Some(text) = text else {
        return GIVER_EVERY_SECS;
    };
    match text.trim().parse::<f32>() {
        Ok(secs) if secs.is_finite() => {
            info!("survivors quests: a giver every {secs} s (WOW_SURVIVORS_QUEST_EVERY)");
            secs.max(MIN_EVERY_SECS)
        }
        _ => {
            warn!(
                "survivors quests: WOW_SURVIVORS_QUEST_EVERY={text:?} is not a number of seconds"
            );
            GIVER_EVERY_SECS
        }
    }
}

/// The run's quests: when the next giver is due, what was offered, and the one on the field.
#[derive(Resource, Debug)]
pub(in crate::survivors) struct QuestRun {
    every: f32,
    /// The quest the run's first giver brings, from `WOW_SURVIVORS_QUEST`.
    forced: Option<String>,
    /// Run seconds the next giver is due.
    next_due: f32,
    /// Quest ids offered this run.
    offered: Vec<&'static str>,
    pub(in crate::survivors) active: Option<Active>,
}

impl QuestRun {
    /// A run's start: the first giver a minute in, or sooner under a shorter interval.
    pub(in crate::survivors) fn new(aids: &QuestAids) -> Self {
        Self {
            every: aids.every,
            forced: aids.forced.clone(),
            next_due: FIRST_GIVER_SECS.min(aids.every),
            offered: Vec::new(),
            active: None,
        }
    }
}

/// Back at the menu: the giver's marker goes with it, and the next run's quests start over.
pub(super) fn reset(
    mut quests: ResMut<QuestRun>,
    aids: Res<QuestAids>,
    mut index: ResMut<GuidIndex>,
    mut markers: ResMut<QuestGiver>,
) {
    if let Some(a) = &quests.active {
        forget_giver(a, &mut index, &mut markers);
    }
    *quests = QuestRun::new(&aids);
}

/// Drop a giver's marker and its place in the guid index.
fn forget_giver(a: &Active, index: &mut GuidIndex, markers: &mut QuestGiver) {
    let Some(guid) = a.guid else { return };
    if index.0.get(&guid) == Some(&a.giver) {
        index.0.remove(&guid);
    }
    if markers.status(guid).is_some() {
        markers.clear_status(guid);
    }
}

/// Every kill counts for the quest under way, whoever landed it; a Collect's foe may drop its item.
/// The zone's last boss falling is noted on the run for the chains.
pub(super) fn count_kills(
    mut commands: Commands,
    mut drops: MessageReader<Loot>,
    mut quests: ResMut<QuestRun>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    selection: Res<Selection>,
) {
    let last_boss = data::roster(selection.map).bosses.last().map(|b| b.name);
    for d in drops.read() {
        if d.boss && Some(d.def.name) == last_boss {
            run.final_boss = true;
        }
        let Some(a) = quests
            .active
            .as_mut()
            .filter(|a| a.stage == Stage::Accepted)
        else {
            continue;
        };
        a.tracker.kill(d.def.name, d.elite);
        let (Some(chance), Goal::Collect { look, .. }) =
            (a.tracker.drop_chance(d.def.name), a.quest.goal)
        else {
            continue;
        };
        if rng.f32() < chance {
            let serial = run.guid();
            spawn_item(&mut commands, serial, look, d.pos);
        }
    }
}

fn spawn_item(commands: &mut Commands, serial: u64, look: ItemLook, at: Vec3) {
    let (display, scale) = look.display();
    let e = units::spawn_object(
        commands,
        serial,
        display,
        scale,
        at + Vec3::new(0.0, 0.5, 0.8),
    );
    commands.entity(e).insert(QuestItem {
        pulled: None,
        speed: ITEM_SPEED,
    });
}

/// Quest items fly to the nearest hero standing whose magnet reaches them, as gems do, and count
/// for the party when they land.
#[allow(clippy::type_complexity)]
pub(super) fn collect_items(
    mut commands: Commands,
    time: Res<Time>,
    mut quests: ResMut<QuestRun>,
    heroes: Query<(Entity, &Transform, &HeroState), (With<Hero>, Without<QuestItem>)>,
    mut items: Query<(Entity, &mut Transform, &mut QuestItem)>,
) {
    let standing: Vec<(Entity, Vec3, f32)> = heroes
        .iter()
        .filter(|(_, _, h)| h.alive())
        .map(|(e, tf, h)| (e, tf.translation + Vec3::Y, h.stats().magnet))
        .collect();
    if standing.is_empty() {
        return;
    }
    let dt = time.delta_secs();
    for (e, mut tf, mut item) in &mut items {
        let at = tf.translation;
        if item.pulled.is_none() {
            let reached = standing
                .iter()
                .filter(|h| flat(h.1 - at).length() <= h.2)
                .map(|h| (h.0, h.1));
            item.pulled = party::nearest(reached, at).map(|(hero, _)| hero);
        }
        let Some(to) = item.pulled else { continue };
        // Its hero fell on the way: it flies on to the nearest one standing.
        let Some(&(hero, dest, _)) = standing.iter().find(|h| h.0 == to).or_else(|| {
            standing
                .iter()
                .min_by(|a, b| hero::reach(at, a.1).total_cmp(&hero::reach(at, b.1)))
        }) else {
            continue;
        };
        item.pulled = Some(hero);
        let d = dest - at;
        let dist = d.length();
        if dist < 0.9 {
            commands.entity(e).try_despawn();
            if let Some(a) = quests
                .active
                .as_mut()
                .filter(|a| a.stage == Stage::Accepted)
            {
                a.tracker.pick_up();
            }
            continue;
        }
        item.speed += 30.0 * dt;
        tf.translation += d / dist * (item.speed * dt).min(dist);
    }
}

/// Whether any of the standing heroes at `heroes` holds the Defend spot at `spot`.
pub(in crate::survivors) fn holds_spot(heroes: impl IntoIterator<Item = Vec3>, spot: Vec3) -> bool {
    heroes
        .into_iter()
        .any(|p| flat(p - spot).length() <= SPOT_RADIUS)
}

/// The clocks: an offer's patience, and an accepted quest's time limit and spot; done, the party
/// is told to go back.
pub(super) fn tick_quests(
    time: Res<Time>,
    mut quests: ResMut<QuestRun>,
    mut run: ResMut<Run>,
    heroes: Query<(&Transform, &HeroState), With<Hero>>,
) {
    let Some(a) = quests.active.as_mut() else {
        return;
    };
    let dt = time.delta_secs();
    match &mut a.stage {
        Stage::Offered(wait) => {
            *wait -= dt;
            return;
        }
        Stage::Leaving(_) => return,
        Stage::Accepted => {}
    }
    let standing = heroes
        .iter()
        .filter(|(_, h)| h.alive())
        .map(|(tf, _)| tf.translation);
    let held = a.spot.is_some_and(|spot| holds_spot(standing, spot));
    a.tracker.tick(dt, held);
    if a.tracker.status == Status::Complete && a.announced != Status::Complete {
        a.announced = Status::Complete;
        run.banner = Some((
            format!("{} complete! Return to {}.", a.quest.name, a.giver_name),
            3.5,
        ));
    }
}

/// What turning a quest in pays: a cache through [`treasure::open_cache`] under the gold of
/// `hero`, who turned it in (its gold for the party's purse, and a hand that every hero is dealt),
/// then [`BONUS_COINS`] coins more and [`BONUS_LEVEL_SHARE`] of a level's experience for the
/// party's bar. Returns the gold paid.
///
/// A cache's worth, but no cache: the run's count of caches opened prices the next locked chest
/// ([`treasure::chest_cost`]) and fills the results' Caches tile, and a turn-in is neither (the
/// quests keep their own count, [`Run::quests`]), so the count is put back as it was.
pub(in crate::survivors) fn pay_out(run: &mut Run, hero: &HeroState) -> u32 {
    let caches = run.caches;
    let cache = treasure::open_cache(run, hero);
    run.caches = caches;
    let bonus = treasure::worth(BONUS_COINS, run.elapsed, hero.stats().gold, 0.5);
    run.gold += bonus;
    run.xp += data::xp_to_next(run.level) * BONUS_LEVEL_SHARE;
    cache + bonus
}

/// A hero walking into the giver: accepts its quest, or turns a finished one in. An offer nobody
/// takes, and a quest that runs out of time, send the giver off.
#[allow(clippy::too_many_arguments)]
pub(super) fn meet_givers(
    mut commands: Commands,
    mut quests: ResMut<QuestRun>,
    mut run: ResMut<Run>,
    mut next: ResMut<NextState<Phase>>,
    mut notes: ResMut<ResultsNotes>,
    selection: Res<Selection>,
    heroes: Query<(Entity, &Transform, &HeroState), With<Hero>>,
    items: Query<Entity, With<QuestItem>>,
    mut fx: Fx,
    world: WorldPoint,
) {
    let Some(a) = quests.active.as_mut() else {
        return;
    };
    let reach = |at: Vec3| {
        let near = heroes
            .iter()
            .filter(|(_, tf, h)| {
                h.alive() && flat(at - tf.translation).length() <= GIVER_REACH + HERO_RADIUS
            })
            .map(|(e, tf, _)| (e, tf.translation));
        party::nearest(near, at)
    };
    match a.stage {
        Stage::Offered(wait) => {
            if let Some((_, from)) = reach(a.post) {
                a.stage = Stage::Accepted;
                a.tracker = Tracker::new(a.quest);
                if matches!(a.quest.goal, Goal::Defend { .. }) {
                    // Beyond the giver, on from the hero who took the quest.
                    let stage = selection.stage_def();
                    let away = flat(a.post - from).normalize_or(Vec3::X);
                    let angle = away.z.atan2(away.x);
                    let spot = schedule::post_near(
                        a.post,
                        hero::arena(stage),
                        stage.leash,
                        angle,
                        schedule::SPOT_GAP,
                    );
                    let spot = enemies::ground(&world, spot, a.post.y);
                    a.props = mark_spot(&mut commands, &mut run, &world, spot);
                    a.spot = Some(spot);
                }
                run.banner = Some((format!("Quest accepted: {}", a.quest.name), 3.0));
            } else if wait <= 0.0 {
                send_off(&mut commands, a);
                run.banner = Some((format!("{} has moved on.", a.giver_name), 2.5));
            }
        }
        Stage::Accepted => match a.tracker.status {
            Status::Active => {}
            Status::Failed => {
                send_off(&mut commands, a);
                clear_quest(&mut commands, a, &items);
                run.banner = Some((format!("Quest failed: {}", a.quest.name), 3.0));
            }
            Status::Complete => {
                let Some((hero, _)) = reach(a.post) else {
                    return;
                };
                let Ok((.., state)) = heroes.get(hero) else {
                    return;
                };
                let gold = pay_out(&mut run, state);
                run.quests += 1;
                fx.number(hero, gold as f32, false, COLOR_GOLD);
                notes.push(format!("Quest complete: {}", a.quest.name), NoteKind::Good);
                run.banner = Some((
                    format!("Quest complete: {}! +{gold} gold", a.quest.name),
                    3.5,
                ));
                send_off(&mut commands, a);
                clear_quest(&mut commands, a, &items);
                if run.pending_levels > 0 {
                    next.set(Phase::LevelUp);
                }
            }
        },
        Stage::Leaving(_) => {}
    }
}

/// Send the giver off, its quest over or its offer not taken: off the minimap, and away.
fn send_off(commands: &mut Commands, a: &mut Active) {
    a.stage = Stage::Leaving(0.0);
    commands.entity(a.giver).try_remove::<MapPin>();
}

/// The Defend spot's marks: the flag in its middle, pinned on the minimap, and the braziers round
/// its edge.
fn mark_spot(
    commands: &mut Commands,
    run: &mut Run,
    world: &WorldPoint,
    spot: Vec3,
) -> Vec<Entity> {
    let (display, scale) = SPOT_FLAG;
    let flag = units::spawn_object(commands, run.guid(), display, scale, spot);
    commands.entity(flag).insert(MapPin::Spot {
        radius: SPOT_RADIUS,
    });
    let mut props = vec![flag];
    let (display, scale) = SPOT_BRAZIER;
    for i in 0..SPOT_BRAZIERS {
        let a = i as f32 / SPOT_BRAZIERS as f32 * std::f32::consts::TAU;
        let at = spot + Vec3::new(a.cos(), 0.0, a.sin()) * SPOT_RADIUS;
        let at = enemies::ground(world, at, spot.y);
        props.push(units::spawn_object(
            commands,
            run.guid(),
            display,
            scale,
            at,
        ));
    }
    props
}

/// A quest over: the spot's marks fade and the items left lying go.
fn clear_quest(commands: &mut Commands, a: &mut Active, items: &Query<Entity, With<QuestItem>>) {
    for prop in a.props.drain(..) {
        commands
            .entity(prop)
            .try_remove::<MapPin>()
            .try_insert(DespawnFade::default());
    }
    a.spot = None;
    for item in items {
        commands.entity(item).try_despawn();
    }
}

/// On the giver rhythm, with no quest on the field and the party standing, a giver walks out near
/// it, inside the leash, bringing a quest that fits the moment (`WOW_SURVIVORS_QUEST`'s, first).
#[allow(clippy::too_many_arguments)]
pub(super) fn offer_quests(
    mut commands: Commands,
    mut quests: ResMut<QuestRun>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    selection: Res<Selection>,
    heroes: Query<(&Transform, &HeroState), With<Hero>>,
    world: WorldPoint,
) {
    if quests.active.is_some() || run.elapsed < quests.next_due {
        return;
    }
    let map = selection.map;
    let Some(zone) = for_map(map) else {
        quests.next_due = f32::INFINITY;
        return;
    };
    match schedule::next_giver(run.elapsed, &schedule::boss_times(map)) {
        None => {
            quests.next_due = f32::INFINITY;
            return;
        }
        Some(t) if t > run.elapsed => {
            quests.next_due = t;
            return;
        }
        Some(_) => {}
    }
    let standing = heroes.iter().filter(|(_, h)| h.alive());
    let Some(centre) = party::centre(standing.map(|(tf, _)| tf.translation)) else {
        return;
    };
    quests.next_due = run.elapsed + quests.every;
    let forced = quests.forced.take().and_then(|id| {
        let found = zone.quests.iter().find(|q| q.id == id);
        if found.is_none() {
            warn!(
                "survivors quests: {id} is not one of {}'s quests",
                zone.zone
            );
        }
        found
    });
    let roster = &data::roster(map).enemies;
    let (elapsed, next_elite) = (run.elapsed, run.next_elite);
    let quest = forced.or_else(|| {
        let roll = rng.next_u64();
        schedule::pick(
            zone.quests,
            &quests.offered,
            |q| schedule::fits(q, roster, elapsed, next_elite),
            roll,
        )
    });
    let Some(quest) = quest else { return };
    let stage = selection.stage_def();
    let angle = rng.range(0.0, std::f32::consts::TAU);
    let post = schedule::post_near(
        centre,
        hero::arena(stage),
        stage.leash,
        angle,
        schedule::GIVER_GAP,
    );
    let post = enemies::ground(&world, post, centre.y);
    let serial = run.guid();
    let giver = units::spawn_unit(
        &mut commands,
        &UnitSpec {
            serial,
            display: zone.giver.display,
            scale: zone.giver.scale,
            level: 60,
            faction: units::FACTION_FRIENDLY,
            bytes0: 1 << 8,
            weapons: &[],
            sheath: 0,
        },
        post,
        yaw_toward(centre - post),
    );
    commands.entity(giver).insert((QuestNpc, MapPin::Giver));
    info!(
        "survivors quests: {} brings {} at {:.0} s",
        zone.giver.name, quest.id, run.elapsed
    );
    quests.offered.push(quest.id);
    quests.active = Some(Active::offered(quest, zone.giver.name, giver, post));
    run.banner = Some((format!("{} has a quest for you!", zone.giver.name), 3.0));
}

/// The horde a quest stirs up: its named foe kept walking in, and a Defend's spot drawing more of
/// the moment's wave while a hero is near it.
#[allow(clippy::too_many_arguments)]
pub(super) fn feed_foes(
    mut commands: Commands,
    time: Res<Time>,
    mut quests: ResMut<QuestRun>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    selection: Res<Selection>,
    opts: Res<RunOptions>,
    mut spawner: ResMut<Spawner>,
    heroes: Query<(&Transform, &HeroState), With<Hero>>,
    foes: Query<&Enemy>,
    world: WorldPoint,
) {
    let Some(a) = quests.active.as_mut().filter(|a| a.running()) else {
        return;
    };
    let standing: Vec<Vec3> = heroes
        .iter()
        .filter(|(_, h)| h.alive())
        .map(|(tf, _)| tf.translation)
        .collect();
    let Some(centre) = party::centre(standing.iter().copied()) else {
        return;
    };
    a.feed -= time.delta_secs();
    if a.feed > 0.0 {
        return;
    }
    let roster = data::roster(selection.map);
    let ring = |rng: &mut Rng, around: Vec3, (lo, hi): (f32, f32)| {
        let angle = rng.range(0.0, std::f32::consts::TAU);
        let r = rng.range(lo, hi);
        enemies::ground(
            &world,
            around + Vec3::new(angle.cos(), 0.0, angle.sin()) * r,
            around.y,
        )
    };
    let (def, pos, toward) = match a.quest.goal {
        Goal::Kill {
            foe: Some(name), ..
        }
        | Goal::Collect { foe: name, .. } => {
            a.feed = FEED_SECS;
            let alive = foes
                .iter()
                .filter(|e| !e.dead && e.def.name == name)
                .count();
            let Some(def) = roster.enemies.iter().find(|e| e.name == name) else {
                return;
            };
            if alive >= FEED_ALIVE {
                return;
            }
            (def, ring(&mut rng, centre, FEED_RING), centre)
        }
        Goal::Defend { .. } => {
            a.feed = SPOT_SPAWN_SECS / opts.density.max(0.25);
            let Some(spot) = a.spot else { return };
            if !standing
                .iter()
                .any(|p| flat(*p - spot).length() <= SPOT_PULL)
            {
                return;
            }
            let pool = data::wave_pool(run.elapsed, roster.enemies.len());
            let def = &roster.enemies[pool[rng.index(pool.len())]];
            (def, ring(&mut rng, spot, SPOT_RING), spot)
        }
        Goal::Kill { foe: None, .. } | Goal::SlayElite => {
            a.feed = f32::INFINITY;
            return;
        }
    };
    enemies::spawn_enemy(
        &mut commands,
        &mut run,
        def,
        pos,
        toward,
        Rank::Horde,
        &opts,
        &mut spawner.xp_carry,
    );
}

/// A giver leaving walks away from the party, then fades; with it gone, the next may come.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn walk_off(
    mut commands: Commands,
    time: Res<Time>,
    mut quests: ResMut<QuestRun>,
    mut index: ResMut<GuidIndex>,
    mut markers: ResMut<QuestGiver>,
    heroes: Query<(&Transform, &HeroState), With<Hero>>,
    mut npcs: Query<(&mut Transform, &mut MovementState), (With<QuestNpc>, Without<Hero>)>,
    world: WorldPoint,
) {
    let Some(a) = quests.active.as_mut() else {
        return;
    };
    let Stage::Leaving(t) = &mut a.stage else {
        return;
    };
    let dt = time.delta_secs();
    *t += dt;
    let gone = *t >= LEAVE_SECS;
    if let Ok((mut tf, mut mv)) = npcs.get_mut(a.giver) {
        if gone {
            mv.speed = 0.0;
            mv.flags = 0;
        } else {
            let standing = heroes.iter().filter(|(_, h)| h.alive());
            let from = party::centre(standing.map(|(tf, _)| tf.translation)).unwrap_or(a.post);
            let dir = flat(tf.translation - from).normalize_or(Vec3::X);
            let to = tf.translation + dir * WALK_SPEED * dt;
            tf.translation = enemies::ground(&world, to, tf.translation.y);
            tf.rotation = Quat::from_rotation_y(yaw_toward(dir));
            mv.speed = WALK_SPEED;
            mv.flags = move_flags::FORWARD;
        }
    }
    if !gone {
        return;
    }
    commands
        .entity(a.giver)
        .try_remove::<(QuestNpc, MapPin)>()
        .try_insert(DespawnFade::default());
    forget_giver(a, &mut index, &mut markers);
    quests.active = None;
}

/// The overhead marker a giver wears: the gold `!` on offer, the grey `?` while its quest is under
/// way, the gold `?` once it can be turned in, and none while it leaves.
pub(in crate::survivors) fn marker_status(stage: Stage, status: Status) -> Option<u32> {
    match (stage, status) {
        (Stage::Offered(_), _) => Some(STATUS_AVAILABLE),
        (Stage::Accepted, Status::Active) => Some(STATUS_INCOMPLETE),
        (Stage::Accepted, Status::Complete) => Some(STATUS_REWARD),
        (Stage::Accepted, Status::Failed) | (Stage::Leaving(_), _) => None,
    }
}

/// The engine's overhead questgiver markers ([`crate::quest_markers`]) draw over a unit the guid
/// index knows, from the dialog status [`QuestGiver`] holds for it: a giver is entered in both,
/// its status kept to [`marker_status`]. There is no server here to ask, so nothing else writes
/// either for it.
pub(super) fn mark_givers(
    mut quests: ResMut<QuestRun>,
    guids: Query<&Guid, With<QuestNpc>>,
    mut index: ResMut<GuidIndex>,
    mut markers: ResMut<QuestGiver>,
) {
    let Some(a) = quests.active.as_mut() else {
        return;
    };
    if a.guid.is_none() {
        a.guid = guids.get(a.giver).ok().map(|g| g.0);
    }
    let Some(guid) = a.guid else { return };
    match marker_status(a.stage, a.tracker.status) {
        Some(status) => {
            if index.0.get(&guid) != Some(&a.giver) {
                index.0.insert(guid, a.giver);
            }
            if markers.status(guid) != Some(status) {
                markers.set_status(guid, status);
            }
        }
        None => {
            if markers.status(guid).is_some() {
                markers.clear_status(guid);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::super::catalog::ZONES;
    use super::*;

    fn index(name: &str) -> usize {
        data::MAPS.iter().position(|m| m.name == name).unwrap()
    }

    fn quest(id: &str) -> &'static QuestDef {
        super::super::catalog::quest(id).unwrap().1
    }

    /// A run on `map` with `quest` accepted, and two heroes ten yards apart.
    fn world_with(map: usize, quest: &'static QuestDef) -> (World, Entity, Entity) {
        let mut world = World::new();
        world.init_resource::<Messages<Loot>>();
        world.init_resource::<Time>();
        world.init_resource::<Run>();
        world.insert_resource(Rng(7));
        world.insert_resource(Selection {
            map,
            stage: map,
            ..Default::default()
        });
        let mut quests = QuestRun::new(&QuestAids {
            every: GIVER_EVERY_SECS,
            forced: None,
        });
        let mut active = Active::offered(quest, "Giver", Entity::PLACEHOLDER, Vec3::ZERO);
        active.stage = Stage::Accepted;
        quests.active = Some(active);
        world.insert_resource(quests);
        let hero = |world: &mut World, player: u8, x: f32| {
            let state = HeroState {
                player,
                ..HeroState::default()
            };
            world
                .spawn((
                    Hero { vy: 0.0, air: 0.0 },
                    Transform::from_xyz(x, 0.0, 0.0),
                    state,
                ))
                .id()
        };
        let a = hero(&mut world, 0, 0.0);
        let b = hero(&mut world, 1, 10.0);
        (world, a, b)
    }

    fn tracker(world: &World) -> &Tracker {
        &world
            .resource::<QuestRun>()
            .active
            .as_ref()
            .unwrap()
            .tracker
    }

    fn slay(world: &mut World, map: usize, name: &str, at: Vec3, boss: bool) {
        let def = data::roster(map)
            .enemies
            .iter()
            .chain(&data::roster(map).bosses)
            .find(|e| e.name == name)
            .unwrap();
        world.resource_mut::<Messages<Loot>>().write(Loot {
            pos: at,
            xp: 1,
            boss,
            elite: false,
            def,
        });
    }

    #[test]
    fn both_heroes_kills_count_for_the_party() {
        let elwynn = index("Elwynn Forest");
        let (mut world, _, _) = world_with(elwynn, quest("elwynn_fargodeep_mine"));
        // One tunneler at each hero's feet: two heroes' kills, one count.
        slay(&mut world, elwynn, "Kobold Tunneler", Vec3::ZERO, false);
        slay(&mut world, elwynn, "Kobold Tunneler", Vec3::X * 10.0, false);
        slay(&mut world, elwynn, "Kobold Miner", Vec3::X * 10.0, false);
        world.run_system_once(count_kills).unwrap();
        assert_eq!(tracker(&world).count, 2);
        assert!(!world.resource::<Run>().final_boss);
        // The zone's last boss falling is kept for the chain.
        slay(&mut world, elwynn, "Gruff Swiftbite", Vec3::ZERO, true);
        world.run_system_once(count_kills).unwrap();
        assert!(world.resource::<Run>().final_boss);
    }

    #[test]
    fn both_heroes_pickups_count_for_the_party() {
        let elwynn = index("Elwynn Forest");
        let (mut world, _, _) = world_with(elwynn, quest("elwynn_kobold_candles"));
        // A candle landing on each hero this frame.
        for x in [0.0, 10.0] {
            world.spawn((
                Transform::from_xyz(x, 1.0, 0.0),
                QuestItem {
                    pulled: None,
                    speed: ITEM_SPEED,
                },
            ));
        }
        // And one far from both, which stays where it fell.
        world.spawn((
            Transform::from_xyz(-40.0, 1.0, 0.0),
            QuestItem {
                pulled: None,
                speed: ITEM_SPEED,
            },
        ));
        world.run_system_once(collect_items).unwrap();
        assert_eq!(tracker(&world).count, 2);
        let mut left = world.query::<&QuestItem>();
        assert_eq!(left.iter(&world).count(), 1);
    }

    #[test]
    fn any_hero_standing_holds_the_spot() {
        let spot = Vec3::new(50.0, 0.0, 0.0);
        assert!(!holds_spot([Vec3::ZERO, Vec3::X * 39.0], spot));
        assert!(holds_spot([Vec3::ZERO, Vec3::new(41.0, 7.0, 0.0)], spot));
        assert!(holds_spot([Vec3::new(50.0, 0.0, SPOT_RADIUS)], spot));
        assert!(!holds_spot([], spot));
    }

    #[test]
    fn a_turned_in_quest_pays_a_cache_and_more() {
        let mut run = Run {
            elapsed: 300.0,
            ..Run::default()
        };
        let hero = HeroState::default();
        let gold = pay_out(&mut run, &hero);
        assert_eq!(run.gold, gold);
        assert!(
            gold > treasure::cache_gold(300.0, 1.0),
            "more than a cache alone"
        );
        assert_eq!(run.pending_levels, 1, "a cache's hand");
        assert_eq!(run.xp, data::xp_to_next(1) * BONUS_LEVEL_SHARE);
    }

    #[test]
    fn a_turn_in_neither_prices_chests_nor_counts_as_a_cache() {
        let mut run = Run {
            elapsed: 300.0,
            caches: 2,
            ..Run::default()
        };
        let price = treasure::chest_cost(run.caches);
        let hero = HeroState::default();
        for _ in 0..3 {
            pay_out(&mut run, &hero);
        }
        assert_eq!(run.caches, 2, "the Caches tile counts caches opened");
        assert_eq!(treasure::chest_cost(run.caches), price);
        assert_eq!(run.pending_levels, 3, "each still deals its hand");
    }

    #[test]
    fn givers_wear_the_marker_their_quest_calls_for() {
        assert_eq!(marker_status(Stage::Offered(3.0), Status::Active), Some(5));
        assert_eq!(marker_status(Stage::Accepted, Status::Active), Some(3));
        assert_eq!(marker_status(Stage::Accepted, Status::Complete), Some(6));
        assert_eq!(marker_status(Stage::Accepted, Status::Failed), None);
        assert_eq!(marker_status(Stage::Leaving(1.0), Status::Complete), None);
    }

    #[test]
    fn the_giver_is_indexed_and_marked_then_forgotten() {
        let mut world = World::new();
        world.init_resource::<GuidIndex>();
        world.init_resource::<QuestGiver>();
        world.init_resource::<QuestAids>();
        let giver = world.spawn((Guid(0xF130_0000_0000_0042), QuestNpc)).id();
        let mut quests = QuestRun::new(world.resource::<QuestAids>());
        quests.active = Some(Active::offered(
            quest("durotar_hold_razor_hill"),
            "Grunt",
            giver,
            Vec3::ZERO,
        ));
        world.insert_resource(quests);
        world.run_system_once(mark_givers).unwrap();
        let guid = 0xF130_0000_0000_0042;
        assert_eq!(world.resource::<GuidIndex>().0.get(&guid), Some(&giver));
        assert_eq!(world.resource::<QuestGiver>().status(guid), Some(5));
        world
            .resource_mut::<QuestRun>()
            .active
            .as_mut()
            .unwrap()
            .stage = Stage::Leaving(0.0);
        world.run_system_once(mark_givers).unwrap();
        assert_eq!(world.resource::<QuestGiver>().status(guid), None);
        // Back at the menu, the index forgets it.
        world.run_system_once(reset).unwrap();
        assert!(world.resource::<GuidIndex>().0.is_empty());
        assert!(world.resource::<QuestRun>().active.is_none());
    }

    #[test]
    fn the_env_aids_read_their_numbers() {
        assert_eq!(every_from(None), GIVER_EVERY_SECS);
        assert_eq!(every_from(Some(" 30 ")), 30.0);
        assert_eq!(every_from(Some("1")), MIN_EVERY_SECS);
        assert_eq!(every_from(Some("soon")), GIVER_EVERY_SECS);
        let quick = QuestRun::new(&QuestAids {
            every: 20.0,
            forced: None,
        });
        assert_eq!(
            quick.next_due, 20.0,
            "a short interval brings the first giver sooner"
        );
        let plain = QuestRun::new(&QuestAids::default());
        assert_eq!(plain.next_due, FIRST_GIVER_SECS);
        assert!(ZONES.iter().all(|z| !z.quests.is_empty()));
    }
}
