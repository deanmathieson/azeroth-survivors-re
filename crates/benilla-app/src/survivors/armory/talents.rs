//! The Armory's Talents tab: the picked class's three trees side by side, each over its
//! `Interface\TalentFrame` painting with its 7 × 4 grid of talents, their rank badges and
//! prerequisite links, and the Details column: the points to spend, the chosen talent and its
//! Learn, Unlearn and Reset buttons. Left-click on a talent learns a rank, right-click unlearns
//! one. [`crate::survivors::talents`] owns the input, the rules and the save; every change
//! rebuilds the tab through [`super::ArmoryState::refresh`].

use bevy::prelude::*;
use bevy::ui_render::ui_material::MaterialNode;

use super::{panel, para, text, ArmoryTip, TabCtx};
use crate::glue::art::{DIM, GOLD};
use crate::glue::widgets::{glue_button, overlay, GlueBtnKind, GlueDisabled, Hilight};
use crate::survivors::profile::Profile;
use crate::survivors::talents::{
    check_learn, check_reset, check_unlearn, describe_rank, earned_points, saved, shade, unmet,
    why_text, xp_for_points, Allocation, Talent, TalentAction, TalentArt, TalentCatalog,
    TalentDevXp, TalentFocus, TalentSave, TalentShade, TalentSlot, Tree, Why, MAX_POINTS,
};

/// `NORMAL_FONT_COLOR`, what a rank grants.
const TIP_GOLD: Color = Color::srgb(1.0, 0.82, 0.0);
/// `GREEN_FONT_COLOR`: points to spend, an open talent.
const TIP_GREEN: Color = Color::srgb(0.1, 1.0, 0.1);
/// `RED_FONT_COLOR`: an unmet requirement.
const TIP_RED: Color = Color::srgb(1.0, 0.1, 0.1);
/// The subtitle's parchment white.
const PARCHMENT: Color = Color::srgb(0.9, 0.85, 0.7);
/// A locked talent's frame and badge.
const LOCKED: Color = Color::srgb(0.4, 0.4, 0.4);
/// A met and an unmet prerequisite's link.
const LINK_ON: Color = Color::srgba(1.0, 0.82, 0.0, 0.9);
const LINK_OFF: Color = Color::srgba(0.55, 0.55, 0.55, 0.7);
/// The paintings, dimmed under the icons and text.
const ART_TINT: Color = Color::srgb(0.7, 0.7, 0.7);
const QUESTION_MARK: &str = "Interface\\Icons\\INV_Misc_QuestionMark";

const TIERS: u8 = 7;
const COLUMNS: u8 = 4;
const TIER_H: f32 = 70.0;
/// A talent's button: the icon inside a [`FRAME`]-wide frame.
const CELL: f32 = 38.0;
const FRAME: f32 = 2.0;
const LINK_W: f32 = 3.0;
const DETAILS_W: f32 = 236.0;
const BTN_H: f32 = 30.0;
/// How far a tree's painting reaches past its panel's body, toward the frame.
const ART_BLEED: f32 = 4.0;

/// `TalentFrameBackground*`: a 320 × 384 painting in four pieces, 256 / 64 wide and 256 / 128
/// tall, as (suffix, left %, top %, width %, height %).
const ART_PIECES: [(&str, f32, f32, f32, f32); 4] = [
    ("TopLeft", 0.0, 0.0, 80.0, 200.0 / 3.0),
    ("TopRight", 80.0, 0.0, 20.0, 200.0 / 3.0),
    ("BottomLeft", 0.0, 200.0 / 3.0, 80.0, 100.0 / 3.0),
    ("BottomRight", 80.0, 200.0 / 3.0, 20.0, 100.0 / 3.0),
];

/// Everything the tab shows of the picked class, read once per build.
struct View<'a> {
    catalog: &'a TalentCatalog,
    save: TalentSave,
    alloc: Allocation,
    /// [`TalentDevXp`], counted as earned.
    dev: u64,
    chosen: Option<u32>,
}

