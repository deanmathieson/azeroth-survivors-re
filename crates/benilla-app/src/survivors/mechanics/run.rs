//! The mechanics at work: every hero's clocks, the frame's procs matched against what they set
//! off, and each effect landing with its visual; ground that follows its hero, and orbiting motes.

use benilla_formats::{SpellVisualCatalog, VisualKit};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::super::abilities::{roll, GroundArea, Hit, Origin, PendingHits};
use super::super::bonus::{Bonus, School};
use super::super::data::{self, AbilityDef};
use super::super::enemies::{Dot, Enemy};
use super::super::party;
use super::super::units::{self, flat, Fx, COLOR_HEAL, COLOR_SPELL};
use super::super::{HeroState, Rng, Run, RunEntity, Stats};
use super::target::{self, Foe};
use super::{
    chance, cheat_death, execute_bonus, fires_on, level_scale, low_health, periodic,
    sync_legendaries, Aim, Armed, Cc, Effect, MechanicSpec, Proc, Procs, Running, Trigger,
    ECHO_GAP, HEALTHSTONE_REACH, HEALTHSTONE_SHARE, HEALTHSTONE_SPELL, MAX_FEED, SHATTER_CHILL,
};
use crate::entities::dest_fx::GroundBurst;

/// How far a mechanic with no enemy of its own looks for one.
const REACH: f32 = 25.0;
/// How far a chain jumps, as Chain Lightning's does.
const CHAIN_REACH: f32 = 9.0;
/// Seconds between one jump of a chain and the next.
const CHAIN_STEP: f32 = 0.08;
/// A split's shards strike this share of the blow.
pub(crate) const SPLIT_SHARE: f32 = 0.5;
/// How fast shards, pierces and a returning blow travel, in yards a second.
const SHARD_SPEED: f32 = 30.0;
/// A pierce strikes this share of the blow, a line this wide reaching this far past the target.
pub(crate) const PIERCE_SHARE: f32 = 0.8;
const PIERCE_WIDTH: f32 = 1.6;
const PIERCE_LENGTH: f32 = 14.0;
/// A returning blow strikes this share of it, along a way this wide.
pub(crate) const BOOMERANG_SHARE: f32 = 0.75;
const BOOMERANG_WIDTH: f32 = 1.6;
/// The thickest pack is counted within this many yards of each enemy.
const PACK: f32 = 5.0;
/// An orbiting mote's turn in radians a second, its reach, and how soon it strikes one enemy again.
const ORBIT_SPEED: f32 = 3.2;
const ORBIT_TOUCH: f32 = 1.5;
const ORBIT_REHIT: f32 = 0.5;
/// More impacts than this per effect only cost frames.
const MAX_VISUALS: usize = 6;
/// `SpellVisualKit`'s base attachment (the unit's feet), where a ring at a point sits.
const BASE_TAG: u16 = 0x13;

/// Every school, in [`School`]'s order.
const SCHOOLS: [School; 7] = [
    School::Physical,
    School::Holy,
    School::Fire,
    School::Nature,
    School::Frost,
    School::Shadow,
    School::Arcane,
];

/// Ground that moves with the hero who laid it.
#[derive(Component)]
pub(crate) struct Follow(pub Entity);

/// One mote circling its hero, striking what it touches.
#[derive(Component)]
pub(crate) struct Orbiter {
    owner: Entity,
    angle: f32,
    radius: f32,
    left: f32,
    damage: f32,
    effect: data::Effect,
    visual: Option<u32>,
    /// Enemies struck lately, and the seconds before this mote strikes each again.
    struck: Vec<(Entity, f32)>,
}

/// How a mechanic's visual reaches the screen: the cast and impact kits on units, and a bare
/// point's model through the dest one-shot.
#[derive(SystemParam)]
pub(crate) struct Show<'w> {
    fx: Fx<'w>,
    bursts: MessageWriter<'w, GroundBurst>,
    spells: Option<Res<'w, crate::ui_action::Spells>>,
    visuals: Option<Res<'w, crate::creature_anim::SpellVisuals>>,
}

impl Show<'_> {
    /// `spell` cast by `hero`, landing on `hits`.
    fn on_hero(&mut self, hero: Entity, spell: Option<u32>, hits: &[Entity]) {
        if let Some(spell) = spell {
            let shown = &hits[..hits.len().min(MAX_VISUALS)];
            self.fx.cast(hero, spell, shown, None);
        }
    }

    /// `spell`'s impact on each of `hits`.
    fn on(&mut self, spell: Option<u32>, hits: &[Entity]) {
        if let Some(spell) = spell {
            for &e in hits.iter().take(MAX_VISUALS) {
                self.fx.impact(e, spell);
            }
        }
    }

    /// `spell`'s model at the bare point `pos`, and its impact on each of `hits`.
    fn at_point(&mut self, spell: Option<u32>, pos: Vec3, hits: &[Entity]) {
        let Some(spell) = spell else { return };
        let path = match (self.spells.as_deref(), self.visuals.as_deref()) {
            (Some(spells), Some(visuals)) => spells
                .catalog
                .get(spell)
                .and_then(|d| point_model(d.visual, &visuals.0))
                .map(str::to_string),
            _ => None,
        };
        if let Some(path) = path {
            self.bursts.write(GroundBurst { path, pos });
        }
        self.on(Some(spell), hits);
    }

    /// An area's visual: cast by the hero when it is centred on the hero, else at its point.
    fn area(&mut self, hero: Entity, spell: Option<u32>, site: Site, hits: &[Entity]) {
        if site.on_hero {
            self.on_hero(hero, spell, hits);
        } else {
            self.at_point(spell, site.pos, hits);
        }
    }
}

