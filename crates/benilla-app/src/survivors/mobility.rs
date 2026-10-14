//! The class's mobility spell: learned at [`LEARN_LEVEL`], cast only on `F`, never by itself. Each
//! is the class's own signature escape: a mage blinks, a rogue sprints, a warrior charges.
//!
//! Where an aimed one goes depends on the view ([`aiming`]): the classic view aims at the cursor;
//! WoW's goes where the hero faces, as its own Blink and Charge do. A dash walks the ground
//! through the hero's collision ([`body::dash`]), so it stops at, or slides along, what is in the
//! way.

use benilla_world::collision::WorldCollision;
use bevy::prelude::*;

use super::body;
use super::enemies::Enemy;
use super::hero::{self, Hero};
use super::units::{flat, Fx};
use super::view::{self, View};
use super::wowcam;
use super::{HeroState, LocalHero, Phase, Run};

pub(super) const LEARN_LEVEL: u32 = 5;
pub(super) const KEY: KeyCode = KeyCode::KeyF;
/// Charge's cone in WoW's view: an enemy within this angle (radians) of the facing, either side.
const CHARGE_CONE: f32 = 60.0 * std::f32::consts::PI / 180.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Move {
    /// Jump this many yards along the way the hero faces.
    Blink(f32),
    /// Rush at the nearest enemy within this many yards, stunning it; else lunge that far.
    Charge(f32),
    /// Run faster by this factor for `secs`.
    Haste(f32),
    /// Haste, and enemies lose the hero (they stop chasing) for `secs`.
    Vanish(f32),
    /// Haste, and every enemy within this many yards flees in fear.
    Scatter(f32),
}

pub(super) struct Mobility {
    pub spell: u32,
    pub name: &'static str,
    pub kind: Move,
    pub cooldown: f32,
    pub secs: f32,
    pub desc: &'static str,
}

/// One per class, indexed by class id.
pub(super) fn for_class(class: u8) -> Option<&'static Mobility> {
    MOBILITY.iter().find(|(c, _)| *c == class).map(|(_, m)| m)
}

static MOBILITY: &[(u8, Mobility)] = &[
    (
        1,
        Mobility {
            spell: 100,
            name: "Charge",
            kind: Move::Charge(25.0),
            cooldown: 8.0,
            secs: 1.5,
            desc: "Charge the nearest enemy within 25 yards, stunning it.",
        },
    ),
    (
        2,
        Mobility {
            spell: 1044,
            name: "Blessing of Freedom",
            kind: Move::Haste(1.6),
            cooldown: 15.0,
            secs: 5.0,
            desc: "Nothing can hold you: run 60% faster for 5 sec.",
        },
    ),
    (
        3,
        Mobility {
            spell: 5118,
            name: "Aspect of the Cheetah",
            kind: Move::Haste(1.7),
            cooldown: 14.0,
            secs: 5.0,
            desc: "Run 70% faster for 5 sec.",
        },
    ),
    (
        4,
        Mobility {
            spell: 2983,
            name: "Sprint",
            kind: Move::Haste(1.8),
            cooldown: 15.0,
            secs: 4.0,
            desc: "Run 80% faster for 4 sec.",
        },
    ),
    (
        5,
        Mobility {
            spell: 586,
            name: "Fade",
            kind: Move::Vanish(1.4),
            cooldown: 18.0,
            secs: 4.0,
            desc: "Fade from the horde's sight: enemies lose you and you run 40% faster for 4 sec.",
        },
    ),
    (
        7,
        Mobility {
            spell: 2645,
            name: "Ghost Wolf",
            kind: Move::Haste(1.6),
            cooldown: 15.0,
            secs: 6.0,
            desc: "Take the form of a ghost wolf, running 60% faster for 6 sec.",
        },
    ),
    (
        8,
        Mobility {
            spell: 1953,
            name: "Blink",
            kind: Move::Blink(18.0),
            cooldown: 10.0,
            secs: 0.0,
            desc: "Teleport 18 yards forward.",
        },
    ),
    (
        9,
        Mobility {
            spell: 5484,
            name: "Howl of Terror",
            kind: Move::Scatter(12.0),
            cooldown: 20.0,
            secs: 4.0,
            desc: "Howl, sending every enemy within 12 yards fleeing in terror for 4 sec.",
        },
    ),
    (
        11,
        Mobility {
            spell: 1850,
            name: "Dash",
            kind: Move::Haste(1.8),
            cooldown: 15.0,
            secs: 4.0,
            desc: "Run 80% faster for 4 sec.",
        },
    ),
];

