//! Gathering in runs (docs/PROFESSIONS.md, 4.1-4.5, 7): nodes and pools spawned by whoever
//! simulates the run, gathered by standing on them while the hero keeps fighting, and each hero's
//! own yield, rolled by the host.
//!
//! - **Spawning** ([`spawn_nodes`], host or alone): a few nodes as the run starts, then one every
//!   60-90 s up to [`MAX_NODES`], from the battleground's band, away from the heroes, inside the
//!   leash, none during a camp or with a boss up. A pool spawns only on water. Each lasts
//!   [`NODE_LIFE`] s (a pool [`POOL_LIFE`]), then sinks.
//! - **A node is the party's** ([`harvest`]): every hero who finishes a gather on it gets their
//!   own yield, once; a vein or herb depletes [`DEPLETE_AFTER`] s after its first harvest, or once
//!   every hero has had it. A pool serves each player once for its whole life.
//! - **Gathering** ([`gather`], every machine, its own hero): within reach for the cast's length
//!   plays the real gather spell (its precast kit and sound) and the stock `CastingBarFrame`; a hit
//!   pushes it back 0.25 s, at most three times; walking off interrupts it. Too low a skill says
//!   "Requires Mining (175)" once and starts nothing.
//! - **Yields** ([`take_yields`]): into [`Bags`], with the real "You receive loot" line and the
//!   skill-up roll and its "Your skill in Mining has increased to 76." line, both in the stock
//!   `UIErrorsFrame`'s info colour.
//! - **Skinning** ([`skin_kills`], host or alone): a beast killed within [`SKIN_RANGE`] yd of a
//!   hero gives that hero leather [`SKIN_CHANCE`]% of the time, an elite, a boss or a great beast
//!   always. P1 skins the kill outright: no corpse is left standing to work.
//! - **Camp fishing** ([`camp_catch`]): the existing bobber's catch also lands the band's fish in
//!   the bags and rolls Fishing, and (P2) the band's cooking fish beside it.
//! - **Kills** (P2, [`kill_yields`], host or alone): beside the skinning, a beast within
//!   [`SKIN_RANGE`] yd of a hero may give that hero its meat and a humanoid its cloth
//!   ([`KILL_DROP_CHANCE`]%, a boss or an elite always), and a boss or an elite may drop a rare
//!   recipe or reagent ([`nodes::rare_drops`]). None of these raises a skill
//!   ([`NO_SKILL_UP`]).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use benilla_ui::script::{ScriptValue, UiScript};
use benilla_world::world_point::{Subject, WorldPoint};

use super::super::campfire::Camp;
use super::super::enemies::{Enemy, Loot};
use super::super::party::{self, HeroState, LocalHero, NetRole};
use super::super::units::{self, Fx};
use super::super::{Rng, Run, RunEntity, Selection};
use super::inventory::Bags;
use super::items;
use super::nodes::{self, Kind, NodeDef, NODES};
use super::skills::{Line, Skills};

/// Most nodes in one run.
pub(crate) const MAX_NODES: u32 = 10;
/// Nodes standing as the run starts.
const FIRST_NODES: u32 = 3;
/// How long a node stands, and a pool floats, in seconds.
pub(crate) const NODE_LIFE: f32 = 90.0;
pub(crate) const POOL_LIFE: f32 = 60.0;
/// A vein or herb sinks this long after its first harvest.
pub(crate) const DEPLETE_AFTER: f32 = 5.0;
/// The most a hit pushes a gather back, and how often.
const PUSHBACK: f32 = 0.25;
const MAX_PUSHBACKS: u8 = 3;
/// Skinning's reach and a horde beast's odds.
pub(crate) const SKIN_RANGE: f32 = 30.0;
pub(crate) const SKIN_CHANCE: usize = 3;
/// A horde kill's odds of meat (a beast) or cloth (a humanoid) for each hero in reach.
pub(crate) const KILL_DROP_CHANCE: usize = 6;
/// A [`YieldMsg`]'s requirement that rolls no skill-up: loot, not a gather.
pub(crate) const NO_SKILL_UP: u16 = u16::MAX;

/// A gatherable node or pool in a run. Replicated, so every machine sees it and its depletion.
#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NetNode {
    /// This run's number for it.
    pub(crate) id: u32,
    /// Its place in [`NODES`].
    pub(crate) def: u16,
    pub(crate) depleted: bool,
}

impl NetNode {
    pub(crate) fn def(&self) -> Option<&'static NodeDef> {
        NODES.get(usize::from(self.def))
    }
}

