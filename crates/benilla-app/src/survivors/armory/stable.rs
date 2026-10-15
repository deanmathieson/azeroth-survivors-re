//! The Stable: account mounts and their hats, with locked entries left visible as a collection.

use bevy::prelude::*;

use super::{columns, para, text, ArmoryTip, Column, TabCtx};
use crate::glue::art::{DIM, GOLD};
use crate::survivors::faire::Kind;
use crate::survivors::gear::ItemIcon;
use crate::survivors::quests::{Reward, ZONES};
use crate::survivors::stable::{prizes, StableAction};

const CELL: f32 = 62.0;
const LOCKED: Color = Color::srgba(0.35, 0.35, 0.35, 0.8);

pub(super) fn build(parent: &mut ChildSpawnerCommands, ctx: &TabCtx) {
    let profile = ctx.res::<crate::survivors::profile::Profile>();
    columns(
        parent,
        ctx,
        Some("Stable"),
        false,
        |column, p| match column {
            Column::Left => {
                let mounts = prizes(Kind::Mount).count();
                let hats = prizes(Kind::MountHat).count();
                let owned_mounts = profile.map_or(0, |s| s.quests.mounts.len());
                let owned_hats = profile.map_or(0, |s| s.quests.mount_hats.len());
                text(p, ctx, "Mount Collection", 18.0, GOLD);
                text(
                    p,
                    ctx,
                    &format!("{owned_mounts} / {mounts} mounts"),
                    14.0,
                    Color::WHITE,
                );
                text(
                    p,
                    ctx,
                    &format!("{owned_hats} / {hats} hats"),
                    14.0,
                    Color::WHITE,
                );
                p.spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                });
                para(p, ctx, "Mounts are cosmetic except for +10% movement speed. Equip one here for runs and the menu preview.", 11.0, DIM);
            }
            Column::Center => {
                text(p, ctx, "Mounts", 18.0, GOLD);
                grid(p, ctx, Kind::Mount, profile, false);
                p.spawn(Node {
                    height: Val::Px(16.0),
                    ..default()
                });
                text(p, ctx, "Mount Hats", 18.0, GOLD);
                grid(p, ctx, Kind::MountHat, profile, true);
            }
            Column::Details => {
                text(p, ctx, "Ride in style", 18.0, GOLD);
                para(p, ctx, "Click an owned mount to equip it; click it again to dismount. Pick a hat below to wear it on compatible mounts.", 12.0, Color::WHITE);
                p.spawn(Node {
                    height: Val::Px(14.0),
                    ..default()
                });
                para(p, ctx, "Locked rewards list their source. Darkmoon Faire prizes are won at the Prize Booth or Counter.", 11.0, DIM);
            }
        },
    );
}

fn grid(
    parent: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    kind: Kind,
    profile: Option<&crate::survivors::profile::Profile>,
    none: bool,
) {
    parent
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(5.0),
            row_gap: Val::Px(6.0),
            margin: UiRect::top(Val::Px(7.0)),
            ..default()
        })
        .with_children(|row| {
            if none {
                tile(
                    row,
                    ctx,
                    "No hat",
                    None,
                    true,
                    false,
                    StableAction::Hat(None),
                    "No mount hat selected.",
                );
            }
            for prize in prizes(kind) {
                let owned = profile.is_some_and(|p| match kind {
                    Kind::Mount => p.quests.mounts.contains(&prize.key),
                    Kind::MountHat => p.quests.mount_hats.contains(&prize.key),
                    _ => false,
                });
                let selected = profile.is_some_and(|p| match kind {
                    Kind::Mount => p.stable.mount.as_deref() == Some(&prize.key),
                    Kind::MountHat => p.stable.mount_hat.as_deref() == Some(&prize.key),
                    _ => false,
                });
                let action = match kind {
                    Kind::Mount => StableAction::Mount(prize.key.clone()),
                    Kind::MountHat => StableAction::Hat(Some(prize.key.clone())),
                    _ => unreachable!(),
                };
                let source = if owned {
                    "Equipped: click to change."
                } else {
                    quest_source(kind, &prize.key).unwrap_or("Darkmoon Faire")
                };
                tile(
                    row,
                    ctx,
                    &prize.name,
                    Some(prize.icon),
                    owned,
                    selected,
                    action,
                    source,
                );
            }
        });
}

/// A chain's named reward is clearer than the Faire's collection fallback for its matching tile.
fn quest_source(kind: Kind, key: &str) -> Option<&'static str> {
    ZONES.iter().find_map(|zone| {
        let reward = zone.chain.reward;
        let matches_kind = matches!(
            (kind, reward),
            (Kind::Mount, Reward::Mount(_)) | (Kind::MountHat, Reward::MountHat(_))
        );
        (matches_kind && reward.key().as_deref() == Some(key)).then_some(zone.chain.name)
    })
}

fn tile(
    parent: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    name: &str,
    icon: Option<&'static str>,
    owned: bool,
    selected: bool,
    action: StableAction,
    source: &str,
) {
    let tint = if owned { Color::WHITE } else { LOCKED };
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(CELL),
                height: Val::Px(86.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(if selected { 2.0 } else { 1.0 })),
                ..default()
            },
            BorderColor::all(if selected { GOLD } else { LOCKED }),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
            action,
            ArmoryTip {
                title: name.into(),
                lines: vec![(source.into(), if owned { Color::WHITE } else { DIM })],
            },
        ))
        .with_children(|c| {
            let mut image = c.spawn((
                ImageNode::default(),
                Node {
                    width: Val::Px(36.0),
                    height: Val::Px(36.0),
                    margin: UiRect::top(Val::Px(3.0)),
                    ..default()
                },
            ));
            if let Some(icon) = icon {
                image.insert(ItemIcon { icon, tint });
            }
            text(c, ctx, name, 9.0, tint);
        });
}
