//! The Armory's Quests tab. On the left, every zone grouped by tier with its chain's progress. On
//! the right, the zone picked: its chain's steps (done, under way or still locked), the chain's
//! reward, the quests its giver offers in a run, and how many have been turned in there.

use bevy::prelude::*;
use bevy::ui_render::ui_material::MaterialNode;

use super::{panel, para, text, TabCtx};
use crate::glue::art::{DIM, GOLD};
use crate::glue::widgets::{overlay, Hilight, LockHighlight};
use crate::survivors::data;
use crate::survivors::profile::{slug, Profile};
use crate::survivors::quests::{
    clock, for_map, objective_line, QuestDef, QuestsView, Reward, Step, ZonePick, CHAIN_QUESTS,
    STEPS,
};
use crate::survivors::zones::{self, Tier};

/// The zone column, as wide as the Armory's own side columns.
const SIDE_W: f32 = 250.0;
const ROW_H: f32 = 21.0;
const STEP_H: f32 = 24.0;
const ICON: f32 = 44.0;

/// A description's parchment.
const DESC: Color = Color::srgb(0.85, 0.82, 0.72);
/// `GREEN_FONT_COLOR`, a step or chain done.
const GREEN: Color = Color::srgb(0.1, 1.0, 0.1);
const CARD_FILL: Color = Color::srgba(0.0, 0.0, 0.0, 0.45);
const CARD_DONE: Color = Color::srgba(0.16, 0.12, 0.02, 0.6);
const CARD_EDGE: Color = Color::srgb(0.3, 0.28, 0.24);
const BADGE_FILL: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);

pub(super) fn build(parent: &mut ChildSpawnerCommands, ctx: &TabCtx) {
    let fresh = Profile::default();
    let profile = ctx.res::<Profile>().unwrap_or(&fresh);
    let picked = ctx
        .res::<QuestsView>()
        .and_then(|v| v.0)
        .unwrap_or(ctx.selection.map)
        .min(data::MAPS.len() - 1);
    let side = Node {
        width: Val::Px(SIDE_W),
        height: Val::Percent(100.0),
        flex_shrink: 0.0,
        ..default()
    };
    panel(parent, ctx, side, Some("Quest Chains"), |p| {
        sidebar(p, ctx, profile, picked)
    });
    let main = Node {
        flex_grow: 1.0,
        height: Val::Percent(100.0),
        ..default()
    };
    let title = for_map(picked).map_or("Quests", |z| z.chain.name);
    panel(parent, ctx, main, Some(title), |p| {
        details(p, ctx, profile, picked)
    });
}

/// A zone's chain steps done.
fn steps_done(profile: &Profile, map: usize) -> u8 {
    profile
        .quests
        .zone(&slug(data::MAPS[map].name))
        .steps
        .min(STEPS)
}

/// The left column: each tier's zones, in the menu's order, with their chain's steps.
fn sidebar(p: &mut ChildSpawnerCommands, ctx: &TabCtx, profile: &Profile, picked: usize) {
    let order = zones::list_order();
    for tier in [Tier::Starting, Tier::Frontier, Tier::Elite, Tier::Raid] {
        let heading = match tier {
            Tier::Starting => "Starting Zones",
            Tier::Frontier => "Frontier Zones",
            Tier::Elite => "Elite Zones",
            Tier::Raid => "Raid Zones",
        };
        p.spawn(Node {
            flex_shrink: 0.0,
            margin: UiRect::vertical(Val::Px(3.0)),
            ..default()
        })
        .with_children(|h| {
            text(h, ctx, heading, 12.0, DIM);
        });
        for &map in order.iter().filter(|&&m| zones::map_tier(m) == tier) {
            zone_row(p, ctx, map, steps_done(profile, map), map == picked);
        }
    }
}

/// One zone button: its name and its chain's progress, lit while picked.
fn zone_row(p: &mut ChildSpawnerCommands, ctx: &TabCtx, map: usize, done: u8, picked: bool) {
    p.spawn((
        Button,
        ZonePick(map),
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
        text(r, ctx, data::MAPS[map].name, 13.0, color);
        let (label, color) = if done >= STEPS {
            ("Done".to_string(), GREEN)
        } else {
            (format!("{done} / {STEPS}"), Color::WHITE)
        };
        text(r, ctx, &label, 12.0, color);
    });
}