/// What the simulating machine keeps about a node: who has had it and how long it has left.
#[derive(Component, Default)]
pub(crate) struct NodeLife {
    pub(crate) harvested: Vec<u8>,
    pub(crate) age: f32,
    pub(crate) deplete_in: Option<f32>,
}

/// One hero's yield, rolled where the run is simulated and stored where the hero's player is.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct YieldMsg {
    /// The skill it rolls a skill-up for, and the requirement the roll is against.
    pub(crate) line: Line,
    pub(crate) req: u16,
    /// `(item, count)`.
    pub(crate) items: Vec<(u32, u16)>,
}

/// Yields for this machine's player, from its own gathers, the host, a kill or the camp.
#[derive(Resource, Default)]
pub(crate) struct YieldInbox(pub(crate) Vec<YieldMsg>);

/// A co-op client's finished gathers, `(node id, rank)`, for the host to roll.
#[derive(Resource, Default)]
pub(crate) struct GatherOutbox(pub(crate) Vec<(u32, u16)>);

/// Yields the host rolled for other players, `(player, yield)`, for the co-op layer to send.
#[derive(Resource, Default)]
pub(crate) struct RemoteYields(pub(crate) Vec<(u8, YieldMsg)>);

/// The spawner's run state.
#[derive(Resource, Default)]
pub(crate) struct NodeSpawner {
    next_id: u32,
    spawned: u32,
    wait: f32,
    last_elapsed: f32,
}

/// The local hero's gather in progress, and the nodes it is done with.
#[derive(Resource, Default)]
pub(crate) struct Gathering {
    current: Option<Current>,
    /// Nodes this player harvested, or was told it cannot: never tried again.
    done: Vec<u32>,
    refused: Vec<u32>,
    last_hp: f32,
}

#[derive(Clone, Copy)]
struct Current {
    node: u32,
    entity: Entity,
    line: Line,
    left: f32,
    pushbacks: u8,
}

/// The roll for one hero's harvest of `node`: refused once depleted, once this player has had
/// it, or below its skill. A vein or herb starts sinking at its first harvest, at once when
/// `party` heroes have all had it.
pub(crate) fn harvest(
    node: &mut NetNode,
    life: &mut NodeLife,
    player: u8,
    rank: u16,
    party: usize,
    rng: &mut Rng,
) -> Option<YieldMsg> {
    let def = node.def()?;
    if node.depleted || life.harvested.contains(&player) || rank < def.req {
        return None;
    }
    life.harvested.push(player);
    if def.kind != Kind::Pool {
        if life.harvested.len() >= party.max(1) {
            node.depleted = true;
        } else if life.deplete_in.is_none() {
            life.deplete_in = Some(DEPLETE_AFTER);
        }
    }
    // 1-2, and one more well past the requirement.
    let count = 1 + rng.index(2) as u16 + u16::from(rank >= def.req + 75);
    let mut items = vec![(def.item, count.min(3))];
    if let Some((extra, pct)) = def.extra {
        if rng.index(100) < pct as usize {
            items.push((extra, 1));
        }
    }
    Some(YieldMsg {
        line: def.kind.line(),
        req: def.req,
        items,
    })
}

/// The node a band's spawner picks: the band's nodes this skill can work (or a little above, so
/// one reaches up), a rare one less often; none gatherable at all falls back to the band's
/// easiest of the kind.
fn pick_node(band: &[usize], skills: &Skills, pools: bool, rng: &mut Rng) -> Option<usize> {
    let fits = |i: &usize| {
        let d = &NODES[*i];
        (d.kind == Kind::Pool) == pools && d.req <= skills.rank(d.kind.line()).rank + 25
    };
    let mut weighted: Vec<usize> = Vec::new();
    for &i in band.iter().filter(|i| fits(i)) {
        let w = 8 / NODES[i].rarity.max(1);
        weighted.extend(std::iter::repeat_n(i, w.max(1) as usize));
    }
    if weighted.is_empty() {
        return band
            .iter()
            .copied()
            .filter(|i| (NODES[*i].kind == Kind::Pool) == pools)
            .min_by_key(|i| NODES[*i].req);
    }
    Some(weighted[rng.index(weighted.len())])
}

