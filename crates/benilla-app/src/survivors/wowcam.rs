//! WoW's camera and controls on the survivors hero, a run's default view ([`super::view`]).
//!
//! Benilla's own camera rig follows the local hero, as it follows the player's body: the look
//! session ([`camera::run_look_session`]: a right-drag turns the hero with the camera and hides the
//! cursor, a left-drag orbits, both buttons run), the world-mouse latch
//! ([`camera::latch_world_mouse`]) fed the HUD's hover as UI, the zoom and its glide within
//! `cameraDistanceMax` ([`apply_zoom_scroll`]), and the seat with its collision, pivot glide and
//! follow-behind ([`camera::seat_on_subject`]), all under the WoW cvars.
//!
//! Redone here, because benilla's are wired to the networked player and its binding VM, which
//! survivors does not run: the key decode ([`axes`], `player/input.rs`'s `move_axes` read off raw
//! keys at 1.12's default bindings), the keyboard turn ([`turn`]), and the movement word the
//! camera's follow and dynamics read ([`command`], [`WowFrame`]).

use benilla_world::collision::WorldCollision;
use benilla_world::interact::{WorldClick, WorldRightClick};
use benilla_world::view::{ViewDistance, WorldCamera};
use bevy::input::mouse::{AccumulatedMouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::window::{CursorOptions, PrimaryWindow};

use super::hero::Hero;
use super::view::{self, View};
use super::{HeroState, LocalHero, Phase};
use crate::creature_anim::move_flags;
use crate::player::camera::{
    self, follow_cmd, CameraControl, CameraPivot, FlyCam, FollowConfig, FollowInput, LookButton,
    LookConfig,
};
use crate::player::camera_dynamics::{CameraOptions, DynamicsInput, SubjectState};
use crate::player::camera_zoom::{apply_zoom_scroll, ZoomLimit};
use crate::player::{ViewSubject, CAPSULE_HEIGHT, CAPSULE_RADIUS};
use crate::ui_script::PointerOverUi;

/// The keyboard turn (rad/s), the mover's own until the server sends one (`player/state.rs`'s
/// `TURN_RATE`).
const TURN_RATE: f32 = std::f32::consts::PI;
/// The turn's share while translating or airborne (`player/state.rs`'s `TURN_RATE_MOVING`).
const TURN_RATE_MOVING: f32 = 0.75;
/// Backpedal over run, the stock 4.5 / 7.0 (`player/state.rs`'s `RUN_BACK_RATIO`).
pub(super) const RUN_BACK_RATIO: f32 = 4.5 / 7.0;
/// The tilt a run's camera starts at, behind the hero: 1.12's third saved view, 20° down.
const START_PITCH: f32 = -20.0 * std::f32::consts::PI / 180.0;
/// Below this self-fade the eye is in the hero's head (the zoom's floor) and his model goes, as
/// the player's own body fades out in first person.
const HIDE_BELOW: f32 = 0.5;

/// The movement keys held, at 1.12's default bindings.
#[derive(Clone, Copy, Default, Debug)]
pub(super) struct Keys {
    pub(super) forward: bool,
    pub(super) backward: bool,
    pub(super) turn_left: bool,
    pub(super) turn_right: bool,
    pub(super) strafe_left: bool,
    pub(super) strafe_right: bool,
}

impl Keys {
    /// W/S or ↑/↓ forward and back, A/D or ←/→ turn, Q/E strafe.
    pub(super) fn read(keys: &ButtonInput<KeyCode>) -> Self {
        Self {
            forward: keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]),
            backward: keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]),
            turn_left: keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]),
            turn_right: keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]),
            strafe_left: keys.pressed(KeyCode::KeyQ),
            strafe_right: keys.pressed(KeyCode::KeyE),
        }
    }
}

/// This frame's netted axes, each −1, 0 or 1.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) struct Axes {
    /// Forward positive.
    pub(super) fwd: i32,
    /// Right positive.
    pub(super) side: i32,
    /// The keyboard turn, left positive.
    pub(super) turn: i32,
}

