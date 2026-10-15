//! The horde: spawning on a ring outside the view, the chase, contact damage, death, and the
//! experience gems and coins the dead leave.

use std::collections::HashMap;

use benilla_world::rig_anim::GlobalSeqDrive;
use benilla_world::world_point::WorldPoint;
use bevy::prelude::*;

use super::abilities::Hit;
use super::data::{self, EnemyDef};
use super::hero::{self, Hero, HERO_RADIUS};
use super::party;
use super::run_options::{self, RunOptions, XpCarry};
use super::treasure;
use super::units::{self, flat, yaw_toward, Fx, Marks, UnitSpec, COLOR_HURT};
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
    /// The hero it must go for until the run second given, whatever else is near: a taunt's
    /// seam ([`party::pick_target`]); nothing taunts yet.
    pub taunted: Option<(Entity, f32)>,
}

pub(super) struct Dot {
    pub dps: f32,
    pub left: f32,
    pub tick: f32,
    /// The hero whose spell it is, credited with its ticks.
    pub source: Entity,
}

/// A corpse playing its death before it fades.
#[derive(Component)]
pub(super) struct Dying(f32);

#[derive(Resource, Default)]
pub(super) struct Spawner {
    timer: f32,
    /// What the Run Options density split owes the next spawns.
    pub xp_carry: XpCarry,
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
    Gold(u32),
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
        GemKind::Gold(gold) => treasure::coin_look(gold),
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
) {
    // The Run Options first, then the run clock's growth, then an elite's own multipliers. Only
    // the horde splits its experience under density: density adds no bosses, elites or guardians.
    let tuned = run_options::scaled(def, opts, rank != Rank::Horde, xp_carry);
    let body = ranked_body(
        treasure::Body {
            hp: tuned.hp * data::hp_scale(run.elapsed),
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
            xp: tuned.xp,
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
        },
        marks,
    ));
}

pub(super) fn ground(world: &WorldPoint, mut p: Vec3, fallback: f32) -> Vec3 {
    p.y = world.terrain_height_under(p).unwrap_or(fallback);
    p
}

/// Keep the horde topped up on a ring outside the view, round the heroes still standing, with the
/// bosses and the encircling waves on the clock.
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
    world: WorldPoint,
) {
    let standing = heroes.iter().filter(|(_, h)| h.alive());
    let Some(hpos) = party::centre(standing.map(|(tf, _)| tf.translation)) else {
        return;
    };
    let elapsed = run.elapsed;
    let roster = data::roster(selection.map);
    let pool = data::wave_pool(elapsed, roster.enemies.len());

    let due_bosses = ((elapsed / data::BOSS_EVERY_SECS) as u32).min(roster.bosses.len() as u32);
    while run.bosses < due_bosses {
        let def = &roster.bosses[run.bosses as usize];
        let a = rng.range(0.0, std::f32::consts::TAU);
        let pos = ground(
            &world,
            hpos + Vec3::new(a.cos(), 0.0, a.sin()) * 30.0,
            hpos.y,
        );
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
        run.bosses += 1;
        run.banner = Some((format!("{} approaches!", def.name), 4.0));
    }

    // Half way between bosses, a closing ring of the moment's weakest.
    let due_rings = ((elapsed + data::BOSS_EVERY_SECS / 2.0) / data::BOSS_EVERY_SECS) as u32;
    if run.rings < due_rings {
        run.rings = due_rings;
        let def = &roster.enemies[pool[0]];
        let n = opts.crowd(28 + (data::progress(elapsed) * 60.0) as usize);
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            let pos = ground(
                &world,
                hpos + Vec3::new(a.cos(), 0.0, a.sin()) * 26.0,
                hpos.y,
            );
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
    let target = opts.crowd(data::target_alive(elapsed));
    if alive >= target {
        return;
    }
    let n = (((target - alive) as f32) / 6.0)
        .ceil()
        .min(4.0 * opts.density) as usize;
    for _ in 0..n {
        let def = &roster.enemies[pool[rng.index(pool.len())]];
        let a = rng.range(0.0, std::f32::consts::TAU);
        let r = rng.range(34.0, 42.0);
        let pos = ground(&world, hpos + Vec3::new(a.cos(), 0.0, a.sin()) * r, hpos.y);
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
    mut run: ResMut<Run>,
    selection: Res<Selection>,
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
    let hero_down = seen.iter().all(|h| !h.up);
    let wake = treasure::wake_radius(selection.stage_def().leash);

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
            let back = hpos + to_hero.normalize_or_zero() * 38.0;
            tf.translation = ground(&world, back, hpos.y);
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
        next = ground(&world, next, pos.y);
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

/// Most gems on the ground at once; past it, a kill's worth joins an existing gem.
const MAX_GEMS: usize = 350;

pub(super) fn drop_loot(
    mut commands: Commands,
    mut drops: MessageReader<Loot>,
    mut rng: ResMut<Rng>,
    mut run: ResMut<Run>,
    mut gems: Query<&mut Gem>,
    local: Query<&HeroState, With<LocalHero>>,
) {
    let mut count = gems.iter().count();
    // Coins are worth what the local player's gold multiplier makes them: the purse is shared.
    let gold = local
        .single()
        .map_or_else(|_| HeroState::default().stats(), HeroState::stats)
        .gold;
    for d in drops.read() {
        if d.boss || rng.f32() < 0.015 {
            let heal = GemKind::Heal(if d.boss { 1000.0 } else { 30.0 });
            spawn_gem(&mut commands, run.guid(), d.pos + Vec3::X * 0.8, heal);
        }
        if let Some(coins) = treasure::purse(d.boss, d.elite, run.elapsed, gold, rng.f32()) {
            spawn_gem(
                &mut commands,
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
        if count >= MAX_GEMS && !d.boss {
            if let Some(mut g) = gems.iter_mut().find(|g| matches!(g.kind, GemKind::Xp(_))) {
                if let GemKind::Xp(xp) = &mut g.kind {
                    *xp += d.xp;
                }
                continue;
            }
        }
        spawn_gem(&mut commands, run.guid(), d.pos, GemKind::Xp(d.xp));
        count += 1;
    }
}

pub(super) fn spawn_gem(commands: &mut Commands, serial: u64, pos: Vec3, kind: GemKind) {
    let (display, scale) = look_for(kind);
    let e = units::spawn_object(commands, serial, display, scale, pos + Vec3::Y * 0.5);
    commands.entity(e).insert(Gem {
        kind,
        pulled: None,
        speed: 6.0,
    });
}

/// Gems in a magnet's reach fly to the nearest hero standing whose magnet reaches them; experience
/// (under the picker's growth) and gold go to the party's pool, healing to the picker, and levels
/// bank as they land.
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
    // Each hero standing: where gems fly to, its magnet, and its growth.
    let standing: Vec<(Entity, Vec3, f32, f32)> = heroes
        .iter()
        .filter(|(_, _, h, _)| h.alive())
        .map(|(e, tf, h, _)| {
            let s = h.stats();
            (e, tf.translation + Vec3::Y * 1.0, s.magnet, s.growth)
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
                GemKind::Gold(gold) => run.gold += gold,
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
        let past = gem_at(&mut world, Vec3::new(4.8, 0.0, 0.0), GemKind::Gold(1));
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
            gem_at(&mut world, on(0.0), GemKind::Gold(7)),
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
