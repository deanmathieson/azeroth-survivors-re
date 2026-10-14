//! A hero's body against the world: the walk step benilla's own movers take
//! ([`crate::player::mover::grounded_step`]: step up, slide, step down) and its airborne slide, over
//! the same avian colliders (terrain, doodads, WMOs) the loading screen counts. A hero slides along
//! trees and walls, climbs kerbs and never tunnels through anything, at any speed or dash length.
//!
//! The remote dead-reckon and the creature clamp walk through the same two steps from outside the
//! controller; only the glue is here: the jump arc, the landing probe, and the terrain heightfield
//! as the floor while colliders are still streaming in.

use std::time::Duration;

use avian3d::prelude::Collider;
use benilla_world::collision::WorldCollision;
use bevy::prelude::*;

use crate::player::mover::{airborne_step, fall_step, grounded_step, Support};
use crate::player::{CAPSULE_HEIGHT, TERMINAL_VELOCITY};

/// The jump's take-off speed (yd/s), the reference `CMovement` seed (`player/state.rs`'s
/// `JUMP_SPEED`).
const JUMP_SPEED: f32 = 7.955_547;
/// Walkable iff the floor's normal is within 50° of up (`player/state.rs`'s `GROUND_COS`).
const GROUND_COS: f32 = 0.642_788;
/// How close the floor must be (yd) for a fall to land (`player/state.rs`'s `LAND_PROBE`).
const LAND_PROBE: f32 = 0.05;
/// The casts' skin (yd) between the capsule and geometry (`player/state.rs`'s `SKIN_WIDTH`).
const SKIN_WIDTH: f32 = 0.02;
/// How far down (yd) a collider floor is looked for before the terrain heightfield stands in.
const FLOOR_SEARCH: f32 = 64.0;
/// The longest walk (yd) one step resolves: a hitch's long frame or a dash runs in pieces this
/// size, so the step-up and the step-down judge each ledge on its own.
const MAX_STEP: f32 = 0.5;
/// A dash piece that gains less than this share of its length along the way is blocked.
const BLOCKED: f32 = 0.1;
/// The one steer a standing jump allows (yd/s), the default walk speed (`player/state.rs`'s
/// `AIR_NUDGE_SPEED`).
const AIR_NUDGE_SPEED: f32 = 2.5;
/// A fall stalls when it achieves under this share of gravity's descent while faster than
/// [`WEDGE_MIN_FALL`]; [`WEDGE_FRAMES`] such frames rest the body there, pinched between steep
/// faces (`player/state.rs`'s `WEDGE_*`).
const WEDGE_STALL_RATIO: f32 = 0.15;
const WEDGE_MIN_FALL: f32 = 1.0;
const WEDGE_FRAMES: u8 = 3;

/// The capsule's centre over its feet.
const HALF: Vec3 = Vec3::new(0.0, CAPSULE_HEIGHT * 0.5, 0.0);

/// What a body carries between frames.
#[derive(Default, Clone, Copy, Debug)]
pub(super) struct Body {
    /// Where the last step left the feet. The creature clamp re-seats every unit's height at the
    /// top of the frame, so a step starts here, not from the transform, while nothing else has
    /// moved the hero across the ground.
    pub(super) feet: Option<Vec3>,
    /// Vertical speed (yd/s), up positive; 0 on the ground.
    pub(super) vy: f32,
    pub(super) airborne: bool,
    /// On a certified steep contact: the step-down keeps its deep reach next frame.
    steep: bool,
    /// The flat velocity a locked jump keeps, latched as it leaves the ground.
    launch: Vec3,
    /// A standing jump's one steer is spent.
    nudged: bool,
    /// Stalled falling frames in a row.
    stalled: u8,
}

impl Body {
    /// Standing where `feet` is, after a teleport, a dash or a respawn.
    pub(super) fn placed(&mut self, feet: Vec3) {
        *self = Body {
            feet: Some(feet),
            ..Body::default()
        };
    }

