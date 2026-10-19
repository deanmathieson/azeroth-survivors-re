//! The Armory's Jewelry tab: the account's Jewelry Collection and the loadout it wears into every
//! run, for every class. The left column is the loadout in WoW's slots (Neck, Finger 1 and 2,
//! Trinket 1 and 2) and how much is collected; the center is the collection, a grid per kind,
//! collected pieces in their icons edged in their rarity's colour (badged "E" while worn), locked
//! ones greyed; Details shows the picked piece: what it grants, its proc, and where it drops.
//!
//! Click a collected piece to wear it, into the first empty slot of its kind (else over the
//! first), and again to take it off; click a loadout slot to empty it. Every piece is
//! Unique-Equipped. Hover any piece for its tooltip.

use bevy::prelude::*;
use bevy::ui_render::ui_material::MaterialNode;

use super::collection::{item_icon, progress_bar, TIP_GREEN};
use super::{columns, para, text, ArmoryState, ArmoryTip, Column, TabCtx};
use crate::glue::art::{DIM, GOLD};
use crate::glue::widgets::{outlined_text, overlay, GlueText, Hilight};
use crate::glue::GlueClicks;
use crate::survivors::gear::{rarity_label, ItemIcon};
use crate::survivors::jewelry::collection::{collection_pieces, source, JewelryCollection};
use crate::survivors::jewelry::{piece, proc_text, Kind, LoadoutSlot, Piece};
use crate::survivors::profile::Profile;

const CELL_FILL: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);
const CELL_EDGE: Color = Color::srgb(0.3, 0.3, 0.3);
/// A locked piece's icon, greyed.
const LOCKED: Color = Color::srgb(0.28, 0.28, 0.28);
/// `YELLOW_FONT_COLOR`: where a locked piece drops, as a tooltip's source line.
const SOURCE: Color = Color::srgb(1.0, 0.82, 0.0);
const CELL: f32 = 40.0;
/// The grid's kinds, in the character sheet's order.
const KINDS: [Kind; 3] = [Kind::Necklace, Kind::Ring, Kind::Trinket];

/// The piece the tab shows under Details.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct JewelryView {
    pub piece: Option<&'static str>,
}

/// A click in the Jewelry tab.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum JewelryAction {
    /// Show this piece, by key, under Details; a collected one goes on, or comes off.
    Piece(&'static str),
    /// Empty this loadout slot.
    Unequip(LoadoutSlot),
}

/// Nothing collected, for a world without a profile.
static EMPTY: std::sync::LazyLock<JewelryCollection> =
    std::sync::LazyLock::new(JewelryCollection::default);

pub(super) fn build(parent: &mut ChildSpawnerCommands, ctx: &TabCtx) {
    let c = ctx.res::<Profile>().map_or(&*EMPTY, |p| &p.collection);
    let picked = ctx
        .res::<JewelryView>()
        .and_then(|v| v.piece)
        .and_then(piece);
    columns(parent, ctx, Some("Loadout"), false, |col, p| match col {
        Column::Left => loadout(p, ctx, c),
        Column::Center => grid(p, ctx, c, picked),
        Column::Details => details(p, ctx, c, picked),
    });
}

fn gap(p: &mut ChildSpawnerCommands, h: f32) {
    p.spawn(Node {
        height: Val::Px(h),
        flex_shrink: 0.0,
        ..default()
    });
}

/// A wrapped line that takes the row's room left, and no more.
fn fill_text(p: &mut ChildSpawnerCommands, ctx: &TabCtx, s: &str, size: f32, color: Color) {
    outlined_text(
        p,
        Node {
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            ..default()
        },
        (),
        (),
        GlueText {
            text: s,
            size,
            color,
            wrap: true,
        },
        ctx.font,
        1.0,
    );
}

/// The hover sheen every clickable cell and row has.
fn sheen(b: &mut ChildSpawnerCommands, ctx: &TabCtx) {
    if let Some(hi) = &ctx.art.mouse_hilight {
        b.spawn((
            Hilight,
            Visibility::Hidden,
            MaterialNode(hi.clone()),
            overlay(),
        ));
    }
}