impl Axes {
    pub(super) fn translating(self) -> bool {
        self.fwd != 0 || self.side != 0
    }

    /// The movement word: the direction and turn bits the animation and the camera read.
    pub(super) fn flags(self) -> u32 {
        let pick = |v: i32, plus: u32, minus: u32| match v.signum() {
            1 => plus,
            -1 => minus,
            _ => 0,
        };
        pick(self.fwd, move_flags::FORWARD, move_flags::BACKWARD)
            | pick(self.side, move_flags::STRAFE_RIGHT, move_flags::STRAFE_LEFT)
            | pick(self.turn, move_flags::TURN_LEFT, move_flags::TURN_RIGHT)
    }
}

/// The 1.12 control model (`0x7c5360`, as `player/input.rs` nets it): forward and both mouse
/// buttons run and back cancels them; A/D turn, or strafe while mouselooking; Q/E always strafe.
pub(super) fn axes(keys: Keys, mouselook: bool, both_buttons: bool) -> Axes {
    let fwd = i32::from(keys.forward) + i32::from(both_buttons) - i32::from(keys.backward);
    let turns = i32::from(keys.turn_left) - i32::from(keys.turn_right);
    let side = i32::from(keys.strafe_right)
        - i32::from(keys.strafe_left)
        - if mouselook { turns } else { 0 };
    Axes {
        fwd: fwd.signum(),
        side: side.signum(),
        turn: if mouselook { 0 } else { turns },
    }
}

/// The flat way the hero faces at yaw `face` (benilla's: 0 faces −Z, left positive).
pub(super) fn facing(face: f32) -> Vec3 {
    Quat::from_rotation_y(face) * Vec3::NEG_Z
}

/// The flat unit direction `axes` walk along for a hero at yaw `face`; zero standing.
pub(super) fn heading(face: f32, axes: Axes) -> Vec3 {
    let rot = Quat::from_rotation_y(face);
    (rot * Vec3::NEG_Z * axes.fwd as f32 + rot * Vec3::X * axes.side as f32).normalize_or_zero()
}

/// This frame's keyboard turn (radians, left positive): slower while moving or in the air.
pub(super) fn turn(axes: Axes, airborne: bool, dt: f32) -> f32 {
    let rate = if axes.translating() || airborne {
        TURN_RATE * TURN_RATE_MOVING
    } else {
        TURN_RATE
    };
    axes.turn as f32 * rate * dt
}

/// Mouselook in WoW's sense: a right-drag, or both buttons, which steer like it.
pub(super) fn mouselooking(rig: &CameraControl) -> bool {
    rig.freelook || rig.world_mouse.both()
}

/// The camera's input command word ([`follow_cmd`], 1.12's `[InputControl+0x4]`): its edges arm
/// the follow-behind.
pub(super) fn command(keys: Keys, rig: &CameraControl) -> u32 {
    [
        (rig.world_mouse.turn(), follow_cmd::RIGHT_MOUSE),
        (
            rig.world_mouse.held(LookButton::Left),
            follow_cmd::LEFT_MOUSE,
        ),
        (keys.forward, follow_cmd::FORWARD),
        (keys.backward, follow_cmd::BACKWARD),
        (keys.strafe_left, follow_cmd::STRAFE_LEFT),
        (keys.strafe_right, follow_cmd::STRAFE_RIGHT),
        (keys.turn_left, follow_cmd::TURN_LEFT),
        (keys.turn_right, follow_cmd::TURN_RIGHT),
    ]
    .into_iter()
    .filter(|(on, _)| *on)
    .fold(0, |w, (_, bit)| w | bit)
}

/// What the hero's walk left for the camera this frame, and the camera's own memory.
#[derive(Resource, Default)]
pub(super) struct WowFrame {
    /// This frame's keyboard turn, which carries the camera round with the hero.
    pub(super) turn_delta: f32,
    /// The command word ([`command`]).
    pub(super) command: u32,
    /// The hero's movement word and pace, for the camera's dynamics.
    pub(super) flags: u32,
    pub(super) speed: f32,
    /// The hero the camera was last seated behind: another, or none, cuts behind the new one.
    seated: Option<Entity>,
    /// The hero's model is hidden for first person.
    hidden: bool,
}