/// Host or alone: spawn, age and sink the run's nodes.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_nodes(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    selection: Res<Selection>,
    camp: Res<Camp>,
    skills: Res<Skills>,
    mut rng: ResMut<Rng>,
    mut spawner: ResMut<NodeSpawner>,
    heroes: Query<&Transform, With<HeroState>>,
    bosses: Query<&Enemy>,
    mut nodes: Query<(Entity, &mut NetNode, &mut NodeLife)>,
    points: WorldPoint,
) {
    let dt = time.delta_secs();
    if run.elapsed < spawner.last_elapsed {
        // A new run.
        *spawner = NodeSpawner {
            next_id: spawner.next_id,
            ..Default::default()
        };
    }
    spawner.last_elapsed = run.elapsed;
    for (e, mut node, mut life) in &mut nodes {
        life.age += dt;
        if let Some(t) = life.deplete_in.as_mut() {
            *t -= dt;
            if *t <= 0.0 && !node.depleted {
                node.depleted = true;
            }
        }
        let max_age = match node.def().map(|d| d.kind) {
            Some(Kind::Pool) => POOL_LIFE,
            _ => NODE_LIFE,
        };
        // A depleted node sinks a second after the last harvest it allowed.
        let spent = node.depleted && life.deplete_in.is_none_or(|t| t < -1.0);
        if life.age > max_age || spent {
            commands
                .entity(e)
                .remove::<NetNode>()
                .insert(benilla_world::model_fade::DespawnFade::default());
        }
    }
    let map = selection.stage_def();
    let band = nodes::band_nodes(nodes::band_of(map.name));
    if band.is_empty()
        || spawner.spawned >= MAX_NODES
        || camp.active(&run)
        || bosses.iter().any(|e| e.boss && !e.dead)
    {
        return;
    }
    spawner.wait -= dt;
    if spawner.spawned >= FIRST_NODES && spawner.wait > 0.0 {
        return;
    }
    let Some(centre) = centroid(heroes.iter().map(|t| t.translation)) else {
        return;
    };
    let home = units::wow_to_bevy(map.center);
    // Every fourth spawn tries for a pool, which needs water in reach.
    let pools = spawner.spawned % 4 == 3;
    let Some(def_i) = pick_node(band, &skills, pools, &mut rng) else {
        return;
    };
    let Some(pos) = place(centre, home, map.leash, pools, &heroes, &points, &mut rng) else {
        spawner.wait = 5.0;
        return;
    };
    let def = &NODES[def_i];
    spawner.next_id = spawner.next_id.wrapping_add(1);
    spawner.spawned += 1;
    spawner.wait = rng.range(60.0, 90.0);
    debug!(
        "survivors: {} at {pos} (display {}, lock {}, {})",
        def.name, def.display, def.lock, def.model
    );
    let e = units::spawn_object(&mut commands, run.guid(), def.display, def.scale, pos);
    commands.entity(e).insert((
        RunEntity,
        NetNode {
            id: spawner.next_id,
            def: def_i as u16,
            depleted: false,
        },
        NodeLife::default(),
    ));
}

fn centroid(points: impl Iterator<Item = Vec3>) -> Option<Vec3> {
    let (sum, n) = points.fold((Vec3::ZERO, 0.0), |(s, n), p| (s + p, n + 1.0));
    (n > 0.0).then(|| sum / n)
}

/// Where a node stands: 18-45 yd from the party, inside the leash, clear of every hero; a pool on
/// the water's surface, a node on the ground.
fn place(
    centre: Vec3,
    home: Vec3,
    leash: f32,
    pool: bool,
    heroes: &Query<&Transform, With<HeroState>>,
    points: &WorldPoint,
    rng: &mut Rng,
) -> Option<Vec3> {
    for _ in 0..24 {
        let a = rng.range(0.0, std::f32::consts::TAU);
        let r = rng.range(18.0, 45.0);
        let mut p = centre + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
        if units::flat(p - home).length() > leash * 0.9
            || heroes
                .iter()
                .any(|t| units::flat(t.translation - p).length() < 12.0)
        {
            continue;
        }
        let ground = points.terrain_height_under(p);
        if pool {
            let wow = [-p.z, -p.x, p.y];
            let surface = points.water_surface_at(Subject::Player, wow);
            match (surface, ground) {
                (Some(s), Some(g)) if s > g + 0.5 => p.y = s,
                _ => continue,
            }
        } else {
            match ground {
                Some(g) => p.y = g,
                None => continue,
            }
            let wow = [-p.z, -p.x, p.y + 0.5];
            if points.water_surface_at(Subject::Player, wow).is_some() {
                continue;
            }
        }
        return Some(p);
    }
    None
}

