//! The allies in the world: a nameplate over each one's head as the client draws a friendly
//! plate (`Nameplate-Border` over the bar, the level in the border's seat, the name over it in
//! its class's colour; a downed ally's skull in the seat and "Downed" over the name), an arrow at
//! the screen's edge, in its class's colour, toward each ally off the screen (red and pulsing for
//! one down), and the raid warning, "<Name> is down!" with its horn, as an ally falls.

use std::collections::HashMap;

use benilla_ui::script::nameplate;
use benilla_world::view::WorldCamera;
use bevy::prelude::*;

use super::{
    blink, class_color, in_run, Member, PartyView, ARROW, MAX_ALLIES, RED, REVIVE_YELLOW,
    SHIELD_TINT, SKULL,
};
use crate::sound::SoundCue;
use crate::survivors::hud::{self, ArtDone, HudFont, UiArt};
use crate::survivors::{hero, Phase};
use crate::vplates::{
    gx_px, plate_basis, text_px, BAR_H, BAR_OFF_X, BAR_OFF_Y, BAR_W, CON_YELLOW, LEVEL_H,
    LEVEL_OFF_X, LEVEL_OFF_Y, PLATE_FRIENDLY, PLATE_H, PLATE_W,
};

/// The plate's name height and its skull's size, in gx units (the client's own, as
/// [`crate::vplates`] keeps them).
const NAME_H: f32 = 0.01;
const SKULL_SIZE: f32 = 0.01;
/// Where over a hero's feet its plate sits, in yards: over the head, by the plate's lift.
const SEAT_HEIGHT: f32 = 2.6;
/// The edge arrow, and how far in from the screen's edge it rides, in UI units.
const ARROW_SIZE: f32 = 28.0;
const EDGE_MARGIN: f32 = 34.0;
/// The arrow's name, toward the screen's middle from the arrow.
const ARROW_LABEL_W: f32 = 140.0;
const ARROW_LABEL_GAP: f32 = 24.0;
/// The plates under the HUD's own windows, the arrows over the plates.
const PLATES_Z: i32 = 401;
const ARROWS_Z: i32 = 402;

/// `PlaySound("RaidWarning")`, as `RaidWarningFrame_OnEvent` plays it.
pub(super) const RAID_WARNING_SOUND: &str = "RaidWarning";
/// `RaidWarningFrame`: 10 under `UIErrorsFrame` (TOP, (0, -122), 60 tall), in
/// `GameFontNormalHuge`, the raid warning's chat colour, newest on top.
const WARNING_TOP: f32 = 192.0;
const WARNING_SIZE: f32 = 20.0;
const WARNING_COLOR: Color = Color::srgb(1.0, 0.28, 0.0);
const WARNING_HOLD: f32 = 5.0;
const WARNING_FADE: f32 = 1.5;
const MAX_WARNINGS: usize = 3;
/// Over the bossbar, under the level-up.
const WARNING_Z: i32 = 407;

#[derive(Component)]
pub(super) struct PlatesRoot;

