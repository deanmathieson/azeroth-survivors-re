//! The Armory's Collection tab: the picked class's 45 pieces as a grid, a row per rarity (with the
//! battlegrounds that drop it) and a column per slot, uncollected pieces a dim "?" and worn ones
//! badged "E". The left column tallies the collection; clicking a piece shows it under Details,
//! with what it grants, who drops it and an Equip button once it is collected.
//!
//! The other gear tabs share this file's kit: [`item_icon`], [`item_tip`], [`progress_bar`] and
//! [`sheen`].

use bevy::prelude::*;
use bevy::ui_render::ui_material::MaterialNode;

use super::{columns, para, text, ArmoryTip, Column, TabCtx};
use crate::glue::art::{DIM, GOLD};
use crate::glue::widgets::{glue_button, outlined_text, overlay, GlueBtnKind, GlueText, Hilight};
use crate::survivors::gear::{
    class_gear, item, items_for, percent, progress, rarity_label, sources, zones_dropping,
    GearAction, GearSave, GearView, Item, ItemIcon, Slot, RARITIES,
};

/// `GREEN_FONT_COLOR`: what a piece grants, and a worn piece.
pub(super) const TIP_GREEN: Color = Color::srgb(0.1, 1.0, 0.1);
/// An icon slot's fill and an uncollected piece's edge.
const CELL_FILL: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);
const CELL_EDGE: Color = Color::srgb(0.3, 0.3, 0.3);
const CELL: f32 = 38.0;

pub(super) fn build(parent: &mut ChildSpawnerCommands, ctx: &TabCtx) {
    let class = ctx.selection.class;
    // The pick shows only while it is the picked class's.
    let picked = ctx
        .res::<GearView>()
        .and_then(|v| v.item)
        .and_then(item)
        .filter(|i| Some(i.class) == class);
    columns(parent, ctx, Some("Collection"), false, |col, p| {
        let Some(class) = class else {
            if col == Column::Center {
                para(p, ctx, "Pick a class to see its gear.", 13.0, DIM);
            }
            return;
        };
        let gear = class_gear(ctx.world, class);
        match col {
            Column::Left => tally(p, ctx, class, gear),
            Column::Center => grid(p, ctx, class, gear, picked),
            Column::Details => details(p, ctx, gear, picked),
        }
    });
}

/// The left column: pieces collected in all and by rarity, and how many slots are filled.
fn tally(p: &mut ChildSpawnerCommands, ctx: &TabCtx, class: usize, gear: &GearSave) {
    let (have, of) = progress(class, gear, None);
    p.spawn(Node {
        justify_content: JustifyContent::Center,
        margin: UiRect::vertical(Val::Px(4.0)),
        ..default()
    })
    .with_children(|c| {
        text(c, ctx, &format!("{have} / {of}"), 22.0, Color::WHITE);
    });
    progress_bar(p, have, of);
    p.spawn(Node {
        height: Val::Px(10.0),
        ..default()
    });
    for rarity in RARITIES {
        let (have, of) = progress(class, gear, Some(rarity));
        row(
            p,
            ctx,
            rarity_label(rarity),
            rarity.color(),
            &format!("{have} / {of}"),
        );
    }
    let worn = gear.equipped_ids().len();
    row(
        p,
        ctx,
        "Slots filled",
        GOLD,
        &format!("{worn} / {}", Slot::ALL.len()),
    );
    p.spawn(Node {
        flex_grow: 1.0,
        ..default()
    });
    para(
        p,
        ctx,
        "Bosses drop gear for the class you play. Click a piece to see where it drops.",
        11.0,
        DIM,
    );
}

/// A label left and a value right.
fn row(p: &mut ChildSpawnerCommands, ctx: &TabCtx, label: &str, color: Color, value: &str) {
    p.spawn(Node {
        height: Val::Px(22.0),
        flex_shrink: 0.0,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        padding: UiRect::horizontal(Val::Px(6.0)),
        ..default()
    })
    .with_children(|r| {
        text(r, ctx, label, 13.0, color);
        text(r, ctx, value, 13.0, Color::WHITE);
    });
}