    /// The flat velocity to step with this frame for `input`. On the ground it is the input,
    /// latched for a take-off; airborne and `locked` (WoW's jump), the launch's, which a standing
    /// jump may steer once at walking pace; unlocked, the input steers in the air.
    pub(super) fn air_velocity(&mut self, input: Vec3, locked: bool) -> Vec3 {
        if !self.airborne {
            self.launch = input;
            self.nudged = false;
            return input;
        }
        if !locked {
            return input;
        }
        if self.launch == Vec3::ZERO && input != Vec3::ZERO && !self.nudged {
            self.launch = input.normalize() * AIR_NUDGE_SPEED;
            self.nudged = true;
        }
        self.launch
    }

    /// Where this frame's step starts: the remembered feet, unless the transform was moved across
    /// the ground since (a dash, a teleport), when it is the transform.
    pub(super) fn start(&self, at: Vec3) -> Vec3 {
        match self.feet {
            Some(f) if f.x == at.x && f.z == at.z => f,
            _ => at,
        }
    }
}

/// One frame of the body, from `feet` at `horiz` (yd/s, flat), jumping on `jump` when grounded.
/// `terrain` is the heightfield under the feet: the floor while no collider is in reach below
/// (the world still streaming), and a body with neither holds its height. Returns the new feet.
#[allow(clippy::too_many_arguments)]
pub(super) fn step(
    world: &WorldCollision,
    capsule: &Collider,
    body: &mut Body,
    feet: Vec3,
    horiz: Vec3,
    jump: bool,
    dt: f32,
    terrain: Option<f32>,
) -> Vec3 {
    let out = if dt <= 0.0 {
        feet
    } else if world
        .ray_body(feet + HALF, Dir3::NEG_Y, FLOOR_SEARCH)
        .is_none()
    {
        on_heightfield(body, feet, horiz, jump, dt, terrain)
    } else {
        on_colliders(world, capsule, body, feet, horiz, jump, dt)
    };
    body.feet = Some(out);
    out
}

/// No collider under the body: the heightfield is the floor, or the height holds without one.
fn on_heightfield(
    body: &mut Body,
    feet: Vec3,
    horiz: Vec3,
    jump: bool,
    dt: f32,
    terrain: Option<f32>,
) -> Vec3 {
    let mut out = feet + horiz * dt;
    body.steep = false;
    let Some(ground) = terrain else {
        body.vy = 0.0;
        body.airborne = false;
        return out;
    };
    if jump && !body.airborne {
        body.vy = JUMP_SPEED;
        body.airborne = true;
    }
    if body.airborne {
        let (vy, mean) = fall_step(body.vy, dt, TERMINAL_VELOCITY);
        body.vy = vy;
        out.y += mean * dt;
    }
    if !body.airborne || out.y <= ground {
        out.y = ground;
        body.vy = 0.0;
        body.airborne = false;
    }
    out
}

/// The colliders' frame, in pieces no longer than [`MAX_STEP`].
fn on_colliders(
    world: &WorldCollision,
    capsule: &Collider,
    body: &mut Body,
    feet: Vec3,
    horiz: Vec3,
    jump: bool,
    dt: f32,
) -> Vec3 {
    let travel = horiz.length() * dt;
    let pieces = ((travel / MAX_STEP).ceil() as u32).clamp(1, 16);
    let piece = dt / pieces as f32;
    let mut center = feet + HALF;
    for i in 0..pieces {
        center = one_piece(world, capsule, body, center, horiz, jump && i == 0, piece);
    }
    center - HALF
}

