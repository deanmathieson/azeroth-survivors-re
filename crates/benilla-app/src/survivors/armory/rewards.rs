//! The Armory's Rewards tab. On the left, the reward points to spend, where they came from, and
//! Reset perks with the resets left. In the middle, every perk: what it does now and at its next
//! rank, its rank pips, the next rank's cost, Buy, and Refund for a rank bought since the tab
//! opened. On the right, the companion: what it does, and its looks to pick from once unlocked.

use bevy::prelude::*;
use bevy::ui_render::ui_material::MaterialNode;

use super::{panel, para, text, TabCtx};
use crate::glue::art::{DIM, GOLD};
use crate::glue::widgets::{
    glue_button, overlay, GlueBtnKind, GlueDisabled, Hilight, LockHighlight,
};
use crate::survivors::achievements::earned_points;
use crate::survivors::profile::Profile;
use crate::survivors::rewards::{
    all_looks, companion_unlocked, look_for, offered, points, DevPoints, Perk, PerkSession,
    RewardsAction, RewardsSave, FETCH_EVERY, FETCH_REACH, PERKS,
};

/// The side columns, as wide as the Armory's own.
const SIDE_W: f32 = 250.0;
const PERK_H: f32 = 58.0;
const ROW_H: f32 = 24.0;
const BUTTON_W: f32 = 62.0;
const BUTTON_H: f32 = 24.0;

/// A description's parchment.
const DESC: Color = Color::srgb(0.85, 0.82, 0.72);
/// `GREEN_FONT_COLOR`, what a perk grants.
const GREEN: Color = Color::srgb(0.1, 1.0, 0.1);
const ROW_FILL: Color = Color::srgba(0.0, 0.0, 0.0, 0.45);
const ROW_EDGE: Color = Color::srgb(0.3, 0.28, 0.24);
const PIP_EMPTY: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);
const PIP_EDGE: Color = Color::srgb(0.55, 0.45, 0.25);

pub(super) fn build(parent: &mut ChildSpawnerCommands, ctx: &TabCtx) {
    let fresh = Profile::default();
    let profile = ctx.res::<Profile>().unwrap_or(&fresh);
    let idle = PerkSession::default();
    let session = ctx.res::<PerkSession>().unwrap_or(&idle);
    let dev = ctx.res::<DevPoints>().copied().unwrap_or_default();
    let left = points(profile, dev);
    let side = || Node {
        width: Val::Px(SIDE_W),
        height: Val::Percent(100.0),
        flex_shrink: 0.0,
        ..default()
    };
    panel(parent, ctx, side(), Some("Reward Points"), |p| {
        wallet(p, ctx, profile, session, dev, left)
    });
    let middle = Node {
        flex_grow: 1.0,
        height: Val::Percent(100.0),
        ..default()
    };
    panel(parent, ctx, middle, Some("Perks"), |p| {
        for (i, perk) in PERKS.iter().enumerate() {
            perk_row(p, ctx, i, perk, &profile.rewards, session, left);
        }
    });
    panel(parent, ctx, side(), Some("Companion"), |p| {
        companion(p, ctx, &profile.rewards)
    });
}

/// The left column: points to spend and their sources, then Reset perks.
fn wallet(
    p: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    profile: &Profile,
    session: &PerkSession,
    dev: DevPoints,
    left: u32,
) {
    let save = &profile.rewards;
    text(p, ctx, "Points to spend", 13.0, GOLD);
    text(p, ctx, &left.to_string(), 28.0, Color::WHITE);
    gap(p, 8.0);
    let earned = earned_points(&profile.achievements);
    line(p, ctx, "From achievements", &earned.to_string());
    line(p, ctx, "Spent on perks", &save.spent.to_string());
    if dev.0 > 0 {
        line(p, ctx, "Test aid (unsaved)", &format!("+{}", dev.0));
    }
    p.spawn(Node {
        flex_grow: 1.0,
        ..default()
    });
    para(
        p,
        ctx,
        "A rank bought since this tab opened can be refunded. Once you leave the tab it stays; \
         Reset perks refunds every rank.",
        11.0,
        DIM,
    );
    gap(p, 8.0);
    p.spawn(Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        row_gap: Val::Px(4.0),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|b| {
        let reset = glue_button(
            b,
            ctx.art,
            ctx.font,
            RewardsAction::Reset,
            "Reset perks",
            170.0,
            32.0,
            GlueBtnKind::Small,
            1.0,
        );
        if !session.can_reset(save) {
            b.commands().entity(reset).insert(GlueDisabled(true));
        }
        let resets = match save.resets_left {
            1 => "1 reset left".to_string(),
            n => format!("{n} resets left"),
        };
        text(b, ctx, &resets, 12.0, Color::WHITE);
    });
}

