//! A spec's summons and forms in the world ([`super::spec`]).
//!
//! - **Guardians.** Each hero carrying a [`HeroSpec`] whose summon is a guardian has its
//!   [`GuardianDef::count`] of them ([`spawn_guardians`]): host-simulated units like the
//!   companions, run entities that go with the run. They keep to their owner's side, and every
//!   [`GuardianDef::every`] seconds act ([`run_guardians`]): a melee guardian runs down the nearest
//!   enemy near its owner and strikes it, a caster flies its spell at the nearest in range, a
//!   healer (a totem, the Lightwell) stands planted and heals its owner while in reach, set down
//!   afresh once its owner has gone past it. Every blow and heal is the owner's, as a companion's
//!   bite is.
//! - **Shapeshifts.** A form with a model wears it on its hero for the run ([`wear_forms`]): the
//!   Night Elf's or the Tauren's, as the hero's race is. Its state kit shows through the hero's
//!   buffs (`hero::hero_vitals`).

use benilla_world::world_point::WorldPoint;
use bevy::prelude::*;

use super::super::abilities::Hit;
use super::super::enemies::{self, Enemy};
use super::super::hero::Hero;
use super::super::mechanics::level_scale;
use super::super::units::{self, flat, yaw_toward, Fx, UnitSpec, COLOR_SPELL};
use super::super::{party, HeroState, Run};
use super::spec::{Act, FormDef, GuardianDef, HeroSpec, Summon};
use crate::creature_anim::{move_flags, MovementState};
use crate::net::{NetEntity, ObjectStore};

/// How near its owner a guardian settles, and how fast it may run.
const HEEL: f32 = 3.0;
const DASH_SPEED: f32 = 14.0;
/// Farther than this from its owner (after a Blink, say), it appears at the owner's side.
const LEASH: f32 = 30.0;
/// A melee guardian strikes within this of an enemy's edge.
const REACH: f32 = 2.5;
/// The races whose shapeshift models a form wears: Night Elf, then Tauren.
const NIGHT_ELF: u8 = 4;
const TAUREN: u8 = 6;

/// One of a hero's spec guardians.
#[derive(Component)]
pub(in crate::survivors) struct SpecGuardian {
    pub(in crate::survivors) owner: Entity,
    def: &'static GuardianDef,
    /// 0.. within its owner's [`GuardianDef::count`].
    slot: u8,
    /// Seconds to its next act.
    next: f32,
}

/// Where `slot` of `count` keeps, around its owner: spread in a half ring behind.
fn heel(slot: u8, count: u8) -> Vec3 {
    let n = f32::from(count.max(1));
    let a = std::f32::consts::PI * (0.5 + (f32::from(slot) + 0.5) / n);
    Vec3::new(a.cos(), 0.0, a.sin()) * HEEL
}

/// The guardian `def` an owner's spec summons, if its summon is one.
fn guardian_of(spec: &HeroSpec) -> Option<&'static GuardianDef> {
    match &spec.0.summon {
        Summon::Guardian(g) => Some(g),
        Summon::Form(_) => None,
    }
}

/// The shapeshift model `form` wears on a hero of `race`, if it has one for the race.
pub(in crate::survivors) fn shape_display(form: &FormDef, race: u8) -> Option<u32> {
    let (elf, tauren) = form.shape?;
    match race {
        NIGHT_ELF => Some(elf),
        TAUREN => Some(tauren),
        _ => None,
    }
}