/// One piece of a frame: the jump's take-off, then the walk step or the airborne slide and its
/// landing.
fn one_piece(
    world: &WorldCollision,
    capsule: &Collider,
    body: &mut Body,
    mut center: Vec3,
    horiz: Vec3,
    jump: bool,
    dt: f32,
) -> Vec3 {
    if jump && !body.airborne {
        body.vy = JUMP_SPEED;
        body.airborne = true;
        body.steep = false;
    }
    if !body.airborne {
        let g = grounded_step(
            world,
            capsule,
            center,
            horiz,
            Duration::from_secs_f32(dt),
            Support {
                steep: body.steep,
                ..Support::default()
            },
        );
        body.steep = g.steep_support;
        return match g.unsupported {
            // Walked off a ledge: the drop is spent and the fall begins, as the mover elects one.
            Some(drop) => {
                body.airborne = true;
                body.vy = 0.0;
                body.steep = false;
                g.center - Vec3::Y * drop
            }
            None => g.center,
        };
    }
    let (vy, mean) = fall_step(body.vy, dt, TERMINAL_VELOCITY);
    body.vy = vy;
    let before = center;
    center = airborne_step(
        world,
        capsule,
        center,
        horiz + Vec3::Y * mean,
        Duration::from_secs_f32(dt),
    );
    // Falling onto a walkable floor within the landing probe ends the arc there.
    if body.vy <= 0.0 {
        if let Some(hit) = world.cast_mover(capsule, center, Vec3::NEG_Y * LAND_PROBE, SKIN_WIDTH) {
            if hit.normal1.y >= GROUND_COS {
                center.y -= hit.distance;
                land(body);
                return center;
            }
        }
    }
    // A fall going nowhere is pinched between steep faces (a tree trunk's funnel): it rests there.
    if body.vy < -WEDGE_MIN_FALL && before.y - center.y < -mean * dt * WEDGE_STALL_RATIO {
        body.stalled += 1;
        if body.stalled >= WEDGE_FRAMES {
            land(body);
        }
    } else {
        body.stalled = 0;
    }
    center
}

/// The arc ends.
fn land(body: &mut Body) {
    body.vy = 0.0;
    body.airborne = false;
    body.stalled = 0;
}

/// A dash of `delta` (flat) from `feet` along the ground: the walk step in [`MAX_STEP`] pieces,
/// sliding along what it meets and stopping where a piece is blocked, so a Blink or a Charge ends
/// short of a wall rather than inside or past it. Returns the new feet.
pub(super) fn dash(world: &WorldCollision, capsule: &Collider, feet: Vec3, delta: Vec3) -> Vec3 {
    let delta = Vec3::new(delta.x, 0.0, delta.z);
    let len = delta.length();
    if len < 1e-3 {
        return feet;
    }
    let dir = delta / len;
    let pieces = (len / MAX_STEP).ceil().max(1.0) as u32;
    let piece = len / pieces as f32;
    let mut center = feet + HALF;
    let mut steep = false;
    for _ in 0..pieces {
        // A piece's length as a one-second velocity, as the creature clamp walks a path.
        let g = grounded_step(
            world,
            capsule,
            center,
            dir * piece,
            Duration::from_secs(1),
            Support {
                steep,
                ..Support::default()
            },
        );
        steep = g.steep_support;
        let gained = (g.center - center).dot(dir);
        // Off a ledge the dash carries on level; the next frame's step finds the fall.
        center = g.center;
        if gained < piece * BLOCKED {
            break;
        }
    }
    center - HALF
}

/// One frame's displacement under gravity from `vy`, for the heightfield's hop: `vy·dt − g·dt²/2`.
#[cfg(test)]
fn free_fall(vy: f32, dt: f32) -> f32 {
    vy * dt - 0.5 * crate::player::GRAVITY * dt * dt
}

#[cfg(test)]
mod tests {
    use super::*;
    use avian3d::prelude::{PhysicsPlugins, RigidBody};
    use bevy::ecs::system::RunSystemOnce;

    const FRAME: f32 = 1.0 / 60.0;