/// The left column: the five loadout slots, then how much is collected.
fn loadout(p: &mut ChildSpawnerCommands, ctx: &TabCtx, c: &JewelryCollection) {
    let worn = c.worn();
    for slot in LoadoutSlot::ALL {
        let on = worn[slot.index()];
        let tip = match on {
            Some(q) => tip(q, c),
            None => ArmoryTip {
                title: slot.label().to_string(),
                lines: vec![(
                    format!(
                        "Empty: click a collected {} to wear it.",
                        slot.kind().label().to_lowercase()
                    ),
                    DIM,
                )],
            },
        };
        p.spawn((
            Button,
            JewelryAction::Unequip(slot),
            tip,
            Node {
                flex_shrink: 0.0,
                column_gap: Val::Px(8.0),
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(3.0)),
                margin: UiRect::bottom(Val::Px(2.0)),
                ..default()
            },
        ))
        .with_children(|r| {
            r.spawn((
                Node {
                    width: Val::Px(CELL),
                    height: Val::Px(CELL),
                    flex_shrink: 0.0,
                    border: UiRect::all(Val::Px(1.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BorderColor::all(on.map_or(CELL_EDGE, |q| q.rarity.color())),
                BackgroundColor(CELL_FILL),
            ))
            .with_children(|b| match on {
                Some(q) => icon(b, q, Color::WHITE),
                None => {
                    text(b, ctx, "-", 16.0, DIM);
                }
            });
            r.spawn(Node {
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                ..default()
            })
            .with_children(|t| {
                text(t, ctx, slot.label(), 11.0, GOLD);
                match on {
                    Some(q) => para(t, ctx, q.name, 12.0, q.rarity.color()),
                    None => para(t, ctx, "Empty", 12.0, DIM),
                };
            });
            sheen(r, ctx);
        });
    }
    gap(p, 10.0);
    let of = collection_pieces().count();
    let have = collection_pieces().filter(|q| c.has(q.key)).count();
    p.spawn(Node {
        justify_content: JustifyContent::Center,
        flex_shrink: 0.0,
        margin: UiRect::vertical(Val::Px(4.0)),
        ..default()
    })
    .with_children(|t| {
        text(
            t,
            ctx,
            &format!("Collected {have} / {of}"),
            16.0,
            Color::WHITE,
        );
    });
    progress_bar(p, have, of);
    p.spawn(Node {
        flex_grow: 1.0,
        ..default()
    });
    para(
        p,
        ctx,
        "Rare and better jewelry joins your collection the first time you get it, for every \
         class. Your loadout is worn from the start of every run. Click a piece to wear it; \
         click a slot to empty it.",
        11.0,
        DIM,
    );
}

/// The center: a section per kind, its collectable pieces in a wrapping row.
fn grid(p: &mut ChildSpawnerCommands, ctx: &TabCtx, c: &JewelryCollection, picked: Option<&Piece>) {
    for kind in KINDS {
        let of: Vec<&'static Piece> = collection_pieces().filter(|q| q.kind == kind).collect();
        let have = of.iter().filter(|q| c.has(q.key)).count();
        p.spawn(Node {
            flex_shrink: 0.0,
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Baseline,
            margin: UiRect::new(Val::ZERO, Val::ZERO, Val::Px(8.0), Val::Px(4.0)),
            ..default()
        })
        .with_children(|h| {
            text(h, ctx, kind.plural(), 14.0, GOLD);
            text(
                h,
                ctx,
                &format!("{have} / {} collected", of.len()),
                11.0,
                DIM,
            );
        });
        p.spawn(Node {
            flex_shrink: 0.0,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(4.0),
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|r| {
            for q in of {
                cell(r, ctx, q, c, picked.is_some_and(|p| p.key == q.key));
            }
        });
    }
}

/// A piece's icon, filling its cell.
fn icon(b: &mut ChildSpawnerCommands, q: &'static Piece, tint: Color) {
    b.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        ImageNode {
            color: Color::srgba(0.0, 0.0, 0.0, 0.6),
            ..default()
        },
        ItemIcon { icon: q.icon, tint },
    ));
}

/// One piece: its icon, edged in its rarity's colour (gold while picked) and badged "E" while
/// worn; greyed and plainly edged while locked.
fn cell(
    r: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    q: &'static Piece,
    c: &JewelryCollection,
    picked: bool,
) {
    let have = c.has(q.key);
    let edge = if picked {
        GOLD
    } else if have {
        q.rarity.color()
    } else {
        CELL_EDGE
    };
    r.spawn((
        Button,
        JewelryAction::Piece(q.key),
        tip(q, c),
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
        icon(b, q, if have { Color::WHITE } else { LOCKED });
        if c.loadout.slot_of(q.key).is_some() && have {
            outlined_text(
                b,
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(2.0),
                    bottom: Val::Px(0.0),
                    ..default()
                },
                (),
                (),
                GlueText {
                    text: "E",
                    size: 11.0,
                    color: TIP_GREEN,
                    wrap: false,
                },
                ctx.font,
                1.0,
            );
        }
        sheen(b, ctx);
    });
}