/// `have` of `of` as a gold bar.
pub(super) fn progress_bar(p: &mut ChildSpawnerCommands, have: usize, of: usize) {
    let frac = if of == 0 {
        0.0
    } else {
        have as f32 / of as f32
    };
    p.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(12.0),
            flex_shrink: 0.0,
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BorderColor::all(CELL_EDGE),
        BackgroundColor(CELL_FILL),
    ))
    .with_children(|b| {
        b.spawn((
            Node {
                width: Val::Percent(frac * 100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(GOLD.with_alpha(0.8)),
        ));
    });
}

/// The center: a header of slot names, then a row of nine pieces per rarity under its label and
/// the battlegrounds that drop it.
fn grid(
    p: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    class: usize,
    gear: &GearSave,
    picked: Option<&Item>,
) {
    let cells = || Node {
        flex_shrink: 0.0,
        column_gap: Val::Px(2.0),
        ..default()
    };
    let cell = || Node {
        flex_grow: 1.0,
        flex_basis: Val::Px(0.0),
        justify_content: JustifyContent::Center,
        ..default()
    };
    p.spawn(cells()).with_children(|h| {
        for slot in Slot::ALL {
            h.spawn(cell()).with_children(|c| {
                text(c, ctx, slot.label(), 10.0, GOLD);
            });
        }
    });
    for rarity in RARITIES {
        p.spawn(Node {
            flex_shrink: 0.0,
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Baseline,
            margin: UiRect::top(Val::Px(10.0)),
            ..default()
        })
        .with_children(|l| {
            text(l, ctx, rarity_label(rarity), 13.0, rarity.color());
            outlined_text(
                l,
                Node {
                    flex_grow: 1.0,
                    flex_basis: Val::Px(0.0),
                    ..default()
                },
                (),
                (),
                GlueText {
                    text: &zones_dropping(rarity).join(", "),
                    size: 10.0,
                    color: DIM,
                    wrap: true,
                },
                ctx.font,
                1.0,
            );
        });
        p.spawn(cells()).with_children(|r| {
            for slot in Slot::ALL {
                let Some(piece) = items_for(class)
                    .iter()
                    .find(|i| i.slot == slot && i.rarity == rarity)
                else {
                    continue;
                };
                r.spawn(cell()).with_children(|c| {
                    piece_cell(
                        c,
                        ctx,
                        piece,
                        gear,
                        picked.is_some_and(|p| p.id == piece.id),
                    );
                });
            }
        });
    }
}

/// One piece of the grid: its icon once collected, else a dim "?"; edged in its rarity's colour
/// (gold while picked) and badged "E" while worn.
fn piece_cell(
    c: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    piece: &'static Item,
    gear: &GearSave,
    picked: bool,
) {
    let have = gear.has(piece.id);
    let edge = if picked {
        GOLD
    } else if have {
        piece.rarity.color()
    } else {
        CELL_EDGE
    };
    c.spawn((
        Button,
        GearAction::Pick(piece.id),
        item_tip(piece, gear),
        Node {
            width: Val::Px(CELL),
            height: Val::Px(CELL),
            flex_shrink: 0.0,
            border: UiRect::all(Val::Px(if picked { 2.0 } else { 1.0 })),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BorderColor::all(edge),
        BackgroundColor(CELL_FILL),
    ))
    .with_children(|b| {
        if have {
            b.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                icon_image(),
                ItemIcon {
                    icon: piece.icon,
                    tint: Color::WHITE,
                },
            ));
        } else {
            text(b, ctx, "?", 18.0, DIM);
        }
        if gear.wears(piece.id) {
            outlined_text(
                b,
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(2.0),
                    top: Val::Px(0.0),
                    ..default()
                },
                (),
                (),
                GlueText {
                    text: "E",
                    size: 12.0,
                    color: TIP_GREEN,
                    wrap: false,
                },
                ctx.font,
                1.0,
            );
        }
        if let Some(hi) = &ctx.art.mouse_hilight {
            b.spawn((
                Hilight,
                Visibility::Hidden,
                MaterialNode(hi.clone()),
                overlay(),
            ));
        }
    });
}

