//! Telegraphs on the ground, drawn on every machine from the replicated [`Telegraph`]: the shape's
//! edge, and a fill that grows from the centre over the wind-up (RAIDS.md section 3), in the
//! colour of what it does. A pool left burning keeps its edge and a slow pulse until it cools.

use bevy::prelude::*;

use super::super::Run;
use super::{forward, Shape, Telegraph, Tone};

/// Drawn just over the ground, so the floor does not swallow the lines.
const LIFT: f32 = 0.15;
/// Line segments in a full circle.
const SEGMENTS: usize = 48;
/// Inner rings that make the fill read as filled.
const FILL_RINGS: usize = 4;

/// The tone's colour (RAIDS.md section 3): red, purple, yellow, blue, orange.
pub(crate) fn color(tone: Tone) -> Color {
    match tone {
        Tone::Damage => Color::srgb(1.0, 0.15, 0.1),
        Tone::Debuff => Color::srgb(0.7, 0.3, 1.0),
        Tone::Control => Color::srgb(1.0, 0.9, 0.15),
        Tone::Safe => Color::srgb(0.25, 0.55, 1.0),
        Tone::Split => Color::srgb(1.0, 0.55, 0.1),
    }
}

/// Points along an arc of radius `r` round `c`, from `from` to `to` radians measured from the
/// facing `yaw`.
fn arc(c: Vec3, yaw: f32, r: f32, from: f32, to: f32) -> Vec<Vec3> {
    let steps = (((to - from).abs() / std::f32::consts::TAU) * SEGMENTS as f32).ceil() as usize;
    let steps = steps.max(2);
    (0..=steps)
        .map(|i| {
            let a = from + (to - from) * i as f32 / steps as f32;
            c + forward(yaw + a) * r
        })
        .collect()
}

fn ring(gizmos: &mut Gizmos, c: Vec3, r: f32, color: Color) {
    if r > 0.05 {
        gizmos.linestrip(arc(c, 0.0, r, 0.0, std::f32::consts::TAU), color);
    }
}

/// One shape's outline, and its fill grown to `fill` (0 to 1).
fn shape(gizmos: &mut Gizmos, t: &Telegraph, fill: f32, edge: Color, inner: Color) {
    let c = t.origin() + Vec3::Y * LIFT;
    match t.shape {
        Shape::Circle { radius } => {
            ring(gizmos, c, radius, edge);
            for k in 1..=FILL_RINGS {
                ring(
                    gizmos,
                    c,
                    radius * fill * k as f32 / FILL_RINGS as f32,
                    inner,
                );
            }
        }
        Shape::Donut { inner: r0, outer } => {
            ring(gizmos, c, r0, edge);
            ring(gizmos, c, outer, edge);
            // The fill closes in from the outer edge.
            for k in 1..=FILL_RINGS {
                let r = outer - (outer - r0) * fill * k as f32 / FILL_RINGS as f32;
                ring(gizmos, c, r, inner);
            }
        }
        Shape::Cone { radius, half_angle } => {
            let mut outline = vec![c];
            outline.extend(arc(c, t.yaw, radius, -half_angle, half_angle));
            outline.push(c);
            gizmos.linestrip(outline, edge);
            for k in 1..=FILL_RINGS {
                let r = radius * fill * k as f32 / FILL_RINGS as f32;
                if r > 0.05 {
                    gizmos.linestrip(arc(c, t.yaw, r, -half_angle, half_angle), inner);
                }
            }
        }
        Shape::Line { length, width } => {
            let ahead = forward(t.yaw);
            let side = Vec3::new(-ahead.z, 0.0, ahead.x) * (width / 2.0);
            let rect = |len: f32| {
                vec![
                    c + side,
                    c + side + ahead * len,
                    c - side + ahead * len,
                    c - side,
                    c + side,
                ]
            };
            gizmos.linestrip(rect(length), edge);
            for k in 1..=FILL_RINGS {
                let len = length * fill * k as f32 / FILL_RINGS as f32;
                if len > 0.05 {
                    gizmos.linestrip(rect(len), inner);
                }
            }
        }
    }
}

/// Every telegraph standing on the run's clock.
pub(super) fn draw_telegraphs(run: Res<Run>, telegraphs: Query<&Telegraph>, mut gizmos: Gizmos) {
    let now = run.elapsed;
    for t in &telegraphs {
        if now > t.until {
            continue;
        }
        let base = color(t.tone);
        if now < t.land {
            let fill = t.progress(now);
            shape(
                &mut gizmos,
                t,
                fill,
                base,
                base.with_alpha(0.45 + 0.4 * fill),
            );
        } else {
            // A burning pool: its edge, and a slow pulse inside.
            let pulse = 0.5 + 0.5 * (now * 3.0).sin();
            shape(
                &mut gizmos,
                t,
                0.6 + 0.4 * pulse,
                base,
                base.with_alpha(0.35),
            );
        }
    }
}
