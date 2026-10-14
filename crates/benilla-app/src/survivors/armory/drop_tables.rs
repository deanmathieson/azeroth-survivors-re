//! The Armory's Drop Tables tab: the battlegrounds down the left, each with the rarity it drops and
//! how many of its pieces the picked class has; the picked one's bosses in the center, in the
//! order and at the run time they walk in, each with its pieces, chances and whether they are
//! collected; the battleground's summary under Details. Starts on the battleground picked at the
//! menu.

use bevy::prelude::*;

use super::collection::{item_icon, item_tip, progress_bar, sheen, status};
use super::{columns, para, text, Column, TabCtx};
use crate::glue::art::{DIM, GOLD};
use crate::glue::widgets::{outlined_text, GlueText, LockHighlight};
use crate::survivors::cards::LEGENDARY;
use crate::survivors::data::{BOSS_EVERY_SECS, MAPS, VICTORY_SECS};
use crate::survivors::gear::{
    boss_rows, class_gear, clock, percent, rarity_label, zone_rows, BossRow, GearAction, GearSave,
    GearView, Rarity, ZoneRow,
};

const ROW_H: f32 = 22.0;

pub(super) fn build(parent: &mut ChildSpawnerCommands, ctx: &TabCtx) {
    let map = ctx
        .res::<GearView>()
        .and_then(|v| v.zone)
        .unwrap_or(ctx.selection.map)
        .min(MAPS.len() - 1);
    columns(parent, ctx, Some("Battlegrounds"), false, |col, p| {
        let Some(class) = ctx.selection.class else {
            if col == Column::Center {
                para(p, ctx, "Pick a class to see what drops for it.", 13.0, DIM);
            }
            return;
        };
        let gear = class_gear(ctx.world, class);
        let zones = zone_rows(class, gear);
        match col {
            Column::Left => zone_list(p, ctx, &zones, map),
            Column::Center => bosses(p, ctx, &boss_rows(map, class, gear), gear),
            Column::Details => summary(p, ctx, &zones[map], &boss_rows(map, class, gear)),
        }
    });
}

/// The battlegrounds, a row each: the name, its rarity and the class's pieces of it collected.
fn zone_list(p: &mut ChildSpawnerCommands, ctx: &TabCtx, zones: &[ZoneRow], picked: usize) {
    for zone in zones {
        let on = zone.map == picked;
        p.spawn((
            Button,
            GearAction::Zone(zone.map),
            LockHighlight(on),
            Node {
                height: Val::Px(ROW_H),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                padding: UiRect::horizontal(Val::Px(6.0)),
                ..default()
            },
        ))
        .with_children(|r| {
            sheen(r, ctx);
            r.spawn(Node {
                flex_grow: 1.0,
                ..default()
            })
            .with_children(|n| {
                text(
                    n,
                    ctx,
                    zone.name,
                    12.0,
                    if on { Color::WHITE } else { GOLD },
                );
            });
            text(r, ctx, rarity_label(zone.rarity), 11.0, zone.rarity.color());
            let done = zone.collected == zone.total;
            text(
                r,
                ctx,
                &format!("{}/{}", zone.collected, zone.total),
                11.0,
                if done {
                    super::collection::TIP_GREEN
                } else {
                    Color::WHITE
                },
            );
        });
    }
}

/// The center: each boss in roster order, its time and its pieces.
fn bosses(p: &mut ChildSpawnerCommands, ctx: &TabCtx, rows: &[BossRow], gear: &GearSave) {
    for boss in rows {
        p.spawn(Node {
            flex_shrink: 0.0,
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Baseline,
            margin: UiRect::top(Val::Px(6.0)),
            ..default()
        })
        .with_children(|h| {
            text(h, ctx, &clock(boss.secs), 13.0, Color::WHITE);
            text(h, ctx, boss.name, 13.0, GOLD);
            if boss.last {
                text(h, ctx, "final boss", 11.0, LEGENDARY);
            }
        });
        for piece in &boss.pieces {
            p.spawn((
                Button,
                item_tip(piece.item, gear),
                Node {
                    min_height: Val::Px(ROW_H),
                    flex_shrink: 0.0,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    padding: UiRect::left(Val::Px(14.0)),
                    ..default()
                },
            ))
            .with_children(|r| {
                sheen(r, ctx);
                let tint = if piece.collected {
                    Color::WHITE
                } else {
                    Color::srgb(0.45, 0.45, 0.45)
                };
                item_icon(r, piece.item.icon, 20.0, tint);
                outlined_text(
                    r,
                    Node {
                        flex_grow: 1.0,
                        flex_basis: Val::Px(0.0),
                        ..default()
                    },
                    (),
                    (),
                    GlueText {
                        text: piece.item.name,
                        size: 12.0,
                        color: piece.item.rarity.color(),
                        wrap: true,
                    },
                    ctx.font,
                    1.0,
                );
                fixed(r, ctx, 64.0, piece.item.slot.label(), DIM);
                fixed(r, ctx, 34.0, &percent(piece.chance), Color::WHITE);
                let (status, color) = status(piece.collected, gear.wears(piece.item.id));
                fixed(r, ctx, 84.0, status, color);
            });
        }
    }
}

/// `s` in a column `w` wide.
fn fixed(r: &mut ChildSpawnerCommands, ctx: &TabCtx, w: f32, s: &str, color: Color) {
    r.spawn(Node {
        width: Val::Px(w),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|c| {
        text(c, ctx, s, 11.0, color);
    });
}

/// The Details column: the battleground's name and continent, its pieces collected, what it
/// drops and when its bosses come.
fn summary(p: &mut ChildSpawnerCommands, ctx: &TabCtx, zone: &ZoneRow, rows: &[BossRow]) {
    let def = &MAPS[zone.map];
    para(p, ctx, zone.name, 16.0, zone.rarity.color());
    para(p, ctx, def.zone, 12.0, DIM);
    p.spawn(Node {
        height: Val::Px(10.0),
        flex_shrink: 0.0,
        ..default()
    });
    para(
        p,
        ctx,
        &format!("{} / {} pieces collected", zone.collected, zone.total),
        12.0,
        Color::WHITE,
    );
    progress_bar(p, zone.collected, zone.total);
    p.spawn(Node {
        height: Val::Px(10.0),
        flex_shrink: 0.0,
        ..default()
    });
    para(
        p,
        ctx,
        &format!("Its bosses drop {} gear.", rarity_label(zone.rarity)),
        12.0,
        Color::WHITE,
    );
    if let Some(last) = rows.last().filter(|b| {
        b.pieces
            .first()
            .is_some_and(|i| i.item.rarity == Rarity::Legendary)
    }) {
        para(
            p,
            ctx,
            &format!("{} drops Legendary pieces.", last.name),
            12.0,
            LEGENDARY,
        );
    }
    para(
        p,
        ctx,
        &format!(
            "A boss every {}, the last at {}; survive to {} to win.",
            clock(BOSS_EVERY_SECS.round() as u32),
            clock(rows.last().map_or(0, |b| b.secs)),
            clock(VICTORY_SECS.round() as u32),
        ),
        12.0,
        Color::WHITE,
    );
    p.spawn(Node {
        flex_grow: 1.0,
        ..default()
    });
    para(
        p,
        ctx,
        "Each piece rolls its chance once a kill. The rewards multiplier and luck raise it.",
        11.0,
        DIM,
    );
}