/// The model `visual` (a `SpellVisual.dbc` row) shows at a bare point: its dest one-shot's (field
/// 12), else a kit's world-planted model (the ring at a caster's feet), else one on the base
/// attachment, else its impact's.
pub(crate) fn point_model(visual: u32, visuals: &SpellVisualCatalog) -> Option<&str> {
    let stages = visuals.stages(visual)?;
    if stages.area_effect != 0 {
        if let Some(path) = visuals.effect_path(stages.area_effect) {
            return Some(path);
        }
    }
    let kits: Vec<&VisualKit> = [stages.cast, stages.impact]
        .into_iter()
        .filter_map(|k| visuals.kit(k))
        .collect();
    let world = kits
        .iter()
        .find_map(|k| k.world_effect.and_then(|e| visuals.effect_path(e)));
    let base = || {
        kits.iter().find_map(|k| {
            k.effects()
                .find(|&(tag, _)| tag == BASE_TAG)
                .and_then(|(_, e)| visuals.effect_path(e))
        })
    };
    let impact = || {
        visuals
            .kit(stages.impact)
            .and_then(|k| k.effects().find_map(|(_, e)| visuals.effect_path(e)))
    };
    world.or_else(base).or_else(impact)
}

/// A mechanic going off for its owner, with what set it off.
#[derive(Clone, Copy)]
struct Fire {
    owner: Entity,
    /// The granted mechanic's, so a burst it set going refreshes.
    uid: u32,
    spec: MechanicSpec,
    /// Where the trigger happened: the struck enemy's place, or the hero's.
    pos: Vec3,
    /// The enemy struck, for a hit, crit or kill.
    target: Option<Entity>,
    /// The blow it carries on: the hit's damage, a cast's per target; 0 for none.
    base: f32,
    /// The blow's own effect, carried on with it.
    effect: data::Effect,
    ability: Option<&'static AbilityDef>,
}

impl Fire {
    fn from_proc(p: &Proc, uid: u32, spec: MechanicSpec) -> Self {
        Self {
            owner: p.owner,
            uid,
            spec,
            pos: p.pos,
            target: p.target,
            base: p.damage,
            effect: p.effect,
            ability: Some(p.ability),
        }
    }
}

/// Where one landing of an effect is.
#[derive(Clone, Copy, Debug)]
struct Site {
    pos: Vec3,
    target: Option<Entity>,
    /// Centred on the hero, so its visual is the hero's cast.
    on_hero: bool,
}

/// The owner's numbers, read once a fire.
struct Caster {
    pos: Vec3,
    stats: Stats,
    schools: [f32; 7],
}

impl Caster {
    fn of(pos: Vec3, hero: &HeroState) -> Self {
        Self {
            pos,
            stats: hero.stats(),
            schools: SCHOOLS.map(|s| hero.school_mult(s)),
        }
    }

    /// A flat number's damage at run level `level`, in `school`.
    fn damage(&self, base: f32, level: u32, school: School) -> f32 {
        base * level_scale(level) * self.stats.might * self.schools[school as usize]
    }
}