/// The Details column: the picked piece in full, else a hint.
fn details(p: &mut ChildSpawnerCommands, ctx: &TabCtx, gear: &GearSave, picked: Option<&Item>) {
    let Some(piece) = picked else {
        para(
            p,
            ctx,
            "Click a piece to see what it grants and who drops it.",
            13.0,
            DIM,
        );
        return;
    };
    let (have, worn) = (gear.has(piece.id), gear.wears(piece.id));
    p.spawn(Node {
        column_gap: Val::Px(8.0),
        align_items: AlignItems::Center,
        flex_shrink: 0.0,
        margin: UiRect::bottom(Val::Px(6.0)),
        ..default()
    })
    .with_children(|h| {
        let tint = if have {
            Color::WHITE
        } else {
            Color::srgb(0.4, 0.4, 0.4)
        };
        item_icon(h, piece.icon, 40.0, tint);
        outlined_text(
            h,
            Node {
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                ..default()
            },
            (),
            (),
            GlueText {
                text: piece.name,
                size: 15.0,
                color: piece.rarity.color(),
                wrap: true,
            },
            ctx.font,
            1.0,
        );
    });
    para(
        p,
        ctx,
        &format!("{} {}", rarity_label(piece.rarity), piece.slot.label()),
        12.0,
        Color::WHITE,
    );
    let (status, color) = status(have, worn);
    para(p, ctx, status, 12.0, color);
    gap(p, 6.0);
    for b in piece.bonuses {
        para(p, ctx, &b.describe(), 12.0, TIP_GREEN);
    }
    gap(p, 8.0);
    para(p, ctx, "Dropped by", 13.0, GOLD);
    for s in sources(piece) {
        para(p, ctx, s.boss, 12.0, Color::WHITE);
        para(
            p,
            ctx,
            &format!("{}, {} a kill", s.zone, percent(s.chance)),
            11.0,
            DIM,
        );
    }
    p.spawn(Node {
        flex_grow: 1.0,
        ..default()
    });
    if have && !worn {
        p.spawn(Node {
            justify_content: JustifyContent::Center,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|b| {
            glue_button(
                b,
                ctx.art,
                ctx.font,
                GearAction::Equip(piece.id),
                "Equip",
                150.0,
                38.0,
                GlueBtnKind::Small,
                1.0,
            );
        });
    }
}

/// A piece's standing: worn, collected or not.
pub(super) fn status(have: bool, worn: bool) -> (&'static str, Color) {
    match (have, worn) {
        (_, true) => ("Equipped", TIP_GREEN),
        (true, false) => ("Collected", Color::WHITE),
        (false, false) => ("Not collected", DIM),
    }
}

fn gap(p: &mut ChildSpawnerCommands, h: f32) {
    p.spawn(Node {
        height: Val::Px(h),
        flex_shrink: 0.0,
        ..default()
    });
}

/// A dark slot an [`ItemIcon`] paints once its file loads.
fn icon_image() -> ImageNode {
    ImageNode {
        color: Color::srgba(0.0, 0.0, 0.0, 0.6),
        ..default()
    }
}

/// A `size`-square icon `Interface\Icons\<icon>`, drawn in `tint`.
pub(super) fn item_icon(p: &mut ChildSpawnerCommands, icon: &'static str, size: f32, tint: Color) {
    p.spawn((
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            flex_shrink: 0.0,
            ..default()
        },
        icon_image(),
        ItemIcon { icon, tint },
    ));
}

/// A piece's hover box: rarity and slot, what it grants, and whether the class has it.
pub(super) fn item_tip(piece: &Item, gear: &GearSave) -> ArmoryTip {
    let (status, color) = status(gear.has(piece.id), gear.wears(piece.id));
    let lines = std::iter::once((
        format!("{} {}", rarity_label(piece.rarity), piece.slot.label()),
        piece.rarity.color(),
    ))
    .chain(piece.bonuses.iter().map(|b| (b.describe(), TIP_GREEN)))
    .chain(std::iter::once((status.to_string(), color)))
    .collect();
    ArmoryTip {
        title: piece.name.to_string(),
        lines,
    }
}

/// A list row's hover sheen: the quest log's, else the zone list's.
pub(super) fn sheen(r: &mut ChildSpawnerCommands, ctx: &TabCtx) {
    let hi = ctx
        .art
        .quest_hilight
        .as_ref()
        .or(ctx.art.select_highlight.as_ref());
    if let Some(hi) = hi {
        r.spawn((
            Hilight,
            Visibility::Hidden,
            MaterialNode(hi.clone()),
            overlay(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::survivors::gear::{piece, Rarity};

    #[test]
    fn tips_and_status_read_the_collection() {
        let p = piece(4, Slot::Legs, Rarity::Epic).unwrap();
        let mut gear = GearSave::default();
        let tip = item_tip(p, &gear);
        assert_eq!(tip.title, p.name);
        assert_eq!(
            tip.lines[0],
            ("Epic Legs".to_string(), Rarity::Epic.color())
        );
        assert_eq!(tip.lines.len(), p.bonuses.len() + 2);
        assert_eq!(tip.lines.last().unwrap().0, "Not collected");
        gear.collect(p);
        assert_eq!(item_tip(p, &gear).lines.last().unwrap().0, "Equipped");
        assert_eq!(status(true, false).0, "Collected");
    }
}