/// The HUD takes the pointer: a press on a card, a tool or a button is the UI's and starts no
/// mouselook, and off the playing screen every press is. Feeds [`PointerOverUi`] ahead of the
/// world-mouse latch, after the player UI has written its own share.
pub(super) fn hud_takes_pointer(
    phase: Res<State<Phase>>,
    hud: Query<&Interaction>,
    mut over: ResMut<PointerOverUi>,
) {
    let taken = *phase.get() != Phase::Playing || hud.iter().any(|i| *i != Interaction::None);
    if taken && !over.0 {
        over.0 = true;
    }
}

/// The camera dynamics' inputs for the hero at yaw `face`.
fn dynamics(
    follow: FollowConfig,
    options: CameraOptions,
    nearclip: f32,
    frame: &WowFrame,
    face: f32,
) -> DynamicsInput {
    DynamicsInput {
        options,
        smooth_style: follow.style,
        tracking_style: follow.tracking_style,
        subject: SubjectState {
            move_flags: frame.flags,
            facing: face,
            speed: frame.speed,
            command: frame.command,
            ..SubjectState::default()
        },
        nearclip,
        surface_y: None,
    }
}

/// Write the cursor only on a change: a `Mut` every frame would re-apply it to the OS.
fn put_cursor(cursor: &mut Mut<CursorOptions>, want: CursorOptions) {
    let cur = cursor.bypass_change_detection();
    if cur.visible != want.visible
        || cur.grab_mode != want.grab_mode
        || cur.hit_test != want.hit_test
    {
        **cursor = want;
    }
}

/// The look session, before the hero moves: a right-drag turns him with the camera. Only while
/// playing in the WoW view; anywhere else a session in flight ends and the cursor comes back, so
/// the level-up cards and the results take clicks.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn wow_look(
    time: Res<Time<Real>>,
    phase: Res<State<Phase>>,
    view: Res<View>,
    auto: Option<Res<super::auto::Auto>>,
    motion: Res<AccumulatedMouseMotion>,
    cfg: (
        Res<LookConfig>,
        Res<FollowConfig>,
        Res<CameraOptions>,
        Res<ViewDistance>,
    ),
    frame: Res<WowFrame>,
    mut rig: ResMut<CameraControl>,
    mut cams: Query<&mut FlyCam, With<WorldCamera>>,
    window: Single<(&mut Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut clicks: (MessageWriter<WorldClick>, MessageWriter<WorldRightClick>),
    mut hero: Query<(&mut Hero, &HeroState), With<LocalHero>>,
) {
    let (mut window, mut cursor) = window.into_inner();
    let mut opts = cursor.bypass_change_detection().clone();
    let playing = *phase.get() == Phase::Playing
        && view::wow_in_play(*phase.get(), view.mode, auto.is_some());
    let (Ok(mut cam), Ok((mut hm, me)), true) = (cams.single_mut(), hero.single_mut(), playing)
    else {
        if rig.is_looking() {
            camera::disengage_look(&mut rig, &mut window, &mut opts);
            put_cursor(&mut cursor, opts);
        }
        return;
    };
    let both = rig.world_mouse.both();
    let dynamics = dynamics(*cfg.1, *cfg.2, cfg.3.nearclip, &frame, hm.face);
    let mut face = hm.face;
    let focused = window.focused;
    // No click tests: a clean click in the world selects nothing in survivors.
    camera::run_look_session(
        &motion,
        both,
        &mut rig,
        &mut cam,
        &mut face,
        &mut window,
        &mut opts,
        false,
        false,
        &mut clicks.0,
        &mut clicks.1,
        &mut None,
        &mut None,
        *cfg.0,
        &dynamics,
        time.elapsed_secs(),
        focused,
    );
    // A fallen hero's view still turns; the body does not.
    if me.dead_t.is_none() && hm.face != face {
        hm.face = face;
    }
    put_cursor(&mut cursor, opts);
}