pub(super) fn build(parent: &mut ChildSpawnerCommands, ctx: &TabCtx) {
    let Some(class) = ctx.selection.class.filter(|_| ctx.class().is_some()) else {
        message(parent, ctx, "Pick a class to see its talents.");
        return;
    };
    let Some(catalog) = ctx.res::<TalentCatalog>() else {
        message(parent, ctx, "The talent trees load with the client's data.");
        return;
    };
    let save = ctx
        .res::<Profile>()
        .and_then(|p| saved(p, class))
        .cloned()
        .unwrap_or_default();
    let view = View {
        catalog,
        alloc: save.allocation(),
        save,
        dev: ctx.res::<TalentDevXp>().map_or(0, |d| d.0),
        chosen: ctx.res::<TalentFocus>().and_then(|f| f.chosen(class)),
    };
    let trees: Vec<&Tree> = catalog.trees_for(class).collect();
    for tree in &trees {
        tree_panel(parent, ctx, &view, tree);
    }
    details_panel(parent, ctx, &view, &trees);
}

/// One framed panel across the content area with a centred line, for a tab with no trees to show.
fn message(parent: &mut ChildSpawnerCommands, ctx: &TabCtx, s: &str) {
    let node = Node {
        flex_grow: 1.0,
        height: Val::Percent(100.0),
        ..default()
    };
    panel(parent, ctx, node, None, |b| {
        b.spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|c| {
            text(c, ctx, s, 18.0, PARCHMENT);
        });
    });
}

// ── A tree ──────────────────────────────────────────────────────────────────────────────────────

/// A tree's panel: its painting, its name and points, and its grid.
fn tree_panel(parent: &mut ChildSpawnerCommands, ctx: &TabCtx, view: &View, tree: &Tree) {
    let node = Node {
        flex_grow: 1.0,
        flex_basis: Val::Px(0.0),
        min_width: Val::Px(0.0),
        height: Val::Percent(100.0),
        ..default()
    };
    panel(parent, ctx, node, None, |b| {
        painting(b, tree);
        tree_header(b, ctx, view, tree);
        grid(b, ctx, view, tree);
    });
}

/// The tree's `TalentFrameBackground` pieces, scaled to the body's height and centred, their sides
/// cropped: the panel is far narrower than the 320 × 384 painting.
fn painting(b: &mut ChildSpawnerCommands, tree: &Tree) {
    if tree.background.is_empty() {
        return;
    }
    let px = Val::Px;
    b.spawn(Node {
        position_type: PositionType::Absolute,
        left: px(-ART_BLEED),
        right: px(-ART_BLEED),
        top: px(-ART_BLEED),
        bottom: px(-ART_BLEED),
        overflow: Overflow::clip(),
        justify_content: JustifyContent::Center,
        ..default()
    })
    .with_children(|clip| {
        clip.spawn(Node {
            height: Val::Percent(100.0),
            aspect_ratio: Some(320.0 / 384.0),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|art| {
            for (piece, left, top, width, height) in ART_PIECES {
                art.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(left),
                        top: Val::Percent(top),
                        width: Val::Percent(width),
                        height: Val::Percent(height),
                        ..default()
                    },
                    ImageNode {
                        color: Color::NONE,
                        ..default()
                    },
                    TalentArt {
                        path: format!("Interface\\TalentFrame\\{}-{piece}.blp", tree.background),
                        tint: ART_TINT,
                    },
                ));
            }
        });
    });
}

/// The tree's icon and name, and the points spent in it.
fn tree_header(b: &mut ChildSpawnerCommands, ctx: &TabCtx, view: &View, tree: &Tree) {
    let spent = view.alloc.spent_in(view.catalog, tree.id);
    b.spawn(Node {
        height: Val::Px(26.0),
        flex_shrink: 0.0,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        padding: UiRect::horizontal(Val::Px(2.0)),
        ..default()
    })
    .with_children(|h| {
        h.spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|l| {
            l.spawn(icon(tree.icon.as_deref(), Val::Px(20.0)));
            text(l, ctx, &tree.name, 15.0, GOLD);
        });
        let color = if spent > 0 { Color::WHITE } else { DIM };
        text(h, ctx, &spent.to_string(), 15.0, color);
    });
}

/// The centre of column `c`, as a share of the grid's width.
fn column_x(c: u8) -> f32 {
    (f32::from(c) + 0.5) * 100.0 / f32::from(COLUMNS)
}

/// The middle of tier `t`, down from the grid's top.
fn tier_y(t: u8) -> f32 {
    (f32::from(t) + 0.5) * TIER_H
}

