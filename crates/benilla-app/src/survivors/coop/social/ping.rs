//! Pings and raid marks. In a co-op run a click of the **middle mouse button** pings the ground
//! under the cursor for the whole party. Holding it opens a wheel round the cursor: the eight raid
//! target icons (`UI-RaidTargetingIcons`: skull, cross, square, moon, triangle, diamond, circle,
//! star) and three quick pings ("Help!", "Boss", "Chest here"); let go over one to place it. A raid
//! icon goes on the enemy under the cursor when there is one, and stays there until it dies;
//! otherwise every mark and ping fades after [`MARK_SECS`]. An icon is only ever in one place.
//! Marks show over their spot or unit in the world and on the minimap, each with its sound.

use benilla_world::view::WorldCamera;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

use super::super::super::enemies::Enemy;
use super::super::super::hero::AimPoint;
use super::super::super::hud::HudFont;
use super::super::super::minimap::{raid_icon_coords, MapMark, MapMarks, RAID_ICONS};
use super::super::super::Phase;
use super::super::host::Peer;
use super::super::wire::Ent;
use super::super::Session;
use super::chat::Chat;
use super::wire::{Body, Heard, Line, PingInput, Pinged};
use super::{label, name_of, player_of, to_all, Art, PARTY};
use crate::sound::SoundCue;

/// Seconds a ping, or a mark on the ground, holds.
pub(super) const MARK_SECS: f32 = 8.0;
/// Held this long, the middle button opens the wheel instead of pinging.
const HOLD_SECS: f32 = 0.25;
/// The wheel's radius, and the dead zone at its heart, in UI pixels.
const WHEEL_R: f32 = 76.0;
const WHEEL_DEAD: f32 = 22.0;
/// How near the cursor's spot an enemy must stand to take a mark, in yards.
const MARK_REACH: f32 = 3.5;
/// A plain ping's art: the minimap's own ping.
pub(super) const PING_ART: &str = "Interface\\Minimap\\Ping\\ping4";

/// What a ping says.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::survivors) enum PingKind {
    /// "Look here."
    Ping,
    Help,
    Boss,
    Chest,
    /// A raid target icon, `RAID_TARGET_1` (star) to `RAID_TARGET_8` (skull).
    Mark(u8),
}

/// The wheel, clockwise from the top: the raid icons skull first, then the quick pings.
pub(super) const KINDS: [PingKind; 11] = [
    PingKind::Mark(8),
    PingKind::Mark(7),
    PingKind::Mark(6),
    PingKind::Mark(5),
    PingKind::Mark(4),
    PingKind::Mark(3),
    PingKind::Mark(2),
    PingKind::Mark(1),
    PingKind::Help,
    PingKind::Boss,
    PingKind::Chest,
];

impl PingKind {
    /// What it says over its spot and in the chat.
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Ping => "Ping",
            Self::Help => "Help!",
            Self::Boss => "Boss",
            Self::Chest => "Chest here",
            // `RAID_TARGET_n` in `GlobalStrings.lua`.
            Self::Mark(n) => match n {
                1 => "Star",
                2 => "Circle",
                3 => "Diamond",
                4 => "Triangle",
                5 => "Moon",
                6 => "Square",
                7 => "Cross",
                _ => "Skull",
            },
        }
    }

    /// The minimap's icon: the raid icon, else 0 (a ping's ripple).
    fn icon(self) -> u8 {
        match self {
            Self::Mark(n) => n.clamp(1, 8),
            _ => 0,
        }
    }

    /// Its art over the spot.
    fn art(self) -> Art {
        match self {
            Self::Mark(n) => Art::crop(RAID_ICONS, raid_icon_coords(n)),
            Self::Ping => Art::whole(PING_ART),
            Self::Help => Art::whole(PING_ART).tinted(Color::srgb(1.0, 0.3, 0.25)),
            Self::Boss => Art::whole(PING_ART).tinted(Color::srgb(0.8, 0.4, 1.0)),
            Self::Chest => Art::whole(PING_ART).tinted(Color::srgb(1.0, 0.85, 0.2)),
        }
    }
}

/// The sound a ping plays: the minimap's ping for a plain one or a mark, the raid warning for
/// "Help!", the battleground's warning horn for "Boss", and the loot window for "Chest here".
pub(super) fn sound(kind: PingKind) -> &'static str {
    match kind {
        PingKind::Ping | PingKind::Mark(_) => "MapPing",
        PingKind::Help => "RaidWarning",
        PingKind::Boss => "PVP_Warning",
        PingKind::Chest => "LOOTWINDOWOPENEMPTY",
    }
}

