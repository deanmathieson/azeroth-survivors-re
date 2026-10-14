//! The Armory's Character tab: the stat sheet a fresh run of the picked class starts on, each row
//! explained on hover, with the worn gear's summed bonuses under it; the pick's class, race and sex
//! and what each slot wears under Details. The center is left clear, so the menu's own 3D hero
//! stands in it.

use bevy::prelude::*;
use bevy::ui_render::ui_material::MaterialNode;

use super::collection::{item_tip, sheen};
use super::{columns, para, text, ArmoryTip, Column, TabCtx};
use crate::char_select::race_name;
use crate::glue::art::{class_tc, tc_rect, DIM, GOLD};
use crate::glue::widgets::{outlined_text, overlay, GlueText, Hilight};
use crate::survivors::data::ClassDef;
use crate::survivors::gear::{class_gear, summed, GearSave, Slot};
use crate::survivors::hud::rgb;
use crate::survivors::{HeroState, Stats, BASE_HP, BASE_SPEED};

/// `NORMAL_FONT_COLOR`, a tooltip's description.
const TIP_GOLD: Color = Color::srgb(1.0, 0.82, 0.0);
/// `GREEN_FONT_COLOR`, for what the class adds.
const TIP_GREEN: Color = Color::srgb(0.1, 1.0, 0.1);
const ROW_H: f32 = 24.0;

pub(super) fn build(parent: &mut ChildSpawnerCommands, ctx: &TabCtx) {
    let class = ctx.class();
    let race_sex = ctx.race_sex();
    columns(
        parent,
        ctx,
        Some("Starting Stats"),
        true,
        |col, p| match col {
            Column::Left => stat_sheet(p, ctx, class),
            Column::Center => hero_name(p, ctx, class, race_sex),
            Column::Details => details(p, ctx, class, race_sex),
        },
    );
}

/// The stats a fresh run of `class` starts on: [`HeroState::stats`] of the hero
/// [`crate::survivors::hud::start_run`] begins, before any level-up.
pub(super) fn starting_stats(class: &'static ClassDef) -> Stats {
    HeroState {
        class,
        ..HeroState::default()
    }
    .stats()
}

/// One stat-sheet row: the label, its value as shown, and the tooltip explaining it.
pub(super) struct StatRow {
    pub label: &'static str,
    pub value: String,
    pub tip: ArmoryTip,
}

/// The sheet for `s`, one row per [`Stats`] number, worded as the level-up cards word them.
pub(super) fn stat_rows(s: &Stats) -> Vec<StatRow> {
    // The class's head start over the shared base, where it has one (`HeroState::stats`).
    let adds = |base: f32, now: f32| {
        ((now - base).abs() > 1e-3).then(|| {
            format!(
                "Your class adds {} to the base {}.",
                num(now - base),
                num(base)
            )
        })
    };
    let row = |label: &'static str, value: String, what: &str, class: Option<String>| StatRow {
        label,
        value,
        tip: ArmoryTip {
            title: label.to_string(),
            lines: std::iter::once((what.to_string(), TIP_GOLD))
                .chain(class.map(|c| (c, TIP_GREEN)))
                .collect(),
        },
    };
    vec![
        row(
            "Health",
            num(s.max_hp),
            "Your maximum health. Enemies wear it down, and the run ends when it runs out.",
            adds(BASE_HP, s.max_hp),
        ),
        row(
            "Armor",
            num(s.armor),
            "Taken off the damage of every hit you take, though a hit always deals at least 1.",
            None,
        ),
        row(
            "Regeneration",
            format!("{} / sec", num(s.regen)),
            "Health restored every second.",
            None,
        ),
        row(
            "Damage",
            pct(s.might),
            "Every ability's damage is multiplied by this.",
            None,
        ),
        row(
            "Critical Strike",
            pct(s.crit),
            "The chance for each hit to critically strike for double damage.",
            None,
        ),
        row(
            "Haste",
            pct(s.haste.min(0.65)),
            "How much shorter every ability's cooldown is, up to 65%.",
            None,
        ),
        row(
            "Area",
            pct(s.area),
            "The size of every area of effect, against its base.",
            None,
        ),
        row(
            "Speed",
            format!("{} yd/sec", num(s.speed)),
            "How fast you run, in yards per second.",
            adds(BASE_SPEED, s.speed),
        ),
        row(
            "Pickup Range",
            format!("{} yd", num(s.magnet)),
            "How near an experience gem must be before it flies to you.",
            None,
        ),
        row(
            "Experience",
            pct(s.growth),
            "The experience you gain from every gem, against the base.",
            None,
        ),
    ]
}