    /// A headless world: a 40 × 40 yd floor at y = 0, one-sided and facing up as terrain is, and
    /// a wall box across +X whose near face is at x = 4.5, 4 yd tall and 20 yd wide.
    fn walled_world() -> App {
        let mut app = App::new();
        app.init_resource::<benilla_world::collision::MoverTraceExclusions>();
        app.add_plugins((
            MinimalPlugins,
            bevy::transform::TransformPlugin,
            bevy::asset::AssetPlugin::default(),
            bevy::scene::ScenePlugin,
            PhysicsPlugins::new(bevy::app::PostUpdate),
        ));
        app.init_asset::<Mesh>();
        let floor = vec![
            Vec3::new(-20.0, 0.0, -20.0),
            Vec3::new(20.0, 0.0, -20.0),
            Vec3::new(20.0, 0.0, 20.0),
            Vec3::new(-20.0, 0.0, 20.0),
        ];
        app.world_mut().spawn((
            RigidBody::Static,
            Collider::trimesh(floor, vec![[0, 2, 1], [0, 3, 2]]),
            Transform::default(),
        ));
        app.world_mut().spawn((
            RigidBody::Static,
            Collider::cuboid(1.0, 4.0, 20.0),
            Transform::from_xyz(5.0, 2.0, 0.0),
        ));
        app.update(); // one frame builds the colliders' poses and the spatial-query trees
        app
    }

    fn capsule() -> Collider {
        Collider::capsule(
            crate::player::CAPSULE_RADIUS,
            CAPSULE_HEIGHT - 2.0 * crate::player::CAPSULE_RADIUS,
        )
    }