/// The hero as the camera follows him this frame.
pub(super) struct Subject {
    pub(super) feet: Vec3,
    pub(super) face: f32,
    /// The framing pivot's height over the feet.
    pub(super) pivot: f32,
}

/// Seat benilla's camera on `subject`, as `control` seats it on the player's body.
#[allow(clippy::too_many_arguments)]
fn seat(
    dt: f32,
    subject: &Subject,
    frame: &WowFrame,
    rig: &mut CameraControl,
    cam: &mut FlyCam,
    cam_t: &mut Mut<Transform>,
    collide: &WorldCollision,
    follow: FollowConfig,
    options: CameraOptions,
    nearclip: f32,
) {
    let dynamics = dynamics(follow, options, nearclip, frame, subject.face);
    // The sweep's root, the capsule's top hemisphere centre, as the controller's.
    let head = subject.feet + Vec3::Y * (CAPSULE_HEIGHT - CAPSULE_RADIUS);
    camera::seat_on_subject(
        dt,
        frame.turn_delta,
        subject.feet,
        head,
        Some(subject.pivot),
        &ViewSubject::default(),
        rig,
        cam,
        cam_t,
        collide,
        &FollowInput {
            cfg: follow,
            face_yaw: subject.face,
            command: frame.command,
        },
        &dynamics,
    );
}

/// The camera behind the hero once he has moved: the wheel's zoom, then the seat. Entering the
/// view, or a new hero, cuts straight behind him. On real time, so the zoom glides while paused.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn wow_camera(
    time: Res<Time<Real>>,
    phase: Res<State<Phase>>,
    view: Res<View>,
    auto: Option<Res<super::auto::Auto>>,
    mut wheel: MessageReader<MouseWheel>,
    over_ui: Res<PointerOverUi>,
    cfg: (
        Res<ZoomLimit>,
        Res<FollowConfig>,
        Res<CameraOptions>,
        Res<ViewDistance>,
    ),
    mut frame: ResMut<WowFrame>,
    mut rig: ResMut<CameraControl>,
    collide: WorldCollision,
    hero: Query<
        (
            Entity,
            &Transform,
            &Hero,
            Option<&CameraPivot>,
            Option<&crate::net::NetEntity>,
        ),
        With<LocalHero>,
    >,
    mut cams: Query<(&mut Transform, &mut FlyCam), (With<WorldCamera>, Without<LocalHero>)>,
    mut models: Query<&mut Visibility, With<LocalHero>>,
) {
    let active = view::wow_in_play(*phase.get(), view.mode, auto.is_some());
    let (Some((e, tf, hm, pivot, net)), Ok((mut cam_t, mut cam))) =
        (hero.single().ok().filter(|_| active), cams.single_mut())
    else {
        wheel.clear();
        frame.seated = None;
        show(&mut frame, &mut models, false);
        return;
    };
    if frame.seated != Some(e) {
        cam.park(hm.face, START_PITCH);
        frame.seated = Some(e);
    }
    let dt = time.delta_secs();
    let notches: f32 = wheel
        .read()
        .map(|w| crate::bindings::wheel_lines(w.unit, w.y))
        .sum();
    // A notch is the stock `CameraZoomIn(1.0)`: a yard, gliding at `cameraDistanceMoveSpeed`.
    apply_zoom_scroll(
        if over_ui.0 { 0.0 } else { notches },
        dt,
        &mut rig,
        cfg.0.max,
    );
    let subject = Subject {
        feet: tf.translation,
        face: hm.face,
        pivot: camera::head_height(pivot, net.map_or(1.0, |n| n.scale)),
    };
    seat(
        dt,
        &subject,
        &frame,
        &mut rig,
        &mut cam,
        &mut cam_t,
        &collide,
        *cfg.1,
        *cfg.2,
        cfg.3.nearclip,
    );
    let hide = rig.self_fade() < HIDE_BELOW;
    show(&mut frame, &mut models, hide);
}