/// How a piece stands: worn in which slot, collected, or where to get it.
fn standing(q: &Piece, c: &JewelryCollection) -> Vec<(String, Color)> {
    if !c.has(q.key) {
        return vec![("Not yet collected".to_string(), DIM), (source(q), SOURCE)];
    }
    match c.loadout.slot_of(q.key) {
        Some(slot) => vec![(format!("Worn: {}", slot.label()), TIP_GREEN)],
        None => vec![("Collected".to_string(), Color::WHITE)],
    }
}

/// What a piece is and grants, as a `GameTooltip` lists it.
fn lines(q: &Piece) -> Vec<(String, Color)> {
    std::iter::once((
        format!("{} {}", rarity_label(q.rarity), q.kind.label()),
        q.rarity.color(),
    ))
    .chain(std::iter::once((
        "Unique-Equipped".to_string(),
        Color::WHITE,
    )))
    .chain(
        q.bonuses
            .iter()
            .map(|b| (format!("Equip: {}", b.describe()), TIP_GREEN)),
    )
    .chain(std::iter::once((
        format!("Equip: {}", proc_text(q.key)),
        TIP_GREEN,
    )))
    .collect()
}

/// A piece's hover box: in full, collected or not, with how it stands.
fn tip(q: &Piece, c: &JewelryCollection) -> ArmoryTip {
    let mut lines = lines(q);
    lines.extend(standing(q, c));
    ArmoryTip {
        title: q.name.to_string(),
        lines,
    }
}

/// The Details column: the picked piece in full; else a hint.
fn details(
    p: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    c: &JewelryCollection,
    picked: Option<&'static Piece>,
) {
    let Some(q) = picked else {
        para(
            p,
            ctx,
            "Click a piece to see what it grants and where it drops.",
            13.0,
            DIM,
        );
        return;
    };
    let have = c.has(q.key);
    p.spawn(Node {
        column_gap: Val::Px(8.0),
        align_items: AlignItems::Center,
        flex_shrink: 0.0,
        margin: UiRect::bottom(Val::Px(6.0)),
        ..default()
    })
    .with_children(|h| {
        item_icon(h, q.icon, 40.0, if have { Color::WHITE } else { LOCKED });
        fill_text(h, ctx, q.name, 15.0, q.rarity.color());
    });
    for (line, color) in lines(q) {
        para(p, ctx, &line, 12.0, color);
    }
    gap(p, 6.0);
    for (line, color) in standing(q, c) {
        para(p, ctx, &line, 12.0, color);
    }
    gap(p, 6.0);
    let hint = match (have, c.loadout.slot_of(q.key)) {
        (false, _) => "Get it once and it is yours for good, on every class.",
        (true, Some(_)) => "Click it again to take it off.",
        (true, None) => "Click it to wear it.",
    };
    para(p, ctx, hint, 11.0, DIM);
}