/// One mark in the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Marker {
    pub kind: PingKind,
    pub at: Vec3,
    /// The unit it is on, which it follows until the unit dies.
    pub target: Option<Entity>,
    pub age: f32,
    pub by: u8,
}

/// Every mark in the world, the same on every machine.
#[derive(Resource, Default, Debug)]
pub(super) struct Markers {
    pub list: Vec<Marker>,
    /// Bumped as marks come and go, so their nodes rebuild.
    version: u32,
}

impl Markers {
    /// Put `m` down. A raid icon is only ever in one place, and a unit wears one icon: placing it
    /// again moves it.
    pub(super) fn place(&mut self, m: Marker) {
        if let PingKind::Mark(_) = m.kind {
            self.list.retain(|o| {
                o.kind != m.kind
                    && !(m.target.is_some()
                        && o.target == m.target
                        && matches!(o.kind, PingKind::Mark(_)))
            });
        }
        self.list.push(m);
        self.version = self.version.wrapping_add(1);
    }

    /// `dt` on: a mark on a unit follows it while `where_alive` finds the unit standing, and goes
    /// when it dies; any other fades after [`MARK_SECS`].
    pub(super) fn tick(&mut self, dt: f32, where_alive: impl Fn(Entity) -> Option<Vec3>) {
        let before = self.list.len();
        for m in &mut self.list {
            m.age += dt;
        }
        self.list.retain_mut(|m| match m.target {
            Some(t) => match where_alive(t) {
                Some(at) => {
                    m.at = at;
                    true
                }
                None => false,
            },
            None => m.age < MARK_SECS,
        });
        if self.list.len() != before {
            self.version = self.version.wrapping_add(1);
        }
    }
}

/// Which wheel option the cursor is over, `offset` (UI pixels, y down) from where the wheel
/// opened: `None` in the dead zone at its heart.
pub(super) fn wheel_pick(offset: Vec2) -> Option<usize> {
    if offset.length() < WHEEL_DEAD {
        return None;
    }
    // Clockwise from straight up.
    let angle = offset.x.atan2(-offset.y).rem_euclid(std::f32::consts::TAU);
    let step = std::f32::consts::TAU / KINDS.len() as f32;
    Some(((angle + step / 2.0) / step) as usize % KINDS.len())
}

/// Where the wheel's option `i` sits, from its middle.
fn wheel_spot(i: usize) -> Vec2 {
    let a = i as f32 * std::f32::consts::TAU / KINDS.len() as f32;
    Vec2::new(a.sin(), -a.cos()) * WHEEL_R
}

/// Where a press went down: the cursor (UI pixels), the ground under it, and the enemy there.
type Down = (Vec2, Option<Vec3>, Option<(Entity, Vec3)>);

/// The middle button's press under way: where it went down (UI pixels and the ground), the enemy
/// there, and how long it has been held.
#[derive(Resource, Default)]
struct Press {
    down: Option<Down>,
    held: f32,
    wheel: bool,
}

#[derive(Component)]
struct WheelRoot;
#[derive(Component)]
struct WheelOption(usize);
#[derive(Component)]
struct MarkerNode(usize);
#[derive(Component)]
struct MarkersRoot;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Markers>()
        .init_resource::<Press>()
        .init_resource::<MapMarks>()
        .add_systems(
            Update,
            (
                ping_input,
                host_pings.run_if(super::hosting),
                hear_pings,
                tick_markers,
                show_markers,
                show_wheel,
            )
                .chain(),
        )
        .add_systems(OnEnter(Phase::Menu), clear_markers);
}

/// The cursor in UI pixels.
fn cursor_ui(windows: &Query<&Window, With<PrimaryWindow>>, scale: f32) -> Option<Vec2> {
    let c = windows.single().ok()?.cursor_position()?;
    Some(c / scale.max(0.01))
}