/// Hide or show the hero's model, writing only on a change of our own wish.
fn show(frame: &mut WowFrame, models: &mut Query<&mut Visibility, With<LocalHero>>, hide: bool) {
    if frame.hidden == hide {
        return;
    }
    frame.hidden = hide;
    if let Ok(mut v) = models.single_mut() {
        *v = if hide {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use avian3d::prelude::PhysicsPlugins;
    use bevy::ecs::system::RunSystemOnce;
    use std::f32::consts::{FRAC_PI_2, PI};

    fn keys(f: impl Fn(&mut Keys)) -> Keys {
        let mut k = Keys::default();
        f(&mut k);
        k
    }

    fn close(a: Vec3, b: Vec3) -> bool {
        a.distance(b) < 1e-4
    }

    #[test]
    fn w_and_s_walk_along_the_facing() {
        let w = axes(keys(|k| k.forward = true), false, false);
        assert_eq!(
            w,
            Axes {
                fwd: 1,
                side: 0,
                turn: 0
            }
        );
        assert!(close(heading(0.0, w), Vec3::NEG_Z));
        // Turned a quarter left, forward is −X.
        assert!(close(heading(FRAC_PI_2, w), Vec3::NEG_X));
        let s = axes(keys(|k| k.backward = true), false, false);
        assert!(close(heading(0.0, s), Vec3::Z));
        assert_eq!(s.flags(), move_flags::BACKWARD);
        // W and S cancel.
        let both = axes(
            keys(|k| {
                k.forward = true;
                k.backward = true;
            }),
            false,
            false,
        );
        assert!(!both.translating());
        assert_eq!(heading(0.0, both), Vec3::ZERO);
    }

    #[test]
    fn a_and_d_turn_in_place_and_slower_on_the_move() {
        let a = axes(keys(|k| k.turn_left = true), false, false);
        assert_eq!(
            a,
            Axes {
                fwd: 0,
                side: 0,
                turn: 1
            }
        );
        assert_eq!(heading(0.0, a), Vec3::ZERO, "a turn does not walk");
        assert!((turn(a, false, 1.0) - PI).abs() < 1e-5);
        assert_eq!(a.flags(), move_flags::TURN_LEFT);
        let d = axes(keys(|k| k.turn_right = true), false, false);
        assert!((turn(d, false, 0.5) + PI * 0.5).abs() < 1e-5);
        let wa = axes(
            keys(|k| {
                k.forward = true;
                k.turn_left = true;
            }),
            false,
            false,
        );
        assert!((turn(wa, false, 1.0) - PI * TURN_RATE_MOVING).abs() < 1e-5);
        // In the air too.
        assert!((turn(a, true, 1.0) - PI * TURN_RATE_MOVING).abs() < 1e-5);
    }

    #[test]
    fn q_and_e_strafe_and_a_and_d_strafe_under_the_right_button() {
        let e = axes(keys(|k| k.strafe_right = true), false, false);
        assert!(close(heading(0.0, e), Vec3::X));
        let q = axes(keys(|k| k.strafe_left = true), false, false);
        assert!(close(heading(0.0, q), Vec3::NEG_X));
        // Mouselooking, A strafes left and turns nothing.
        let a = axes(keys(|k| k.turn_left = true), true, false);
        assert_eq!(
            a,
            Axes {
                fwd: 0,
                side: -1,
                turn: 0
            }
        );
        assert_eq!(turn(a, false, 1.0), 0.0);
        assert_eq!(a.flags(), move_flags::STRAFE_LEFT);
        let d = axes(keys(|k| k.turn_right = true), true, false);
        assert!(close(heading(0.0, d), Vec3::X));
        // Forward and strafe walk the diagonal at full pace.
        let we = axes(
            keys(|k| {
                k.forward = true;
                k.strafe_right = true;
            }),
            false,
            false,
        );
        assert!(close(
            heading(0.0, we),
            Vec3::new(1.0, 0.0, -1.0).normalize()
        ));
        assert_eq!(we.flags(), move_flags::FORWARD | move_flags::STRAFE_RIGHT);
    }

    #[test]
    fn both_buttons_run_forward_and_back_stops_them() {
        let run = axes(Keys::default(), true, true);
        assert_eq!(run.fwd, 1);
        assert!(close(heading(0.3, run), facing(0.3)));
        let held = axes(keys(|k| k.backward = true), true, true);
        assert_eq!(held.fwd, 0);
    }

    #[test]
    fn the_command_word_carries_the_keys_for_the_follow() {
        let rig = CameraControl::default();
        let word = command(
            keys(|k| {
                k.forward = true;
                k.strafe_left = true;
            }),
            &rig,
        );
        assert_eq!(word, follow_cmd::FORWARD | follow_cmd::STRAFE_LEFT);
        assert_eq!(command(Keys::default(), &rig), 0);
    }

    /// A headless world with no colliders, for the seat alone.
    fn empty_world() -> App {
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
        app.world_mut().spawn((
            Transform::default(),
            FlyCam {
                yaw: 0.0,
                pitch: 0.0,
                speed: 0.0,
            },
        ));
        app.update();
        app
    }

    /// Seat `frames` frames behind a hero at `feet` facing `face`, zoomed to `zoom`, the camera
    /// parked at `cam_yaw` and 20° down, with `word` held from the second frame; the camera's
    /// position and yaw.
    fn seated(
        feet: Vec3,
        face: f32,
        zoom: f32,
        cam_yaw: f32,
        word: u32,
        frames: usize,
    ) -> (Vec3, f32) {
        empty_world()
            .world_mut()
            .run_system_once(
                move |collide: WorldCollision, mut q: Query<(&mut Transform, &mut FlyCam)>| {
                    let (mut cam_t, mut cam) = q.single_mut().unwrap();
                    cam.park(cam_yaw, START_PITCH);
                    let mut rig = CameraControl::default();
                    rig.park_distance(zoom);
                    let subject = Subject {
                        feet,
                        face,
                        pivot: 1.9,
                    };
                    let mut frame = WowFrame::default();
                    for i in 0..frames {
                        frame.command = if i == 0 { 0 } else { word };
                        seat(
                            1.0 / 60.0,
                            &subject,
                            &frame,
                            &mut rig,
                            &mut cam,
                            &mut cam_t,
                            &collide,
                            FollowConfig::default(),
                            CameraOptions::default(),
                            0.2,
                        );
                    }
                    (cam_t.translation, cam.yaw)
                },
            )
            .unwrap()
    }

    #[test]
    fn the_camera_sits_behind_the_hero_at_the_zoomed_distance() {
        let feet = Vec3::new(3.0, 1.0, -2.0);
        let face = 0.7;
        let (eye, _) = seated(feet, face, 10.0, face, 0, 120);
        let pivot = feet + Vec3::Y * 1.9;
        let look = Quat::from_euler(EulerRot::YXZ, face, START_PITCH, 0.0) * Vec3::NEG_Z;
        let want = pivot - look * 10.0;
        assert!(eye.distance(want) < 0.05, "{eye} vs {want}");
        assert!((eye.distance(pivot) - 10.0).abs() < 0.05);
        // Behind him and above him.
        assert!((eye - feet).dot(facing(face)) < 0.0);
        assert!(eye.y > pivot.y);
    }

    #[test]
    fn running_swings_an_orbited_camera_back_behind_the_hero() {
        // Orbited a radian off his back, then W: the follow returns it behind him.
        let (_, yaw) = seated(Vec3::ZERO, 0.0, 10.0, 1.0, follow_cmd::FORWARD, 180);
        assert!(yaw.abs() < 0.01, "still {yaw} off");
        // Standing still, Smart following leaves it where it was put.
        let (_, yaw) = seated(Vec3::ZERO, 0.0, 10.0, 1.0, 0, 180);
        assert!((yaw - 1.0).abs() < 1e-4, "moved to {yaw}");
    }
}
