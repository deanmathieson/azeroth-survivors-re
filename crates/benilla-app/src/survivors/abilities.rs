//! Auto-cast: every hero's owned abilities fire on their own cooldowns, from that hero, at the
//! targets their kind picks, with the spell's own visuals; damage lands when its missile would and
//! is credited to the hero who cast it.

use bevy::prelude::*;

use super::bonus::School;
use super::data::{self, AbilityDef, Effect, Kind};
use super::enemies::{self, Enemy, Loot};
use super::hero::{self, Hero};
use super::mechanics::{Armed, Proc, ProcKind, Procs};
use super::party;
use super::synergy;
use super::units::{self, flat, Fx, Marks, UnitSpec, COLOR_HEAL, COLOR_MELEE, COLOR_SPELL};
use super::{HeroState, Rng, Run, RunEntity, Stats};
use crate::net::{FieldChanged, Guid, ObjectStore};

/// One blow landing on one enemy.
#[derive(Message, Clone)]
pub(super) struct Hit {
    pub target: Entity,
    pub damage: f32,
    pub dot: f32,
    pub dot_dur: f32,
    pub effect: Effect,
    pub mark: u32,
    pub mark_dur: f32,
    /// Also hits everything this close to the target, for less.
    pub splash: f32,
    pub crit: bool,
    pub color: u32,
    /// The hero it comes from: its damage, lifesteal and drain are that hero's.
    pub source: Entity,
    /// What dealt it, for the mechanics it may set off.
    pub origin: Origin,
    /// Its spell school, where known: the party's shared debuffs on the target that raise one
    /// school's damage taken reach only blows of it ([`super::synergy`]).
    pub school: Option<School>,
}

