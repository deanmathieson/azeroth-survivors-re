//! The hero: its body, WASD movement over the terrain, the top-down camera, and its health.

use benilla_world::view::WorldCamera;
use benilla_world::world_point::WorldPoint;
use bevy::input::mouse::{AccumulatedMouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use super::data;
use super::enemies::Enemy;
use super::party;
use super::units::{self, flat, wow_to_bevy, yaw_toward, Fx, Marks, UnitSpec};
use super::{HeroState, LocalHero, Phase, Run, Selection};
use crate::creature_anim::{move_flags, MovementState};
use crate::net::{FieldChanged, Guid, ObjectStore};

#[derive(Component)]
pub(super) struct Hero {
    /// Vertical velocity in yards/second while airborne; 0 on the ground.
    pub(super) vy: f32,
    /// Height above the ground while airborne. Kept here, not in the transform: the creature
    /// ground clamp (`net::motion::ground_clamp_creatures`) re-seats every unit on the floor at
    /// the top of the frame, so the arc is re-applied over the ground each frame.
    pub(super) air: f32,
}

/// The body radius enemies stop at.
pub(super) const HERO_RADIUS: f32 = 0.7;

/// The hop's initial upward speed and the gravity that brings it back down — the reference
/// `CMovement` jump seed and gravity (`player/state.rs`'s `JUMP_SPEED` / `GRAVITY`).
const JUMP_SPEED: f32 = 7.955_547;
const GRAVITY: f32 = 19.291_105;

/// Radians of orbit per raw mouse unit while right-dragging.
const ORBIT_RATE: f32 = 0.008;
/// Elevation clamp (radians): ~10°..~83° above the horizon, so the camera never dives underground.
const PITCH_MIN: f32 = 0.18;
const PITCH_MAX: f32 = 1.45;
/// The locked view's tilt.
const FIXED_PITCH: f32 = 61.0 * std::f32::consts::PI / 180.0;
/// Wheel zoom range, yards from the hero's shoulders.
const ZOOM_MIN: f32 = 1.5;
const ZOOM_MAX: f32 = 70.0;

// The camera's feel. Each ease closes `1 − e^(−dt/τ)` of its gap a frame, at any frame rate.
/// Focus follow across the ground (s): a running hero leads the view by about speed × this.
const FOCUS_TAU_XZ: f32 = 0.1;
/// Focus follow in height (s): slower, so hops and small terrain bumps don't bob the view.
const FOCUS_TAU_Y: f32 = 0.3;
/// A focus further than this (yards) from the hero cuts straight to him instead of drifting in.
const SNAP_GAP: f32 = 20.0;
/// Zoom ease (s) toward the distance the wheel asked for.
const ZOOM_TAU: f32 = 0.15;
/// Orbit ease (s) toward the dragged yaw and pitch: light, so a drag still feels direct.
const ORBIT_TAU: f32 = 0.05;
/// The swing (s) back to the top-down view when `C` locks the camera again.
const RELOCK_TAU: f32 = 0.2;
/// The lens's hard floor over the terrain beneath it (yards).
const GROUND_CLEARANCE: f32 = 0.6;
/// How far over that floor the eased lift aims (yards), so a quick rise rarely meets the floor.
const GROUND_CUSHION: f32 = 0.5;
/// Lift ease (s) while the terrain pushes the lens up: quick.
const LIFT_RISE_TAU: f32 = 0.05;
/// Lift ease (s) once the terrain falls away: slow, so the view settles rather than drops.
const LIFT_FALL_TAU: f32 = 0.4;

/// Whether the camera is locked; `C` toggles it.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum CamMode {
    /// Locked top-down (58°), laptop-friendly — the original feel.
    #[default]
    Fixed,
    /// Unlocked: a mouse drag swings and tilts it around the hero.
    Orbit,
}

/// The camera's targets, which input sets, and what it shows, eased toward them.
#[derive(Resource)]
pub(super) struct CamRig {
    /// The zoom the wheel asked for, yards; the shown distance eases toward it.
    pub dist: f32,
    pub orbit: f32,
    /// Orbit azimuth (radians) around the hero; right-drag swings it during play.
    pub yaw: f32,
    /// Elevation above the horizon (radians), clamped so the camera never dives underground.
    pub pitch: f32,
    pub mode: CamMode,
    /// Last frame's view; `None` cuts straight to the targets.
    shown: Option<Shown>,
}

impl Default for CamRig {
    fn default() -> Self {
        Self {
            dist: 30.0,
            orbit: 0.6,
            yaw: 0.0,
            pitch: FIXED_PITCH,
            mode: CamMode::Fixed,
            shown: None,
        }
    }
}