/// The middle button: a click pings the ground under the cursor; held, the wheel opens and the
/// option it is let go over goes down, a raid icon on the enemy under the cursor if there is one.
#[allow(clippy::too_many_arguments)]
fn ping_input(
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time<Real>>,
    phase: Res<State<Phase>>,
    session: Res<Session>,
    chat: Res<Chat>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scale: Res<UiScale>,
    aim: Res<AimPoint>,
    enemies: Query<(Entity, &Transform, &Enemy)>,
    mut press: ResMut<Press>,
    mut out: MessageWriter<PingInput>,
) {
    let live = matches!(
        session.link,
        super::super::Link::Hosting { .. } | super::super::Link::Joined { .. }
    ) && session.in_run
        && *phase.get() == Phase::Playing
        && !chat.typing();
    if !live {
        if press.down.is_some() {
            *press = Press::default();
        }
        return;
    }
    if mouse.just_pressed(MouseButton::Middle) {
        let Some(at) = cursor_ui(&windows, scale.0) else {
            return;
        };
        let ground = aim.0;
        let enemy = ground.and_then(|g| {
            enemies
                .iter()
                .filter(|(_, _, e)| !e.dead)
                .map(|(e, tf, _)| (e, tf.translation))
                .map(|(e, p)| (e, p, Vec2::new(p.x - g.x, p.z - g.z).length()))
                .filter(|(.., d)| *d <= MARK_REACH)
                .min_by(|a, b| a.2.total_cmp(&b.2))
                .map(|(e, p, _)| (e, p))
        });
        *press = Press {
            down: Some((at, ground, enemy)),
            held: 0.0,
            wheel: false,
        };
        return;
    }
    let Some((at, ground, enemy)) = press.down else {
        return;
    };
    if mouse.pressed(MouseButton::Middle) {
        press.held += time.delta_secs();
        if press.held >= HOLD_SECS && !press.wheel {
            press.wheel = true;
        }
        return;
    }
    // Let go.
    let wheel = press.wheel;
    *press = Press::default();
    let kind = if wheel {
        let now = cursor_ui(&windows, scale.0).unwrap_or(at);
        match wheel_pick(now - at) {
            Some(i) => KINDS[i],
            None => return,
        }
    } else {
        PingKind::Ping
    };
    let target = match kind {
        PingKind::Mark(_) => enemy,
        _ => None,
    };
    let Some(spot) = target.map(|t| t.1).or(ground) else {
        return;
    };
    out.write(PingInput {
        kind,
        at: spot.to_array(),
        target: target.map(|t| Ent(t.0)),
    });
}

/// The host passes every ping on to everyone, the quick ones into the chat too.
fn host_pings(
    mut pings: MessageReader<FromClient<PingInput>>,
    peers: Query<&Peer>,
    session: Res<Session>,
    mut out: MessageWriter<ToClients<Pinged>>,
    mut lines: MessageWriter<ToClients<Heard>>,
) {
    for p in pings.read() {
        let Some(player) = player_of(p.client_id, &peers) else {
            continue;
        };
        let m = p.message;
        if !Vec3::from_array(m.at).is_finite() {
            continue;
        }
        out.write(ToClients {
            targets: SendTargets::All,
            message: Pinged {
                player,
                kind: m.kind,
                at: m.at,
                target: m.target,
            },
        });
        if matches!(m.kind, PingKind::Help | PingKind::Boss | PingKind::Chest) {
            lines.write(to_all(Heard::Line(Line {
                player,
                name: name_of(&session, player),
                body: Body::Pinged(m.kind),
            })));
        }
    }
}

/// A ping heard goes down here, with its sound.
fn hear_pings(
    mut pinged: MessageReader<Pinged>,
    mut markers: ResMut<Markers>,
    mut cues: MessageWriter<SoundCue>,
) {
    for p in pinged.read() {
        markers.place(Marker {
            kind: p.kind,
            at: Vec3::from_array(p.at),
            target: p.target.map(|t| t.0),
            age: 0.0,
            by: p.player,
        });
        cues.write(SoundCue::flat(sound(p.kind)));
    }
}

/// Marks age, follow their units and go; the minimap is told where they are.
fn tick_markers(
    time: Res<Time<Real>>,
    mut markers: ResMut<Markers>,
    enemies: Query<(&Transform, &Enemy)>,
    mut map: ResMut<MapMarks>,
) {
    if markers.list.is_empty() && map.0.is_empty() {
        return;
    }
    markers.tick(time.delta_secs(), |e| {
        enemies
            .get(e)
            .ok()
            .filter(|(_, en)| !en.dead)
            .map(|(tf, en)| tf.translation + Vec3::Y * (2.2 * en.def.scale.max(0.6) + 0.6))
    });
    map.0 = markers
        .list
        .iter()
        .map(|m| MapMark {
            at: m.at,
            icon: m.kind.icon(),
            age: m.age,
        })
        .collect();
}