/// One perk: name and pips, what it does, its effect now and next; the cost, Buy and Refund.
fn perk_row(
    p: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    i: usize,
    perk: &Perk,
    save: &RewardsSave,
    session: &PerkSession,
    left: u32,
) {
    let rank = perk.rank(save);
    let cost = perk.next_cost(save);
    let affordable = cost.is_some_and(|c| c <= left);
    let effect = match (perk.effect_at(rank), cost.and(perk.effect_at(rank + 1))) {
        (None, Some(next)) => format!("Next: {next}"),
        (Some(now), Some(next)) => format!("Now: {now}   Next: {next}"),
        (Some(now), None) => format!("Now: {now} (top rank)"),
        (None, None) => String::new(),
    };
    p.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(PERK_H),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
            margin: UiRect::bottom(Val::Px(4.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(ROW_FILL),
        BorderColor::all(ROW_EDGE),
    ))
    .with_children(|r| {
        // No content-sized minimum, so a long effect line is clipped, never pushing out the buttons.
        r.spawn(Node {
            flex_grow: 1.0,
            min_width: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            overflow: Overflow::clip(),
            ..default()
        })
        .with_children(|t| {
            t.spawn(Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|n| {
                text(n, ctx, perk.name, 14.0, GOLD);
                pips(n, rank, perk.max_rank());
            });
            text(t, ctx, perk.desc, 11.0, DESC);
            text(t, ctx, &effect, 11.0, GREEN);
        });
        r.spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(3.0),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|a| {
            let (label, color) = match cost {
                Some(c) if affordable => (format!("Cost: {c}"), Color::WHITE),
                Some(c) => (format!("Cost: {c}"), DIM),
                None => ("Top rank".to_string(), DIM),
            };
            text(a, ctx, &label, 12.0, color);
            a.spawn(Node {
                column_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|b| {
                let buy = glue_button(
                    b,
                    ctx.art,
                    ctx.font,
                    RewardsAction::Buy(i),
                    "Buy",
                    BUTTON_W,
                    BUTTON_H,
                    GlueBtnKind::Small,
                    1.0,
                );
                if !affordable {
                    b.commands().entity(buy).insert(GlueDisabled(true));
                }
                let refund = glue_button(
                    b,
                    ctx.art,
                    ctx.font,
                    RewardsAction::Refund(i),
                    "Refund",
                    BUTTON_W,
                    BUTTON_H,
                    GlueBtnKind::Small,
                    1.0,
                );
                if !session.can_refund(save, perk) {
                    b.commands().entity(refund).insert(GlueDisabled(true));
                }
            });
        });
    });
}

/// A perk's ranks as pips, the bought ones gold.
fn pips(p: &mut ChildSpawnerCommands, rank: u32, max: u32) {
    p.spawn(Node {
        column_gap: Val::Px(3.0),
        align_items: AlignItems::Center,
        ..default()
    })
    .with_children(|row| {
        for i in 0..max {
            let fill = if i < rank { GOLD } else { PIP_EMPTY };
            row.spawn((
                Node {
                    width: Val::Px(9.0),
                    height: Val::Px(9.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(fill),
                BorderColor::all(PIP_EDGE),
            ));
        }
    });
}

/// The right column: what the companion does, and, once the perk is bought, its looks.
fn companion(p: &mut ChildSpawnerCommands, ctx: &TabCtx, save: &RewardsSave) {
    let does = format!(
        "Every {FETCH_EVERY} seconds it fetches the nearest gem or coin within {FETCH_REACH} \
         yards that lies past your pickup range."
    );
    if !companion_unlocked(save) {
        para(
            p,
            ctx,
            &format!("Buy the Companion perk and a critter joins every run. {does}"),
            12.0,
            DESC,
        );
        gap(p, 10.0);
        text(p, ctx, "Looks to pick from", 13.0, DIM);
        for look in all_looks().filter(|l| offered(l, save)) {
            text(p, ctx, look.name, 13.0, DIM);
        }
        return;
    }
    para(
        p,
        ctx,
        &format!("Your companion follows you on every run. {does}"),
        12.0,
        DESC,
    );
    gap(p, 10.0);
    text(p, ctx, "Look", 13.0, GOLD);
    gap(p, 4.0);
    let worn = look_for(&save.companion).key;
    // Indexed over every look, so a quest chain's look keeps its place once unlocked.
    for (i, look) in all_looks().enumerate().filter(|(_, l)| offered(l, save)) {
        let picked = look.key == worn;
        p.spawn((
            Button,
            RewardsAction::Look(i),
            LockHighlight(picked),
            Node {
                height: Val::Px(ROW_H),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(6.0)),
                ..default()
            },
        ))
        .with_children(|r| {
            let art = ctx.art;
            if let Some(hi) = art.quest_hilight.as_ref().or(art.select_highlight.as_ref()) {
                r.spawn((
                    Hilight,
                    Visibility::Hidden,
                    MaterialNode(hi.clone()),
                    overlay(),
                ));
            }
            let color = if picked { Color::WHITE } else { GOLD };
            text(r, ctx, look.name, 13.0, color);
        });
    }
}

/// A label and its value on one line.
fn line(p: &mut ChildSpawnerCommands, ctx: &TabCtx, label: &str, value: &str) {
    p.spawn(Node {
        height: Val::Px(20.0),
        flex_shrink: 0.0,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        ..default()
    })
    .with_children(|r| {
        text(r, ctx, label, 12.0, GOLD);
        text(r, ctx, value, 12.0, Color::WHITE);
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