/// A mechanic's blow: the owner's, setting off nothing.
fn blow(owner: Entity, target: Entity, damage: f32, crit: bool, effect: data::Effect) -> Hit {
    Hit {
        effect,
        color: COLOR_SPELL,
        origin: Origin::Mechanic,
        ..Hit::direct(owner, target, damage, crit)
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

/// Every hero's clocks run, the frame's procs are matched, and what went off lands: after the
/// frame's blows, before the vitals decide who falls.
#[allow(clippy::type_complexity)]
pub(crate) fn tick(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    mut procs: ResMut<Procs>,
    mut heroes: Query<(Entity, &Transform, &mut HeroState)>,
    mut enemies: Query<(Entity, &mut Transform, &mut Enemy), Without<HeroState>>,
    mut show: Show,
    mut pending: ResMut<PendingHits>,
    mut hits: MessageWriter<Hit>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let party: Vec<(Entity, Vec3)> = heroes
        .iter()
        .map(|(e, tf, _)| (e, tf.translation))
        .collect();
    share_healthstones(&mut heroes, &party, &mut show);
    let mut fires = Vec::new();
    for (me, tf, mut hero) in &mut heroes {
        clocks(me, tf.translation, &mut hero, dt, &mut show, &mut fires);
    }
    for p in procs.0.drain(..) {
        if let Ok((.., mut hero)) = heroes.get_mut(p.owner) {
            matched(&p, &mut hero, &mut rng, &mut fires);
        }
    }
    if fires.is_empty() {
        return;
    }
    let foes: Vec<Foe> = enemies
        .iter()
        .filter(|(_, _, en)| !en.dead)
        .map(|(e, tf, en)| foe_of(e, tf.translation, en))
        .collect();
    let mut shoves: Vec<(Entity, Vec3)> = Vec::new();
    let mut consumes: Vec<(Entity, Vec3, MechanicSpec)> = Vec::new();
    let (now, level) = (run.elapsed, run.level);
    for fire in &fires {
        if let Effect::ConsumeDot { .. } = fire.spec.effect {
            if let Ok((_, htf, _)) = heroes.get(fire.owner) {
                consumes.push((fire.owner, htf.translation, fire.spec));
            }
            continue;
        }
        land(
            fire,
            &mut heroes,
            &foes,
            &party,
            now,
            level,
            &mut commands,
            &mut run,
            &mut rng,
            &mut show,
            &mut pending,
            &mut hits,
            &mut shoves,
        );
    }
    for (e, shove) in shoves {
        if let Ok((_, mut tf, _)) = enemies.get_mut(e) {
            tf.translation += shove;
        }
    }
    for (owner, at, spec) in consumes {
        consume(owner, at, spec, &mut enemies, &mut show, &mut hits);
    }
}

/// One living enemy as the mechanics see it.
fn foe_of(e: Entity, pos: Vec3, en: &Enemy) -> Foe {
    let burns = en
        .dots
        .iter()
        .filter(|d| d.school == Some(School::Fire))
        .count();
    Foe {
        e,
        pos,
        hp: en.hp,
        max_hp: en.max_hp,
        radius: en.radius,
        boss: en.boss,
        frozen: frozen(en),
        burns: u8::try_from(burns).unwrap_or(u8::MAX),
    }
}

/// Whether Shatter shatters `en`: rooted, or chilled to half speed or slower.
pub(crate) fn frozen(en: &Enemy) -> bool {
    en.root_t > 0.0 || (en.slow_t > 0.0 && en.slow <= SHATTER_CHILL)
}

/// A Healthstone taken reaches every other living hero within [`HEALTHSTONE_REACH`] of its taker,
/// healing [`HEALTHSTONE_SHARE`] of their max health, credited to the taker.
fn share_healthstones(
    heroes: &mut Query<(Entity, &Transform, &mut HeroState)>,
    party: &[(Entity, Vec3)],
    show: &mut Show,
) {
    let mut takers = Vec::new();
    for &(e, at) in party {
        if let Ok((.., mut h)) = heroes.get_mut(e) {
            if std::mem::take(&mut h.mechanics.healthstone) {
                takers.push((e, at));
            }
        }
    }
    for (taker, at) in takers {
        let mut total = 0.0;
        let mut healed = Vec::new();
        for &(e, pos) in party {
            if e == taker || flat(pos - at).length() > HEALTHSTONE_REACH {
                continue;
            }
            let Ok((.., mut ally)) = heroes.get_mut(e) else {
                continue;
            };
            if !ally.alive() {
                continue;
            }
            let max = ally.stats().max_hp;
            let landed = party::heal(&mut ally, max * HEALTHSTONE_SHARE);
            if landed > 0.0 {
                show.fx.number(e, landed, false, COLOR_HEAL);
            }
            total += landed;
            healed.push(e);
        }
        if let Ok((.., mut me)) = heroes.get_mut(taker) {
            me.healing_done += f64::from(total);
        }
        show.on(Some(HEALTHSTONE_SPELL), &healed);
    }
}

/// Conflagrate: `owner`'s damage over time of the spec's school on the nearest enemy carrying one
/// within [`REACH`] of `at`, consumed for its remaining damage at once, and spread afresh to the
/// enemies near it that lack one.
fn consume(
    owner: Entity,
    at: Vec3,
    spec: MechanicSpec,
    enemies: &mut Query<(Entity, &mut Transform, &mut Enemy), Without<HeroState>>,
    show: &mut Show,
    hits: &mut MessageWriter<Hit>,
) {
    let Effect::ConsumeDot {
        school,
        burst,
        spread,
        radius,
        secs,
    } = spec.effect
    else {
        return;
    };
    let carries = |en: &Enemy| {
        !en.dead
            && en
                .dots
                .iter()
                .any(|d| d.source == owner && d.school == Some(school))
    };
    let Some((target, pos)) = enemies
        .iter()
        .filter(|(_, tf, en)| carries(en) && flat(tf.translation - at).length() <= REACH)
        .map(|(e, tf, _)| (e, tf.translation))
        .min_by(|a, b| flat(a.1 - at).length().total_cmp(&flat(b.1 - at).length()))
    else {
        return;
    };
    let Ok((.., mut en)) = enemies.get_mut(target) else {
        return;
    };
    let (left, dps) = consumed(&mut en.dots, owner, school);
    if left <= 0.0 {
        return;
    }
    hits.write(Hit {
        school: Some(school),
        ..blow(owner, target, left * burst, false, data::Effect::None)
    });
    let mut struck = vec![target];
    let mut near: Vec<(Entity, f32)> = enemies
        .iter()
        .filter(|(e, tf, en)| {
            *e != target
                && !en.dead
                && !carries(en)
                && flat(tf.translation - pos).length() <= radius + en.radius * 0.5
        })
        .map(|(e, tf, _)| (e, flat(tf.translation - pos).length()))
        .collect();
    near.sort_by(|a, b| a.1.total_cmp(&b.1));
    for (e, _) in near.into_iter().take(usize::from(spread)) {
        hits.write(Hit {
            dot: dps,
            dot_dur: secs,
            school: Some(school),
            ..blow(owner, e, 0.0, false, data::Effect::None)
        });
        struck.push(e);
    }
    show.on(spec.visual, &struck);
}

/// `owner`'s damages over time of `school` taken off `dots`: their remaining damage, and the
/// strongest one's damage a second.
pub(crate) fn consumed(dots: &mut Vec<Dot>, owner: Entity, school: School) -> (f32, f32) {
    let (mut left, mut dps) = (0.0f32, 0.0f32);
    dots.retain(|d| {
        if d.source == owner && d.school == Some(school) {
            left += d.dps * d.left.max(0.0);
            dps = dps.max(d.dps);
            false
        } else {
            true
        }
    });
    (left, dps)
}

/// `dt` on the hero's running bursts: each one out takes its bonuses off.
pub(crate) fn wear_bursts(hero: &mut HeroState, dt: f32) {
    let mut ended: Vec<&'static [Bonus]> = Vec::new();
    hero.mechanics.bursts.retain_mut(|b| {
        b.left -= dt;
        if b.left > 0.0 {
            true
        } else {
            ended.push(b.bonuses);
            false
        }
    });
    for bonuses in ended {
        for b in bonuses {
            hero.bonuses.add(&Bonus {
                value: -b.value,
                ..*b
            });
        }
    }
}

/// One hero's own clocks: new transformations granted, bursts running out, and the periodic,
/// low-health and lethal triggers.
fn clocks(
    me: Entity,
    pos: Vec3,
    hero: &mut HeroState,
    dt: f32,
    show: &mut Show,
    fires: &mut Vec<Fire>,
) {
    sync_legendaries(hero);
    for spell in std::mem::take(&mut hero.mechanics.flourish) {
        show.on_hero(me, Some(spell), &[me]);
    }
    wear_bursts(hero, dt);
    if !hero.alive() {
        return;
    }
    let max = hero.stats().max_hp;
    let frac = if max > 0.0 { hero.hp / max } else { 1.0 };
    let lethal = hero.hp <= 0.0;
    for g in &mut hero.mechanics.granted {
        g.cd = (g.cd - dt).max(0.0);
        let went = match g.spec.trigger {
            Trigger::Periodic { every } => periodic(&mut g.clock, every, dt),
            Trigger::OnLowHealth { threshold } => low_health(&mut g.armed, frac, threshold),
            Trigger::OnLethal => lethal,
            _ => false,
        };
        if went && g.cd <= 0.0 {
            g.cd = g.spec.cooldown;
            fires.push(Fire {
                owner: me,
                uid: g.uid,
                spec: g.spec,
                pos,
                target: None,
                base: 0.0,
                effect: data::Effect::None,
                ability: None,
            });
        }
    }
}

/// What one proc sets off of its hero's mechanics.
fn matched(p: &Proc, hero: &mut HeroState, rng: &mut Rng, fires: &mut Vec<Fire>) {
    if !hero.alive() {
        return;
    }
    for g in &mut hero.mechanics.granted {
        let Some(odds) = fires_on(g.spec.trigger, p.kind, p.ability) else {
            continue;
        };
        if g.cd > 0.0 || !chance(rng, odds) {
            continue;
        }
        g.cd = g.spec.cooldown;
        fires.push(Fire::from_proc(p, g.uid, g.spec));
    }
}

/// Where a fire lands, by its aim.
fn sites(at: Aim, fire: &Fire, hero: Vec3, foes: &[Foe], rng: &mut Rng) -> Vec<Site> {
    let on = |i: usize| Site {
        pos: foes[i].pos,
        target: Some(foes[i].e),
        on_hero: false,
    };
    match at {
        Aim::Trigger => vec![Site {
            pos: fire.pos,
            target: fire.target,
            on_hero: fire.target.is_none(),
        }],
        Aim::Hero | Aim::Follow => vec![Site {
            pos: hero,
            target: None,
            on_hero: true,
        }],
        Aim::Spread { radius, n } => {
            target::nearest_n(foes, fire.pos, radius, usize::from(n), fire.target)
                .into_iter()
                .map(on)
                .collect()
        }
        Aim::Random { range } => target::random(foes, hero, range, rng)
            .map(on)
            .into_iter()
            .collect(),
        Aim::Densest { range } => target::densest(foes, hero, range, PACK)
            .map(|pos| Site {
                pos,
                target: None,
                on_hero: false,
            })
            .into_iter()
            .collect(),
    }
}

/// The enemy an effect at `site` works on: the one struck there, else the nearest.
fn subject(site: Site, foes: &[Foe]) -> Option<usize> {
    match site.target {
        Some(e) => foes.iter().position(|f| f.e == e),
        None => target::nearest(foes, site.pos, REACH, None),
    }
}

/// Where a carried blow starts: the enemy struck at `site` (already struck), else the nearest
/// enemy, which the blow strikes first. `(from, at, struck first)`.
fn carrier(site: Site, foes: &[Foe]) -> Option<(Option<Entity>, Vec3, Option<usize>)> {
    match site.target {
        Some(e) => Some((Some(e), site.pos, None)),
        None => target::nearest(foes, site.pos, REACH, None)
            .map(|i| (Some(foes[i].e), foes[i].pos, Some(i))),
    }
}

/// One fire lands: the hero's own effects on the hero, the rest at each of its sites.
fn land(
    fire: &Fire,
    heroes: &mut Query<(Entity, &Transform, &mut HeroState)>,
    foes: &[Foe],
    party: &[(Entity, Vec3)],
    now: f32,
    level: u32,
    commands: &mut Commands,
    run: &mut Run,
    rng: &mut Rng,
    show: &mut Show,
    pending: &mut PendingHits,
    hits: &mut MessageWriter<Hit>,
    shoves: &mut Vec<(Entity, Vec3)>,
) {
    let owner = fire.owner;
    let Ok((_, htf, hero)) = heroes.get(owner) else {
        return;
    };
    let caster = Caster::of(htf.translation, hero);
    let spec = fire.spec;
    if own(fire, heroes, now, level, show) {
        return;
    }
    for site in sites(spec.at, fire, caster.pos, foes, rng) {
        match spec.effect {
            Effect::Nova {
                school,
                radius,
                damage,
                knockback,
                cc,
            } => {
                let dmg = caster.damage(damage, level, school);
                let effect = cc.map_or(data::Effect::None, Cc::effect);
                let mut struck = Vec::new();
                for i in target::around(foes, site.pos, radius) {
                    let f = foes[i];
                    if dmg > 0.0 || cc.is_some() {
                        let (d, crit) = if dmg > 0.0 {
                            roll(rng, &caster.stats, dmg)
                        } else {
                            (0.0, false)
                        };
                        hits.write(Hit {
                            school: Some(school),
                            ..blow(owner, f.e, d, crit, effect)
                        });
                    }
                    if knockback > 0.0 && !f.boss {
                        let away = flat(f.pos - site.pos).normalize_or_zero();
                        shoves.push((f.e, away * knockback));
                    }
                    struck.push(f.e);
                }
                show.area(owner, spec.visual, site, &struck);
            }
            Effect::BurningGround {
                school,
                radius,
                dps,
                duration,
            } => {
                let dps = caster.damage(dps, level, school);
                let anchor = match spec.visual {
                    Some(spell) => {
                        let serial = run.guid();
                        units::spawn_area_object(commands, serial, spell, radius, site.pos)
                    }
                    None => commands
                        .spawn((Transform::from_translation(site.pos), RunEntity))
                        .id(),
                };
                let mut ground = commands.entity(anchor);
                ground.insert(GroundArea::burning(owner, dps, radius, duration, school));
                if spec.at == Aim::Follow {
                    ground.insert(Follow(owner));
                }
            }
            Effect::Chain { jumps, falloff } => {
                let Some((from, start, first)) = carrier(site, foes).filter(|_| fire.base > 0.0)
                else {
                    continue;
                };
                let mut struck = Vec::new();
                if let Some(i) = first {
                    hits.write(blow(owner, foes[i].e, fire.base, false, fire.effect));
                    struck.push(foes[i].e);
                }
                let path = target::chain(foes, from, start, usize::from(jumps), CHAIN_REACH);
                let mut dmg = fire.base;
                for (k, &i) in path.iter().enumerate() {
                    dmg *= falloff;
                    let hit = blow(owner, foes[i].e, dmg, false, fire.effect);
                    schedule(pending, hits, now, CHAIN_STEP * (k + 1) as f32, hit);
                    struck.push(foes[i].e);
                }
                show.on(spec.visual, &struck);
            }
            Effect::Split { count, spread } => {
                let Some((from, start, first)) = carrier(site, foes).filter(|_| fire.base > 0.0)
                else {
                    continue;
                };
                let mut struck = Vec::new();
                if let Some(i) = first {
                    hits.write(blow(owner, foes[i].e, fire.base, false, fire.effect));
                    struck.push(foes[i].e);
                }
                for i in target::nearest_n(foes, start, spread, usize::from(count), from) {
                    let delay = flat(foes[i].pos - start).length() / SHARD_SPEED;
                    let hit = blow(
                        owner,
                        foes[i].e,
                        fire.base * SPLIT_SHARE,
                        false,
                        fire.effect,
                    );
                    schedule(pending, hits, now, delay, hit);
                    struck.push(foes[i].e);
                }
                show.on(spec.visual, &struck);
            }
            Effect::Pierce { n } => {
                let Some((from, start, first)) = carrier(site, foes).filter(|_| fire.base > 0.0)
                else {
                    continue;
                };
                let mut struck = Vec::new();
                if let Some(i) = first {
                    hits.write(blow(owner, foes[i].e, fire.base, false, fire.effect));
                    struck.push(foes[i].e);
                }
                let line = target::pierce(
                    foes,
                    caster.pos,
                    start,
                    from,
                    usize::from(n),
                    PIERCE_WIDTH,
                    PIERCE_LENGTH,
                );
                for i in line {
                    let delay = flat(foes[i].pos - start).length() / SHARD_SPEED;
                    let hit = blow(
                        owner,
                        foes[i].e,
                        fire.base * PIERCE_SHARE,
                        false,
                        fire.effect,
                    );
                    schedule(pending, hits, now, delay, hit);
                    struck.push(foes[i].e);
                }
                show.on(spec.visual, &struck);
            }
            Effect::Boomerang => {
                let Some((from, start, first)) = carrier(site, foes).filter(|_| fire.base > 0.0)
                else {
                    continue;
                };
                let mut struck = Vec::new();
                if let Some(i) = first {
                    hits.write(blow(owner, foes[i].e, fire.base, false, fire.effect));
                    struck.push(foes[i].e);
                }
                for i in target::boomerang(foes, start, caster.pos, from, BOOMERANG_WIDTH) {
                    let delay = flat(foes[i].pos - start).length() / SHARD_SPEED;
                    let dmg = fire.base * BOOMERANG_SHARE;
                    schedule(
                        pending,
                        hits,
                        now,
                        delay,
                        blow(owner, foes[i].e, dmg, false, fire.effect),
                    );
                    struck.push(foes[i].e);
                }
                show.on(spec.visual, &struck);
            }
            Effect::Orbit {
                n,
                radius,
                duration,
            } => {
                if fire.base <= 0.0 {
                    continue;
                }
                for k in 0..n {
                    let angle = std::f32::consts::TAU * f32::from(k) / f32::from(n);
                    commands.spawn((
                        Transform::from_translation(caster.pos),
                        RunEntity,
                        Orbiter {
                            owner,
                            angle,
                            radius,
                            left: duration,
                            damage: fire.base,
                            effect: fire.effect,
                            visual: spec.visual,
                            struck: Vec::new(),
                        },
                    ));
                }
                show.on_hero(owner, spec.visual, &[]);
            }
            Effect::Execute { threshold, mult } => {
                let Some(i) = subject(site, foes) else {
                    continue;
                };
                let f = foes[i];
                let bonus = execute_bonus(f.hp, f.max_hp, fire.base, threshold, mult);
                if bonus > 0.0 {
                    hits.write(blow(owner, f.e, bonus, false, data::Effect::None));
                    show.on(spec.visual, &[f.e]);
                }
            }
            Effect::Dot { school, dps, secs } => {
                let Some(i) = subject(site, foes) else {
                    continue;
                };
                hits.write(Hit {
                    dot: caster.damage(dps, level, school),
                    dot_dur: secs,
                    school: Some(school),
                    ..blow(owner, foes[i].e, 0.0, false, data::Effect::None)
                });
                show.on(spec.visual, &[foes[i].e]);
            }
            Effect::ApplyCc { kind, secs } => {
                let Some(i) = subject(site, foes) else {
                    continue;
                };
                let effect = Cc { kind, secs }.effect();
                // The proc's own state kit (a stun's swirl, Frostbite's ice) marks the target.
                hits.write(Hit {
                    mark: spec.visual.unwrap_or(0),
                    mark_dur: secs,
                    ..blow(owner, foes[i].e, 0.0, false, effect)
                });
                show.on(spec.visual, &[foes[i].e]);
            }
            Effect::HealPulse { radius, amount } => {
                let amount = amount * level_scale(level);
                let mut total = 0.0;
                let mut healed = Vec::new();
                for &(e, pos) in party {
                    if flat(pos - site.pos).length() > radius {
                        continue;
                    }
                    let Ok((.., mut ally)) = heroes.get_mut(e) else {
                        continue;
                    };
                    if !ally.alive() {
                        continue;
                    }
                    let landed = party::heal(&mut ally, amount);
                    if landed > 0.0 {
                        show.fx.number(e, landed, false, COLOR_HEAL);
                    }
                    total += landed;
                    healed.push(e);
                }
                if let Ok((.., mut me)) = heroes.get_mut(owner) {
                    me.healing_done += f64::from(total);
                }
                show.area(owner, spec.visual, site, &healed);
            }
            Effect::PartyAura {
                radius,
                duration,
                bonuses,
            } => {
                let mut affected = Vec::new();
                for &(e, pos) in party {
                    if flat(pos - site.pos).length() > radius {
                        continue;
                    }
                    if let Ok((.., mut ally)) = heroes.get_mut(e) {
                        if ally.alive() {
                            burst(&mut ally, fire.uid, duration, bonuses);
                            affected.push(e);
                        }
                    }
                }
                show.area(owner, spec.visual, site, &affected);
            }
            Effect::BestAllyAura {
                radius,
                duration,
                bonuses,
            } => {
                let target = party
                    .iter()
                    .filter(|&&(_, pos)| flat(pos - site.pos).length() <= radius)
                    .filter_map(|&(e, _)| {
                        heroes
                            .get(e)
                            .ok()
                            .and_then(|(.., ally)| ally.alive().then_some((e, ally.damage_done)))
                    })
                    .max_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(e, _)| e);
                if let Some(target) = target {
                    if let Ok((.., mut ally)) = heroes.get_mut(target) {
                        burst(&mut ally, fire.uid, duration, bonuses);
                        show.on_hero(owner, spec.visual, &[target]);
                    }
                }
            }
            Effect::Ignite {
                dps,
                secs,
                per_burn,
            } => {
                let Some(i) = subject(site, foes) else {
                    continue;
                };
                let fed = 1.0 + per_burn * f32::from(foes[i].burns.min(MAX_FEED));
                hits.write(Hit {
                    dot: caster.damage(dps, level, School::Fire) * fed,
                    dot_dur: secs,
                    school: Some(School::Fire),
                    mark: spec.visual.unwrap_or(0),
                    mark_dur: secs,
                    ..blow(owner, foes[i].e, 0.0, false, data::Effect::None)
                });
                show.on(spec.visual, &[foes[i].e]);
            }
            Effect::Shatter { bonus } => {
                let Some(i) = subject(site, foes) else {
                    continue;
                };
                if foes[i].frozen && fire.base > 0.0 {
                    let e = foes[i].e;
                    hits.write(blow(owner, e, fire.base * bonus, true, data::Effect::None));
                    show.on(spec.visual, &[e]);
                }
            }
            Effect::ShareHeal {
                school,
                share,
                radius,
            } => {
                let pooled = match heroes.get_mut(owner) {
                    Ok((.., mut me)) => me.mechanics.drain_pool(school),
                    Err(_) => 0.0,
                };
                let amount = pooled * share;
                if amount <= 0.0 {
                    continue;
                }
                let mut total = 0.0;
                let mut healed = Vec::new();
                for &(e, pos) in party {
                    if flat(pos - site.pos).length() > radius {
                        continue;
                    }
                    let Ok((.., mut ally)) = heroes.get_mut(e) else {
                        continue;
                    };
                    if !ally.alive() {
                        continue;
                    }
                    let landed = party::heal(&mut ally, amount);
                    if landed > 0.0 {
                        show.fx.number(e, landed, false, COLOR_HEAL);
                        healed.push(e);
                    }
                    total += landed;
                }
                if let Ok((.., mut me)) = heroes.get_mut(owner) {
                    me.healing_done += f64::from(total);
                }
                show.on(spec.visual, &healed);
            }
            // The hero's own, landed above; Conflagrate's, after the rest.
            Effect::Burst { .. }
            | Effect::Shield { .. }
            | Effect::CheatDeath { .. }
            | Effect::ResetCooldowns
            | Effect::Echo { .. }
            | Effect::NextCast { .. }
            | Effect::TotemReach { .. }
            | Effect::ConsumeDot { .. } => {}
        }
    }
}

/// The hero's own effects, which land on the hero whatever the aim: whether `fire` was one.
fn own(
    fire: &Fire,
    heroes: &mut Query<(Entity, &Transform, &mut HeroState)>,
    now: f32,
    level: u32,
    show: &mut Show,
) -> bool {
    let owner = fire.owner;
    let visual = fire.spec.visual;
    match fire.spec.effect {
        Effect::Burst { duration, bonuses } => {
            if let Ok((.., mut hero)) = heroes.get_mut(owner) {
                burst(&mut hero, fire.uid, duration, bonuses);
                show.on_hero(owner, visual, &[owner]);
            }
        }
        Effect::Shield { amount, duration } => {
            let target = party::support_target(owner);
            if let Ok((.., mut hero)) = heroes.get_mut(target) {
                party::shield(&mut hero, amount * level_scale(level), duration);
                show.on_hero(target, visual, &[target]);
            }
        }
        Effect::CheatDeath { immune, heal_frac } => {
            if let Ok((.., mut hero)) = heroes.get_mut(owner) {
                cheat_death(&mut hero, immune, heal_frac);
                show.on_hero(owner, visual, &[owner]);
            }
        }
        Effect::ResetCooldowns => {
            if let Ok((.., mut hero)) = heroes.get_mut(owner) {
                for o in &mut hero.abilities {
                    o.cd = o.cd.min(0.0);
                }
                show.on_hero(owner, visual, &[owner]);
            }
        }
        Effect::Echo { extra_casts } => {
            if let (Some(ability), Ok((.., mut hero))) = (fire.ability, heroes.get_mut(owner)) {
                hero.mechanics
                    .echo(ability.key, now + ECHO_GAP, extra_casts);
            }
        }
        Effect::NextCast {
            instant,
            crit,
            cooldown,
            aura,
        } => {
            if let Ok((.., mut hero)) = heroes.get_mut(owner) {
                arm(
                    &mut hero,
                    instant,
                    Armed {
                        crit,
                        cooldown,
                        aura,
                    },
                );
                show.on_hero(owner, visual, &[owner]);
                // The buff it leaves rings in as it goes up: Clearcasting's flash and chime.
                if aura != 0 && Some(aura) != visual {
                    show.on(Some(aura), &[owner]);
                }
            }
        }
        // A standing property of the hero, read where its totems go down.
        Effect::TotemReach { .. } => {}
        _ => return false,
    }
    true
}

/// A next-cast guarantee armed on `hero`; `instant` readies at once the ability furthest from
/// ready, as Presence of Mind makes the next spell instant.
pub(crate) fn arm(hero: &mut HeroState, instant: bool, armed: Armed) {
    hero.mechanics.arm(armed);
    if instant {
        if let Some(o) = hero
            .abilities
            .iter_mut()
            .max_by(|a, b| a.cd.total_cmp(&b.cd))
        {
            o.cd = o.cd.min(0.0);
        }
    }
}

/// A burst's bonuses on `hero` for `duration`: refreshed if mechanic `uid`'s is running already.
pub(crate) fn burst(hero: &mut HeroState, uid: u32, duration: f32, bonuses: &'static [Bonus]) {
    if let Some(running) = hero.mechanics.bursts.iter_mut().find(|b| b.uid == uid) {
        running.left = running.left.max(duration);
        return;
    }
    hero.bonuses.add_all(bonuses);
    hero.mechanics.bursts.push(Running {
        uid,
        left: duration,
        bonuses,
    });
}

/// Ground that follows its hero keeps under it.
pub(crate) fn follow_owners(
    mut grounds: Query<(&mut Transform, &Follow), Without<HeroState>>,
    heroes: Query<&Transform, With<HeroState>>,
) {
    for (mut tf, follow) in &mut grounds {
        if let Ok(hero) = heroes.get(follow.0) {
            tf.translation = hero.translation;
        }
    }
}

/// Motes circle their heroes, striking what they touch, until they run out.
#[allow(clippy::type_complexity)]
pub(crate) fn spin_orbiters(
    mut commands: Commands,
    time: Res<Time>,
    mut rng: ResMut<Rng>,
    mut orbiters: Query<
        (Entity, &mut Transform, &mut Orbiter),
        (Without<HeroState>, Without<Enemy>),
    >,
    heroes: Query<(&Transform, &HeroState)>,
    enemies: Query<(Entity, &Transform, &Enemy), Without<HeroState>>,
    mut fx: Fx,
    mut hits: MessageWriter<Hit>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    for (e, mut tf, mut mote) in &mut orbiters {
        mote.left -= dt;
        let Ok((htf, hero)) = heroes.get(mote.owner) else {
            commands.entity(e).despawn();
            continue;
        };
        if mote.left <= 0.0 || !hero.alive() {
            commands.entity(e).despawn();
            continue;
        }
        mote.angle = (mote.angle + ORBIT_SPEED * dt) % std::f32::consts::TAU;
        let at = htf.translation + Vec3::new(mote.angle.cos(), 0.0, mote.angle.sin()) * mote.radius;
        tf.translation = at;
        mote.struck.retain_mut(|(_, t)| {
            *t -= dt;
            *t > 0.0
        });
        let stats = hero.stats();
        for (foe, ftf, en) in &enemies {
            if en.dead
                || flat(ftf.translation - at).length() > ORBIT_TOUCH + en.radius * 0.5
                || mote.struck.iter().any(|(s, _)| *s == foe)
            {
                continue;
            }
            let (dmg, crit) = roll(&mut rng, &stats, mote.damage);
            hits.write(blow(mote.owner, foe, dmg, crit, mote.effect));
            if let Some(spell) = mote.visual {
                fx.impact(foe, spell);
            }
            mote.struck.push((foe, ORBIT_REHIT));
        }
    }
}