/// Each mark's icon (and a quick ping's words) over its spot, where the camera sees it.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn show_markers(
    mut commands: Commands,
    markers: Res<Markers>,
    font: Option<Res<HudFont>>,
    session: Res<Session>,
    scale: Res<UiScale>,
    cams: Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
    roots: Query<Entity, With<MarkersRoot>>,
    mut nodes: Query<(&MarkerNode, &mut Node, &mut Visibility)>,
    mut built: Local<Option<u32>>,
) {
    let Some(font) = font else { return };
    let root = match roots.single() {
        Ok(r) => r,
        Err(_) => {
            commands.spawn((
                MarkersRoot,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                GlobalZIndex(150),
            ));
            return;
        }
    };
    if *built != Some(markers.version) {
        *built = Some(markers.version);
        commands.entity(root).despawn_related::<Children>();
        commands.entity(root).with_children(|r| {
            for (i, m) in markers.list.iter().enumerate() {
                r.spawn((
                    MarkerNode(i),
                    Node {
                        position_type: PositionType::Absolute,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        width: Val::Px(120.0),
                        ..default()
                    },
                    Visibility::Hidden,
                ))
                .with_children(|n| {
                    n.spawn((
                        Node {
                            width: Val::Px(30.0),
                            height: Val::Px(30.0),
                            ..default()
                        },
                        m.kind.art(),
                    ));
                    if !matches!(m.kind, PingKind::Mark(_) | PingKind::Ping) {
                        let who = name_of(&session, m.by);
                        n.spawn((
                            label(&font.0, format!("{} ({who})", m.kind.label()), 13.0, PARTY),
                            TextShadow::default(),
                        ));
                    }
                });
            }
        });
        return;
    }
    let Ok((cam, cam_tf)) = cams.single() else {
        return;
    };
    let s = scale.0.max(0.01);
    for (n, mut node, mut vis) in &mut nodes {
        let Some(m) = markers.list.get(n.0) else {
            continue;
        };
        let lift = if m.target.is_some() { 0.0 } else { 1.0 };
        let shown = cam
            .world_to_viewport(cam_tf, m.at + Vec3::Y * lift)
            .ok()
            .map(|p| p / s);
        match shown {
            Some(p) => {
                let (left, top) = (Val::Px(p.x - 60.0), Val::Px(p.y - 15.0));
                if node.left != left || node.top != top {
                    node.left = left;
                    node.top = top;
                }
                vis.set_if_neq(Visibility::Inherited);
            }
            None => {
                vis.set_if_neq(Visibility::Hidden);
            }
        }
    }
}

/// The wheel round the cursor while the middle button is held, the option under it lit.
#[allow(clippy::type_complexity)]
fn show_wheel(
    mut commands: Commands,
    press: Res<Press>,
    font: Option<Res<HudFont>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scale: Res<UiScale>,
    mut roots: Query<(&mut Node, &mut Visibility), With<WheelRoot>>,
    mut options: Query<(&WheelOption, &mut BackgroundColor, &mut BorderColor)>,
) {
    let Some(font) = font else { return };
    if roots.is_empty() {
        commands
            .spawn((
                WheelRoot,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(0.0),
                    height: Val::Px(0.0),
                    ..default()
                },
                GlobalZIndex(330),
                Visibility::Hidden,
            ))
            .with_children(|w| {
                for (i, kind) in KINDS.iter().enumerate() {
                    let at = wheel_spot(i);
                    w.spawn((
                        WheelOption(i),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(at.x - 20.0),
                            top: Val::Px(at.y - 20.0),
                            width: Val::Px(40.0),
                            height: Val::Px(40.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BorderColor::all(Color::srgba(1.0, 0.82, 0.0, 0.6)),
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
                    ))
                    .with_children(|o| match kind {
                        PingKind::Mark(_) => {
                            o.spawn((
                                Node {
                                    width: Val::Px(32.0),
                                    height: Val::Px(32.0),
                                    ..default()
                                },
                                kind.art(),
                            ));
                        }
                        _ => {
                            o.spawn((
                                label(&font.0, kind.label(), 11.0, Color::WHITE),
                                TextShadow::default(),
                            ));
                        }
                    });
                }
            });
        return;
    }
    let s = scale.0.max(0.01);
    let open = press.wheel.then_some(()).and(press.down.map(|d| d.0));
    let hot = open.and_then(|at| {
        let now = windows.single().ok()?.cursor_position()? / s;
        wheel_pick(now - at)
    });
    for (mut node, mut vis) in &mut roots {
        match open {
            Some(at) => {
                let (left, top) = (Val::Px(at.x), Val::Px(at.y));
                if node.left != left || node.top != top {
                    node.left = left;
                    node.top = top;
                }
                vis.set_if_neq(Visibility::Inherited);
            }
            None => {
                vis.set_if_neq(Visibility::Hidden);
            }
        }
    }
    for (o, mut bg, mut edge) in &mut options {
        let lit = hot == Some(o.0);
        let (fill, rim) = if lit {
            (Color::srgba(0.45, 0.35, 0.05, 0.85), Color::WHITE)
        } else {
            (
                Color::srgba(0.0, 0.0, 0.0, 0.6),
                Color::srgba(1.0, 0.82, 0.0, 0.6),
            )
        };
        if bg.0 != fill {
            bg.0 = fill;
        }
        if edge.top != rim {
            *edge = BorderColor::all(rim);
        }
    }
}