/// Every machine, its own hero: start, push back, interrupt and finish a gather on the node it
/// stands on.
#[allow(clippy::too_many_arguments)]
pub(super) fn gather(
    time: Res<Time>,
    skills: Res<Skills>,
    role: Res<NetRole>,
    mut state: ResMut<Gathering>,
    mut ui: ResMut<UiLines>,
    mut outbox: ResMut<GatherOutbox>,
    mut inbox: ResMut<YieldInbox>,
    mut rng: ResMut<Rng>,
    hero: Query<(Entity, &Transform, &HeroState), With<LocalHero>>,
    mut nodes: Query<(Entity, &Transform, &mut NetNode, Option<&mut NodeLife>)>,
    party_size: Query<(), With<HeroState>>,
    mut fx: Fx,
) {
    let Ok((me, at, hero)) = hero.single() else {
        return;
    };
    let hit = hero.hp < state.last_hp;
    state.last_hp = hero.hp;
    let down = hero.hp <= 0.0 || hero.dead_t.is_some();
    if let Some(mut cur) = state.current {
        let live = nodes.get(cur.entity).ok().filter(|(_, t, n, _)| {
            !n.depleted
                && n.id == cur.node
                && n.def().is_some_and(|d| {
                    units::flat(t.translation - at.translation).length() <= d.kind.reach()
                })
        });
        if live.is_none() || down {
            fx.fail(me, cur.line.spell());
            ui.0.push(CastLine::Interrupted);
            state.current = None;
            return;
        }
        if hit && cur.pushbacks < MAX_PUSHBACKS {
            cur.pushbacks += 1;
            cur.left += PUSHBACK;
            ui.0.push(CastLine::Delayed);
        }
        cur.left -= time.delta_secs();
        state.current = Some(cur);
        if cur.left > 0.0 {
            return;
        }
        state.current = None;
        state.done.push(cur.node);
        ui.0.push(CastLine::Stop);
        fx.cast(me, cur.line.spell(), &[cur.entity], None);
        let rank = skills.rank(cur.line).rank;
        if *role == NetRole::Client {
            outbox.0.push((cur.node, rank));
        } else if let Ok((_, _, mut node, Some(mut life))) = nodes.get_mut(cur.entity) {
            let party = party_size.iter().count();
            if let Some(y) = harvest(
                &mut node,
                &mut life,
                party::LOCAL_PLAYER,
                rank,
                party,
                &mut rng,
            ) {
                inbox.0.push(y);
            }
        }
        return;
    }
    if down {
        return;
    }
    // The nearest node in reach this player has not had.
    let near = nodes
        .iter()
        .filter(|(_, _, n, _)| !n.depleted && !state.done.contains(&n.id))
        .filter_map(|(e, t, n, _)| {
            let d = n.def()?;
            let dist = units::flat(t.translation - at.translation).length();
            (dist <= d.kind.reach()).then_some((e, *n, d, dist))
        })
        .min_by(|a, b| a.3.total_cmp(&b.3));
    let Some((entity, node, def, _)) = near else {
        return;
    };
    let line = def.kind.line();
    if skills.rank(line).rank < def.req {
        if !state.refused.contains(&node.id) {
            state.refused.push(node.id);
            ui.0.push(CastLine::Requires(line, def.req));
        }
        return;
    }
    fx.start(me, line.spell());
    ui.0.push(CastLine::Start(line, def.kind.cast_secs()));
    state.current = Some(Current {
        node: node.id,
        entity,
        line,
        left: def.kind.cast_secs(),
        pushbacks: 0,
    });
}