/// The mobility's run state, on each [`HeroState`]: cooldown left, and the haste or vanish still
/// running.
#[derive(Default, Clone, Copy)]
pub(super) struct MobilityState {
    pub cd: f32,
    pub haste: f32,
    pub haste_t: f32,
    pub vanish_t: f32,
}

impl HeroState {
    /// The class's mobility once the party's `level` is high enough.
    pub(super) fn mobility(&self, level: u32) -> Option<&'static Mobility> {
        (level >= LEARN_LEVEL)
            .then(|| for_class(self.class.id))
            .flatten()
    }

    /// The run speed's multiplier from a haste still running.
    pub(super) fn haste_mult(&self) -> f32 {
        if self.mobile.haste_t > 0.0 {
            self.mobile.haste
        } else {
            1.0
        }
    }
}

/// Where an aimed mobility heads: the flat unit `toward`, and the cursor's ground when it aims
/// there.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct Aiming {
    pub(super) toward: Vec3,
    pub(super) cursor: Option<Vec3>,
}

/// The aim for `kind` cast from `pos` by a hero facing `facing`, the cursor's ground at `cursor`.
/// The classic view aims at the cursor where there is one. WoW's view sends Blink and Charge
/// where the hero faces; any other aimed move takes the cursor only while it is free (not
/// mouselooking) and over ground, else the facing too.
pub(super) fn aiming(
    kind: Move,
    wow: bool,
    pos: Vec3,
    facing: Vec3,
    cursor: Option<Vec3>,
    mouselook: bool,
) -> Aiming {
    let cursor = cursor.filter(|a| flat(*a - pos).length() > 0.5);
    let cursor = match kind {
        _ if !wow => cursor,
        Move::Blink(_) | Move::Charge(_) => None,
        _ => cursor.filter(|_| !mouselook),
    };
    Aiming {
        toward: cursor.map_or(facing, |a| flat(a - pos).normalize()),
        cursor,
    }
}

/// Charge's mark among `foes` (each standing enemy and where it is): the nearest to `mark` within
/// `reach` of the hero at `pos`. With a `cone` (WoW's view: the facing) only those ahead count,
/// within [`CHARGE_CONE`] of it.
pub(super) fn charge_target(
    foes: impl IntoIterator<Item = (Entity, Vec3)>,
    pos: Vec3,
    reach: f32,
    mark: Vec3,
    cone: Option<Vec3>,
) -> Option<(Entity, Vec3)> {
    let cos = CHARGE_CONE.cos();
    foes.into_iter()
        .filter(|&(_, at)| hero::reach(pos, at) <= reach)
        .filter(|&(_, at)| {
            cone.is_none_or(|ahead| {
                let d = flat(at - pos);
                d.length() < 1e-3 || d.dot(ahead) >= cos * d.length()
            })
        })
        .map(|(e, at)| (e, at, hero::reach(mark, at)))
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(e, at, _)| (e, at))
}