/// A number as the tooltips print it: whole when near enough, else to one decimal.
fn num(v: f32) -> String {
    if (v - v.round()).abs() < 0.05 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.1}")
    }
}

fn pct(v: f32) -> String {
    format!("{}%", num(v * 100.0))
}

/// The left column: the class's starting numbers, a hoverable row each.
fn stat_sheet(p: &mut ChildSpawnerCommands, ctx: &TabCtx, class: Option<&'static ClassDef>) {
    let Some(class) = class else {
        para(
            p,
            ctx,
            "Pick a class to see the numbers its runs start on.",
            13.0,
            DIM,
        );
        return;
    };
    for row in stat_rows(&starting_stats(class)) {
        stat_row(p, ctx, row);
    }
    if let Some(gear) = ctx.selection.class.map(|c| class_gear(ctx.world, c)) {
        gear_bonuses(p, ctx, gear);
    }
    p.spawn(Node {
        flex_grow: 1.0,
        ..default()
    });
    para(
        p,
        ctx,
        "A fresh run, before any level-up. Hover a stat to see what it does.",
        11.0,
        DIM,
    );
}

/// Under the stats: the worn gear's bonuses, like ones summed, which every run of the class adds.
fn gear_bonuses(p: &mut ChildSpawnerCommands, ctx: &TabCtx, gear: &GearSave) {
    p.spawn(Node {
        flex_shrink: 0.0,
        margin: UiRect::new(Val::Px(6.0), Val::Px(0.0), Val::Px(10.0), Val::Px(2.0)),
        ..default()
    })
    .with_children(|h| {
        text(h, ctx, "Gear Bonuses", 13.0, GOLD);
    });
    let lines = summed(&gear.equipped_ids());
    if lines.is_empty() {
        p.spawn(Node {
            padding: UiRect::horizontal(Val::Px(6.0)),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|r| {
            para(r, ctx, "None: no gear worn yet.", 12.0, DIM);
        });
        return;
    }
    for b in lines {
        p.spawn(Node {
            padding: UiRect::horizontal(Val::Px(6.0)),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|r| {
            text(r, ctx, &b.describe(), 12.0, TIP_GREEN);
        });
    }
}

/// One stat line, label left and value right, lit on hover with its tooltip up.
fn stat_row(p: &mut ChildSpawnerCommands, ctx: &TabCtx, row: StatRow) {
    p.spawn((
        Button,
        row.tip,
        Node {
            height: Val::Px(ROW_H),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::horizontal(Val::Px(6.0)),
            ..default()
        },
    ))
    .with_children(|r| {
        // The quest log's row sheen, else the zone list's.
        let sheen = ctx
            .art
            .quest_hilight
            .as_ref()
            .or(ctx.art.select_highlight.as_ref());
        if let Some(hi) = sheen {
            r.spawn((
                Hilight,
                Visibility::Hidden,
                MaterialNode(hi.clone()),
                overlay(),
            ));
        }
        text(r, ctx, row.label, 13.0, GOLD);
        text(r, ctx, &row.value, 13.0, Color::WHITE);
    });
}

/// Under the hero's feet, the pick screen's name line: race and class in the class's colour.
fn hero_name(
    p: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    class: Option<&'static ClassDef>,
    race_sex: Option<(u8, u8)>,
) {
    let (Some(class), Some((race, _))) = (class, race_sex) else {
        return;
    };
    p.spawn(Node {
        position_type: PositionType::Absolute,
        bottom: Val::Px(24.0),
        width: Val::Percent(100.0),
        justify_content: JustifyContent::Center,
        ..default()
    })
    .with_children(|c| {
        outlined_text(
            c,
            Node::default(),
            (),
            (),
            GlueText {
                text: &format!("{} {}", race_name(race), class.name),
                size: 22.0,
                color: rgb(class.color),
                wrap: false,
            },
            ctx.font,
            1.0,
        );
    });
}

/// The Details column: the class's icon and name, then class, race and sex.
fn details(
    p: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    class: Option<&'static ClassDef>,
    race_sex: Option<(u8, u8)>,
) {
    let Some(class) = class else {
        para(p, ctx, "No hero picked yet.", 13.0, DIM);
        return;
    };
    p.spawn(Node {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        row_gap: Val::Px(6.0),
        margin: UiRect::bottom(Val::Px(12.0)),
        ..default()
    })
    .with_children(|h| {
        let icon = ctx.art.classes.as_ref().and_then(|(sheet, size)| {
            class_tc(class.id).map(|tc| (sheet.clone(), tc_rect(*size, tc)))
        });
        if let Some((sheet, rect)) = icon {
            h.spawn((
                ImageNode {
                    image: sheet,
                    rect: Some(rect),
                    ..default()
                },
                Node {
                    width: Val::Px(48.0),
                    height: Val::Px(48.0),
                    ..default()
                },
            ));
        }
        text(h, ctx, class.name, 18.0, rgb(class.color));
    });
    let (race, sex) = match race_sex {
        Some((race, sex)) => (race_name(race), if sex == 1 { "Female" } else { "Male" }),
        None => ("-", "-"),
    };
    for (label, value) in [("Class", class.name), ("Race", race), ("Sex", sex)] {
        p.spawn(Node {
            height: Val::Px(ROW_H),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::horizontal(Val::Px(6.0)),
            ..default()
        })
        .with_children(|r| {
            text(r, ctx, label, 13.0, GOLD);
            text(r, ctx, value, 13.0, Color::WHITE);
        });
    }
    if let Some(gear) = ctx.selection.class.map(|c| class_gear(ctx.world, c)) {
        equipment(p, ctx, gear);
    }
}

/// What each slot wears, in its rarity's colour; an empty slot greyed. Hover a piece for it in
/// full.
fn equipment(p: &mut ChildSpawnerCommands, ctx: &TabCtx, gear: &GearSave) {
    p.spawn(Node {
        flex_shrink: 0.0,
        margin: UiRect::new(Val::Px(6.0), Val::Px(0.0), Val::Px(12.0), Val::Px(2.0)),
        ..default()
    })
    .with_children(|h| {
        text(h, ctx, "Equipment", 13.0, GOLD);
    });
    for slot in Slot::ALL {
        let worn = gear.worn(slot);
        let mut row = p.spawn(Node {
            min_height: Val::Px(20.0),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            padding: UiRect::horizontal(Val::Px(6.0)),
            ..default()
        });
        if let Some(piece) = worn {
            row.insert((Button, item_tip(piece, gear)));
        }
        row.with_children(|r| {
            if worn.is_some() {
                sheen(r, ctx);
            }
            r.spawn(Node {
                width: Val::Px(70.0),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|c| {
                text(
                    c,
                    ctx,
                    slot.label(),
                    12.0,
                    if worn.is_some() { GOLD } else { DIM },
                );
            });
            let (name, color) = worn.map_or(("Empty", DIM), |i| (i.name, i.rarity.color()));
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
                    text: name,
                    size: 12.0,
                    color,
                    wrap: true,
                },
                ctx.font,
                1.0,
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::survivors::{data, hud, MetaBonuses, Run, Selection};

    fn numbers(s: &Stats) -> [f32; 10] {
        [
            s.might, s.max_hp, s.speed, s.regen, s.haste, s.area, s.magnet, s.growth, s.armor,
            s.crit,
        ]
    }

    #[test]
    fn the_sheet_is_a_fresh_runs_stats_for_every_class() {
        for (i, class) in data::CLASSES.iter().enumerate() {
            let mut selection = Selection {
                class: Some(i),
                looks: vec![(0, 2, 0)],
                look: Some(0),
                ..Default::default()
            };
            let (mut run, mut hero) = (Run::default(), HeroState::default());
            assert!(
                hud::start_run(
                    &mut run,
                    Some(&mut hero),
                    &mut selection,
                    &MetaBonuses::default()
                ),
                "{}",
                class.name
            );
            assert_eq!(
                numbers(&starting_stats(class)),
                numbers(&hero.stats()),
                "{}",
                class.name
            );
        }
    }
}