/// The view on screen, trailing the hero and the [`CamRig`] targets.
#[derive(Clone, Copy)]
struct Shown {
    /// The eased hero position the view centres on.
    focus: Vec3,
    dist: f32,
    yaw: f32,
    pitch: f32,
    /// Yards the lens rides above its orbit to clear the terrain.
    lift: f32,
    /// The view (the menu's or the run's) and the hero these were eased for: a change cuts.
    menu: bool,
    hero: Option<Entity>,
}

/// Where a battleground's runs start.
pub(super) fn arena(map: &data::MapDef) -> Vec3 {
    wow_to_bevy(map.center)
}

/// The world is the battleground being stood in ([`Selection::stage`]): entering one on another
/// continent worldports, which raises the loading screen until the new ground is in.
pub(super) fn follow_map(
    selection: Res<Selection>,
    current: Option<ResMut<benilla_world::world_map::CurrentMap>>,
) {
    let Some(mut current) = current else {
        return;
    };
    let want = selection.stage_def().map;
    if current.0 != want {
        current.0 = want;
    }
}

/// A hero in its class trainer's look, weapons drawn, carrying `state`; player
/// [`party::LOCAL_PLAYER`]'s is the [`LocalHero`].
pub(super) fn spawn_hero(
    commands: &mut Commands,
    state: HeroState,
    display: u32,
    sex: u8,
    serial: u64,
    pos: Vec3,
) -> Entity {
    let (class, race) = (state.class, state.race);
    let local = state.player == party::LOCAL_PLAYER;
    let bytes0 = u32::from(race)
        | u32::from(class.id) << 8
        | u32::from(sex) << 16
        | u32::from(class.power) << 24;
    let e = units::spawn_unit(
        commands,
        &UnitSpec {
            serial,
            display,
            scale: 1.0,
            level: 60,
            faction: units::FACTION_FRIENDLY,
            bytes0,
            weapons: class.weapons,
            sheath: class.sheath,
        },
        pos,
        0.0,
    );
    commands
        .entity(e)
        .insert((Hero { vy: 0.0, air: 0.0 }, Marks::buffs(), state));
    if local {
        commands.entity(e).insert(LocalHero);
    }
    e
}

/// Midday over the battleground, whatever the clock says.
pub(super) const MINUTE_OF_DAY: u32 = 780;

pub(super) fn daylight(debug: Option<ResMut<benilla_world::dev_state::DebugState>>) {
    let Some(mut debug) = debug else { return };
    if debug.lighting.follow_server_time || debug.lighting.manual_minute != MINUTE_OF_DAY {
        debug.lighting.follow_server_time = false;
        debug.lighting.manual_minute = MINUTE_OF_DAY;
    }
}

/// The ground under the mouse cursor, `None` with the cursor off the window. The locked camera's
/// hero faces it, and the aimed mobility spells (Blink, Charge) go toward it.
#[derive(Resource, Default)]
pub(super) struct AimPoint(pub(super) Option<Vec3>);

/// Cast the cursor onto the terrain: the camera ray meets the plane at the hero's height, then the
/// terrain height there, a few times over, which settles on hills within a frame.
pub(super) fn aim_cursor(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cams: Query<(&Camera, &Transform), With<WorldCamera>>,
    hero: Query<&Transform, (With<LocalHero>, Without<WorldCamera>)>,
    world: WorldPoint,
    mut aim: ResMut<AimPoint>,
) {
    let point = (|| {
        let cursor = windows.single().ok()?.cursor_position()?;
        let (cam, tf) = cams.single().ok()?;
        let ray = cam
            .viewport_to_world(&GlobalTransform::from(*tf), cursor)
            .ok()?;
        let mut y = hero.single().ok()?.translation.y;
        let mut at = None;
        for _ in 0..4 {
            if ray.direction.y.abs() < 1e-4 {
                break;
            }
            let t = (y - ray.origin.y) / ray.direction.y;
            if t <= 0.0 {
                break;
            }
            let p = ray.origin + *ray.direction * t;
            at = Some(p);
            match world.terrain_height_under(p) {
                Some(h) if (h - y).abs() > 0.05 => y = h,
                _ => break,
            }
        }
        at
    })();
    if aim.0 != point {
        aim.0 = point;
    }
}

