//! The Armory's Achievements tab. On the left, the account's points ("N of M"), a bar of how many
//! achievements are earned, and the category list with each shelf's count. On the right, the
//! category picked, two cards to a row, or the summary: each shelf's progress, the latest earned
//! and the nearest to done. A card shows its points, and the run that earned it or how far along
//! the profile is.

use bevy::prelude::*;
use bevy::ui_render::ui_material::MaterialNode;

use super::{panel, para, text, TabCtx};
use crate::glue::art::{DIM, GOLD};
use crate::glue::widgets::{overlay, Hilight, LockHighlight};
use crate::survivors::achievements::{
    almost_there, catalog, counts, earned_points, recently_earned, total_points, Achievement,
    AchievementsView, Category, CategoryPick,
};
use crate::survivors::profile::Profile;

/// The category column, as wide as the Armory's own side columns.
const SIDE_W: f32 = 250.0;
const ROW_H: f32 = 24.0;
const CARD_H: f32 = 56.0;
const CARD_GAP: f32 = 4.0;
/// How many cards each summary list shows.
const SUMMARY_CARDS: usize = 5;

/// A description's parchment.
const DESC: Color = Color::srgb(0.85, 0.82, 0.72);
/// `GREEN_FONT_COLOR`, an earned date.
const GREEN: Color = Color::srgb(0.1, 1.0, 0.1);
const CARD_FILL: Color = Color::srgba(0.0, 0.0, 0.0, 0.45);
const CARD_EARNED: Color = Color::srgba(0.16, 0.12, 0.02, 0.6);
const CARD_EDGE: Color = Color::srgb(0.3, 0.28, 0.24);
const BADGE_FILL: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);
const BAR_BACK: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);
const BAR_EDGE: Color = Color::srgb(0.45, 0.4, 0.3);
const BAR_FILL: Color = Color::srgb(0.12, 0.45, 0.1);

pub(super) fn build(parent: &mut ChildSpawnerCommands, ctx: &TabCtx) {
    let fresh = Profile::default();
    let profile = ctx.res::<Profile>().unwrap_or(&fresh);
    let picked = ctx.res::<AchievementsView>().copied().unwrap_or_default().0;
    let side = Node {
        width: Val::Px(SIDE_W),
        height: Val::Percent(100.0),
        flex_shrink: 0.0,
        ..default()
    };
    panel(parent, ctx, side, Some("Achievements"), |p| {
        sidebar(p, ctx, profile, picked)
    });
    let main = Node {
        flex_grow: 1.0,
        height: Val::Percent(100.0),
        ..default()
    };
    let title = picked.map_or("Summary", Category::label);
    panel(parent, ctx, main, Some(title), |p| match picked {
        None => summary(p, ctx, profile),
        Some(category) => shelf(p, ctx, profile, category),
    });
}

/// The left column: the points, the earned bar and the category list.
fn sidebar(
    p: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    profile: &Profile,
    picked: Option<Category>,
) {
    let save = &profile.achievements;
    text(p, ctx, "Achievement Points", 13.0, GOLD);
    let points = format!("{} of {}", earned_points(save), total_points());
    text(p, ctx, &points, 24.0, Color::WHITE);
    gap(p, 6.0);
    let (have, all) = counts(save, None);
    bar(
        p,
        ctx,
        have as f32 / all.max(1) as f32,
        &format!("{have} of {all} earned"),
        16.0,
    );
    gap(p, 14.0);
    category_row(p, ctx, None, "Summary", "", picked.is_none());
    for category in Category::ALL {
        let (have, all) = counts(save, Some(category));
        category_row(
            p,
            ctx,
            Some(category),
            category.label(),
            &format!("{have} / {all}"),
            picked == Some(category),
        );
    }
    p.spawn(Node {
        flex_grow: 1.0,
        ..default()
    });
    para(
        p,
        ctx,
        "Achievements are judged as each run ends. Their points buy perks on the Rewards tab.",
        11.0,
        DIM,
    );
}

/// One category button, its name and its count, lit while picked.
fn category_row(
    p: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    category: Option<Category>,
    name: &str,
    count: &str,
    picked: bool,
) {
    p.spawn((
        Button,
        CategoryPick(category),
        LockHighlight(picked),
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
        sheen(r, ctx);
        let color = if picked { Color::WHITE } else { GOLD };
        text(r, ctx, name, 13.0, color);
        text(r, ctx, count, 12.0, Color::WHITE);
    });
}