    /// Walk `frames` frames at `vel` from the origin; every frame's feet.
    fn walk(vel: Vec3, frames: usize, dt: f32) -> Vec<Vec3> {
        walled_world()
            .world_mut()
            .run_system_once(move |world: WorldCollision| {
                let capsule = capsule();
                let mut body = Body::default();
                let mut feet = Vec3::ZERO;
                (0..frames)
                    .map(|_| {
                        feet = step(&world, &capsule, &mut body, feet, vel, false, dt, None);
                        feet
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap()
    }

    /// The wall's near face, less the capsule's radius: as far as a body's centre line gets.
    const STOP: f32 = 4.5 - crate::player::CAPSULE_RADIUS;

    #[test]
    fn a_run_into_a_wall_stops_at_it_on_the_floor() {
        let path = walk(Vec3::X * 7.0, 120, FRAME);
        let end = *path.last().unwrap();
        assert!(end.x <= STOP + 1e-3, "through the wall: {end}");
        assert!(end.x > STOP - 0.2, "stopped short: {end}");
        for p in &path {
            assert!(p.y.abs() < 0.05, "left the floor: {p}");
        }
    }

    #[test]
    fn a_run_along_a_wall_slides_instead_of_sticking() {
        // 1.5 s at 5 yd/s each way: the wall is met after about 0.83 s, short of its end at z 10.
        let end = *walk(Vec3::new(5.0, 0.0, 5.0), 90, FRAME).last().unwrap();
        assert!(end.x <= STOP + 1e-3, "through the wall: {end}");
        assert!(end.z > 6.0, "stuck on the wall: {end}");
    }

    #[test]
    fn no_speed_or_hitch_tunnels_through() {
        for (speed, dt) in [(60.0, FRAME), (40.0, 0.25), (300.0, 0.1)] {
            let end = *walk(Vec3::X * speed, 20, dt).last().unwrap();
            assert!(end.x <= STOP + 1e-3, "{speed} yd/s at {dt}s: {end}");
        }
    }

    #[test]
    fn a_dash_stops_short_of_the_wall_and_crosses_open_ground_in_full() {
        let (blocked, open) = walled_world()
            .world_mut()
            .run_system_once(|world: WorldCollision| {
                let capsule = capsule();
                (
                    dash(&world, &capsule, Vec3::ZERO, Vec3::X * 18.0),
                    dash(&world, &capsule, Vec3::ZERO, Vec3::NEG_X * 18.0),
                )
            })
            .unwrap();
        assert!(blocked.x <= STOP + 1e-3, "blinked through: {blocked}");
        assert!(blocked.x > STOP - 0.6, "stopped too soon: {blocked}");
        assert!((open.x + 18.0).abs() < 0.05, "short on open ground: {open}");
        assert!(blocked.y.abs() < 0.05 && open.y.abs() < 0.05);
    }

    #[test]
    fn a_jump_rises_and_lands_back_on_the_floor() {
        let path = walled_world()
            .world_mut()
            .run_system_once(|world: WorldCollision| {
                let capsule = capsule();
                let mut body = Body::default();
                let mut feet = Vec3::ZERO;
                let mut path = Vec::new();
                for i in 0..90 {
                    feet = step(
                        &world,
                        &capsule,
                        &mut body,
                        feet,
                        Vec3::ZERO,
                        i == 0,
                        FRAME,
                        None,
                    );
                    path.push((feet, body.airborne));
                }
                path
            })
            .unwrap();
        let top = path.iter().map(|(f, _)| f.y).fold(0.0, f32::max);
        // v²/2g ≈ 1.64 yd.
        assert!(top > 1.4 && top < 1.8, "apex {top}");
        let (end, airborne) = *path.last().unwrap();
        assert!(!airborne && end.y.abs() < 0.05, "never landed: {end}");
    }

    #[test]
    fn with_no_colliders_the_heightfield_is_the_floor() {
        let mut body = Body::default();
        let feet = on_heightfield(&mut body, Vec3::ZERO, Vec3::X, false, 0.5, Some(2.0));
        assert_eq!(feet, Vec3::new(0.5, 2.0, 0.0));
        // A hop over the heightfield rises by the free-fall arc and comes back down onto it.
        let up = on_heightfield(&mut body, feet, Vec3::ZERO, true, FRAME, Some(2.0));
        assert!(
            (up.y - 2.0 - free_fall(JUMP_SPEED, FRAME)).abs() < 1e-3,
            "{up}"
        );
        let mut at = up;
        for _ in 0..90 {
            at = on_heightfield(&mut body, at, Vec3::ZERO, false, FRAME, Some(2.0));
        }
        assert!(!body.airborne && at.y == 2.0);
        // Nothing streamed at all: the height holds.
        let held = on_heightfield(
            &mut body,
            Vec3::new(0.0, 9.0, 0.0),
            Vec3::ZERO,
            false,
            1.0,
            None,
        );
        assert_eq!(held.y, 9.0);
    }

    #[test]
    fn a_locked_jump_keeps_its_launch_and_a_standing_one_steers_once() {
        let mut body = Body::default();
        let run = Vec3::new(7.0, 0.0, 0.0);
        assert_eq!(body.air_velocity(run, true), run);
        body.airborne = true;
        // Letting go or turning mid-air changes nothing.
        assert_eq!(body.air_velocity(Vec3::ZERO, true), run);
        assert_eq!(body.air_velocity(Vec3::Z * 7.0, true), run);
        // Unlocked (the classic view), the input steers.
        assert_eq!(body.air_velocity(Vec3::Z * 7.0, false), Vec3::Z * 7.0);
        // A standing jump: one steer, at walking pace, then it holds.
        let mut body = Body::default();
        body.air_velocity(Vec3::ZERO, true);
        body.airborne = true;
        assert_eq!(body.air_velocity(Vec3::ZERO, true), Vec3::ZERO);
        let nudge = body.air_velocity(Vec3::X * 7.0, true);
        assert_eq!(nudge, Vec3::X * AIR_NUDGE_SPEED);
        assert_eq!(body.air_velocity(Vec3::NEG_Z * 7.0, true), nudge);
    }

    #[test]
    fn a_step_starts_from_the_remembered_feet_unless_moved_across_the_ground() {
        let mut body = Body::default();
        body.placed(Vec3::new(1.0, 3.0, 2.0));
        // The clamp re-seated the height only.
        assert_eq!(
            body.start(Vec3::new(1.0, 0.0, 2.0)),
            Vec3::new(1.0, 3.0, 2.0)
        );
        // A dash moved it.
        assert_eq!(
            body.start(Vec3::new(4.0, 0.0, 2.0)),
            Vec3::new(4.0, 0.0, 2.0)
        );
    }
}