/// WASD over the terrain within the battleground's leash, and the streaming focus on the local
/// hero; no input reaches the others yet.
#[allow(clippy::too_many_arguments)]
pub(super) fn drive_hero(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<Phase>>,
    selection: Res<Selection>,
    rig: Res<CamRig>,
    aim: Res<AimPoint>,
    world: WorldPoint,
    auto: Option<Res<super::auto::Auto>>,
    mut player: ResMut<crate::player::Player>,
    mut hero: Query<(&mut Transform, &mut MovementState, &mut Hero, &HeroState), With<LocalHero>>,
) {
    let focus = match hero.single_mut() {
        Ok((mut tf, mut mv, mut hm, me)) => {
            let dt = time.delta_secs();
            let mut dir = Vec3::ZERO;
            if let Some(auto) = auto
                .as_deref()
                .filter(|_| *phase.get() == Phase::Playing && me.dead_t.is_none())
            {
                dir = super::auto::steer(auto);
            } else if *phase.get() == Phase::Playing && me.dead_t.is_none() {
                if keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
                    dir.z -= 1.0;
                }
                if keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
                    dir.z += 1.0;
                }
                if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
                    dir.x -= 1.0;
                }
                if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
                    dir.x += 1.0;
                }
                // In orbit mode WASD is camera-relative; in fixed mode it stays world-relative.
                if rig.mode == CamMode::Orbit {
                    let (sy, cy) = rig.yaw.sin_cos();
                    dir = Vec3::new(dir.x * cy + dir.z * sy, 0.0, dir.z * cy - dir.x * sy);
                }
            }
            let mut pos = tf.translation;
            // Locked, the hero faces the cursor and WASD moves him under it, so he backpedals and
            // strafes as a player does; unlocked (or driven), he faces where he runs.
            let face = (rig.mode == CamMode::Fixed
                && auto.is_none()
                && *phase.get() == Phase::Playing
                && me.dead_t.is_none())
            .then(|| aim.0.map(|a| flat(a - pos)))
            .flatten()
            .filter(|d| d.length() > 0.5)
            .map(|d| d.normalize());
            if let Some(look) = face {
                let target = Quat::from_rotation_y(yaw_toward(look));
                tf.rotation = tf.rotation.slerp(target, (dt * 20.0).min(1.0));
            }
            if dir != Vec3::ZERO && dt > 0.0 {
                let dir = dir.normalize();
                let speed = me.stats().speed * me.haste_mult();
                pos += dir * speed * dt;
                mv.speed = speed;
                mv.flags = match face {
                    Some(look) => gait(look, dir),
                    None => {
                        let target = Quat::from_rotation_y(yaw_toward(dir));
                        tf.rotation = tf.rotation.slerp(target, (dt * 20.0).min(1.0));
                        move_flags::FORWARD
                    }
                };
            } else if mv.speed != 0.0 || mv.flags != 0 {
                mv.speed = 0.0;
                mv.flags = 0;
            }
            let map = selection.stage_def();
            let center = arena(map);
            let off = flat(pos - center);
            if off.length() > map.leash * 2.0 {
                // Just entered another battleground: start at its centre.
                pos = center;
            } else if off.length() > map.leash {
                let held = center + off.normalize() * map.leash;
                pos.x = held.x;
                pos.z = held.z;
            }
            let ground = world.terrain_height_under(pos).unwrap_or(pos.y);
            let playing = *phase.get() == Phase::Playing && me.dead_t.is_none();
            // Space hops: lift off the terrain, then gravity brings the hero back down.
            if playing && keys.just_pressed(KeyCode::Space) && hm.vy == 0.0 && hm.air == 0.0 {
                hm.vy = JUMP_SPEED;
            }
            if playing && (hm.vy != 0.0 || hm.air > 0.0) {
                hm.vy -= GRAVITY * dt;
                hm.air += hm.vy * dt;
                if hm.air <= 0.0 {
                    hm.air = 0.0;
                    hm.vy = 0.0;
                }
            } else {
                hm.air = 0.0;
                hm.vy = 0.0;
            }
            pos.y = ground + hm.air;
            // Feed the animation driver: `vertical_speed`'s sign and the `FALLING` flag are what
            // select the Jump (37/38) / Fall (40) animation (`creature_anim::select`).
            mv.vertical_speed = hm.vy;
            if hm.vy != 0.0 {
                mv.flags |= move_flags::FALLING;
            }
            if tf.translation != pos {
                tf.translation = pos;
            }
            pos
        }
        Err(_) => {
            let mut p = arena(selection.stage_def());
            if let Some(h) = world.terrain_height_under(p) {
                p.y = h;
            }
            p
        }
    };
    // The terrain streams and the loading screen waits on the ground under this.
    player.active = true;
    player.detached = false;
    player.settling = false;
    player.world_stale = false;
    player.pos = focus;
}