/// What the stock UI is told this frame: the casting bar's events and the info lines.
#[derive(Resource, Default)]
pub(crate) struct UiLines(pub(crate) Vec<CastLine>);

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum CastLine {
    Start(Line, f32),
    /// A craft's cast, by its spell's name.
    Cast(&'static str, f32),
    Delayed,
    Interrupted,
    Stop,
    Requires(Line, u16),
    Loot(u32, u16),
    /// A craft's product: "You create: [Copper Bar]."
    Created(u32, u32),
    SkillUp(Line, u16),
    /// A trainer rank bought: the line and the step.
    Learned(Line, usize),
    /// A recipe learned, by its name: "You have learned how to create a new item: …".
    LearnedRecipe(&'static str),
    /// An enchant put on the worn gear, by its `SpellItemEnchantment` name.
    Enchanted(&'static str),
    Full,
    NotReady,
    /// A refusal: the `GlobalStrings.lua` key and the words if the key is missing.
    Refused(&'static str, &'static str),
}

/// Every machine: this player's yields into the bags, each announced, and the skill-up roll.
pub(super) fn take_yields(
    mut inbox: ResMut<YieldInbox>,
    mut bags: ResMut<Bags>,
    mut skills: ResMut<Skills>,
    mut rng: ResMut<Rng>,
    mut ui: ResMut<UiLines>,
) {
    for y in std::mem::take(&mut inbox.0) {
        apply_yield(&y, &mut bags, &mut skills, &mut rng, &mut ui.0);
    }
}

/// One yield into `bags`, with its lines, and the skill-up roll.
pub(crate) fn apply_yield(
    y: &YieldMsg,
    bags: &mut Bags,
    skills: &mut Skills,
    rng: &mut Rng,
    ui: &mut Vec<CastLine>,
) {
    for &(item, n) in &y.items {
        let left = bags.add(item, u32::from(n));
        let got = u32::from(n) - left;
        if got > 0 {
            ui.push(CastLine::Loot(item, got as u16));
        }
        if left > 0 {
            ui.push(CastLine::Full);
        }
    }
    if let Some(rank) = skills.gathered(y.line, y.req, rng.index(100) as u32) {
        ui.push(CastLine::SkillUp(y.line, rank));
    }
}

/// What one kill gives one hero in reach of it: a beast's skin (a skill-up roll) and its meat, a
/// humanoid's cloth, and a boss's or an elite's rare drops, each rolled on its odds.
pub(in crate::survivors) fn kill_rolls(
    kill: &Loot,
    band: nodes::Band,
    rng: &mut Rng,
) -> Vec<YieldMsg> {
    let mut out = Vec::new();
    let sure = kill.boss || kill.elite || kill.def.big;
    let level = kill.def.level;
    // One roll: a horde kill on its odds, 1-3 sure from a boss, an elite or a great beast.
    let mut roll = |line: Line, req: u16, item: u32, chance: usize| {
        if sure || rng.index(100) < chance {
            let count = if sure { 1 + rng.index(3) as u16 } else { 1 };
            out.push(YieldMsg {
                line,
                req,
                items: vec![(item, count)],
            });
        }
    };
    if nodes::is_beast(kill.def.name) {
        let (skin, req) = nodes::skin(level);
        roll(Line::Skinning, req, skin, SKIN_CHANCE);
        roll(
            Line::Cooking,
            NO_SKILL_UP,
            nodes::meat(level),
            KILL_DROP_CHANCE,
        );
    }
    if nodes::is_humanoid(kill.def.name) {
        roll(
            Line::FirstAid,
            NO_SKILL_UP,
            nodes::cloth(level),
            KILL_DROP_CHANCE,
        );
    }
    if kill.boss || kill.elite {
        let items: Vec<(u32, u16)> = nodes::rare_drops(band)
            .iter()
            // An elite drops a fifth as often as a boss.
            .filter(|&&(_, pct)| rng.index(100) < if kill.boss { pct } else { pct / 5 } as usize)
            .map(|&(item, _)| (item, 1))
            .collect();
        if !items.is_empty() {
            out.push(YieldMsg {
                line: Line::Enchanting,
                req: NO_SKILL_UP,
                items,
            });
        }
    }
    out
}

/// Host or alone: every kill's yields for every hero in reach, into this player's bags or to the
/// player's machine.
pub(super) fn kill_yields(
    mut kills: MessageReader<Loot>,
    role: Res<NetRole>,
    selection: Res<Selection>,
    mut rng: ResMut<Rng>,
    heroes: Query<(&Transform, &HeroState)>,
    mut inbox: ResMut<YieldInbox>,
    mut remote: ResMut<RemoteYields>,
) {
    let band = nodes::band_of(selection.stage_def().name);
    for kill in kills.read() {
        for (t, hero) in &heroes {
            if units::flat(t.translation - kill.pos).length() > SKIN_RANGE {
                continue;
            }
            for y in kill_rolls(kill, band, &mut rng) {
                if hero.player == party::LOCAL_PLAYER {
                    inbox.0.push(y);
                } else if *role == NetRole::Host {
                    remote.0.push((hero.player, y));
                }
            }
        }
    }
}

/// Every machine: the camp bobber's catch for this player also lands the band's fish, and (P2)
/// its cooking fish.
pub(super) fn camp_catch(
    camp: Res<Camp>,
    selection: Res<Selection>,
    mut was: Local<bool>,
    mut inbox: ResMut<YieldInbox>,
) {
    let caught = camp.fished.contains(&party::LOCAL_PLAYER);
    if caught && !*was {
        let band = nodes::band_of(selection.stage_def().name);
        let (fish, req) = nodes::camp_fish(band);
        let mut items = vec![(fish, 1)];
        if let Some(cook) = nodes::cooking_fish(band).filter(|&c| c != fish) {
            items.push((cook, 1));
        }
        inbox.0.push(YieldMsg {
            line: Line::Fishing,
            req,
            items,
        });
    }
    *was = caught;
}

/// The lines into the stock UI: the casting bar's `SPELLCAST_*` events (`CastingBarFrame`) and
/// the info and error lines (`UIErrorsFrame`), in GlobalStrings' own words.
pub(super) fn show_lines(script: Option<NonSendMut<UiScript>>, mut ui: ResMut<UiLines>) {
    let lines = take_lines(&mut ui);
    let Some(mut script) = script else {
        return;
    };
    for line in lines {
        announce(&mut script, &line);
    }
}

/// This frame's lines, taken.
fn take_lines(ui: &mut UiLines) -> Vec<CastLine> {
    std::mem::take(&mut ui.0)
}

/// A GlobalStrings format filled in Lua, `fallback` if the string is missing.
fn format(script: &UiScript, key: &str, args: &str, fallback: String) -> String {
    script
        .eval::<String>(&format!(
            "if {key} then return string.format({key}, {args}) end return nil"
        ))
        .unwrap_or(fallback)
}

/// One line into the stock UI.
pub(crate) fn announce(script: &mut UiScript, line: &CastLine) {
    let lua_str = |s: &str| format!("{s:?}");
    match line {
        CastLine::Start(l, secs) => script.fire_event(
            "SPELLCAST_START",
            vec![
                ScriptValue::Str(l.spell_name().into()),
                ScriptValue::Int((secs * 1000.0) as i64),
            ],
        ),
        CastLine::Cast(name, secs) => script.fire_event(
            "SPELLCAST_START",
            vec![
                ScriptValue::Str((*name).into()),
                ScriptValue::Int((secs * 1000.0) as i64),
            ],
        ),
        CastLine::Delayed => script.fire_event(
            "SPELLCAST_DELAYED",
            vec![ScriptValue::Int((PUSHBACK * 1000.0) as i64)],
        ),
        CastLine::Interrupted => script.fire_event("SPELLCAST_INTERRUPTED", vec![]),
        CastLine::Stop => script.fire_event("SPELLCAST_STOP", vec![]),
        CastLine::Requires(l, req) => {
            let text = format!("Requires {} ({req})", l.name());
            script.fire_event("UI_ERROR_MESSAGE", vec![ScriptValue::Str(text)]);
        }
        CastLine::Full | CastLine::NotReady | CastLine::Refused(..) => {
            let (key, fallback) = match line {
                CastLine::Full => ("ERR_INV_FULL", "Inventory is full."),
                CastLine::Refused(key, fallback) => (*key, *fallback),
                _ => ("ERR_ITEM_COOLDOWN", "Item is not ready yet."),
            };
            let text = script
                .lua()
                .globals()
                .get::<String>(key)
                .unwrap_or_else(|_| fallback.to_string());
            script.fire_event("UI_ERROR_MESSAGE", vec![ScriptValue::Str(text)]);
        }
        CastLine::Loot(item, n) => {
            let Some(def) = items::template(*item) else {
                return;
            };
            let link = items::link(def);
            let text = if *n > 1 {
                format(
                    script,
                    "LOOT_ITEM_SELF_MULTIPLE",
                    &format!("{}, {n}", lua_str(&link)),
                    format!("You receive loot: {link}x{n}."),
                )
            } else {
                format(
                    script,
                    "LOOT_ITEM_SELF",
                    &lua_str(&link),
                    format!("You receive loot: {link}."),
                )
            };
            script.fire_event("UI_INFO_MESSAGE", vec![ScriptValue::Str(text)]);
        }
        CastLine::Created(item, n) => {
            let Some(def) = items::template(*item) else {
                return;
            };
            let link = items::link(def);
            let text = if *n > 1 {
                format(
                    script,
                    "LOOT_ITEM_CREATED_SELF_MULTIPLE",
                    &format!("{}, {n}", lua_str(&link)),
                    format!("You create: {link}x{n}."),
                )
            } else {
                format(
                    script,
                    "LOOT_ITEM_CREATED_SELF",
                    &lua_str(&link),
                    format!("You create: {link}."),
                )
            };
            script.fire_event("UI_INFO_MESSAGE", vec![ScriptValue::Str(text)]);
        }
        CastLine::Learned(l, step) => {
            let rank = super::skills::TRAINER_RANKS
                .get(*step)
                .map_or("", |r| r.name);
            let spell = format!("{} ({rank})", l.spell_name());
            let text = format(
                script,
                "ERR_LEARN_SPELL_S",
                &lua_str(&spell),
                format!("You have learned a new spell: {spell}."),
            );
            script.fire_event("UI_INFO_MESSAGE", vec![ScriptValue::Str(text)]);
        }
        CastLine::LearnedRecipe(name) => {
            let text = format(
                script,
                "ERR_LEARN_RECIPE_S",
                &lua_str(name),
                format!("You have learned how to create a new item: {name}."),
            );
            script.fire_event("UI_INFO_MESSAGE", vec![ScriptValue::Str(text)]);
        }
        CastLine::Enchanted(name) => {
            let text = format!("{name} is on your gear: it takes hold from your next run.");
            script.fire_event("UI_INFO_MESSAGE", vec![ScriptValue::Str(text)]);
        }
        CastLine::SkillUp(l, rank) => {
            let text = format(
                script,
                "SKILL_RANK_UP",
                &format!("{}, {rank}", lua_str(l.name())),
                format!("Your skill in {} has increased to {rank}.", l.name()),
            );
            script.fire_event("UI_INFO_MESSAGE", vec![ScriptValue::Str(text)]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copper() -> NetNode {
        NetNode {
            id: 1,
            def: 0,
            depleted: false,
        }
    }

    /// A vein is the party's: each hero gets their own yield once, it sinks when everyone has
    /// had it, and a repeat or a third harvest after that is refused.
    #[test]
    fn survivors_gather_each_hero_gets_their_own_yield_once() {
        let mut rng = Rng(5);
        let mut node = copper();
        let mut life = NodeLife::default();
        let a = harvest(&mut node, &mut life, 0, 1, 2, &mut rng).expect("the host's");
        assert_eq!(a.line, Line::Mining);
        assert_eq!(a.items[0].0, items::COPPER_ORE);
        assert!((1..=3).contains(&a.items[0].1));
        assert_eq!(
            life.deplete_in,
            Some(DEPLETE_AFTER),
            "the first harvest starts the sink"
        );
        assert!(
            harvest(&mut node, &mut life, 0, 1, 2, &mut rng).is_none(),
            "once each"
        );
        assert!(
            harvest(&mut node, &mut life, 1, 1, 2, &mut rng).is_some(),
            "the friend's own"
        );
        assert!(node.depleted, "everyone has had it");
        assert!(
            harvest(&mut node, &mut life, 2, 1, 3, &mut rng).is_none(),
            "depleted"
        );
    }

    #[test]
    fn survivors_gather_too_low_a_skill_is_refused() {
        let mut node = NetNode {
            def: 4, // Tin, 65
            ..copper()
        };
        let mut life = NodeLife::default();
        assert!(harvest(&mut node, &mut life, 0, 64, 1, &mut Rng(1)).is_none());
        assert!(life.harvested.is_empty());
        assert!(harvest(&mut node, &mut life, 0, 65, 1, &mut Rng(1)).is_some());
    }

    #[test]
    fn survivors_gather_a_pool_serves_each_player_without_sinking() {
        let mut pool = NetNode { def: 3, ..copper() };
        let mut life = NodeLife::default();
        let y = harvest(&mut pool, &mut life, 0, 30, 1, &mut Rng(2)).unwrap();
        assert_eq!(y.line, Line::Fishing);
        assert!(!pool.depleted && life.deplete_in.is_none());
        assert!(harvest(&mut pool, &mut life, 0, 30, 1, &mut Rng(2)).is_none());
    }

    #[test]
    fn survivors_gather_yields_fill_the_bags_and_raise_the_skill() {
        let mut bags = Bags::default();
        let mut skills = Skills::default();
        let mut lines = Vec::new();
        let y = YieldMsg {
            line: Line::Herbalism,
            req: 1,
            items: vec![(items::PEACEBLOOM, 2)],
        };
        apply_yield(&y, &mut bags, &mut skills, &mut Rng(9), &mut lines);
        assert_eq!(
            bags.get(0, 1).map(|s| (s.item, s.count)),
            Some((items::PEACEBLOOM, 2))
        );
        assert_eq!(skills.rank(Line::Herbalism).rank, 2, "orange: always");
        assert_eq!(
            lines,
            vec![
                CastLine::Loot(items::PEACEBLOOM, 2),
                CastLine::SkillUp(Line::Herbalism, 2)
            ]
        );
    }

    #[test]
    fn survivors_gather_node_picks_follow_band_and_skill() {
        let skills = Skills::default();
        let band = nodes::band_nodes(nodes::Band::B30);
        let mut rng = Rng(3);
        for _ in 0..50 {
            // A rank-1 gatherer in a 30-45 zone gets the band's easiest, never a Purple Lotus.
            let i = pick_node(band, &skills, false, &mut rng).unwrap();
            assert!(NODES[i].req <= 125, "{}", NODES[i].name);
        }
        let pool = pick_node(band, &skills, true, &mut rng).unwrap();
        assert_eq!(NODES[pool].kind, Kind::Pool);
    }

    /// A boss beast always gives its skin and meat, a humanoid its cloth now and then, and loot
    /// never raises a skill.
    #[test]
    fn survivors_gather_kills_give_skins_meat_cloth_and_drops() {
        use super::super::super::data::EnemyDef;
        static WOLF: EnemyDef = EnemyDef {
            name: "Mangy Wolf",
            display: 0,
            hp: 1.0,
            speed: 1.0,
            damage: 1.0,
            scale: 1.0,
            xp: 1,
            level: 5,
            big: false,
        };
        static THUG: EnemyDef = EnemyDef {
            name: "Defias Thug",
            level: 18,
            ..WOLF
        };
        let kill = |def: &'static EnemyDef, boss: bool| Loot {
            pos: Vec3::ZERO,
            xp: 1,
            boss,
            elite: false,
            def,
        };
        let mut rng = Rng(7);
        let got = kill_rolls(&kill(&WOLF, true), nodes::Band::B55, &mut rng);
        assert!(got
            .iter()
            .any(|y| y.line == Line::Skinning && y.items[0].0 == items::RUINED_LEATHER_SCRAPS));
        assert!(got.iter().any(|y| y.line == Line::Cooking
            && y.items[0].0 == items::STRINGY_WOLF_MEAT
            && y.req == NO_SKILL_UP));
        let mut cloth = 0;
        for _ in 0..400 {
            for y in kill_rolls(&kill(&THUG, false), nodes::Band::B1, &mut rng) {
                assert_eq!((y.line, y.items[0].0), (Line::FirstAid, items::WOOL_CLOTH));
                cloth += 1;
            }
        }
        assert!((8..=48).contains(&cloth), "about 6% of 400: {cloth}");
        // Loot rolls no skill-up.
        let mut bags = Bags::default();
        let mut skills = Skills::default();
        let mut lines = Vec::new();
        let meat = YieldMsg {
            line: Line::Cooking,
            req: NO_SKILL_UP,
            items: vec![(items::STRINGY_WOLF_MEAT, 1)],
        };
        apply_yield(&meat, &mut bags, &mut skills, &mut Rng(1), &mut lines);
        assert_eq!(bags.count(items::STRINGY_WOLF_MEAT), 1);
        assert_eq!(skills.rank(Line::Cooking).rank, 1);
    }

    /// The lines reach the stock UI in GlobalStrings' words.
    #[test]
    fn survivors_gather_lines_use_the_stock_strings() {
        let mut s = UiScript::new().unwrap();
        s.run(
            "LOOT_ITEM_SELF_MULTIPLE = 'You receive loot: %sx%d.' \
             SKILL_RANK_UP = 'Your skill in %s has increased to %d.' \
             SEEN = {} \
             local f = CreateFrame('Frame') \
             f:RegisterEvent('UI_INFO_MESSAGE') f:RegisterEvent('SPELLCAST_START') \
             f:SetScript('OnEvent', function() table.insert(SEEN, event .. ':' .. tostring(arg1)) end)",
        )
        .unwrap();
        announce(&mut s, &CastLine::Loot(items::COPPER_ORE, 2));
        announce(&mut s, &CastLine::SkillUp(Line::Mining, 76));
        announce(&mut s, &CastLine::Start(Line::Mining, 1.5));
        announce(&mut s, &CastLine::Cast("Smelt Copper", 3.0));
        announce(&mut s, &CastLine::Created(items::COPPER_BAR, 1));
        announce(&mut s, &CastLine::Learned(Line::Mining, 1));
        let seen: Vec<String> = s.eval("return SEEN").unwrap();
        assert_eq!(
            seen,
            vec![
                "UI_INFO_MESSAGE:You receive loot: |cffffffff|Hitem:2770:0:0:0|h[Copper Ore]|h|rx2."
                    .to_string(),
                "UI_INFO_MESSAGE:Your skill in Mining has increased to 76.".to_string(),
                "SPELLCAST_START:Mining".to_string(),
                "SPELLCAST_START:Smelt Copper".to_string(),
                "UI_INFO_MESSAGE:You create: |cffffffff|Hitem:2840:0:0:0|h[Copper Bar]|h|r."
                    .to_string(),
                "UI_INFO_MESSAGE:You have learned a new spell: Mining (Journeyman).".to_string(),
            ]
        );
    }
}