/// Every spec hero has its guardians, and no guardian outlives its owner's spec.
#[allow(clippy::type_complexity)]
pub(super) fn spawn_guardians(
    mut commands: Commands,
    mut run: ResMut<Run>,
    heroes: Query<(Entity, &Transform, Option<&HeroSpec>, Option<&NetEntity>), With<Hero>>,
    guardians: Query<(Entity, &SpecGuardian)>,
) {
    for (e, g) in &guardians {
        let kept = heroes
            .get(g.owner)
            .ok()
            .and_then(|(.., spec, _)| spec)
            .and_then(guardian_of)
            .is_some_and(|def| std::ptr::eq(def, g.def) && g.slot < def.count);
        if !kept {
            commands.entity(e).despawn();
        }
    }
    for (owner, tf, spec, net) in &heroes {
        let Some(def) = spec.and_then(guardian_of) else {
            continue;
        };
        for slot in 0..def.count {
            if guardians
                .iter()
                .any(|(_, g)| g.owner == owner && g.slot == slot)
            {
                continue;
            }
            // A shadow of its owner wears the owner's own body.
            let display = match def.display {
                0 => net.and_then(|n| n.display_id).unwrap_or(0),
                d => d,
            };
            if display == 0 {
                continue;
            }
            let pos = tf.translation + heel(slot, def.count);
            let e = if def.object {
                units::spawn_object(&mut commands, run.guid(), display, def.scale, pos)
            } else {
                units::spawn_unit(
                    &mut commands,
                    &UnitSpec {
                        serial: run.guid(),
                        display,
                        scale: def.scale,
                        level: 60,
                        faction: units::FACTION_FRIENDLY,
                        bytes0: 1 << 8,
                        weapons: &[],
                        sheath: 0,
                    },
                    pos,
                    yaw_toward(tf.translation - pos),
                )
            };
            commands.entity(e).insert(SpecGuardian {
                owner,
                def,
                slot,
                next: def.every,
            });
        }
    }
}