/// `F`: the local hero casts the mobility if it is learned and ready; every hero's cooldown and
/// running effect tick.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn cast_mobility(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<Phase>>,
    run: Res<Run>,
    world: (WorldCollision, Res<crate::player::PlayerCapsule>),
    looks: (
        Res<View>,
        Option<Res<super::auto::Auto>>,
        Res<crate::player::camera::CameraControl>,
    ),
    mut heroes: Query<(
        Entity,
        &mut Transform,
        &mut HeroState,
        &mut Hero,
        Has<LocalHero>,
    )>,
    mut enemies: Query<(Entity, &Transform, &mut Enemy), Without<Hero>>,
    selection: Res<super::Selection>,
    aim: Res<hero::AimPoint>,
    mut fx: Fx,
) {
    let dt = time.delta_secs();
    for (_, _, mut hero, ..) in &mut heroes {
        let m = &mut hero.mobile;
        m.cd = (m.cd - dt).max(0.0);
        m.haste_t = (m.haste_t - dt).max(0.0);
        m.vanish_t = (m.vanish_t - dt).max(0.0);
    }
    if *phase.get() != Phase::Playing || !keys.just_pressed(KEY) {
        return;
    }
    let Some((me, mut tf, mut state, mut hm, _)) = heroes.iter_mut().find(|(.., local)| *local)
    else {
        return;
    };
    if state.dead_t.is_some() {
        return;
    }
    let Some(def) = state.mobility(run.level) else {
        return;
    };
    if state.mobile.cd > 0.0 {
        return;
    }
    let (collide, capsule) = (&world.0, &world.1 .0);
    let wow = view::wow_in_play(*phase.get(), looks.0.mode, looks.1.is_some());
    // From the body's own feet: the creature clamp may have re-seated the transform's height.
    let pos = hm.body.start(tf.translation);
    let facing = if wow {
        wowcam::facing(hm.face)
    } else {
        flat(tf.rotation * Vec3::NEG_Z).normalize_or(Vec3::NEG_Z)
    };
    let Aiming { toward, cursor } = aiming(
        def.kind,
        wow,
        pos,
        facing,
        aim.0,
        wowcam::mouselooking(&looks.2),
    );
    // Keep any jump inside the battleground's leash.
    let map = selection.map_def();
    let center = hero::arena(map);
    let leash = |p: Vec3| {
        let off = flat(p - center);
        if off.length() > map.leash {
            center + off.normalize() * map.leash
        } else {
            p
        }
    };
    // Along the ground to `to`, stopping at or sliding along what stands in the way.
    let dash_to = |to: Vec3, tf: &mut Transform, hm: &mut Hero| {
        let feet = body::dash(collide, capsule, pos, to - pos);
        tf.translation = feet;
        hm.body.placed(feet);
    };
    match def.kind {
        Move::Blink(yd) => {
            fx.cast(me, def.spell, &[me], None);
            // To the cursor, or as far as the spell reaches toward it.
            let far = cursor.map_or(yd, |a| flat(a - pos).length().min(yd));
            tf.rotation = Quat::from_rotation_y(super::units::yaw_toward(toward));
            dash_to(leash(pos + toward * far), &mut tf, &mut hm);
        }
        Move::Charge(yd) => {
            // The enemy in reach nearest the cursor (or the hero, with none); in WoW's view, the
            // nearest ahead.
            let mark = cursor.unwrap_or(pos);
            let foes = enemies
                .iter()
                .filter(|(_, _, e)| !e.dead)
                .map(|(e, t, _)| (e, t.translation));
            let to = match charge_target(foes, pos, yd, mark, wow.then_some(facing)) {
                Some((e, at)) => {
                    if let Ok((_, _, mut en)) = enemies.get_mut(e) {
                        en.stun_t = en.stun_t.max(def.secs);
                    }
                    let dir = flat(at - pos).normalize_or(facing);
                    hm.face = super::units::yaw_toward(dir);
                    tf.rotation = Quat::from_rotation_y(hm.face);
                    fx.cast(me, def.spell, &[e], None);
                    at - dir * 1.5
                }
                None => {
                    fx.cast(me, def.spell, &[me], None);
                    tf.rotation = Quat::from_rotation_y(super::units::yaw_toward(toward));
                    pos + toward * yd * 0.5
                }
            };
            dash_to(leash(to), &mut tf, &mut hm);
        }
        Move::Haste(x) => {
            fx.cast(me, def.spell, &[me], None);
            state.mobile.haste = x;
            state.mobile.haste_t = def.secs;
        }
        Move::Vanish(x) => {
            fx.cast(me, def.spell, &[me], None);
            state.mobile.haste = x;
            state.mobile.haste_t = def.secs;
            state.mobile.vanish_t = def.secs;
        }
        Move::Scatter(r) => {
            fx.cast(me, def.spell, &[me], None);
            for (_, t, mut en) in &mut enemies {
                if !en.dead && !en.boss && hero::reach(pos, t.translation) <= r {
                    en.fear_t = en.fear_t.max(def.secs);
                }
            }
        }
    }
    state.mobile.cd = def.cooldown;
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT: Vec3 = Vec3::new(10.0, 2.0, 10.0);
    const NORTH: Vec3 = Vec3::NEG_Z;

    #[test]
    fn the_classic_view_aims_at_the_cursor() {
        let cursor = Some(AT + Vec3::new(6.0, 0.5, 0.0));
        for kind in [Move::Blink(18.0), Move::Charge(25.0), Move::Haste(1.5)] {
            let a = aiming(kind, false, AT, NORTH, cursor, false);
            assert_eq!(a.cursor, cursor, "{kind:?}");
            assert!(a.toward.distance(Vec3::X) < 1e-5, "{kind:?}: {a:?}");
        }
        // Under the hero's feet is no aim: the facing.
        let a = aiming(Move::Blink(18.0), false, AT, NORTH, Some(AT), false);
        assert_eq!((a.toward, a.cursor), (NORTH, None));
    }

    #[test]
    fn wow_blink_and_charge_go_where_the_hero_faces() {
        let cursor = Some(AT + Vec3::new(6.0, 0.0, 0.0));
        for kind in [Move::Blink(18.0), Move::Charge(25.0)] {
            for mouselook in [false, true] {
                let a = aiming(kind, true, AT, NORTH, cursor, mouselook);
                assert_eq!((a.toward, a.cursor), (NORTH, None), "{kind:?}");
            }
        }
    }

    #[test]
    fn wow_other_moves_take_a_free_cursor_over_ground() {
        let cursor = Some(AT + Vec3::new(0.0, 0.0, 6.0));
        let free = aiming(Move::Haste(1.5), true, AT, NORTH, cursor, false);
        assert_eq!(free.cursor, cursor);
        assert!(free.toward.distance(Vec3::Z) < 1e-5);
        // Mouselooking, or off the ground: the facing.
        let looking = aiming(Move::Haste(1.5), true, AT, NORTH, cursor, true);
        assert_eq!((looking.toward, looking.cursor), (NORTH, None));
        let off = aiming(Move::Scatter(12.0), true, AT, NORTH, None, false);
        assert_eq!((off.toward, off.cursor), (NORTH, None));
    }

    #[test]
    fn wow_charge_takes_the_nearest_enemy_ahead() {
        let e = |i: u32| Entity::from_raw_u32(i).unwrap();
        let foes = [
            // Close, but behind and beside the hero.
            (e(1), AT + Vec3::new(0.0, 0.0, 3.0)),
            (e(2), AT + Vec3::new(4.0, 0.0, 0.0)),
            // Ahead: the nearer is picked.
            (e(3), AT + Vec3::new(2.0, 0.0, -12.0)),
            (e(4), AT + Vec3::new(-1.0, 0.0, -8.0)),
            // Ahead but out of reach.
            (e(5), AT + Vec3::new(0.0, 0.0, -40.0)),
        ];
        let pick = charge_target(foes, AT, 25.0, AT, Some(NORTH));
        assert_eq!(pick.map(|p| p.0), Some(e(4)));
        // The classic view takes the nearest anywhere in reach.
        let pick = charge_target(foes, AT, 25.0, AT, None);
        assert_eq!(pick.map(|p| p.0), Some(e(1)));
        // Nothing ahead: no mark, so the charge lunges forward.
        assert_eq!(
            charge_target(foes[..2].to_vec(), AT, 25.0, AT, Some(NORTH)),
            None
        );
        // The cone's edge: 59° in, 61° out.
        let at = |deg: f32| {
            let r = deg.to_radians();
            AT + Vec3::new(r.sin(), 0.0, -r.cos()) * 10.0
        };
        assert!(charge_target([(e(6), at(59.0))], AT, 25.0, AT, Some(NORTH)).is_some());
        assert!(charge_target([(e(6), at(61.0))], AT, 25.0, AT, Some(NORTH)).is_none());
    }
}