/// What dealt a blow. Only an ability's own blows set off its hero's mechanics
/// ([`super::mechanics`]): a splash, a mechanic's blow or anything else sets off nothing, so
/// procs never cascade.
#[derive(Clone, Copy)]
pub(super) enum Origin {
    Ability(&'static AbilityDef),
    Mechanic,
    Other,
}

impl Hit {
    pub fn direct(source: Entity, target: Entity, damage: f32, crit: bool) -> Self {
        Self {
            target,
            damage,
            dot: 0.0,
            dot_dur: 0.0,
            effect: Effect::None,
            mark: 0,
            mark_dur: 0.0,
            splash: 0.0,
            crit,
            color: COLOR_MELEE,
            source,
            origin: Origin::Other,
            school: None,
        }
    }
}

/// Hits in flight, by the run time they land.
#[derive(Resource, Default)]
pub(super) struct PendingHits(pub Vec<(f32, Hit)>);

/// A ground spell's area, ticking on whatever stands in it.
#[derive(Component)]
pub(super) struct GroundArea {
    damage: f32,
    dot: f32,
    dot_dur: f32,
    radius: f32,
    left: f32,
    every: f32,
    next_tick: f32,
    effect: Effect,
    mark: u32,
    /// The spell whose impact plays on each enemy a tick hits, 0 for none.
    impact: u32,
    /// The hero who cast it, whose crits and credit its ticks are.
    owner: Entity,
    origin: Origin,
    school: Option<School>,
}

impl GroundArea {
    /// Ground a mechanic leaves burning: `dps` a second of `school` to whatever stands within
    /// `radius`, for `secs`, the first tick at once.
    pub(super) fn burning(owner: Entity, dps: f32, radius: f32, secs: f32, school: School) -> Self {
        Self {
            damage: dps,
            dot: 0.0,
            dot_dur: 0.0,
            radius,
            left: secs,
            every: 1.0,
            next_tick: 0.0,
            effect: Effect::None,
            mark: 0,
            impact: 0,
            owner,
            origin: Origin::Mechanic,
            school: Some(school),
        }
    }
}

#[derive(Component)]
pub(super) struct Totem {
    cast: u32,
    damage: f32,
    range: f32,
    radius: f32,
    left: f32,
    tick: f32,
    pulse: bool,
    /// The hero who set it down, whose crits and credit its blows are.
    owner: Entity,
    ability: &'static AbilityDef,
    school: School,
}

#[derive(Clone, Copy)]
struct Foe {
    e: Entity,
    pos: Vec3,
    hp: f32,
    dist: f32,
    radius: f32,
}

/// A heal or shield a hero's spell puts on [`party::support_target`]'s pick, landed once every hero
/// has cast.
enum Support {
    Shield { amount: f32, secs: f32 },
    HealOverTime { per_sec: f32, secs: f32 },
}

/// The numbers one cast uses, from the ability's level and the hero's stats.
struct Params {
    dmg: f32,
    dot: f32,
    count: usize,
    radius: f32,
    duration: f32,
    cooldown: f32,
    /// The spell's own school.
    school: School,
}

fn mark_secs(def: &AbilityDef, duration: f32) -> f32 {
    match def.effect {
        Effect::Slow(_, t) | Effect::Weaken(_, t) => t,
        Effect::Root(t) | Effect::Stun(t) | Effect::Fear(t) => t,
        _ if duration > 0.0 => duration,
        _ => 3.0,
    }
}

fn hit_for(
    def: &'static AbilityDef,
    p: &Params,
    source: Entity,
    target: Entity,
    damage: f32,
    crit: bool,
) -> Hit {
    Hit {
        target,
        damage,
        dot: p.dot,
        dot_dur: p.duration,
        effect: def.effect,
        mark: def.mark,
        mark_dur: mark_secs(def, p.duration),
        splash: if matches!(def.kind, Kind::Bolt | Kind::Strike) {
            p.radius
        } else {
            0.0
        },
        crit,
        color: if def.kind == Kind::Melee {
            COLOR_MELEE
        } else {
            COLOR_SPELL
        },
        source,
        origin: Origin::Ability(def),
        school: Some(p.school),
    }
}

fn schedule(
    pending: &mut PendingHits,
    hits: &mut MessageWriter<Hit>,
    now: f32,
    delay: f32,
    hit: Hit,
) {
    if delay <= 0.0 {
        hits.write(hit);
    } else {
        pending.0.push((now + delay, hit));
    }
}

pub(super) fn roll(rng: &mut Rng, stats: &Stats, dmg: f32) -> (f32, bool) {
    if rng.f32() < stats.crit {
        (dmg * stats.crit_damage, true)
    } else {
        (dmg, false)
    }
}

/// `Spell.dbc`'s `School` index (0 physical to 6 arcane) as a [`School`]; anything else is
/// physical.
fn dbc_school(index: u32) -> School {
    match index {
        1 => School::Holy,
        2 => School::Fire,
        3 => School::Nature,
        4 => School::Frost,
        5 => School::Shadow,
        6 => School::Arcane,
        _ => School::Physical,
    }
}

fn nearest(foes: &[Foe], range: f32, n: usize) -> Vec<Foe> {
    foes.iter()
        .filter(|f| f.dist <= range + f.radius)
        .take(n)
        .copied()
        .collect()
}

fn toughest(foes: &[Foe], range: f32, n: usize) -> Vec<Foe> {
    let mut v: Vec<Foe> = foes
        .iter()
        .filter(|f| f.dist <= range + f.radius)
        .copied()
        .collect();
    v.sort_by(|a, b| b.hp.total_cmp(&a.hp));
    v.truncate(n);
    v
}

fn around(foes: &[Foe], center: Vec3, r: f32) -> Vec<Foe> {
    foes.iter()
        .filter(|f| flat(f.pos - center).length() <= r + f.radius * 0.5)
        .copied()
        .collect()
}

/// The point among the nearest enemies in range with the most others within `r` of it.
fn densest(foes: &[Foe], range: f32, r: f32) -> Option<Vec3> {
    let near: Vec<&Foe> = foes.iter().filter(|f| f.dist <= range).take(40).collect();
    near.iter()
        .map(|f| {
            (
                f.pos,
                near.iter()
                    .filter(|o| flat(o.pos - f.pos).length() <= r)
                    .count(),
            )
        })
        .max_by_key(|&(_, n)| n)
        .map(|(p, _)| p)
}

fn entities(foes: &[Foe]) -> Vec<Entity> {
    foes.iter().map(|f| f.e).collect()
}

/// Every hero standing casts whatever of its own is off cooldown, from where it stands, under its
/// own stats.
pub(super) fn cast_abilities(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    spells: Option<Res<crate::ui_action::Spells>>,
    mut heroes: Query<(Entity, &Transform, &mut HeroState), With<Hero>>,
    enemies: Query<(Entity, &Transform, &Enemy)>,
    mut fx: Fx,
    mut pending: ResMut<PendingHits>,
    mut hits: MessageWriter<Hit>,
    mut procs: ResMut<Procs>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    // Each hero's health and max health as the frame began, for whom a heal is wanted.
    let healths: Vec<(Entity, f32, f32)> = heroes
        .iter()
        .map(|(e, _, h)| (e, h.hp, h.stats().max_hp))
        .collect();
    let mut supports = Vec::new();
    for (me, htf, mut hero) in &mut heroes {
        // Feared or mind-controlled by a raid boss, the auto-casts wait ([`super::raid`]).
        if hero.alive() && hero.raid.can_cast() {
            cast_for(
                me,
                htf.translation,
                &mut hero,
                dt,
                &healths,
                &mut supports,
                &mut commands,
                &mut run,
                &mut rng,
                spells.as_deref(),
                &enemies,
                &mut fx,
                &mut pending,
                &mut hits,
                &mut procs,
            );
        }
    }
    for (target, support) in supports {
        let Ok((.., mut hero)) = heroes.get_mut(target) else {
            continue;
        };
        match support {
            Support::Shield { amount, secs } => party::shield(&mut hero, amount, secs),
            Support::HealOverTime { per_sec, secs } => {
                party::heal_over_time(&mut hero, per_sec, secs);
            }
        }
    }
}

/// One hero's casts this frame: `me` at `hpos`, with its own abilities and stats. Its heals and
/// shields join `supports` for whom they land on; `healths` is every hero's health and max health.
/// An echo a mechanic queued casts at once, cooldown or not; every other cast is recorded in
/// `procs` for the hero's mechanics. A transformed ability flies its legendary's missile.
fn cast_for(
    me: Entity,
    hpos: Vec3,
    hero: &mut HeroState,
    dt: f32,
    healths: &[(Entity, f32, f32)],
    supports: &mut Vec<(Entity, Support)>,
    commands: &mut Commands,
    run: &mut Run,
    rng: &mut Rng,
    spells: Option<&crate::ui_action::Spells>,
    enemies: &Query<(Entity, &Transform, &Enemy)>,
    fx: &mut Fx,
    pending: &mut PendingHits,
    hits: &mut MessageWriter<Hit>,
    procs: &mut Procs,
) {
    let base_stats = hero.stats();
    let now = run.elapsed;
    // A raid curse slows the cooldowns' turn ([`super::raid::HeroRaid::cooldown_rate`]).
    let rate = hero.raid.cooldown_rate();
    let mut foes: Vec<Foe> = enemies
        .iter()
        .filter(|(_, _, e)| !e.dead)
        .map(|(e, tf, en)| Foe {
            e,
            pos: tf.translation,
            hp: en.hp,
            dist: hero::reach(hpos, tf.translation),
            radius: en.radius,
        })
        .collect();
    foes.sort_by(|a, b| a.dist.total_cmp(&b.dist));
    let speed_of = |spell: u32| {
        spells
            .and_then(|s| s.catalog.get(spell))
            .map_or(0.0, |d| d.speed)
    };
    // The spell's own school, so a school's damage bonus lands where its spells do.
    let school_of = |spell: u32| {
        spells
            .and_then(|s| s.catalog.get(spell))
            .map_or(School::Physical, |d| dbc_school(d.school))
    };

    for i in 0..hero.abilities.len() {
        let def = hero.abilities[i].def;
        // An echo leaves the cooldown to run as it was.
        let echo = hero.mechanics.take_echo(def.key, now);
        hero.abilities[i].cd -= dt * rate;
        if hero.abilities[i].cd > 0.0 && !echo {
            continue;
        }
        // A next-cast guarantee (Presence of Mind, Clearcasting) waits for a cast of its own.
        let armed = if echo { None } else { hero.mechanics.armed() };
        let stats = empowered(base_stats, armed);
        let o = &hero.abilities[i];
        let look = look(def, o.level, &speed_of);
        let school = school_of(def.spell);
        let mult = o.damage_mult() * stats.might * hero.school_mult(school);
        let p = Params {
            dmg: def.damage * mult,
            dot: def.dot * mult,
            count: o.count(&stats) as usize,
            radius: o.radius(&stats),
            duration: o.duration(),
            cooldown: o.cooldown(&stats),
            school,
        };
        let speed = speed_of(look);
        let fired = match def.kind {
            Kind::Bolt | Kind::Strike | Kind::Melee => {
                let reach = if def.kind == Kind::Melee {
                    def.range + 0.5
                } else {
                    def.range
                };
                let targets = if matches!(
                    def.key,
                    "pyroblast" | "aimed_shot" | "exorcism" | "starfire"
                ) {
                    toughest(&foes, reach, p.count)
                } else {
                    nearest(&foes, reach, p.count)
                };
                if targets.is_empty() {
                    false
                } else {
                    if def.kind == Kind::Melee {
                        fx.swing(me, targets[0].e, p.dmg);
                        for f in &targets {
                            fx.impact(f.e, def.spell);
                        }
                    } else {
                        fx.cast(me, look, &entities(&targets), None);
                    }
                    for f in &targets {
                        let (dmg, crit) = roll(rng, &stats, p.dmg);
                        let delay = if speed > 0.0 { f.dist / speed } else { 0.0 };
                        schedule(
                            pending,
                            hits,
                            now,
                            delay,
                            hit_for(def, &p, me, f.e, dmg, crit),
                        );
                    }
                    true
                }
            }
            Kind::Nova | Kind::Cone => {
                let mut targets = around(&foes, hpos, p.radius);
                if def.kind == Kind::Cone {
                    if let Some(first) = targets.first().copied() {
                        let aim = flat(first.pos - hpos).normalize_or_zero();
                        targets.retain(|f| {
                            let d = flat(f.pos - hpos);
                            d.length() < 2.0 || d.normalize_or_zero().dot(aim) >= 0.8
                        });
                    }
                }
                if targets.is_empty() {
                    false
                } else {
                    fx.cast(me, def.spell, &entities(&targets), None);
                    for f in &targets {
                        let (dmg, crit) = roll(rng, &stats, p.dmg);
                        let delay = if speed > 0.0 { f.dist / speed } else { 0.0 };
                        schedule(
                            pending,
                            hits,
                            now,
                            delay,
                            hit_for(def, &p, me, f.e, dmg, crit),
                        );
                    }
                    true
                }
            }
            Kind::Ground | Kind::GroundSelf => {
                let center = if def.kind == Kind::GroundSelf {
                    foes.first()
                        .filter(|f| f.dist <= p.radius + 4.0)
                        .map(|_| hpos)
                } else {
                    densest(&foes, def.range, p.radius)
                };
                match center {
                    None => false,
                    Some(center) => {
                        fx.cast(me, def.spell, &[], Some(center));
                        let area = if def.overhead {
                            commands
                                .spawn((Transform::from_translation(center), RunEntity))
                                .id()
                        } else {
                            let serial = run.guid();
                            units::spawn_area_object(commands, serial, def.spell, p.radius, center)
                        };
                        let burst = def.duration <= 0.0;
                        commands.entity(area).insert(GroundArea {
                            damage: p.dmg,
                            dot: p.dot,
                            dot_dur: if burst { 5.0 } else { 0.0 },
                            radius: p.radius,
                            left: if burst { 1.5 } else { p.duration },
                            every: if burst { f32::INFINITY } else { 1.0 },
                            next_tick: if burst { 0.0 } else { 0.4 },
                            effect: def.effect,
                            mark: def.mark,
                            impact: if def.overhead { def.spell } else { 0 },
                            owner: me,
                            origin: Origin::Ability(def),
                            school: Some(p.school),
                        });
                        true
                    }
                }
            }
            Kind::Chain => match nearest(&foes, def.range, 1).first().copied() {
                None => false,
                Some(first) => {
                    let mut chain = vec![first];
                    while chain.len() < p.count + 1 {
                        let last = chain[chain.len() - 1].pos;
                        let next = foes
                            .iter()
                            .filter(|f| !chain.iter().any(|c| c.e == f.e))
                            .filter(|f| flat(f.pos - last).length() <= p.radius)
                            .min_by(|a, b| {
                                flat(a.pos - last)
                                    .length()
                                    .total_cmp(&flat(b.pos - last).length())
                            });
                        match next {
                            Some(f) => chain.push(*f),
                            None => break,
                        }
                    }
                    fx.cast(me, def.spell, &entities(&chain), None);
                    for (k, f) in chain.iter().enumerate() {
                        let (dmg, crit) = roll(rng, &stats, p.dmg * 0.85f32.powi(k as i32));
                        schedule(
                            pending,
                            hits,
                            now,
                            0.08 * k as f32,
                            hit_for(def, &p, me, f.e, dmg, crit),
                        );
                    }
                    true
                }
            },
            Kind::Dot => {
                let mut pool: Vec<Foe> = foes
                    .iter()
                    .filter(|f| f.dist <= def.range)
                    .take(p.count * 3)
                    .copied()
                    .collect();
                if pool.is_empty() {
                    false
                } else {
                    let mut targets = Vec::with_capacity(p.count);
                    while targets.len() < p.count && !pool.is_empty() {
                        targets.push(pool.swap_remove(rng.index(pool.len())));
                    }
                    fx.cast(me, def.spell, &entities(&targets), None);
                    for f in &targets {
                        let delay = if speed > 0.0 { f.dist / speed } else { 0.0 };
                        schedule(
                            pending,
                            hits,
                            now,
                            delay,
                            hit_for(def, &p, me, f.e, 0.0, false),
                        );
                    }
                    true
                }
            }
            Kind::Aura => {
                let targets = around(&foes, hpos, p.radius);
                if targets.is_empty() {
                    false
                } else {
                    for f in targets.iter().take(6) {
                        fx.impact(f.e, def.spell);
                    }
                    for f in &targets {
                        let (dmg, crit) = roll(rng, &stats, p.dmg);
                        hits.write(hit_for(def, &p, me, f.e, dmg, crit));
                    }
                    true
                }
            }
            Kind::Orbit => {
                let targets = nearest(&foes, p.radius, p.count);
                if targets.is_empty() {
                    false
                } else {
                    for f in &targets {
                        fx.impact(f.e, 403);
                        let (dmg, crit) = roll(rng, &stats, p.dmg);
                        hits.write(hit_for(def, &p, me, f.e, dmg, crit));
                    }
                    true
                }
            }
            Kind::Shield => {
                let target = party::support_target(me);
                let shield = Support::Shield {
                    amount: p.dmg,
                    secs: p.duration,
                };
                supports.push((target, shield));
                fx.cast(me, def.spell, &[target], None);
                true
            }
            Kind::Immune => {
                if foes.first().is_some_and(|f| f.dist <= 5.0) {
                    hero.immune_t = p.duration;
                    fx.cast(me, def.spell, &[me], None);
                    true
                } else {
                    false
                }
            }
            Kind::Thorns => {
                if foes.first().is_some_and(|f| f.dist <= 8.0) {
                    hero.thorns = p.dmg;
                    hero.thorns_t = p.duration;
                    hero.thorns_spell = def.spell;
                    fx.cast(me, def.spell, &[me], None);
                    true
                } else {
                    false
                }
            }
            Kind::Heal => {
                let target = party::support_target(me);
                let hurt = healths
                    .iter()
                    .find(|(e, ..)| *e == target)
                    .is_some_and(|&(_, hp, max)| hp < max * 0.85);
                if hurt {
                    let renew = Support::HealOverTime {
                        per_sec: p.dmg / p.duration.max(1.0),
                        secs: p.duration,
                    };
                    supports.push((target, renew));
                    fx.cast(me, def.spell, &[target], None);
                    fx.number(target, p.dmg, false, COLOR_HEAL);
                    true
                } else {
                    false
                }
            }
            Kind::Totem => {
                if foes
                    .first()
                    .is_some_and(|f| f.dist <= def.range.max(p.radius) + 6.0)
                {
                    fx.cast(me, def.spell, &[], None);
                    let a = rng.range(0.0, std::f32::consts::TAU);
                    let pos = hpos + Vec3::new(a.cos(), 0.0, a.sin()) * 2.0;
                    let serial = run.guid();
                    let totem = units::spawn_unit(
                        commands,
                        &UnitSpec {
                            serial,
                            display: def.totem.0,
                            scale: 1.0,
                            level: 60,
                            faction: units::FACTION_FRIENDLY,
                            bytes0: 0,
                            weapons: &[],
                            sheath: 0,
                        },
                        pos,
                        0.0,
                    );
                    commands.entity(totem).insert(Totem {
                        cast: def.totem.1,
                        damage: p.dmg,
                        range: def.range,
                        radius: p.radius,
                        left: p.duration,
                        tick: 0.3,
                        pulse: def.key == "magma_totem",
                        owner: me,
                        ability: def,
                        school: p.school,
                    });
                    true
                } else {
                    false
                }
            }
        };
        if !echo {
            hero.abilities[i].cd = if fired { p.cooldown } else { 0.2 };
            if let Some(a) = armed.filter(|_| fired) {
                hero.mechanics.spend_armed();
                hero.abilities[i].cd = p.cooldown * a.cooldown;
            }
            if fired {
                procs.0.push(Proc {
                    owner: me,
                    ability: def,
                    kind: ProcKind::Cast,
                    target: None,
                    pos: hpos,
                    damage: p.dmg,
                    effect: def.effect,
                });
            }
        }
    }
}

/// The stats a cast rolls under: a guarantee armed to crit makes every blow of it a crit.
fn empowered(stats: Stats, armed: Option<Armed>) -> Stats {
    match armed {
        Some(a) if a.crit => Stats { crit: 1.0, ..stats },
        _ => stats,
    }
}

/// The spell a cast of `def` at `level` shows: once transformed, its legendary's missile when the
/// legendary's spell and its own both fly one (Fireball's Pyroblast), else its own.
fn look(def: &AbilityDef, level: u8, speed_of: &dyn Fn(u32) -> f32) -> u32 {
    if level < data::MAX_ABILITY_LEVEL || speed_of(def.spell) <= 0.0 {
        return def.spell;
    }
    match data::legendary(def.key) {
        Some(l) if speed_of(l.visual_spell) > 0.0 => l.visual_spell,
        _ => def.spell,
    }
}

/// Ground areas tick, and totems fight, until they run out.
#[allow(clippy::type_complexity)]
pub(super) fn tick_areas(
    mut commands: Commands,
    time: Res<Time>,
    run: Res<Run>,
    mut rng: ResMut<Rng>,
    spells: Option<Res<crate::ui_action::Spells>>,
    heroes: Query<(Entity, &HeroState)>,
    mut areas: Query<(Entity, &Transform, &mut GroundArea)>,
    mut totems: Query<(Entity, &Transform, &mut Totem), Without<GroundArea>>,
    enemies: Query<(Entity, &Transform, &Enemy), (Without<GroundArea>, Without<Totem>)>,
    mut fx: Fx,
    mut pending: ResMut<PendingHits>,
    mut hits: MessageWriter<Hit>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    // Each hero's stats once a frame, for the crits of what it left behind.
    let owners: Vec<(Entity, Stats)> = heroes.iter().map(|(e, h)| (e, h.stats())).collect();
    let fallback = HeroState::default().stats();
    let stats_of = |owner: Entity| {
        owners
            .iter()
            .find(|(e, _)| *e == owner)
            .map_or(fallback, |(_, s)| *s)
    };
    let now = run.elapsed;
    let live: Vec<Foe> = enemies
        .iter()
        .filter(|(_, _, e)| !e.dead)
        .map(|(e, tf, en)| Foe {
            e,
            pos: tf.translation,
            hp: en.hp,
            dist: 0.0,
            radius: en.radius,
        })
        .collect();

    for (e, tf, mut area) in &mut areas {
        area.left -= dt;
        area.next_tick -= dt;
        if area.next_tick <= 0.0 {
            area.next_tick += area.every;
            for (i, f) in around(&live, tf.translation, area.radius)
                .into_iter()
                .enumerate()
            {
                if area.impact != 0 && i < 12 {
                    fx.impact(f.e, area.impact);
                }
                let (dmg, crit) = roll(&mut rng, &stats_of(area.owner), area.damage);
                hits.write(Hit {
                    target: f.e,
                    damage: dmg,
                    dot: area.dot,
                    dot_dur: area.dot_dur,
                    effect: area.effect,
                    mark: area.mark,
                    mark_dur: 1.5,
                    splash: 0.0,
                    crit,
                    color: COLOR_SPELL,
                    source: area.owner,
                    origin: area.origin,
                    school: area.school,
                });
            }
        }
        if area.left <= 0.0 {
            commands.entity(e).despawn();
        }
    }

    for (e, tf, mut totem) in &mut totems {
        totem.left -= dt;
        if totem.left <= 0.0 {
            commands
                .entity(e)
                .remove::<Totem>()
                .insert(benilla_world::model_fade::DespawnFade::default());
            continue;
        }
        totem.tick -= dt;
        if totem.tick > 0.0 {
            continue;
        }
        let pos = tf.translation;
        let stats = stats_of(totem.owner);
        if totem.pulse {
            let targets = around(&live, pos, totem.radius);
            totem.tick = 2.0;
            if targets.is_empty() {
                continue;
            }
            fx.cast(e, totem.cast, &[], None);
            for f in &targets {
                let (dmg, crit) = roll(&mut rng, &stats, totem.damage);
                hits.write(Hit {
                    color: COLOR_SPELL,
                    origin: Origin::Ability(totem.ability),
                    school: Some(totem.school),
                    ..Hit::direct(totem.owner, f.e, dmg, crit)
                });
            }
        } else {
            totem.tick = 1.0;
            let target = live
                .iter()
                .map(|f| (flat(f.pos - pos).length(), f))
                .filter(|(d, _)| *d <= totem.range)
                .min_by(|a, b| a.0.total_cmp(&b.0));
            let Some((dist, f)) = target else { continue };
            fx.cast(e, totem.cast, &[f.e], None);
            let speed = spells
                .as_deref()
                .and_then(|s| s.catalog.get(totem.cast))
                .map_or(0.0, |d| d.speed);
            let delay = if speed > 0.0 { dist / speed } else { 0.0 };
            let (dmg, crit) = roll(&mut rng, &stats, totem.damage);
            schedule(
                &mut pending,
                &mut hits,
                now,
                delay,
                Hit {
                    color: COLOR_SPELL,
                    origin: Origin::Ability(totem.ability),
                    school: Some(totem.school),
                    ..Hit::direct(totem.owner, f.e, dmg, crit)
                },
            );
        }
    }
}

/// What each hero dealt this frame, in the order it landed.
fn credit(dealt: &mut Vec<(Entity, f32)>, hero: Entity, amount: f32) {
    match dealt.iter_mut().find(|(e, _)| *e == hero) {
        Some((_, sum)) => *sum += amount,
        None => dealt.push((hero, amount)),
    }
}

/// An enemy as a blow lands on it: its unit, its body, its marks, whether it can't die yet
/// ([`super::raid::Undying`]), and the ward its raid master put on it ([`super::raid::Ward`]).
type Struck = (
    Entity,
    &'static Guid,
    &'static Transform,
    &'static mut Enemy,
    &'static mut ObjectStore,
    &'static mut Marks,
    Has<super::raid::Undying>,
    Option<&'static super::raid::Ward>,
);

/// Land every hit due: damage and its number, the over-time part, crowd control, the marks, and
/// death with its loot. Each hero is credited with its own damage, and drains and leeches for it.
/// An ability's own blows are recorded in [`Procs`] for its hero's mechanics, and put its class's
/// shared debuffs on the target; the debuffs already there raise what every hero's blows deal to
/// it ([`synergy`]).
pub(super) fn apply_hits(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    mut heroes: Query<&mut HeroState>,
    mut pending: ResMut<PendingHits>,
    mut incoming: MessageReader<Hit>,
    mut enemies: Query<Struck>,
    mut fx: Fx,
    mut loot: MessageWriter<Loot>,
    mut fields: MessageWriter<FieldChanged>,
    mut procs: ResMut<Procs>,
) {
    let dt = time.delta_secs();
    let now = run.elapsed;
    // Everything each hero dealt this frame, for lifesteal.
    let mut leeched: Vec<(Entity, f32)> = Vec::new();
    let mut due: Vec<Hit> = incoming.read().cloned().collect();
    pending.0.retain(|(t, h)| {
        if *t <= now {
            due.push(h.clone());
            false
        } else {
            true
        }
    });

    // Splash reaches the target's neighbours at seventy percent.
    let mut all = Vec::with_capacity(due.len());
    for h in due {
        if h.splash > 0.0 {
            if let Ok((_, _, center, ..)) = enemies.get(h.target) {
                let c = center.translation;
                for (e, _, tf, en, ..) in enemies.iter() {
                    if e != h.target && !en.dead && flat(tf.translation - c).length() <= h.splash {
                        all.push(Hit {
                            target: e,
                            damage: h.damage * 0.7,
                            splash: 0.0,
                            mark: 0,
                            origin: Origin::Other,
                            ..h.clone()
                        });
                    }
                }
            }
        }
        all.push(Hit { splash: 0.0, ..h });
    }

    let mut kill =
        |commands: &mut Commands, run: &mut Run, e: Entity, pos: Vec3, en: &mut Enemy| {
            en.dead = true;
            en.hp = 0.0;
            en.dots.clear();
            enemies::mark_dying(commands, e);
            loot.write(Loot {
                pos,
                xp: en.xp,
                boss: en.boss,
                elite: en.elite,
                def: en.def,
            });
            run.kills += 1;
            if en.boss {
                run.banner = Some((format!("{} is slain!", en.def.name), 3.0));
            }
        };

    for h in all {
        let Ok((e, guid, tf, mut en, mut store, mut marks, undying, ward)) =
            enemies.get_mut(h.target)
        else {
            continue;
        };
        if en.dead {
            continue;
        }
        let mut dmg = h.damage;
        let mut crit = h.crit;
        if dmg > 0.0 && !en.debuffs.is_empty() {
            // The party's shared debuffs: more of the blow's school taken, and a frost blow's crit.
            dmg *= en.debuffs.taken(h.school);
            let extra = en.debuffs.crit_taken(h.school);
            if !crit && extra > 0.0 && rng.f32() < extra {
                if let Ok(hero) = heroes.get(h.source) {
                    dmg *= hero.stats().crit_damage;
                    crit = true;
                }
            }
        }
        if let Effect::Execute(threshold) = h.effect {
            if !en.boss && en.hp / en.max_hp <= threshold {
                dmg = dmg.max(en.hp);
            }
        }
        if dmg > 0.0 {
            en.hp -= dmg;
            run.damage_done += f64::from(dmg);
            credit(&mut leeched, h.source, dmg);
            fx.number(e, dmg, crit, h.color);
            if let Ok(mut hero) = heroes.get_mut(h.source) {
                hero.damage_done += f64::from(dmg);
                // A raid add's ward answers the blow on its hero (Magic Reflection, Damage Shield).
                if let Some(ward) = ward {
                    let back = ward.answer(now, h.school, dmg, hero.stats().max_hp);
                    if back > 0.0 {
                        let taken = hero::hurt_hero(&mut hero, back);
                        if taken > 0.0 {
                            fx.impact(h.source, ward.spell);
                            fx.number(h.source, taken, false, units::COLOR_HURT);
                        }
                    }
                }
                // Vampiric Embrace's share of its school, pooled for the party heal.
                if let Some(school) = h.school {
                    hero.mechanics.pool(school, dmg);
                }
                // A drain heals its dealer, as lifesteal does.
                if let Effect::Drain(frac) = h.effect {
                    let healed = party::heal(&mut hero, dmg * frac);
                    hero.healing_done += f64::from(healed);
                }
            }
        }
        if h.dot > 0.0 && h.dot_dur > 0.0 {
            if en.dots.len() >= 6 {
                en.dots.remove(0);
            }
            en.dots.push(enemies::Dot {
                dps: h.dot,
                left: h.dot_dur,
                tick: 1.0,
                source: h.source,
                school: h.school,
            });
        }
        let boss = en.boss;
        match h.effect {
            Effect::Slow(m, t) => {
                let m = if boss { m.max(0.75) } else { m };
                en.slow = if en.slow_t > 0.0 { en.slow.min(m) } else { m };
                en.slow_t = en.slow_t.max(t);
            }
            Effect::Root(t) => en.root_t = en.root_t.max(if boss { t * 0.3 } else { t }),
            Effect::Stun(t) => en.stun_t = en.stun_t.max(if boss { t * 0.3 } else { t }),
            Effect::Fear(t) if !boss => en.fear_t = en.fear_t.max(t),
            Effect::Weaken(m, t) => {
                en.weaken = m;
                en.weaken_t = t;
            }
            _ => {}
        }
        if h.mark != 0 {
            marks.add(h.mark, h.mark_dur);
        }
        if let Origin::Ability(def) = h.origin {
            if let Ok(hero) = heroes.get(h.source) {
                if hero.mechanics.listens() {
                    procs.0.push(Proc {
                        owner: h.source,
                        ability: def,
                        kind: ProcKind::Hit {
                            crit,
                            killed: en.hp <= 0.0,
                            bane: enemies::bane(en.def),
                        },
                        target: Some(e),
                        pos: tf.translation,
                        damage: dmg,
                        effect: h.effect,
                    });
                }
                if en.hp > 0.0 {
                    let en = &mut *en;
                    synergy::afflict(
                        hero.class,
                        h.school,
                        &mut en.debuffs,
                        &mut marks,
                        &mut rng,
                        &mut fx,
                        e,
                    );
                }
            }
        }
        // A raid add that can't die while its master stands keeps a sliver ([`super::raid`]).
        if en.hp <= 0.0 && undying {
            en.hp = 1.0;
        }
        if en.hp <= 0.0 {
            kill(&mut commands, &mut run, e, tf.translation, &mut en);
            units::kill_unit(&mut store);
            marks.clear();
        }
        marks.flush(e, guid.0, &mut store, &mut fields);
    }

    // The over-time ticks, once a second each, credited by whose they are.
    let mut by: Vec<(Entity, f32)> = Vec::new();
    let mut by_school: Vec<(Entity, School, f32)> = Vec::new();
    for (e, guid, tf, mut en, mut store, mut marks, undying, _) in &mut enemies {
        if en.dead || en.dots.is_empty() {
            continue;
        }
        let mut dealt = 0.0;
        by.clear();
        let en = &mut *en;
        for d in &mut en.dots {
            d.left -= dt;
            d.tick -= dt;
            if d.tick <= 0.0 {
                d.tick += 1.0;
                // A tick of a school the target's debuffs raise is raised with it.
                let dps = d.dps * en.debuffs.taken(d.school);
                dealt += dps;
                credit(&mut by, d.source, dps);
                if let Some(school) = d.school {
                    by_school.push((d.source, school, dps));
                }
            }
        }
        en.dots.retain(|d| d.left > 0.0);
        if dealt > 0.0 {
            en.hp -= dealt;
            run.damage_done += f64::from(dealt);
            for &(source, amount) in &by {
                credit(&mut leeched, source, amount);
                if let Ok(mut hero) = heroes.get_mut(source) {
                    hero.damage_done += f64::from(amount);
                }
            }
            fx.number(e, dealt, false, COLOR_SPELL);
            if en.hp <= 0.0 && undying {
                en.hp = 1.0;
            }
            if en.hp <= 0.0 {
                kill(&mut commands, &mut run, e, tf.translation, en);
                units::kill_unit(&mut store);
                marks.clear();
                marks.flush(e, guid.0, &mut store, &mut fields);
            }
        }
    }

    for (source, school, amount) in by_school {
        if let Ok(mut hero) = heroes.get_mut(source) {
            hero.mechanics.pool(school, amount);
        }
    }

    // A fallen hero stays down.
    for (source, amount) in leeched {
        if let Ok(mut hero) = heroes.get_mut(source) {
            if hero.alive() {
                let before = hero.hp;
                hero.hp = hero.stats().leech(hero.hp, amount);
                hero.healing_done += f64::from(hero.hp - before);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::super::bonus::{Bonus, StatKey};
    use super::super::data;
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-4
    }

    /// A world `apply_hits` runs in, a second passing, with one sturdy foe standing in it.
    fn hits_world() -> (World, Entity) {
        let mut world = World::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs(1));
        world.insert_resource(time);
        world.init_resource::<Run>();
        world.insert_resource(Rng(1));
        world.init_resource::<PendingHits>();
        world.init_resource::<Procs>();
        world.init_resource::<Messages<Hit>>();
        world.init_resource::<Messages<Loot>>();
        world.init_resource::<Messages<FieldChanged>>();
        world.init_resource::<crate::creature_anim::PlaySeq>();
        world.init_resource::<Messages<crate::creature_anim::CastEvent>>();
        world.init_resource::<Messages<crate::creature_anim::SpellGoTargets>>();
        world.init_resource::<Messages<crate::creature_anim::SwingMessage>>();
        world.init_resource::<Messages<crate::combat_text::CombatTextSpawn>>();
        let foe = Enemy {
            def: &data::roster(0).enemies[0],
            hp: 1000.0,
            max_hp: 1000.0,
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
        let store = ObjectStore(Default::default());
        let body = (Guid(99), Transform::default(), store, Marks::debuffs());
        let foe = world.spawn((foe, body)).id();
        (world, foe)
    }

    #[test]
    fn each_hero_is_credited_with_its_own_damage_drains_and_leech() {
        let (mut world, foe) = hits_world();
        let a = world
            .spawn(HeroState {
                hp: 50.0,
                ..HeroState::default()
            })
            .id();
        let mut leech = HeroState {
            player: 1,
            hp: 50.0,
            ..HeroState::default()
        };
        leech.bonuses.add(&Bonus::new(StatKey::Lifesteal, 0.1));
        let b = world.spawn(leech).id();
        // `a` drains for 10 and lays a dot that ticks 4 this second; `b` strikes for 5.
        world.write_message(Hit {
            effect: Effect::Drain(0.5),
            ..Hit::direct(a, foe, 10.0, false)
        });
        world.write_message(Hit {
            dot: 4.0,
            dot_dur: 3.0,
            ..Hit::direct(a, foe, 0.0, false)
        });
        world.write_message(Hit::direct(b, foe, 5.0, false));
        world.run_system_once(apply_hits).unwrap();
        assert!(
            close(world.resource::<Run>().damage_done, 19.0),
            "the total"
        );
        let hero = |e: Entity| world.get::<HeroState>(e).unwrap();
        assert!(close(hero(a).damage_done, 14.0));
        assert!(close(hero(b).damage_done, 5.0));
        // The drain heals its dealer, lifesteal its own: neither the other.
        assert!(close(f64::from(hero(a).hp), 55.0));
        assert!(close(hero(a).healing_done, 5.0));
        assert!(close(f64::from(hero(b).hp), 50.5));
        assert!(close(hero(b).healing_done, 0.5));
        assert_eq!(world.get::<Enemy>(foe).unwrap().hp, 1000.0 - 19.0);
    }

    /// Presence of Mind's guarantee rolls every blow of the cast a crit; Clearcasting's alone
    /// leaves the crit chance be.
    #[test]
    fn a_cast_armed_to_crit_rolls_every_blow_a_crit() {
        let base = HeroState::default().stats();
        let armed = Armed {
            crit: true,
            cooldown: 1.0,
            aura: 12043,
        };
        assert_eq!(empowered(base, Some(armed)).crit, 1.0);
        let clearcasting = Armed {
            crit: false,
            cooldown: 0.5,
            aura: 12536,
        };
        assert_eq!(empowered(base, Some(clearcasting)).crit, base.crit);
        assert_eq!(empowered(base, None).crit, base.crit);
        let mut rng = Rng(3);
        let sure = empowered(base, Some(armed));
        assert!((0..50).all(|_| roll(&mut rng, &sure, 10.0).1));
    }

    /// Vampiric Embrace pools its owner's Shadow damage, the blows' and the ticks', and only it.
    #[test]
    fn shadow_damage_pools_for_a_shared_heal() {
        use super::super::mechanics::{self, Effect as M, MechanicId, MechanicSpec, Trigger};
        let (mut world, foe) = hits_world();
        let mut priest = HeroState::default();
        mechanics::grant(
            &mut priest,
            MechanicId::Other("vampiric"),
            [MechanicSpec::new(
                Trigger::Periodic { every: 2.0 },
                M::ShareHeal {
                    school: School::Shadow,
                    share: 0.15,
                    radius: 40.0,
                },
            )],
        );
        let priest = world.spawn(priest).id();
        let plain = world.spawn(HeroState::default()).id();
        world.write_message(Hit {
            school: Some(School::Shadow),
            ..Hit::direct(priest, foe, 10.0, false)
        });
        world.write_message(Hit {
            school: Some(School::Fire),
            ..Hit::direct(priest, foe, 5.0, false)
        });
        world.write_message(Hit {
            dot: 4.0,
            dot_dur: 3.0,
            school: Some(School::Shadow),
            ..Hit::direct(priest, foe, 0.0, false)
        });
        world.write_message(Hit {
            school: Some(School::Shadow),
            ..Hit::direct(plain, foe, 10.0, false)
        });
        world.run_system_once(apply_hits).unwrap();
        let pooled = |e: Entity| {
            world
                .get::<HeroState>(e)
                .unwrap()
                .mechanics
                .pooled(School::Shadow)
        };
        assert!((pooled(priest) - 14.0).abs() < 1e-4, "the blow and a tick");
        assert_eq!(pooled(plain), 0.0, "no Vampiric Embrace, nothing kept");
    }

    #[test]
    fn crits_use_the_crit_multiplier() {
        let mut hero = HeroState::default();
        hero.bonuses.add(&Bonus::new(StatKey::CritChance, 1.0));
        hero.bonuses.add(&Bonus::new(StatKey::CritDamage, 0.5));
        assert_eq!(roll(&mut Rng::default(), &hero.stats(), 10.0), (25.0, true));
    }

    #[test]
    fn dbc_schools_map_in_order() {
        let all = [
            School::Physical,
            School::Holy,
            School::Fire,
            School::Nature,
            School::Frost,
            School::Shadow,
            School::Arcane,
        ];
        for (i, school) in all.into_iter().enumerate() {
            assert_eq!(dbc_school(i as u32), school);
        }
        assert_eq!(dbc_school(99), School::Physical);
    }
}