/// The tree's tiers × columns: the prerequisite links under the talents' buttons.
fn grid(b: &mut ChildSpawnerCommands, ctx: &TabCtx, view: &View, tree: &Tree) {
    let talents: Vec<&Talent> = view.catalog.talents_in(tree.id).collect();
    b.spawn(Node {
        width: Val::Percent(100.0),
        height: Val::Px(TIER_H * f32::from(TIERS)),
        flex_shrink: 0.0,
        margin: UiRect::top(Val::Px(4.0)),
        ..default()
    })
    .with_children(|g| {
        for to in &talents {
            let Some((p, need)) = to.prereq else { continue };
            if let Some(from) = view.catalog.talent(p) {
                link(g, from, to, view.alloc.rank(p) >= need);
            }
        }
        for t in &talents {
            talent_button(g, ctx, view, t);
        }
    });
}

/// A prerequisite's link: across at its tier to the talent's column, then down to the talent.
/// The ends hide under the two buttons.
fn link(g: &mut ChildSpawnerCommands, from: &Talent, to: &Talent, met: bool) {
    let color = BackgroundColor(if met { LINK_ON } else { LINK_OFF });
    let (fx, tx) = (column_x(from.column), column_x(to.column));
    let (fy, ty) = (tier_y(from.tier), tier_y(to.tier));
    if from.column != to.column {
        g.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(fx.min(tx)),
                width: Val::Percent((fx - tx).abs()),
                top: Val::Px(fy - LINK_W / 2.0),
                height: Val::Px(LINK_W),
                ..default()
            },
            color,
        ));
    }
    if from.tier != to.tier {
        g.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(tx),
                margin: UiRect::left(Val::Px(-LINK_W / 2.0)),
                width: Val::Px(LINK_W),
                top: Val::Px(fy.min(ty)),
                height: Val::Px((ty - fy).abs()),
                ..default()
            },
            color,
        ));
    }
}

/// An icon from `Interface\Icons` (the question mark without one), `size` square, on black until
/// it loads.
fn icon(path: Option<&str>, size: Val) -> impl Bundle {
    (
        Node {
            width: size,
            height: size,
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(Color::BLACK),
        ImageNode {
            color: Color::NONE,
            ..default()
        },
        TalentArt {
            path: format!("{}.blp", path.unwrap_or(QUESTION_MARK)),
            tint: Color::WHITE,
        },
    )
}

/// The frame's and the rank badge's colours for a talent of `rank` ranks shaded `shade`, as WoW
/// colours them: gold at the last rank, green while learnable, grey otherwise, the badge keeping
/// the normal gold while the talent has ranks.
fn shade_colors(shade: TalentShade, rank: u8) -> (Color, Color) {
    match shade {
        TalentShade::Maxed => (TIP_GOLD, TIP_GOLD),
        TalentShade::Open => (TIP_GREEN, TIP_GREEN),
        TalentShade::Closed if rank > 0 => (LOCKED, TIP_GOLD),
        TalentShade::Closed => (LOCKED, LOCKED),
    }
}

/// A talent's button: its icon in a frame coloured as WoW colours the rank ([`shade_colors`]),
/// dimmed while it has no ranks and cannot take one, with the `rank/max` badge, a white ring while
/// chosen, and its tooltip.
fn talent_button(g: &mut ChildSpawnerCommands, ctx: &TabCtx, view: &View, t: &Talent) {
    let px = Val::Px;
    let rank = view.alloc.rank(t.id);
    let needs = unmet(view.catalog, &view.alloc, t);
    let state = shade(view.catalog, &view.save, view.dev, t);
    let locked = rank == 0 && state == TalentShade::Closed;
    let (shade, badge_tone) = shade_colors(state, rank);
    let mut button = g.spawn((
        Button,
        TalentSlot(t.id),
        talent_tip(view, t, rank, &needs),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(column_x(t.column)),
            top: px(tier_y(t.tier) - CELL / 2.0),
            margin: UiRect::left(px(-CELL / 2.0)),
            width: px(CELL),
            height: px(CELL),
            padding: UiRect::all(px(FRAME)),
            ..default()
        },
        BackgroundColor(shade),
    ));
    if view.chosen == Some(t.id) {
        button.insert(Outline::new(px(2.0), px(1.0), Color::WHITE));
    }
    button.with_children(|c| {
        c.spawn(icon(t.icon.as_deref(), Val::Percent(100.0)));
        if locked {
            c.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(FRAME),
                    right: px(FRAME),
                    top: px(FRAME),
                    bottom: px(FRAME),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.65)),
            ));
        }
        if let Some(hi) = &ctx.art.mouse_hilight {
            c.spawn((
                Hilight,
                Visibility::Hidden,
                MaterialNode(hi.clone()),
                overlay(),
            ));
        }
        c.spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(-7.0),
                bottom: px(-7.0),
                padding: UiRect::axes(px(3.0), px(0.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        ))
        .with_children(|badge| {
            badge.spawn((
                Text::new(format!("{rank}/{}", t.max_rank)),
                TextFont {
                    font: ctx.font.clone(),
                    font_size: 10.0,
                    ..default()
                },
                TextColor(badge_tone),
                TextShadow {
                    offset: Vec2::splat(1.0),
                    color: Color::BLACK,
                },
            ));
        });
    });
}