/// A slow orbit at the menu; in play, top-down until `C` unlocks it to swing and tilt. The view
/// eases toward the hero, the zoom and the dragged angles, and cuts straight to them when a run
/// starts or ends, the hero is respawned, a loading screen covers a worldport, or he is suddenly
/// far away.
///
/// Chained after `drive_hero`, the last writer of the hero's transform in the frame, so the view
/// is this frame's; nothing interpolates the hero (he has no rigid body, and avian's
/// interpolation is off).
#[allow(clippy::type_complexity)] // one Bevy system's full input set
pub(super) fn place_camera(
    time: Res<Time<Real>>,
    phase: Res<State<Phase>>,
    mut rig: ResMut<CamRig>,
    mut wheel: MessageReader<MouseWheel>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    keys: Res<ButtonInput<KeyCode>>,
    player: Res<crate::player::Player>,
    loading: Option<Res<crate::loading_screen::LoadingScreen>>,
    world: WorldPoint,
    hero: Query<(Entity, &Transform), (With<LocalHero>, Without<WorldCamera>)>,
    mut cams: Query<&mut Transform, With<WorldCamera>>,
) {
    let dt = time.delta_secs();
    // `C` unlocks the camera to swing and tilt; locking it again swings back to the top-down view,
    // so world-relative WASD lines up with the screen again.
    if keys.just_pressed(KeyCode::KeyC) {
        rig.mode = match rig.mode {
            CamMode::Fixed => CamMode::Orbit,
            CamMode::Orbit => {
                rig.yaw = 0.0;
                rig.pitch = FIXED_PITCH;
                // Home the short way round, however many turns the drags wound up.
                if let Some(shown) = rig.shown.as_mut() {
                    shown.yaw = wrap_angle(shown.yaw);
                }
                CamMode::Fixed
            }
        };
    }
    // With no hero yet, the focus `drive_hero` grounded.
    let (hero_id, focus) = hero
        .single()
        .map_or((None, player.pos), |(e, t)| (Some(e), t.translation));
    for w in wheel.read() {
        let notches = match w.unit {
            MouseScrollUnit::Line => w.y,
            MouseScrollUnit::Pixel => w.y * 0.02,
        };
        rig.dist = zoom_notch(rig.dist, notches);
    }
    let menu = *phase.get() == Phase::Menu;
    if menu {
        rig.orbit += dt * 0.2;
    } else if rig.mode == CamMode::Orbit
        && mouse.any_pressed([MouseButton::Left, MouseButton::Right])
    {
        // Unlocked, either mouse button drags the camera around the hero, as in WoW.
        rig.yaw -= motion.delta.x * ORBIT_RATE;
        rig.pitch = (rig.pitch + motion.delta.y * ORBIT_RATE).clamp(PITCH_MIN, PITCH_MAX);
    }
    let cut = loading.is_some_and(|l| l.covering())
        || rig
            .shown
            .is_none_or(|s| s.menu != menu || s.hero != hero_id);
    let angle_tau = match rig.mode {
        CamMode::Fixed => RELOCK_TAU,
        CamMode::Orbit => ORBIT_TAU,
    };
    let mut shown = match rig.shown.filter(|_| !cut) {
        Some(s) => Shown {
            focus: follow_focus(s.focus, focus, dt),
            dist: ease_zoom(s.dist, rig.dist, dt),
            yaw: ease(s.yaw, rig.yaw, dt, angle_tau),
            pitch: ease(s.pitch, rig.pitch, dt, angle_tau),
            ..s
        },
        None => Shown {
            focus,
            dist: rig.dist,
            yaw: rig.yaw,
            pitch: rig.pitch,
            lift: 0.0,
            menu,
            hero: hero_id,
        },
    };
    let (eye, look) = if menu {
        let d = 7.0;
        (
            shown.focus + Vec3::new(rig.orbit.sin() * d, 2.4, rig.orbit.cos() * d),
            shown.focus + Vec3::Y * 1.1,
        )
    } else {
        let (sy, cy) = shown.yaw.sin_cos();
        let (sp, cp) = shown.pitch.sin_cos();
        let look = shown.focus + Vec3::Y * 1.4;
        let mut eye = look + Vec3::new(cp * sy, sp, cp * cy) * shown.dist;
        // Never under the ground: a low tilt on a slope would bury the lens. A cut lifts at once.
        let ground = world.terrain_height_under(eye);
        let step = if cut { f32::INFINITY } else { dt };
        (shown.lift, eye.y) = clear_ground(shown.lift, eye.y, ground, step);
        (eye, look)
    };
    rig.shown = Some(shown);
    let placed = Transform::from_translation(eye).looking_at(look, Vec3::Y);
    for mut cam in &mut cams {
        *cam = placed;
    }
}

/// The share of the remaining gap an exponential follow with time constant `tau` closes in `dt`:
/// `1 − e^(−dt/τ)`, so two half steps close exactly what one whole step does.
fn ease_factor(dt: f32, tau: f32) -> f32 {
    if tau <= 0.0 {
        return 1.0;
    }
    1.0 - (-dt.max(0.0) / tau).exp()
}

/// `from` eased toward `to` over `dt`.
fn ease(from: f32, to: f32, dt: f32, tau: f32) -> f32 {
    from + (to - from) * ease_factor(dt, tau)
}