/// Follow, hunt, strike, cast and heal: see the module doc.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn run_guardians(
    time: Res<Time>,
    run: Res<Run>,
    world: WorldPoint,
    mut heroes: Query<(Entity, &Transform, &mut HeroState), (With<Hero>, Without<SpecGuardian>)>,
    mut guardians: Query<
        (
            Entity,
            &mut Transform,
            Option<&mut MovementState>,
            &mut SpecGuardian,
        ),
        (Without<Hero>, Without<Enemy>),
    >,
    foes: Query<(Entity, &Transform, &Enemy), (Without<Hero>, Without<SpecGuardian>)>,
    mut hits: MessageWriter<Hit>,
    mut fx: Fx,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    for (me, mut tf, mv, mut g) in &mut guardians {
        let Ok((_, h, owner)) = heroes.get_mut(g.owner) else {
            continue;
        };
        let def = g.def;
        let hpos = h.translation;
        let pos = tf.translation;
        let spot = hpos + heel(g.slot, def.count);
        let away = flat(hpos - pos).length();
        let planted = def.act == Act::Heal;
        if away > LEASH || (planted && away > def.range) {
            tf.translation = enemies::ground(&world, spot, hpos.y);
            continue;
        }
        let down = !owner.alive();
        let stats = owner.stats();
        let power =
            def.amount * level_scale(run.level) * stats.might * owner.school_mult(def.school);
        // The nearest enemy standing, from `from`, within `reach` of `around`.
        let nearest = |from: Vec3, around: Vec3, reach: f32| {
            foes.iter()
                .filter(|(_, at, en)| !en.dead && flat(at.translation - around).length() <= reach)
                .map(|(e, at, en)| {
                    (
                        e,
                        at.translation,
                        flat(at.translation - from).length() - en.radius,
                    )
                })
                .min_by(|a, b| a.2.total_cmp(&b.2))
        };
        g.next = (g.next - dt).max(0.0);
        let mut dest = (
            spot,
            (stats.speed * 1.2).max(flat(spot - pos).length() * 2.0),
        );
        // No guardian action needs the owner's mutable state until a healer credits its work.
        // Releasing this borrow lets a Lightwell/Healing Stream reach every party member.
        let _ = h;
        let _ = owner;
        match def.act {
            Act::Melee if !down => {
                if let Some((foe, at, gap)) = nearest(pos, hpos, def.range) {
                    if gap <= REACH {
                        dest = (pos, 0.0);
                        let face = Quat::from_rotation_y(yaw_toward(at - pos));
                        tf.rotation = tf.rotation.slerp(face, (dt * 10.0).min(1.0));
                        if g.next <= 0.0 {
                            g.next = def.every;
                            fx.swing(me, foe, power);
                            if def.spell != 0 {
                                fx.impact(foe, def.spell);
                            }
                            hits.write(Hit::direct(g.owner, foe, power, false));
                        }
                    } else {
                        dest = (at, DASH_SPEED);
                    }
                }
            }
            Act::Bolt if !down && g.next <= 0.0 => {
                if let Some((foe, ..)) = nearest(pos, pos, def.range) {
                    g.next = def.every;
                    fx.cast(me, def.spell, &[foe], None);
                    hits.write(Hit {
                        color: COLOR_SPELL,
                        ..Hit::direct(g.owner, foe, power, false)
                    });
                }
            }
            Act::Heal if !down && g.next <= 0.0 && away <= def.range => {
                g.next = def.every;
                let mut healed = 0.0;
                let mut targets = Vec::new();
                for (ally, pos, mut state) in heroes.iter_mut() {
                    if flat(pos.translation - tf.translation).length() <= def.range {
                        healed += party::heal(&mut state, power);
                        targets.push(ally);
                    }
                }
                if let Ok((.., mut owner)) = heroes.get_mut(g.owner) {
                    owner.healing_done += f64::from(healed);
                }
                fx.cast(me, def.spell, &targets, None);
            }
            _ => {}
        }
        let Some(mut mv) = mv.filter(|_| !planted) else {
            continue;
        };
        let (to, speed) = dest;
        let step = flat(to - pos);
        let dist = step.length();
        if speed > 0.0 && dist > 0.3 {
            let dir = step / dist;
            let next = pos + dir * (speed.min(DASH_SPEED) * dt).min(dist);
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

/// A shapeshift form's hero wears the form's model while it plays, as its race has one.
pub(super) fn wear_forms(
    mut heroes: Query<(&HeroState, &HeroSpec, &mut NetEntity, &mut ObjectStore), With<Hero>>,
) {
    for (hero, spec, mut net, mut store) in &mut heroes {
        let Summon::Form(form) = &spec.0.summon else {
            continue;
        };
        if let Some(display) = shape_display(form, hero.race) {
            super::super::gear::wear(display, &mut net, &mut store);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::spec::{by_talent, SPECS};
    use super::*;

    #[test]
    fn guardians_keep_apart_behind_their_owner() {
        for count in 1..=3u8 {
            let spots: Vec<Vec3> = (0..count).map(|s| heel(s, count)).collect();
            for (i, a) in spots.iter().enumerate() {
                assert!((a.length() - HEEL).abs() < 1e-4);
                for b in &spots[i + 1..] {
                    assert!(a.distance(*b) > 1.0, "{count}: {a} and {b}");
                }
            }
        }
    }

    #[test]
    fn shapeshifts_wear_the_races_model() {
        let Summon::Form(cat) = &by_talent(809).unwrap().summon else {
            panic!("Feral is a form")
        };
        assert_eq!(shape_display(cat, NIGHT_ELF), Some(892));
        assert_eq!(shape_display(cat, TAUREN), Some(8571));
        assert_eq!(shape_display(cat, 1), None, "only druids' races shift");
        let shifts = SPECS
            .iter()
            .filter(|s| matches!(&s.summon, Summon::Form(f) if f.shape.is_some()))
            .count();
        assert_eq!(shifts, 2, "Moonkin and Cat");
    }

    #[test]
    fn party_healing_guardians_have_group_sized_reach() {
        for talent in [1637, 590] {
            let Summon::Guardian(def) = &by_talent(talent).unwrap().summon else {
                panic!("{talent} is a guardian");
            };
            assert_eq!(def.act, Act::Heal);
            assert!(def.range >= 15.0, "{talent} must reach nearby allies");
        }
    }
}