/// A talent's hover text: its rank, what stands in its way, what it grants now and next, and
/// what a click would do.
fn talent_tip(view: &View, t: &Talent, rank: u8, unmet: &[String]) -> ArmoryTip {
    let mut lines = vec![(format!("Rank {rank}/{}", t.max_rank), Color::WHITE)];
    if rank == 0 {
        lines.extend(unmet.iter().map(|u| (u.clone(), TIP_RED)));
    }
    if t.effect().is_empty() {
        lines.push(("No effect in this mode".into(), DIM));
    } else {
        lines.push((describe_rank(t, rank.max(1)), TIP_GOLD));
        if rank > 0 && rank < t.max_rank {
            lines.push(("Next rank:".into(), Color::WHITE));
            lines.push((describe_rank(t, rank + 1), TIP_GOLD));
        }
    }
    match check_learn(view.catalog, &view.save, view.dev, t.id) {
        Ok(()) => lines.push(("Click to learn".into(), TIP_GREEN)),
        Err(Why::NoPoints) => lines.push((why_text(view.catalog, t.id, Why::NoPoints), TIP_RED)),
        Err(_) => {}
    }
    if rank > 0 {
        lines.push(("Right-click to unlearn".into(), DIM));
    }
    ArmoryTip {
        title: t.name.clone(),
        lines,
    }
}

// ── Details ─────────────────────────────────────────────────────────────────────────────────────

/// The Details column: the points, the chosen talent, and the buttons.
fn details_panel(parent: &mut ChildSpawnerCommands, ctx: &TabCtx, view: &View, trees: &[&Tree]) {
    let node = Node {
        width: Val::Px(DETAILS_W),
        height: Val::Percent(100.0),
        flex_shrink: 0.0,
        ..default()
    };
    panel(parent, ctx, node, Some("Details"), |b| {
        points(b, ctx, view, trees);
        b.spawn(Node {
            height: Val::Px(14.0),
            flex_shrink: 0.0,
            ..default()
        });
        match view.chosen.and_then(|id| view.catalog.talent(id)) {
            Some(t) => chosen(b, ctx, view, t),
            None => {
                para(
                    b,
                    ctx,
                    "Choose a talent to see what each rank grants.",
                    12.0,
                    DIM,
                );
            }
        }
        b.spawn(Node {
            flex_grow: 1.0,
            ..default()
        });
        buttons(b, ctx, view);
        para(
            b,
            ctx,
            "Left-click a talent to learn a rank, right-click to unlearn one. A reset is free.",
            11.0,
            DIM,
        );
    });
}

/// "N to spend", "Spent X of Y · a/b/c", and the XP toward the next point.
fn points(b: &mut ChildSpawnerCommands, ctx: &TabCtx, view: &View, trees: &[&Tree]) {
    let earned = earned_points(&view.save, view.dev);
    let spent = view.alloc.spent();
    let free = earned.saturating_sub(spent);
    let color = if free > 0 { TIP_GREEN } else { DIM };
    text(b, ctx, &format!("{free} to spend"), 18.0, color);
    let split: Vec<String> = trees
        .iter()
        .map(|t| view.alloc.spent_in(view.catalog, t.id).to_string())
        .collect();
    text(
        b,
        ctx,
        &format!("Spent {spent} of {earned} · {}", split.join("/")),
        12.0,
        Color::WHITE,
    );
    let xp = view.save.xp.saturating_add(view.dev);
    let next = if earned < MAX_POINTS {
        let to_go = xp_for_points(earned + 1).saturating_sub(xp);
        format!("{xp} talent XP; the next point in {to_go}")
    } else {
        format!("{xp} talent XP; all {MAX_POINTS} points earned")
    };
    para(b, ctx, &next, 11.0, DIM);
}