#[derive(Component, Clone, Copy, Debug)]
pub(super) struct PlatePart {
    slot: usize,
    piece: Piece,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Piece {
    Plate,
    Bar,
    Fill,
    Shield,
    Revive,
    Seat,
    Skull,
    /// The column the name (and "Downed" over it) stands in, its bottom on the plate's middle.
    Names,
}

#[derive(Component, Clone, Copy, Debug)]
pub(super) struct PlateLabel {
    slot: usize,
    line: PlateLine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PlateLine {
    Name,
    Tag,
    Level,
}

#[derive(Component)]
pub(super) struct ArrowsRoot;

#[derive(Component, Clone, Copy, Debug)]
pub(super) struct ArrowPart {
    slot: usize,
    label: bool,
}

#[derive(Component)]
pub(super) struct ArrowText(usize);

#[derive(Component)]
pub(super) struct WarningRoot;

#[derive(Component)]
pub(super) struct WarningLine(usize);

/// The raid warnings on screen, newest first, each with its age in seconds; and whether each
/// member was down when last seen, so a fall is told once.
#[derive(Resource, Default)]
pub(super) struct RaidWarnings {
    lines: Vec<(String, f32)>,
    down: HashMap<Entity, bool>,
}

fn unpainted() -> ImageNode {
    ImageNode {
        color: Color::NONE,
        ..default()
    }
}

fn absolute() -> Node {
    Node {
        position_type: PositionType::Absolute,
        ..default()
    }
}

/// The plates, the arrows and the warning line, all hidden until a party needs them.
pub(super) fn setup(mut commands: Commands, font: Res<HudFont>) {
    let f = &font.0;
    let full = || Node {
        position_type: PositionType::Absolute,
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        ..default()
    };
    commands
        .spawn((
            PlatesRoot,
            full(),
            GlobalZIndex(PLATES_Z),
            Visibility::Hidden,
            bevy::ui::FocusPolicy::Pass,
        ))
        .with_children(|root| {
            for slot in 0..MAX_ALLIES {
                let part = |piece| PlatePart { slot, piece };
                let label = |line| PlateLabel { slot, line };
                let bar_fill = |piece, tint| {
                    (
                        part(piece),
                        Node {
                            height: Val::Percent(100.0),
                            width: Val::Percent(0.0),
                            ..absolute()
                        },
                        unpainted(),
                        UiArt::tinted(nameplate::BAR_FILL_TEXTURE, tint),
                    )
                };
                root.spawn((
                    part(Piece::Plate),
                    Node {
                        display: Display::None,
                        ..absolute()
                    },
                ))
                .with_children(|plate| {
                    let [r, g, b, _] = PLATE_FRIENDLY;
                    plate
                        .spawn((part(Piece::Bar), absolute(), BackgroundColor(super::TRACK)))
                        .with_children(|bar| {
                            bar.spawn(bar_fill(Piece::Fill, Color::srgb(r, g, b)));
                            bar.spawn(bar_fill(Piece::Shield, SHIELD_TINT));
                            bar.spawn(bar_fill(Piece::Revive, REVIVE_YELLOW));
                        });
                    plate.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..absolute()
                        },
                        unpainted(),
                        UiArt::crop(nameplate::BORDER_TEXTURE, [0.0, 1.0, 0.0, 1.0]),
                    ));
                    plate
                        .spawn((
                            part(Piece::Names),
                            Node {
                                left: Val::Px(0.0),
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                overflow: Overflow::visible(),
                                ..absolute()
                            },
                        ))
                        .with_children(|names| {
                            for (line, color) in
                                [(PlateLine::Tag, RED), (PlateLine::Name, Color::WHITE)]
                            {
                                names.spawn((
                                    label(line),
                                    hud::text_bundle(f, "", 12.0, color),
                                    TextLayout::new(Justify::Center, LineBreak::NoWrap),
                                    Node::default(),
                                ));
                            }
                        });
                    let [r, g, b, _] = CON_YELLOW;
                    plate
                        .spawn((
                            part(Piece::Seat),
                            Node {
                                width: Val::Px(0.0),
                                height: Val::Px(0.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                overflow: Overflow::visible(),
                                ..absolute()
                            },
                        ))
                        .with_children(|seat| {
                            seat.spawn((
                                label(PlateLine::Level),
                                hud::text_bundle(f, "", 11.0, Color::srgb(r, g, b)),
                                TextLayout::new(Justify::Center, LineBreak::NoWrap),
                                Node {
                                    position_type: PositionType::Absolute,
                                    ..default()
                                },
                            ));
                            seat.spawn((
                                part(Piece::Skull),
                                Node {
                                    display: Display::None,
                                    ..absolute()
                                },
                                unpainted(),
                                UiArt::crop(SKULL, [0.0, 1.0, 0.0, 1.0]),
                            ));
                        });
                });
            }
        });
    commands
        .spawn((
            ArrowsRoot,
            full(),
            GlobalZIndex(ARROWS_Z),
            Visibility::Hidden,
            bevy::ui::FocusPolicy::Pass,
        ))
        .with_children(|root| {
            for slot in 0..MAX_ALLIES {
                root.spawn((
                    ArrowPart { slot, label: false },
                    Node {
                        width: Val::Px(ARROW_SIZE),
                        height: Val::Px(ARROW_SIZE),
                        display: Display::None,
                        ..absolute()
                    },
                    UiTransform::default(),
                    unpainted(),
                    UiArt::tinted(ARROW, Color::WHITE),
                ));
                root.spawn((
                    ArrowPart { slot, label: true },
                    Node {
                        width: Val::Px(ARROW_LABEL_W),
                        justify_content: JustifyContent::Center,
                        display: Display::None,
                        ..absolute()
                    },
                ))
                .with_children(|holder| {
                    holder.spawn((
                        ArrowText(slot),
                        hud::text_bundle(f, "", 10.0, Color::WHITE),
                        TextLayout::new(Justify::Center, LineBreak::NoWrap),
                    ));
                });
            }
        });
    commands
        .spawn((
            WarningRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(WARNING_TOP),
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
            GlobalZIndex(WARNING_Z),
            Visibility::Hidden,
            bevy::ui::FocusPolicy::Pass,
        ))
        .with_children(|root| {
            for i in 0..MAX_WARNINGS {
                root.spawn((
                    WarningLine(i),
                    hud::text_bundle(f, "", WARNING_SIZE, WARNING_COLOR),
                    TextLayout::new(Justify::Center, LineBreak::NoWrap),
                    Node {
                        display: Display::None,
                        ..default()
                    },
                ));
            }
        });
}