/// A click: show the piece, and wear or remove it when collected; or empty a slot. The tab is
/// rebuilt to show it, and the profile saved.
pub(super) fn input(
    clicks: Res<GlueClicks>,
    actions: Query<(Entity, &JewelryAction)>,
    mut view: ResMut<JewelryView>,
    profile: Option<ResMut<Profile>>,
    mut armory: ResMut<ArmoryState>,
) {
    if !armory.open {
        return;
    }
    let Some(action) = actions
        .iter()
        .find(|(e, _)| clicks.hit(*e))
        .map(|(_, a)| *a)
    else {
        return;
    };
    match profile {
        Some(mut profile) => {
            if act(action, &mut profile.collection, &mut view) {
                profile.mark_dirty();
            }
        }
        None => {
            if let JewelryAction::Piece(key) = action {
                view.piece = Some(key);
            }
        }
    }
    armory.refresh();
}

/// What a click does to the collection and the view: whether the loadout changed.
fn act(action: JewelryAction, c: &mut JewelryCollection, view: &mut JewelryView) -> bool {
    match action {
        JewelryAction::Piece(key) => {
            view.piece = Some(key);
            c.has(key) && c.toggle(key).is_ok()
        }
        JewelryAction::Unequip(slot) => {
            let Some(key) = c.loadout.get(slot).and_then(piece).map(|q| q.key) else {
                return false;
            };
            view.piece = Some(key);
            c.unequip(slot);
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicks_wear_and_remove_collected_pieces_only() {
        let mut c = JewelryCollection::default();
        let mut view = JewelryView::default();
        for key in [
            "hand_of_justice",
            "darkmoon_maelstrom",
            "kiss_of_the_spider",
        ] {
            c.unlock(piece(key).unwrap());
        }
        // A locked piece is only shown.
        assert!(!act(
            JewelryAction::Piece("band_of_accuria"),
            &mut c,
            &mut view
        ));
        assert_eq!(view.piece, Some("band_of_accuria"));
        assert_eq!(c.loadout, Default::default());
        // Collected trinkets fill Trinket 1, then Trinket 2, then replace Trinket 1.
        assert!(act(
            JewelryAction::Piece("hand_of_justice"),
            &mut c,
            &mut view
        ));
        assert!(act(
            JewelryAction::Piece("darkmoon_maelstrom"),
            &mut c,
            &mut view
        ));
        assert!(act(
            JewelryAction::Piece("kiss_of_the_spider"),
            &mut c,
            &mut view
        ));
        assert_eq!(
            c.loadout.get(LoadoutSlot::Trinket1),
            Some("kiss_of_the_spider")
        );
        assert_eq!(
            c.loadout.get(LoadoutSlot::Trinket2),
            Some("darkmoon_maelstrom")
        );
        // A slot clicked empties, showing what it wore; an empty one does nothing.
        assert!(act(
            JewelryAction::Unequip(LoadoutSlot::Trinket2),
            &mut c,
            &mut view
        ));
        assert_eq!(view.piece, Some("darkmoon_maelstrom"));
        assert_eq!(c.loadout.get(LoadoutSlot::Trinket2), None);
        assert!(!act(
            JewelryAction::Unequip(LoadoutSlot::Neck),
            &mut c,
            &mut view
        ));
    }

    #[test]
    fn tips_show_the_proc_and_where_a_locked_piece_drops() {
        let mut c = JewelryCollection::default();
        let q = piece("band_of_accuria").unwrap();
        let locked = tip(q, &c);
        assert_eq!(locked.title, "Band of Accuria");
        assert_eq!(locked.lines[0].0, "Legendary Ring");
        assert!(locked
            .lines
            .iter()
            .any(|(l, _)| l == &format!("Equip: {}", proc_text(q.key))));
        assert!(locked.lines.iter().any(|(l, _)| l == "Drops from Ragnaros"));
        assert!(locked.lines.iter().any(|(l, _)| l == "Unique-Equipped"));
        c.unlock(q);
        c.toggle(q.key).unwrap();
        let worn = tip(q, &c);
        assert_eq!(worn.lines.last().unwrap().0, "Worn: Finger 1");
        assert!(!worn.lines.iter().any(|(l, _)| l.starts_with("Drops from")));
        let dm = tip(piece("darkmoon_heroism").unwrap(), &c);
        assert!(dm
            .lines
            .iter()
            .any(|(l, _)| l == "Darkmoon Faire: complete the Warlords deck"));
    }
}