/// The chosen talent: its icon, name, rank and tree, what stands in its way, and what its current
/// and next ranks grant.
fn chosen(b: &mut ChildSpawnerCommands, ctx: &TabCtx, view: &View, t: &Talent) {
    let rank = view.alloc.rank(t.id);
    let tree = view
        .catalog
        .tree(t.tree)
        .map_or("", |tree| tree.name.as_str());
    b.spawn(Node {
        column_gap: Val::Px(8.0),
        align_items: AlignItems::Center,
        flex_shrink: 0.0,
        margin: UiRect::bottom(Val::Px(8.0)),
        ..default()
    })
    .with_children(|row| {
        row.spawn(icon(t.icon.as_deref(), Val::Px(36.0)));
        row.spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            min_width: Val::Px(0.0),
            ..default()
        })
        .with_children(|col| {
            para(col, ctx, &t.name, 15.0, Color::WHITE);
            let (_, color) = shade_colors(shade(view.catalog, &view.save, view.dev, t), rank);
            text(
                col,
                ctx,
                &format!("Rank {rank}/{}", t.max_rank),
                12.0,
                color,
            );
            text(col, ctx, tree, 11.0, DIM);
        });
    });
    if rank == 0 {
        for u in unmet(view.catalog, &view.alloc, t) {
            para(b, ctx, &u, 12.0, TIP_RED);
        }
    }
    let effect = |r: u8| {
        if t.effect().is_empty() {
            "No effect in this mode".to_string()
        } else {
            describe_rank(t, r)
        }
    };
    text(b, ctx, "Current rank", 11.0, DIM);
    if rank == 0 {
        para(b, ctx, "Not learned", 12.0, DIM);
    } else {
        para(b, ctx, &effect(rank), 12.0, TIP_GOLD);
    }
    if rank < t.max_rank {
        text(b, ctx, "Next rank", 11.0, DIM);
        para(b, ctx, &effect(rank + 1), 12.0, TIP_GREEN);
    }
}

/// Learn and Unlearn for the chosen talent, and Reset; each disabled with its reason on hover
/// when it cannot act.
fn buttons(b: &mut ChildSpawnerCommands, ctx: &TabCtx, view: &View) {
    let why = |id: u32, r: Result<(), _>| r.err().map(|w| why_text(view.catalog, id, w));
    let none = || Some("Choose a talent first".to_string());
    let (learn, unlearn) = match view.chosen {
        Some(id) => (
            (
                TalentAction::Learn(id),
                why(id, check_learn(view.catalog, &view.save, view.dev, id)),
            ),
            (
                TalentAction::Unlearn(id),
                why(id, check_unlearn(view.catalog, &view.save, id)),
            ),
        ),
        None => (
            (TalentAction::Learn(0), none()),
            (TalentAction::Unlearn(0), none()),
        ),
    };
    b.spawn(Node {
        justify_content: JustifyContent::SpaceBetween,
        flex_shrink: 0.0,
        margin: UiRect::bottom(Val::Px(6.0)),
        ..default()
    })
    .with_children(|row| {
        action_button(row, ctx, learn.0, "Learn", 98.0, learn.1);
        action_button(row, ctx, unlearn.0, "Unlearn", 98.0, unlearn.1);
    });
    b.spawn(Node {
        flex_shrink: 0.0,
        margin: UiRect::bottom(Val::Px(8.0)),
        ..default()
    })
    .with_children(|row| {
        let reset = why(0, check_reset(&view.save));
        action_button(row, ctx, TalentAction::Reset, "Reset Talents", 202.0, reset);
    });
}

/// A glue button for `action`; with a `blocked` reason, disabled and saying so on hover.
fn action_button(
    p: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    action: TalentAction,
    caption: &str,
    w: f32,
    blocked: Option<String>,
) {
    let id = glue_button(
        p,
        ctx.art,
        ctx.font,
        action,
        caption,
        w,
        BTN_H,
        GlueBtnKind::Small,
        1.0,
    );
    if let Some(why) = blocked {
        p.commands_mut().entity(id).insert((
            GlueDisabled(true),
            ArmoryTip {
                title: caption.to_string(),
                lines: vec![(why, TIP_RED)],
            },
        ));
    }
}
