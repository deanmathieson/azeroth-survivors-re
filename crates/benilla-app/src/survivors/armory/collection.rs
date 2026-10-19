//! The Armory's Collection tab: the picked class's 45 pieces as a grid, a row per rarity (with the
//! battlegrounds that drop it) and a column per slot, uncollected pieces a dim "?" and worn ones
//! badged "E". Under them, Molten Core: the class's Tier 1 set in its slots, and the raid's named
//! pieces it may wear; then the crafted pieces it may wear (P3), equipped from the bags. The left
//! column tallies the collection and the account's raid vault; clicking a piece shows it under
//! Details, with what it grants, who drops it (or which craft makes it) and an Equip button once
//! it is collected.
//!
//! The other gear tabs share this file's kit: [`item_icon`], [`item_tip`], [`progress_bar`] and
//! [`sheen`].

use bevy::prelude::*;
use bevy::ui_render::ui_material::MaterialNode;

use super::{columns, para, text, ArmoryTip, Column, TabCtx};
use crate::glue::art::{DIM, GOLD};
use crate::glue::widgets::{glue_button, outlined_text, overlay, GlueBtnKind, GlueText, Hilight};
use crate::survivors::gear::raid::RaidKind;
use crate::survivors::gear::{
    class_gear, crafted, gear_for, item, items_for, percent, progress, raid_piece, raid_vault,
    rarity_label, set_of, sources, tier_set, usable_by, zones_dropping, GearAction, GearSave,
    GearView, Item, ItemIcon, RaidVault, Slot, RARITIES,
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
        .filter(|i| class.is_some_and(|c| usable_by(i, c)));
    columns(parent, ctx, Some("Collection"), false, |col, p| {
        let Some(class) = class else {
            if col == Column::Center {
                para(p, ctx, "Pick a class to see its gear.", 13.0, DIM);
            }
            return;
        };
        let gear = class_gear(ctx.world, class);
        match col {
            Column::Left => tally(p, ctx, class, gear, raid_vault(ctx.world)),
            Column::Center => grid(p, ctx, class, gear, picked),
            Column::Details => details(p, ctx, gear, picked),
        }
    });
}