/// The summary: every shelf's bar, then the latest earned beside the nearest to done.
fn summary(p: &mut ChildSpawnerCommands, ctx: &TabCtx, profile: &Profile) {
    heading(p, ctx, "Progress");
    for category in Category::ALL {
        let (have, all) = counts(&profile.achievements, Some(category));
        p.spawn(Node {
            height: Val::Px(22.0),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            ..default()
        })
        .with_children(|r| {
            r.spawn(Node {
                width: Val::Px(80.0),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|l| {
                text(l, ctx, category.label(), 13.0, GOLD);
            });
            r.spawn(Node {
                flex_grow: 1.0,
                ..default()
            })
            .with_children(|b| {
                let frac = have as f32 / all.max(1) as f32;
                bar(b, ctx, frac, &format!("{have} / {all}"), 16.0);
            });
        });
    }
    gap(p, 12.0);
    p.spawn(Node {
        flex_direction: FlexDirection::Row,
        column_gap: Val::Px(12.0),
        flex_grow: 1.0,
        ..default()
    })
    .with_children(|cols| {
        let list = |cols: &mut ChildSpawnerCommands,
                    title: &str,
                    items: Vec<&'static Achievement>,
                    empty: &str| {
            cols.spawn(Node {
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                min_width: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(CARD_GAP),
                ..default()
            })
            .with_children(|c| {
                heading(c, ctx, title);
                if items.is_empty() {
                    para(c, ctx, empty, 12.0, DIM);
                }
                for a in items {
                    card(c, ctx, a, profile, Val::Percent(100.0));
                }
            });
        };
        list(
            cols,
            "Recently Earned",
            recently_earned(&profile.achievements, SUMMARY_CARDS),
            "Nothing yet: finish a run to earn your first.",
        );
        list(
            cols,
            "Almost There",
            almost_there(profile, SUMMARY_CARDS),
            "Finish a run to start on the lifetime goals.",
        );
    });
}

/// One category's achievements, two cards to a row.
fn shelf(p: &mut ChildSpawnerCommands, ctx: &TabCtx, profile: &Profile, category: Category) {
    p.spawn(Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        justify_content: JustifyContent::SpaceBetween,
        align_content: AlignContent::FlexStart,
        row_gap: Val::Px(CARD_GAP),
        ..default()
    })
    .with_children(|grid| {
        for a in catalog().iter().filter(|a| a.category == category) {
            card(grid, ctx, a, profile, Val::Percent(49.4));
        }
    });
}

/// An achievement's card: its points badge, name and description, then the run that earned it,
/// a progress bar, or "Not yet earned" for a single-run feat.
fn card(
    p: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    a: &Achievement,
    profile: &Profile,
    width: Val,
) {
    let earned = profile.achievements.run_earned(&a.id);
    let (fill, name_color) = match earned {
        Some(_) => (CARD_EARNED, GOLD),
        None => (CARD_FILL, Color::WHITE),
    };
    p.spawn((
        Node {
            width,
            height: Val::Px(CARD_H),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
            border: UiRect::all(Val::Px(1.0)),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(fill),
        BorderColor::all(CARD_EDGE),
    ))
    .with_children(|c| {
        badge(c, ctx, a.points, earned.is_some());
        // No content-sized minimum, so a long line is clipped instead of widening the card.
        c.spawn(Node {
            flex_grow: 1.0,
            min_width: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(1.0),
            overflow: Overflow::clip(),
            ..default()
        })
        .with_children(|t| {
            text(t, ctx, &a.name, 13.0, name_color);
            text(t, ctx, &a.desc, 11.0, DESC);
            match earned {
                Some(run) => {
                    text(t, ctx, &format!("Earned on run {run}"), 11.0, GREEN);
                }
                None if a.measured() => {
                    let label = a.progress_label(profile);
                    bar(t, ctx, a.fraction(profile), &label, 13.0);
                }
                None => {
                    text(t, ctx, "Not yet earned", 11.0, DIM);
                }
            }
        });
    });
}

/// The points an achievement is worth, in a small framed square: gold once earned.
fn badge(p: &mut ChildSpawnerCommands, ctx: &TabCtx, points: u32, earned: bool) {
    let color = if earned { GOLD } else { DIM };
    p.spawn((
        Node {
            width: Val::Px(34.0),
            height: Val::Px(34.0),
            flex_shrink: 0.0,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(BADGE_FILL),
        BorderColor::all(color),
    ))
    .with_children(|b| {
        text(b, ctx, &points.to_string(), 14.0, color);
    });
}

/// A progress bar `frac` full, `height` tall, with `label` over its middle.
fn bar(p: &mut ChildSpawnerCommands, ctx: &TabCtx, frac: f32, label: &str, height: f32) {
    p.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(height),
            flex_shrink: 0.0,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(BAR_BACK),
        BorderColor::all(BAR_EDGE),
    ))
    .with_children(|b| {
        b.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                width: Val::Percent(frac.clamp(0.0, 1.0) * 100.0),
                ..default()
            },
            BackgroundColor(BAR_FILL),
        ));
        text(b, ctx, label, (height - 4.0).clamp(9.0, 12.0), Color::WHITE);
    });
}

/// A section's gold heading.
fn heading(p: &mut ChildSpawnerCommands, ctx: &TabCtx, s: &str) {
    p.spawn(Node {
        flex_shrink: 0.0,
        margin: UiRect::bottom(Val::Px(4.0)),
        ..default()
    })
    .with_children(|h| {
        text(h, ctx, s, 14.0, GOLD);
    });
}

/// Empty room, `h` tall.
fn gap(p: &mut ChildSpawnerCommands, h: f32) {
    p.spawn(Node {
        height: Val::Px(h),
        flex_shrink: 0.0,
        ..default()
    });
}

/// A list row's hover and pick sheen: the quest log's, else the zone list's.
fn sheen(r: &mut ChildSpawnerCommands, ctx: &TabCtx) {
    let art = ctx.art;
    if let Some(hi) = art.quest_hilight.as_ref().or(art.select_highlight.as_ref()) {
        r.spawn((
            Hilight,
            Visibility::Hidden,
            MaterialNode(hi.clone()),
            overlay(),
        ));
    }
}