/// The right column: the chain's story, its steps and reward, then the zone's quests.
fn details(p: &mut ChildSpawnerCommands, ctx: &TabCtx, profile: &Profile, map: usize) {
    let Some(zone) = for_map(map) else {
        para(p, ctx, "This zone has no quests.", 13.0, DIM);
        return;
    };
    let record = profile.quests.zone(&slug(data::MAPS[map].name));
    let done = record.steps.min(STEPS);
    para(p, ctx, zone.chain.text, 12.0, DESC);
    gap(p, 6.0);
    heading(p, ctx, "Chain");
    for (i, step) in Step::ALL.into_iter().enumerate() {
        let (state, color) = match (i as u8).cmp(&done) {
            std::cmp::Ordering::Less => ("Done", GREEN),
            std::cmp::Ordering::Equal => ("Under way", GOLD),
            std::cmp::Ordering::Greater => ("Locked", DIM),
        };
        let mut line = step.line(map);
        if step == Step::Quests && i as u8 == done {
            line = format!(
                "{line} ({}/{CHAIN_QUESTS})",
                record.completed.min(CHAIN_QUESTS)
            );
        }
        p.spawn(Node {
            height: Val::Px(STEP_H),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            ..default()
        })
        .with_children(|r| {
            r.spawn(Node {
                width: Val::Px(70.0),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|l| {
                text(l, ctx, state, 12.0, color);
            });
            let line_color = if color == DIM { DIM } else { Color::WHITE };
            text(r, ctx, &line, 13.0, line_color);
        });
    }
    gap(p, 8.0);
    heading(p, ctx, "Reward");
    reward(p, ctx, zone.chain.reward, done >= STEPS);
    gap(p, 8.0);
    heading(p, ctx, &format!("Quests from {}", zone.giver.name));
    for quest in zone.quests {
        quest_card(p, ctx, quest);
    }
    gap(p, 6.0);
    let s = if record.completed == 1 { "" } else { "s" };
    text(
        p,
        ctx,
        &format!("{} quest{s} turned in here so far", record.completed),
        12.0,
        DESC,
    );
}

/// The chain's reward: a badge naming its kind, its name, and what it does.
fn reward(p: &mut ChildSpawnerCommands, ctx: &TabCtx, reward: Reward, earned: bool) {
    let (badge, what) = match reward {
        Reward::TalentXp(_) => ("XP", "Talent XP for the class that finishes the chain."),
        Reward::CompanionLook(_) => (
            "Pet",
            "A new look for your companion, worn from the Companion tab.",
        ),
        Reward::Mount(_) => (
            "Mount",
            "A mount, kept for when mounts come to the battlegrounds.",
        ),
        Reward::MountHat(_) => ("Hat", "A hat for your mount, kept for when mounts arrive."),
        Reward::Tabard(_) => ("Tabard", "A tabard, kept for when tabards can be worn."),
    };
    p.spawn((
        Node {
            width: Val::Percent(100.0),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            padding: UiRect::all(Val::Px(6.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(if earned { CARD_DONE } else { CARD_FILL }),
        BorderColor::all(CARD_EDGE),
    ))
    .with_children(|c| {
        let edge = if earned { GOLD } else { DIM };
        c.spawn((
            Node {
                width: Val::Px(ICON),
                height: Val::Px(ICON),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(BADGE_FILL),
            BorderColor::all(edge),
        ))
        .with_children(|b| {
            text(b, ctx, badge, 10.0, edge);
        });
        c.spawn(Node {
            flex_grow: 1.0,
            min_width: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(1.0),
            ..default()
        })
        .with_children(|t| {
            let name = format!("{}: {}", reward.kind(), reward.name());
            text(
                t,
                ctx,
                &name,
                13.0,
                if earned { GOLD } else { Color::WHITE },
            );
            para(t, ctx, what, 11.0, DESC);
            if earned {
                text(t, ctx, "Unlocked", 11.0, GREEN);
            }
        });
    });
}

/// One in-run quest: its name, its objective and time limit, and the giver's line.
fn quest_card(p: &mut ChildSpawnerCommands, ctx: &TabCtx, quest: &QuestDef) {
    p.spawn((
        Node {
            width: Val::Percent(100.0),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(1.0),
            padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
            margin: UiRect::bottom(Val::Px(3.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(CARD_FILL),
        BorderColor::all(CARD_EDGE),
    ))
    .with_children(|c| {
        text(c, ctx, quest.name, 13.0, GOLD);
        let goal = format!(
            "{}  ·  {}",
            objective_line(quest.goal, 0, 0.0),
            clock(quest.limit)
        );
        text(c, ctx, &goal, 11.0, Color::WHITE);
        para(c, ctx, quest.text, 11.0, DESC);
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