/// The left column: pieces collected in all and by rarity, and how many slots are filled; then
/// Molten Core's pieces, the Tier 1 set worn, and what the account's raid vault holds.
fn tally(
    p: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    class: usize,
    gear: &GearSave,
    vault: &RaidVault,
) {
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
    let raid: Vec<_> = gear_for(class).collect();
    let have = raid.iter().filter(|r| gear.has(r.item.id)).count();
    row(
        p,
        ctx,
        "Molten Core",
        GOLD,
        &format!("{have} / {}", raid.len()),
    );
    if let Some(set) = tier_set(class) {
        let on = set.worn(&gear.equipped_ids());
        row(p, ctx, set.name, GOLD, &format!("{on} / 8"));
    }
    let made: Vec<_> = crafted::for_class(class).collect();
    let have = made.iter().filter(|c| gear.has(c.item.id)).count();
    row(p, ctx, "Crafted", GOLD, &format!("{have} / {}", made.len()));
    row(p, ctx, "Eye of Sulfuras", GOLD, &vault.eyes.to_string());
    row(p, ctx, "Sulfuron Ingots", GOLD, &vault.ingots.to_string());
    for id in &vault.bound {
        if let Some(r) = raid_piece(*id) {
            para(
                p,
                ctx,
                &format!("Bound: {}", r.item.name),
                11.0,
                r.item.rarity.color(),
            );
        }
    }
    p.spawn(Node {
        flex_grow: 1.0,
        ..default()
    });
    para(
        p,
        ctx,
        "Bosses drop gear for the class you play. Molten Core's Bound jewelry is worn from the \
         start of every run. Click a piece to see where it drops.",
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
    raid_rows(p, ctx, class, gear, picked);
    crafted_rows(p, ctx, class, gear, picked);
}

/// The crafted pieces class `class` may wear, as many to a row as there are slots: what
/// Blacksmithing, Leatherworking and Tailoring make, equipped from the bags.
fn crafted_rows(
    p: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    class: usize,
    gear: &GearSave,
    picked: Option<&Item>,
) {
    let made: Vec<&'static Item> = crafted::for_class(class).map(|c| &c.item).collect();
    if made.is_empty() {
        return;
    }
    p.spawn(Node {
        flex_shrink: 0.0,
        margin: UiRect::top(Val::Px(10.0)),
        ..default()
    })
    .with_children(|l| {
        text(l, ctx, "Crafted", 13.0, GOLD);
    });
    for chunk in made.chunks(Slot::ALL.len()) {
        p.spawn(Node {
            flex_shrink: 0.0,
            column_gap: Val::Px(2.0),
            ..default()
        })
        .with_children(|r| {
            for i in 0..Slot::ALL.len() {
                r.spawn(Node {
                    flex_grow: 1.0,
                    flex_basis: Val::Px(0.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|c| {
                    if let Some(&piece) = chunk.get(i) {
                        let on = picked.is_some_and(|p| p.id == piece.id);
                        piece_cell(c, ctx, piece, gear, on);
                    }
                });
            }
        });
    }
}

/// Which craft makes crafted piece `piece`: its profession and the skill its recipe needs.
fn craft_of(piece: &Item) -> Option<(&'static str, u16)> {
    let real = crafted::crafted_piece(piece.id)?.real;
    crate::survivors::professions::recipes::all()
        .find(|r| r.product == real)
        .map(|r| (r.line.name(), r.req))
}

/// Molten Core under the rarities: the class's Tier 1 set in its slots, then the raid's named
/// pieces it may wear, as many to a row as there are slots.
fn raid_rows(
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
    let label = |p: &mut ChildSpawnerCommands, text_: &str, color: Color| {
        p.spawn(Node {
            flex_shrink: 0.0,
            margin: UiRect::top(Val::Px(10.0)),
            ..default()
        })
        .with_children(|l| {
            text(l, ctx, text_, 13.0, color);
        });
    };
    let is_picked = |id: u16| picked.is_some_and(|p| p.id == id);
    if let Some(set) = tier_set(class) {
        label(p, &format!("Molten Core: {}", set.name), GOLD);
        p.spawn(cells()).with_children(|r| {
            for slot in Slot::ALL {
                r.spawn(cell()).with_children(|c| {
                    if let Some(piece) = set.pieces().find(|q| q.item.slot == slot) {
                        piece_cell(c, ctx, &piece.item, gear, is_picked(piece.item.id));
                    }
                });
            }
        });
    }
    let named: Vec<&'static Item> = gear_for(class)
        .filter(|r| set_of(r.item.id).is_none())
        .map(|r| &r.item)
        .collect();
    if named.is_empty() {
        return;
    }
    label(p, "Molten Core: named drops", GOLD);
    for chunk in named.chunks(Slot::ALL.len()) {
        p.spawn(cells()).with_children(|r| {
            for i in 0..Slot::ALL.len() {
                r.spawn(cell()).with_children(|c| {
                    if let Some(&piece) = chunk.get(i) {
                        piece_cell(c, ctx, piece, gear, is_picked(piece.id));
                    }
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
    para(p, ctx, &kind_line(piece), 12.0, Color::WHITE);
    let (status, color) = status(have, worn);
    para(p, ctx, status, 12.0, color);
    gap(p, 6.0);
    for b in piece.bonuses {
        para(p, ctx, &b.describe(), 12.0, TIP_GREEN);
    }
    for (line, color) in set_lines(piece, gear) {
        para(p, ctx, &line, 11.0, color);
    }
    if let Some(r) = raid_piece(piece.id) {
        para(p, ctx, &format!("Item {} in 1.12", r.real), 10.0, DIM);
    }
    gap(p, 8.0);
    if piece.id == crate::survivors::gear::SULFURAS {
        para(
            p,
            ctx,
            "Forged from an Eye of Sulfuras and a Sulfuron Ingot, for the Warrior, the Paladin \
             and the Shaman.",
            11.0,
            DIM,
        );
    }
    if let Some((line, req)) = craft_of(piece) {
        para(p, ctx, "Crafted by", 13.0, GOLD);
        para(p, ctx, &format!("{line} ({req})"), 12.0, Color::WHITE);
        para(
            p,
            ctx,
            "Binds when equipped: right-click it in your bags to wear it.",
            11.0,
            DIM,
        );
    } else {
        para(p, ctx, "Dropped by", 13.0, GOLD);
    }
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

/// A piece's hover box: rarity and slot, its enchant as 1.12's green line under it, what it
/// grants, and whether the class has it. The worn line is the useful 1.12-style comparison when
/// the same slot already has an item.
pub(in crate::survivors) fn item_tip(piece: &Item, gear: &GearSave) -> ArmoryTip {
    let (status, color) = status(gear.has(piece.id), gear.wears(piece.id));
    let enchant = gear.enchant_of(piece.id).map(|e| {
        (
            crate::survivors::professions::recipes::enchant_line(e),
            TIP_GREEN,
        )
    });
    let lines = std::iter::once((kind_line(piece), piece.rarity.color()))
        .chain(enchant)
        .chain(
            piece
                .bonuses
                .iter()
                .map(|b| (format!("Equip: {}", b.describe()), TIP_GREEN)),
        )
        .chain(set_lines(piece, gear))
        .chain(
            gear.worn(piece.slot)
                .filter(|_| raid_piece(piece.id).is_none_or(|r| r.kind == RaidKind::Gear))
                .filter(|worn| worn.id != piece.id)
                .map(|worn| (format!("Currently equipped: {}", worn.name), DIM)),
        )
        .chain(std::iter::once((status.to_string(), color)))
        .collect();
    ArmoryTip {
        title: piece.name.to_string(),
        lines,
    }
}

/// A piece's second tooltip line: "Epic Head"; a raid piece's own ("Bound Epic Ring",
/// "Quest Item").
fn kind_line(piece: &Item) -> String {
    match raid_piece(piece.id) {
        Some(r) => r.label(),
        None => format!("{} {}", rarity_label(piece.rarity), piece.slot.label()),
    }
}

/// A Tier 1 piece's set block, as the 1.12 tooltip prints it: the set and how many of it `gear`
/// wears, then each step, green once reached.
pub(in crate::survivors) fn set_lines(piece: &Item, gear: &GearSave) -> Vec<(String, Color)> {
    let Some(set) = set_of(piece.id) else {
        return Vec::new();
    };
    let on = set.worn(&gear.equipped_ids());
    std::iter::once((format!("{} ({on}/8)", set.name), GOLD))
        .chain(set.bonuses.iter().map(|b| {
            let color = if on >= b.pieces { TIP_GREEN } else { DIM };
            (format!("({}) Set: {}", b.pieces, b.text), color)
        }))
        .collect()
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

    #[test]
    fn tier_one_tips_show_the_set_and_its_steps() {
        let set = tier_set(4).unwrap();
        let pieces: Vec<_> = set.pieces().collect();
        let mut gear = GearSave::default();
        for p in &pieces[..3] {
            gear.collect(&p.item);
        }
        let tip = item_tip(&pieces[0].item, &gear);
        assert_eq!(tip.lines[0].0, "Epic Head");
        let at = |s: &str| {
            tip.lines
                .iter()
                .find(|l| l.0.starts_with(s))
                .unwrap()
                .clone()
        };
        assert_eq!(at(set.name).0, "Vestments of Prophecy (3/8)");
        assert_eq!(at("(3) Set:").1, TIP_GREEN);
        assert_eq!(at("(5) Set:").1, DIM);
        assert_eq!(at("(8) Set:").1, DIM);
        // A named raid piece has no set; jewelry and quest items say what they are.
        let staff = raid_piece(2111).unwrap();
        assert!(set_lines(&staff.item, &gear).is_empty());
        assert_eq!(
            kind_line(&raid_piece(2135).unwrap().item),
            "Bound Legendary Ring"
        );
        assert_eq!(kind_line(&raid_piece(2150).unwrap().item), "Quest Item");
    }

    #[test]
    fn item_tip_uses_the_hovered_item_and_its_slot_comparison() {
        let worn = piece(0, Slot::Head, Rarity::Common).unwrap();
        let hovered = piece(0, Slot::Head, Rarity::Epic).unwrap();
        let mut gear = GearSave::default();
        gear.collect(worn);
        let tip = item_tip(hovered, &gear);
        assert_eq!(tip.title, hovered.name);
        assert!(tip.lines.iter().any(|(line, _)| line.contains(worn.name)));
        assert!(tip.lines.iter().any(|(line, _)| line.starts_with("Equip:")));
    }

    /// An enchanted piece shows its enchant as the green line under its kind, as 1.12 does.
    #[test]
    fn item_tip_shows_the_enchant_in_green() {
        let weapon = piece(0, Slot::MainHand, Rarity::Rare).unwrap();
        let mut gear = GearSave::default();
        gear.collect(weapon);
        assert_eq!(
            item_tip(weapon, &gear).lines.len(),
            weapon.bonuses.len() + 2
        );
        gear.enchant(weapon.id, 1900);
        let tip = item_tip(weapon, &gear);
        assert_eq!(
            tip.lines[1],
            (
                "Crusader (+5% damage, +1 health per second)".to_string(),
                TIP_GREEN
            )
        );
    }
}