/// The focus eased toward the hero, quickly across the ground and slowly in height; a gap wider
/// than [`SNAP_GAP`] cuts straight to him.
fn follow_focus(from: Vec3, to: Vec3, dt: f32) -> Vec3 {
    if from.distance(to) > SNAP_GAP {
        return to;
    }
    let across = ease_factor(dt, FOCUS_TAU_XZ);
    let up = ease_factor(dt, FOCUS_TAU_Y);
    from + (to - from) * Vec3::new(across, up, across)
}

/// The wheel's zoom after `notches` (positive zooms in). Multiplicative, so a notch moves as far by
/// eye close in as far out; all the way down to an over-the-shoulder view.
fn zoom_notch(dist: f32, notches: f32) -> f32 {
    (dist * (1.0 - 0.12 * notches).clamp(0.5, 1.5)).clamp(ZOOM_MIN, ZOOM_MAX)
}

/// The shown distance eased toward the wheel's, in log space so it glides alike at every zoom.
fn ease_zoom(from: f32, to: f32, dt: f32) -> f32 {
    let (a, b) = (from.ln(), to.ln());
    (a + (b - a) * ease_factor(dt, ZOOM_TAU))
        .exp()
        .clamp(ZOOM_MIN, ZOOM_MAX)
}

/// The lens's lift over its orbit and its height after it, given the terrain beneath. The lift
/// eases toward clearing the ground by the cushion, quickly up and slowly down; whatever it has
/// reached, the eye is never below `ground + GROUND_CLEARANCE`.
fn clear_ground(lift: f32, eye_y: f32, ground: Option<f32>, dt: f32) -> (f32, f32) {
    let need = ground.map_or(0.0, |h| {
        (h + GROUND_CLEARANCE + GROUND_CUSHION - eye_y).max(0.0)
    });
    let tau = if need > lift {
        LIFT_RISE_TAU
    } else {
        LIFT_FALL_TAU
    };
    let lift = ease(lift, need, dt, tau);
    let y = eye_y + lift;
    (lift, ground.map_or(y, |h| y.max(h + GROUND_CLEARANCE)))
}