/// Whether a point `p` on a screen of `size` (the same units) is on it.
pub(super) fn on_screen(p: Vec2, size: Vec2) -> bool {
    p.x >= 0.0 && p.y >= 0.0 && p.x <= size.x && p.y <= size.y
}

/// Where an edge arrow pointing `dir` from the screen's middle (+x right, +y down) rides, on a
/// screen of `size`, `margin` in from its edges.
pub(super) fn edge_point(dir: Vec2, size: Vec2, margin: f32) -> Vec2 {
    let centre = size / 2.0;
    let half = (centre - Vec2::splat(margin)).max(Vec2::ONE);
    let d = dir.normalize_or(Vec2::Y);
    let reach = |h: f32, d: f32| {
        if d.abs() > 1e-6 {
            h / d.abs()
        } else {
            f32::INFINITY
        }
    };
    centre + d * reach(half.x, d.x).min(reach(half.y, d.y))
}

/// How far clockwise the arrow art (which points up) turns to point `dir`, in radians.
pub(super) fn arrow_angle(dir: Vec2) -> f32 {
    dir.x.atan2(-dir.y)
}

/// Where a member's plate seats: over its head.
fn head(m: &Member) -> Vec3 {
    m.position + Vec3::Y * SEAT_HEIGHT
}

fn set_display(node: &mut Node, on: bool) {
    let want = if on { Display::Flex } else { Display::None };
    if node.display != want {
        node.display = want;
    }
}