/// Back at the menu, the marks are gone.
fn clear_markers(mut markers: ResMut<Markers>, mut map: ResMut<MapMarks>) {
    markers.list.clear();
    markers.version = markers.version.wrapping_add(1);
    map.0.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: f32) -> Marker {
        Marker {
            kind: PingKind::Ping,
            at: Vec3::new(x, 0.0, 0.0),
            target: None,
            age: 0.0,
            by: 0,
        }
    }

    #[test]
    fn marks_on_the_ground_fade_and_marks_on_enemies_last_until_they_die() {
        let mut world = World::new();
        let boar = world.spawn_empty().id();
        let mut m = Markers::default();
        m.place(at(1.0));
        m.place(Marker {
            kind: PingKind::Mark(8),
            target: Some(boar),
            ..at(2.0)
        });
        m.place(Marker {
            kind: PingKind::Mark(1),
            ..at(3.0)
        });
        let alive = |pos: Vec3| move |_: Entity| Some(pos);
        m.tick(MARK_SECS - 0.1, alive(Vec3::Y));
        assert_eq!(m.list.len(), 3, "everything holds for eight seconds");
        assert_eq!(m.list[1].at, Vec3::Y, "a mark follows its unit");
        m.tick(0.2, alive(Vec3::Y));
        assert_eq!(
            m.list.iter().map(|x| x.kind).collect::<Vec<_>>(),
            [PingKind::Mark(8)],
            "only the skull on the boar is left"
        );
        m.tick(60.0, alive(Vec3::X));
        assert_eq!(m.list.len(), 1, "it stays while the boar lives");
        m.tick(0.1, |_| None);
        assert!(m.list.is_empty(), "and goes when it dies");
    }

    #[test]
    fn a_raid_icon_is_in_one_place_and_a_unit_wears_one() {
        let mut world = World::new();
        let (a, b) = (world.spawn_empty().id(), world.spawn_empty().id());
        let mut m = Markers::default();
        let mark = |icon, target| Marker {
            kind: PingKind::Mark(icon),
            target,
            ..at(0.0)
        };
        m.place(mark(8, Some(a)));
        m.place(mark(8, Some(b)));
        assert_eq!(m.list.len(), 1, "the skull moved");
        assert_eq!(m.list[0].target, Some(b));
        m.place(mark(7, Some(b)));
        assert_eq!(m.list.len(), 1, "b swapped its skull for a cross");
        assert_eq!(m.list[0].kind, PingKind::Mark(7));
        m.place(at(5.0));
        m.place(at(6.0));
        assert_eq!(m.list.len(), 3, "pings pile up");
    }

    #[test]
    fn the_wheel_reads_the_cursor_clockwise_from_the_top() {
        assert_eq!(wheel_pick(Vec2::new(3.0, -4.0)), None, "the dead zone");
        assert_eq!(wheel_pick(Vec2::new(0.0, -60.0)), Some(0), "up: the skull");
        for (i, kind) in KINDS.iter().enumerate() {
            assert_eq!(wheel_pick(wheel_spot(i)), Some(i), "{kind:?}");
        }
        assert_eq!(KINDS[0], PingKind::Mark(8));
        assert_eq!(PingKind::Mark(8).label(), "Skull");
        assert_eq!(PingKind::Mark(1).label(), "Star");
        assert_eq!(raid_icon_coords(8), [0.75, 1.0, 0.25, 0.5]);
        assert_eq!(raid_icon_coords(1), [0.0, 0.25, 0.0, 0.25]);
        assert_eq!(raid_icon_coords(5), [0.0, 0.25, 0.25, 0.5]);
    }
}