/// `a` brought into [−π, π), the same direction.
fn wrap_angle(a: f32) -> f32 {
    (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

/// Damage a hero takes from one blow, after immunity, armour and the shield; returns what got
/// through.
pub(super) fn hurt_hero(hero: &mut HeroState, raw: f32) -> f32 {
    if hero.immune_t > 0.0 || hero.dead_t.is_some() {
        return 0.0;
    }
    let mut dmg = hero.stats().after_armor(raw);
    if hero.shield_t > 0.0 && hero.shield > 0.0 {
        let absorbed = dmg.min(hero.shield);
        hero.shield -= absorbed;
        dmg -= absorbed;
    }
    hero.hp -= dmg;
    hero.damage_taken += f64::from(dmg);
    dmg
}

/// How far an extra life throws the crowd clear, in yards.
const REVIVE_KNOCKBACK: f32 = 8.0;

/// The clock, each hero's regeneration, timed buffs and their visuals, extra lives and death, and
/// the run's end: lost once every hero is down, won when the clock runs out with one standing.
#[allow(clippy::type_complexity)] // one Bevy system's full input set
pub(super) fn hero_vitals(
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut next: ResMut<NextState<Phase>>,
    mut heroes: Query<
        (
            Entity,
            &Guid,
            &Transform,
            &mut ObjectStore,
            &mut Marks,
            &mut HeroState,
        ),
        With<Hero>,
    >,
    mut enemies: Query<(&mut Transform, &Enemy), Without<Hero>>,
    mut fields: MessageWriter<FieldChanged>,
    mut fx: Fx,
) {
    let dt = time.delta_secs();
    if heroes.is_empty() {
        return;
    }
    // The whole party down: the deaths play out with the clock stopped, then the run is lost.
    if party::all_down(heroes.iter().map(|(.., hero)| hero)) {
        let mut over = true;
        for (.., mut hero) in &mut heroes {
            if let Some(t) = hero.dead_t.as_mut() {
                *t += dt;
                over &= *t > 3.0;
            }
        }
        if over {
            next.set(Phase::GameOver);
        }
        return;
    }
    run.elapsed += dt;
    // A hero getting up or falling this frame holds victory off a frame, as it always has.
    let mut eventful = false;
    let mut standing = Vec::new();
    for (entity, guid, htf, mut store, mut marks, mut hero) in &mut heroes {
        // Fallen while others fight on: the death plays out.
        if let Some(t) = hero.dead_t.as_mut() {
            *t += dt;
            continue;
        }
        let stats = hero.stats();
        let r = &mut *hero;
        for t in [&mut r.shield_t, &mut r.immune_t, &mut r.thorns_t] {
            *t = (*t - dt).max(0.0);
        }
        // Regeneration and the heal over time it carries, credited to it (the caster, for now).
        let mut heal = stats.regen * dt;
        if hero.heal_t > 0.0 {
            heal += hero.heal_rate * dt.min(hero.heal_t);
            hero.heal_t = (hero.heal_t - dt).max(0.0);
        }
        let healed = party::heal(&mut hero, heal);
        hero.healing_done += f64::from(healed);

        // The buffs show as their own state kits: the shield bubble, the immunity glow, the orbs.
        let mut shown: Vec<u32> = Vec::new();
        if hero.shield_t > 0.0 && hero.shield > 0.0 {
            shown.push(17);
        }
        if hero.immune_t > 0.0 {
            shown.push(642);
        }
        if hero.thorns_t > 0.0 {
            shown.push(hero.thorns_spell);
        }
        if hero.heal_t > 0.0 {
            shown.push(774);
        }
        for owned in &hero.abilities {
            if owned.def.key == "lightning_shield" {
                shown.push(owned.def.spell);
            }
        }
        if hero.passives.iter().any(|(p, _)| p.key == "renew") {
            shown.push(139);
        }
        marks.set(&shown);
        marks.flush(entity, guid.0, &mut store, &mut fields);

        if hero.hp <= 0.0 && hero.revive() {
            eventful = true;
            // Up again, with the crowd thrown clear; the immunity glow shows from the next frame.
            fx.cast(entity, 642, &[entity], None);
            let hpos = htf.translation;
            for (mut tf, e) in &mut enemies {
                let away = flat(tf.translation - hpos);
                let dist = away.length();
                if !e.dead && dist > 0.01 && dist < REVIVE_KNOCKBACK {
                    tf.translation += away / dist * (REVIVE_KNOCKBACK - dist);
                }
            }
        } else if hero.hp <= 0.0 {
            eventful = true;
            hero.hp = 0.0;
            hero.dead_t = Some(0.0);
            units::kill_unit(&mut store);
            marks.clear();
            marks.flush(entity, guid.0, &mut store, &mut fields);
        } else {
            standing.push(entity);
        }
    }
    if !eventful && run.elapsed >= data::VICTORY_SECS {
        run.won = true;
        // A last flourish before the results.
        for entity in standing {
            fx.cast(entity, 642, &[entity], None);
        }
        next.set(Phase::GameOver);
    }
}

/// The ground-plane distance from the hero, the measure every range in the mode uses.
/// The movement flags for running `dir` while facing `look`: forward, backpedal or strafe, by
/// which quarter the run falls in, so the walk, backpedal and strafe animations play.
fn gait(look: Vec3, dir: Vec3) -> u32 {
    let ahead = look.dot(dir);
    // Positive when `dir` is to the hero's left: WoW faces −Z with +X on the right.
    let left = look.cross(dir).y;
    let quarter = std::f32::consts::FRAC_1_SQRT_2;
    if ahead >= quarter {
        move_flags::FORWARD
    } else if ahead <= -quarter {
        move_flags::BACKWARD
    } else if left > 0.0 {
        move_flags::STRAFE_LEFT
    } else {
        move_flags::STRAFE_RIGHT
    }
}

pub(super) fn reach(hero: Vec3, other: Vec3) -> f32 {
    flat(other - hero).length()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{PI, TAU};

    const FRAME: f32 = 1.0 / 60.0;

    /// A world `hero_vitals` runs in, a second passing each run, with heroes at these healths.
    fn vitals_world(hps: &[f32]) -> (World, Vec<Entity>) {
        let mut world = World::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs(1));
        world.insert_resource(time);
        world.init_resource::<Run>();
        world.init_resource::<NextState<Phase>>();
        world.init_resource::<crate::creature_anim::PlaySeq>();
        world.init_resource::<Messages<FieldChanged>>();
        world.init_resource::<Messages<crate::creature_anim::CastEvent>>();
        world.init_resource::<Messages<crate::creature_anim::SpellGoTargets>>();
        world.init_resource::<Messages<crate::creature_anim::SwingMessage>>();
        world.init_resource::<Messages<crate::combat_text::CombatTextSpawn>>();
        let heroes = hps
            .iter()
            .enumerate()
            .map(|(i, &hp)| {
                let state = HeroState {
                    player: i as u8,
                    hp,
                    ..HeroState::default()
                };
                let store = ObjectStore(Default::default());
                let hero = Hero { vy: 0.0, air: 0.0 };
                let body = (Guid(i as u64 + 1), Transform::default(), store);
                world.spawn((hero, body, Marks::buffs(), state)).id()
            })
            .collect();
        (world, heroes)
    }

    fn vitals(world: &mut World) {
        use bevy::ecs::system::RunSystemOnce;
        world.run_system_once(hero_vitals).unwrap();
    }

    fn lost(world: &World) -> bool {
        matches!(
            world.resource::<NextState<Phase>>(),
            NextState::Pending(Phase::GameOver)
        )
    }

    #[test]
    fn the_run_is_lost_once_every_hero_is_down() {
        let (mut world, heroes) = vitals_world(&[100.0, -1.0]);
        let down = |world: &World, i: usize| world.get::<HeroState>(heroes[i]).unwrap().dead_t;
        vitals(&mut world);
        assert_eq!(down(&world, 1), Some(0.0), "no extra life: it falls");
        assert_eq!(down(&world, 0), None);
        assert_eq!(world.resource::<Run>().elapsed, 1.0);
        // One standing keeps the run, and its clock, going.
        for _ in 0..5 {
            vitals(&mut world);
        }
        assert!(!lost(&world));
        assert_eq!(world.resource::<Run>().elapsed, 6.0);
        // The last falls: its death plays out with the clock stopped, then the run is lost.
        world.get_mut::<HeroState>(heroes[0]).unwrap().hp = -1.0;
        vitals(&mut world);
        assert_eq!(down(&world, 0), Some(0.0));
        for _ in 0..3 {
            vitals(&mut world);
            assert!(!lost(&world));
        }
        vitals(&mut world);
        assert!(lost(&world));
        assert_eq!(world.resource::<Run>().elapsed, 7.0);
        assert!(!world.resource::<Run>().won);
    }

    #[test]
    fn a_lone_hero_falls_and_loses_the_run_as_before() {
        let (mut world, heroes) = vitals_world(&[-1.0]);
        vitals(&mut world);
        let hero = world.get::<HeroState>(heroes[0]).unwrap();
        assert_eq!((hero.hp, hero.dead_t), (0.0, Some(0.0)));
        for _ in 0..3 {
            vitals(&mut world);
            assert!(!lost(&world), "the death plays out first");
        }
        vitals(&mut world);
        assert!(lost(&world));
        assert_eq!(world.resource::<Run>().elapsed, 1.0);
    }

    #[test]
    fn one_long_frame_eases_as_far_as_two_short_ones() {
        for tau in [FOCUS_TAU_XZ, FOCUS_TAU_Y, ZOOM_TAU, ORBIT_TAU, RELOCK_TAU] {
            let once = ease(0.0, 10.0, 2.0 * FRAME, tau);
            let twice = ease(ease(0.0, 10.0, FRAME, tau), 10.0, FRAME, tau);
            assert!((once - twice).abs() < 1e-4, "τ {tau}: {once} vs {twice}");
        }
        let (from, to) = (Vec3::ZERO, Vec3::new(6.0, 2.0, -4.0));
        let once = follow_focus(from, to, 2.0 * FRAME);
        let twice = follow_focus(follow_focus(from, to, FRAME), to, FRAME);
        assert!(once.distance(twice) < 1e-4, "{once} vs {twice}");
        let once = ease_zoom(30.0, 10.0, 2.0 * FRAME);
        let twice = ease_zoom(ease_zoom(30.0, 10.0, FRAME), 10.0, FRAME);
        assert!((once - twice).abs() < 1e-3, "{once} vs {twice}");
        let (once, _) = clear_ground(0.0, 0.0, Some(3.0), 2.0 * FRAME);
        let (half, _) = clear_ground(0.0, 0.0, Some(3.0), FRAME);
        let (twice, _) = clear_ground(half, 0.0, Some(3.0), FRAME);
        assert!((once - twice).abs() < 1e-4, "{once} vs {twice}");
    }

    #[test]
    fn the_focus_settles_on_the_hero_without_overshooting() {
        let to = Vec3::new(8.0, 1.5, -3.0);
        let mut at = Vec3::ZERO;
        let mut gap = at.distance(to);
        for _ in 0..120 {
            at = follow_focus(at, to, FRAME);
            let next = at.distance(to);
            assert!(next <= gap, "the follow backed off: {next} > {gap}");
            gap = next;
        }
        assert!(gap < 0.01, "still {gap} yd off after 2 s");
        // Height trails: a hop lifts the view less than a step slides it.
        let step = follow_focus(Vec3::ZERO, Vec3::new(1.0, 1.0, 0.0), 0.1);
        assert!(step.y < step.x, "{step}");
        // A frame with no time moves nothing.
        assert_eq!(follow_focus(Vec3::ZERO, to, 0.0), Vec3::ZERO);
    }

    #[test]
    fn a_far_hero_is_cut_to_rather_than_drifted_toward() {
        let far = Vec3::new(SNAP_GAP + 5.0, 0.0, 0.0);
        assert_eq!(follow_focus(Vec3::ZERO, far, FRAME), far);
        // Straight down counts too: a hero settling from a stale height onto streamed ground.
        let below = Vec3::new(0.0, -(SNAP_GAP + 1.0), 0.0);
        assert_eq!(follow_focus(Vec3::ZERO, below, FRAME), below);
        // A blink's worth is eased.
        let near = Vec3::new(SNAP_GAP * 0.5, 0.0, 0.0);
        let eased = follow_focus(Vec3::ZERO, near, FRAME);
        assert!(eased.x > 0.0 && eased.x < near.x, "{eased}");
    }

    #[test]
    fn the_zoom_eases_inside_its_range() {
        let mut want = 30.0;
        for _ in 0..40 {
            want = zoom_notch(want, 3.0);
        }
        assert_eq!(want, ZOOM_MIN);
        for _ in 0..40 {
            want = zoom_notch(want, -3.0);
        }
        assert_eq!(want, ZOOM_MAX);
        for (from, to) in [(ZOOM_MAX, ZOOM_MIN), (ZOOM_MIN, ZOOM_MAX), (30.0, 33.6)] {
            let mut d = from;
            for i in 0..240 {
                // Uneven frames, a hitch among them.
                let dt = [FRAME, 1.0 / 144.0, 0.25, 1.0 / 30.0][i % 4];
                let next = ease_zoom(d, to, dt);
                assert!((ZOOM_MIN..=ZOOM_MAX).contains(&next), "{next} out of range");
                assert!(
                    (next - to).abs() <= (d - to).abs() + 1e-4,
                    "{d} -> {next} away"
                );
                d = next;
            }
            assert!((d - to).abs() < 1e-2 * to, "{from} -> {to} stuck at {d}");
        }
        // A frame goes part of the way: eased, not instant.
        let one = ease_zoom(30.0, 15.0, FRAME);
        assert!(one < 30.0 && one > 25.0, "{one}");
    }

    #[test]
    fn the_lens_never_dips_under_the_ground() {
        // The orbit holds the eye at 0 while the terrain under it ramps, jumps a cliff, holds,
        // then falls away; frames come uneven.
        let ground = |t: f32| match t {
            t if t < 0.5 => -2.0 + 20.0 * t,
            t if t < 1.0 => 8.0,
            t if t < 1.5 => 11.0,
            _ => -10.0,
        };
        let mut lift = 0.0;
        let mut t = 0.0;
        let mut i = 0;
        while t < 3.0 {
            let dt = [FRAME, 1.0 / 144.0, 0.1, 1.0 / 30.0][i % 4];
            t += dt;
            i += 1;
            let h = ground(t);
            let (next, y) = clear_ground(lift, 0.0, Some(h), dt);
            assert!(
                y >= h + GROUND_CLEARANCE - 1e-5,
                "eye {y} under {h} at {t:.2}s"
            );
            assert!(next >= 0.0, "lift {next}");
            lift = next;
        }
        // Up quickly: a steady need is met within a quarter second.
        let mut lift = 0.0;
        for _ in 0..15 {
            lift = clear_ground(lift, 0.0, Some(2.0), FRAME).0;
        }
        let need = 2.0 + GROUND_CLEARANCE + GROUND_CUSHION;
        assert!(
            (lift - need).abs() < 0.01 * need,
            "lift {lift} short of {need}"
        );
        // Down slowly: once the ground falls away a frame lets go of little.
        let (after, y) = clear_ground(lift, 0.0, Some(-10.0), FRAME);
        assert!(after > 0.9 * lift && after < lift, "{lift} -> {after}");
        assert_eq!(y, after);
        // No terrain streamed under the eye: the lift still eases down.
        let (after, _) = clear_ground(lift, 0.0, None, FRAME);
        assert!(after < lift);
        // A cut seats the lift at once.
        let (cut, y) = clear_ground(0.0, 0.0, Some(5.0), f32::INFINITY);
        assert!((cut - (5.0 + GROUND_CLEARANCE + GROUND_CUSHION)).abs() < 1e-5);
        assert!(y >= 5.0 + GROUND_CLEARANCE);
    }

    #[test]
    fn relocking_swings_home_the_short_way() {
        for wound in [4.0 * TAU + 0.3, -(4.0 * TAU + 0.3), 3.0, -3.0, PI] {
            let w = wrap_angle(wound);
            assert!((-PI..PI).contains(&w), "{wound} -> {w}");
            assert!((w.sin() - wound.sin()).abs() < 1e-4, "{wound} -> {w}");
            assert!((w.cos() - wound.cos()).abs() < 1e-4, "{wound} -> {w}");
        }
        // Eased home, the swing never passes through more than a half turn.
        let mut yaw = wrap_angle(4.0 * TAU + 3.0);
        for _ in 0..120 {
            let next = ease(yaw, 0.0, FRAME, RELOCK_TAU);
            assert!(next.abs() <= yaw.abs() && next.abs() <= PI);
            yaw = next;
        }
        assert!(yaw.abs() < 1e-3, "{yaw}");
    }
}