fn set_visible(v: &mut Visibility, on: bool) {
    let want = if on {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *v != want {
        *v = want;
    }
}

fn set_px(slot: &mut Val, px: f32) {
    let want = Val::Px(px);
    if *slot != want {
        *slot = want;
    }
}

fn set_pct(slot: &mut Val, share: f32) {
    let want = Val::Percent(share * 100.0);
    if *slot != want {
        *slot = want;
    }
}

/// Each ally's plate over its head while it is on the screen, sized from the screen's diagonal
/// as the client sizes a plate.
#[allow(clippy::type_complexity)] // one Bevy system's full input set
pub(super) fn refresh_plates(
    phase: Res<State<Phase>>,
    view: Res<PartyView>,
    ui_scale: Res<UiScale>,
    cams: Query<(&Camera, &Transform), With<WorldCamera>>,
    mut root: Query<&mut Visibility, With<PlatesRoot>>,
    mut parts: Query<(&PlatePart, &mut Node, Option<&mut ImageNode>, Has<ArtDone>)>,
    mut labels: Query<
        (
            &PlateLabel,
            &mut Text,
            &mut TextFont,
            &mut TextColor,
            &mut Node,
        ),
        Without<PlatePart>,
    >,
) {
    let show = view.coop() && in_run(*phase.get());
    if let Ok(mut v) = root.single_mut() {
        set_visible(&mut v, show);
    }
    let cam = cams.single().ok().and_then(|(cam, tf)| {
        let viewport = cam.logical_viewport_size()?;
        Some((cam, GlobalTransform::from(*tf), viewport))
    });
    let allies = view.allies();
    // Each slot's ally and the point over its head, on the screen.
    let seats: Vec<Option<(&Member, Vec2)>> = (0..MAX_ALLIES)
        .map(|slot| {
            let m = allies.get(slot).copied().filter(|_| show)?;
            let (cam, tf, viewport) = cam.as_ref()?;
            let p = cam.world_to_viewport(tf, head(m)).ok()?;
            on_screen(p, *viewport).then_some((m, p))
        })
        .collect();
    let s = ui_scale.0.max(1e-3);
    let basis = cam
        .as_ref()
        .map_or(0.0, |(.., viewport)| plate_basis(*viewport));
    let gx = |v: f32| gx_px(v, basis) / s;
    let (pw, ph) = (gx(PLATE_W), gx(PLATE_H));
    for (part, mut node, img, ready) in &mut parts {
        let seat = seats.get(part.slot).copied().flatten();
        if part.piece == Piece::Plate {
            set_display(&mut node, seat.is_some());
        }
        let Some((m, p)) = seat else { continue };
        match part.piece {
            Piece::Plate => {
                set_px(&mut node.left, p.x / s - pw / 2.0);
                set_px(&mut node.top, p.y / s - ph);
                set_px(&mut node.width, pw);
                set_px(&mut node.height, ph);
            }
            Piece::Bar => {
                set_px(&mut node.left, gx(BAR_OFF_X));
                set_px(&mut node.top, ph - gx(BAR_OFF_Y) - gx(BAR_H));
                set_px(&mut node.width, gx(BAR_W));
                set_px(&mut node.height, gx(BAR_H));
            }
            Piece::Fill => set_pct(&mut node.width, if m.downed { 0.0 } else { m.health() }),
            Piece::Shield => set_pct(
                &mut node.width,
                if m.downed { 0.0 } else { m.shield_share() },
            ),
            Piece::Revive => set_pct(&mut node.width, if m.downed { m.revive } else { 0.0 }),
            Piece::Seat => {
                set_px(&mut node.left, pw - gx(LEVEL_OFF_X));
                set_px(&mut node.top, ph - gx(LEVEL_OFF_Y));
            }
            Piece::Skull => {
                let size = gx(SKULL_SIZE);
                set_px(&mut node.width, size);
                set_px(&mut node.height, size);
                set_px(&mut node.left, -size / 2.0);
                set_px(&mut node.top, -size / 2.0);
                set_display(&mut node, m.downed);
                if let (Some(mut img), true) = (img, ready) {
                    if img.color != Color::WHITE {
                        img.color = Color::WHITE;
                    }
                }
            }
            Piece::Names => set_px(&mut node.bottom, ph / 2.0),
        }
    }
    let name_px = text_px(NAME_H, basis) / s;
    let level_px = text_px(LEVEL_H, basis) / s;
    for (label, mut text, mut font, mut color, mut node) in &mut labels {
        let Some((m, _)) = seats.get(label.slot).copied().flatten() else {
            continue;
        };
        let (want, size, tone, on) = match label.line {
            PlateLine::Name => (m.name.clone(), name_px, class_color(m.class), true),
            PlateLine::Tag => ("Downed".to_string(), name_px, RED, m.downed),
            PlateLine::Level => {
                let [r, g, b, _] = CON_YELLOW;
                (
                    m.level.to_string(),
                    level_px,
                    Color::srgb(r, g, b),
                    !m.downed,
                )
            }
        };
        if **text != want {
            **text = want;
        }
        if font.font_size != size {
            font.font_size = size;
        }
        if color.0 != tone {
            color.0 = tone;
        }
        set_display(&mut node, on);
    }
}

/// An arrow at the screen's edge toward each ally off it, in its class's colour, its name and
/// distance beside it; red and pulsing for an ally down.
#[allow(clippy::type_complexity)] // one Bevy system's full input set
pub(super) fn refresh_arrows(
    phase: Res<State<Phase>>,
    view: Res<PartyView>,
    ui_scale: Res<UiScale>,
    time: Res<Time<Real>>,
    cams: Query<(&Camera, &Transform), With<WorldCamera>>,
    mut root: Query<&mut Visibility, With<ArrowsRoot>>,
    mut parts: Query<(
        &ArrowPart,
        &mut Node,
        Option<&mut UiTransform>,
        Option<&mut ImageNode>,
        Has<ArtDone>,
    )>,
    mut texts: Query<(&ArrowText, &mut Text, &mut TextColor)>,
) {
    let show = view.coop() && in_run(*phase.get());
    if let Ok(mut v) = root.single_mut() {
        set_visible(&mut v, show);
    }
    let s = ui_scale.0.max(1e-3);
    let cam = cams.single().ok().and_then(|(cam, tf)| {
        let viewport = cam.logical_viewport_size()?;
        Some((cam, GlobalTransform::from(*tf), viewport))
    });
    let allies = view.allies();
    let me = view.local();
    // Each slot's ally off the screen, the way to it from the screen's middle.
    let off: Vec<Option<(&Member, Vec2)>> = (0..MAX_ALLIES)
        .map(|slot| {
            let m = allies.get(slot).copied().filter(|_| show)?;
            let (cam, tf, viewport) = cam.as_ref()?;
            let at = head(m);
            if let Ok(p) = cam.world_to_viewport(tf, at) {
                if on_screen(p, *viewport) {
                    return None;
                }
            }
            // The camera's own axes: +x right, +y up, looking down −z; behind it as in front.
            let local = tf.affine().inverse().transform_point3(at);
            Some((m, Vec2::new(local.x, -local.y)))
        })
        .collect();
    let screen = cam.map_or(Vec2::ZERO, |(.., viewport)| viewport / s);
    let pulse = blink(time.elapsed_secs() * 1.5);
    for (part, mut node, transform, img, ready) in &mut parts {
        let arrow = off.get(part.slot).copied().flatten();
        set_display(&mut node, arrow.is_some());
        let Some((m, dir)) = arrow else { continue };
        let at = edge_point(dir, screen, EDGE_MARGIN);
        if part.label {
            let inward = at - dir.normalize_or(Vec2::Y) * ARROW_LABEL_GAP;
            set_px(&mut node.left, inward.x - ARROW_LABEL_W / 2.0);
            set_px(&mut node.top, inward.y - 6.0);
            continue;
        }
        set_px(&mut node.left, at.x - ARROW_SIZE / 2.0);
        set_px(&mut node.top, at.y - ARROW_SIZE / 2.0);
        if let Some(mut tf) = transform {
            let want = UiTransform {
                rotation: Rot2::radians(arrow_angle(dir)),
                scale: Vec2::splat(if m.downed { 0.9 + 0.3 * pulse } else { 1.0 }),
                ..default()
            };
            if *tf != want {
                *tf = want;
            }
        }
        if let (Some(mut img), true) = (img, ready) {
            let want = if m.downed {
                RED.with_alpha(pulse)
            } else {
                class_color(m.class)
            };
            if img.color != want {
                img.color = want;
            }
        }
    }
    for (text_of, mut text, mut color) in &mut texts {
        let Some((m, _)) = off.get(text_of.0).copied().flatten() else {
            continue;
        };
        let (want, tone) = if m.downed {
            (format!("{} is down", m.name), RED)
        } else {
            let yards = me.map(|me| hero::reach(me.position, m.position).round() as u32);
            let text = match yards {
                Some(yd) => format!("{} {yd} yd", m.name),
                None => m.name.clone(),
            };
            (text, class_color(m.class))
        };
        if **text != want {
            **text = want;
        }
        if color.0 != tone {
            color.0 = tone;
        }
    }
}

/// The warnings for allies that fell since last seen (`was`): "<Name> is down!" for each ally,
/// in a party, that was seen standing and is down now. The local hero knows it fell.
pub(super) fn newly_down(was: &HashMap<Entity, bool>, view: &PartyView) -> Vec<String> {
    if !view.coop() {
        return Vec::new();
    }
    view.members
        .iter()
        .filter(|m| !m.is_local && m.downed && was.get(&m.entity) == Some(&false))
        .map(|m| format!("{} is down!", m.name))
        .collect()
}

/// A warning `age` seconds old: its alpha, and whether it still shows.
pub(super) fn warning_alpha(age: f32) -> Option<f32> {
    if age < WARNING_HOLD {
        Some(1.0)
    } else if age < WARNING_HOLD + WARNING_FADE {
        Some(1.0 - (age - WARNING_HOLD) / WARNING_FADE)
    } else {
        None
    }
}

/// `RaidWarningFrame`: each ally's fall in the warning line with the raid warning's horn, held
/// and faded, newest on top. Back at the menu the run's warnings go.
#[allow(clippy::type_complexity)] // one Bevy system's full input set
pub(super) fn raid_warnings(
    phase: Res<State<Phase>>,
    view: Res<PartyView>,
    time: Res<Time<Real>>,
    mut warnings: ResMut<RaidWarnings>,
    mut cues: MessageWriter<SoundCue>,
    mut root: Query<&mut Visibility, With<WarningRoot>>,
    mut lines: Query<(&WarningLine, &mut Text, &mut TextColor, &mut Node)>,
) {
    let p = *phase.get();
    let w = &mut *warnings;
    if p == Phase::Menu {
        w.lines.clear();
        w.down.clear();
    } else {
        let fallen = newly_down(&w.down, &view);
        if !fallen.is_empty() {
            cues.write(SoundCue::flat(RAID_WARNING_SOUND));
        }
        for line in fallen {
            w.lines.insert(0, (line, 0.0));
        }
        w.down = view.members.iter().map(|m| (m.entity, m.downed)).collect();
        let dt = time.delta_secs();
        for (_, age) in &mut w.lines {
            *age += dt;
        }
        w.lines.retain(|(_, age)| warning_alpha(*age).is_some());
        w.lines.truncate(MAX_WARNINGS);
    }
    if let Ok(mut v) = root.single_mut() {
        set_visible(&mut v, in_run(p) && !w.lines.is_empty());
    }
    for (line, mut text, mut color, mut node) in &mut lines {
        let shown = w.lines.get(line.0);
        set_display(&mut node, shown.is_some());
        let Some((s, age)) = shown else { continue };
        if **text != *s {
            **text = s.clone();
        }
        let tone = WARNING_COLOR.with_alpha(warning_alpha(*age).unwrap_or(0.0));
        if color.0 != tone {
            color.0 = tone;
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::super::tests::{member, view_of};
    use super::*;

    #[test]
    fn an_arrow_rides_the_edge_toward_its_ally() {
        let size = Vec2::new(1000.0, 600.0);
        // Straight right: the right edge, at mid height.
        assert_eq!(edge_point(Vec2::X, size, 20.0), Vec2::new(980.0, 300.0));
        // Straight up (screen y down): the top edge.
        assert_eq!(edge_point(-Vec2::Y, size, 20.0), Vec2::new(500.0, 20.0));
        // A corner's diagonal meets the nearer edge first.
        let p = edge_point(Vec2::new(1.0, 1.0), size, 0.0);
        assert!(
            (p.y - 600.0).abs() < 1e-3 && (p.x - 800.0).abs() < 1e-3,
            "{p}"
        );
        // No way at all points down.
        assert_eq!(edge_point(Vec2::ZERO, size, 20.0), Vec2::new(500.0, 580.0));
        // The art points up: right is a quarter turn clockwise, down a half.
        assert!(arrow_angle(-Vec2::Y).abs() < 1e-6);
        assert!((arrow_angle(Vec2::X) - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        assert!((arrow_angle(Vec2::Y).abs() - std::f32::consts::PI).abs() < 1e-6);
        assert!(on_screen(Vec2::new(10.0, 599.0), size));
        assert!(!on_screen(Vec2::new(-1.0, 10.0), size));
        assert!(!on_screen(Vec2::new(10.0, 601.0), size));
    }

    #[test]
    fn a_fall_is_warned_once_for_an_ally_in_a_party() {
        let mut ally = member(1, "Priest");
        ally.name = "Anduin".into();
        let me = member(0, "Warrior");
        let standing = view_of(vec![me.clone(), ally.clone()]);
        let seen = |view: &PartyView| -> HashMap<Entity, bool> {
            view.members.iter().map(|m| (m.entity, m.downed)).collect()
        };
        assert!(newly_down(&HashMap::new(), &standing).is_empty());
        let mut fallen = ally.clone();
        fallen.downed = true;
        let down = view_of(vec![me.clone(), fallen.clone()]);
        assert_eq!(newly_down(&seen(&standing), &down), ["Anduin is down!"]);
        // Still down: told once.
        assert!(newly_down(&seen(&down), &down).is_empty());
        // Seen down from the first (a join): no news.
        assert!(newly_down(&HashMap::new(), &down).is_empty());
        // The local hero's own fall is not announced to it.
        let mut me_down = me.clone();
        me_down.downed = true;
        let view = view_of(vec![me_down, ally.clone()]);
        assert!(newly_down(&seen(&standing), &view).is_empty());
        // Alone, there is no one to warn.
        let mut solo = me;
        let before = seen(&view_of(vec![solo.clone()]));
        solo.downed = true;
        assert!(newly_down(&before, &view_of(vec![solo])).is_empty());
        // Held, then faded, then gone.
        assert_eq!(warning_alpha(0.0), Some(1.0));
        assert!(warning_alpha(WARNING_HOLD + WARNING_FADE / 2.0).is_some_and(|a| a < 1.0));
        assert_eq!(warning_alpha(WARNING_HOLD + WARNING_FADE), None);
    }

    fn plates_world(view: PartyView) -> World {
        let mut world = World::new();
        world.insert_resource(HudFont(Handle::default()));
        world.insert_resource(State::new(Phase::Playing));
        world.init_resource::<Time<Real>>();
        world.init_resource::<UiScale>();
        world.init_resource::<RaidWarnings>();
        world.init_resource::<Messages<SoundCue>>();
        world.insert_resource(view);
        world.run_system_once(setup).unwrap();
        world
    }

    fn visible<T: Component>(world: &mut World) -> bool {
        let mut q = world.query_filtered::<&Visibility, With<T>>();
        *q.single(world).unwrap() != Visibility::Hidden
    }

    #[test]
    fn plates_and_arrows_are_up_only_for_a_party() {
        let mut world = plates_world(view_of(vec![member(0, "Warrior")]));
        world.run_system_once(refresh_plates).unwrap();
        world.run_system_once(refresh_arrows).unwrap();
        assert!(!visible::<PlatesRoot>(&mut world));
        assert!(!visible::<ArrowsRoot>(&mut world));
        let mut world = plates_world(view_of(vec![member(0, "Warrior"), member(1, "Mage")]));
        world.run_system_once(refresh_plates).unwrap();
        world.run_system_once(refresh_arrows).unwrap();
        assert!(visible::<PlatesRoot>(&mut world));
        assert!(visible::<ArrowsRoot>(&mut world));
        // No camera, no seat: no plate is laid out.
        let mut q = world.query::<(&PlatePart, &Node)>();
        assert!(q
            .iter(&world)
            .filter(|(p, _)| p.piece == Piece::Plate)
            .all(|(_, n)| n.display == Display::None));
    }

    #[test]
    fn the_warning_line_tells_a_fall_with_the_horn() {
        let mut ally = member(1, "Priest");
        let mut world = plates_world(view_of(vec![member(0, "Warrior"), ally.clone()]));
        world.run_system_once(raid_warnings).unwrap();
        assert!(!visible::<WarningRoot>(&mut world));
        ally.downed = true;
        world.insert_resource(view_of(vec![member(0, "Warrior"), ally]));
        world.run_system_once(raid_warnings).unwrap();
        assert!(visible::<WarningRoot>(&mut world));
        let mut q = world.query::<(&WarningLine, &Text, &Node)>();
        let shown: Vec<String> = q
            .iter(&world)
            .filter(|(_, _, n)| n.display != Display::None)
            .map(|(_, t, _)| t.0.clone())
            .collect();
        assert_eq!(shown, ["Player 2 is down!"]);
        let cues = world.resource::<Messages<SoundCue>>();
        assert_eq!(cues.len(), 1);
        // Back at the menu, the run's warnings go.
        world.insert_resource(State::new(Phase::Menu));
        world.run_system_once(raid_warnings).unwrap();
        assert!(!visible::<WarningRoot>(&mut world));
    }
}
