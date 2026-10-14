//! The Armory's Codex tab: every piece of jewelry ever offered in a run, per account. The center
//! is a grid per slot type, rings, trinkets and necklaces, Uncommon first: a piece offered at
//! least once shows its icon edged in its rarity's colour (badged with how often it was taken),
//! one never offered a dim "?". The left column counts what has been seen and taken; clicking a
//! seen piece shows under Details what it grants and how many times it was taken.

use bevy::prelude::*;
use bevy::ui_render::ui_material::MaterialNode;

use super::collection::progress_bar;
use super::{columns, para, text, ArmoryTip, Column, TabCtx};
use crate::glue::art::{DIM, GOLD};
use crate::glue::widgets::{outlined_text, overlay, GlueText, Hilight};
use crate::survivors::gear::{rarity_label, ItemIcon, RARITIES};
use crate::survivors::jewelry::{piece, pieces, CodexAction, CodexSave, CodexView, Kind, Piece};
use crate::survivors::profile::Profile;

/// `GREEN_FONT_COLOR`: what a piece grants, and how often it was taken.
const TIP_GREEN: Color = Color::srgb(0.1, 1.0, 0.1);
const CELL_FILL: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);
const CELL_EDGE: Color = Color::srgb(0.3, 0.3, 0.3);
const CELL: f32 = 38.0;

/// Nothing seen, for a world without a profile.
static EMPTY: CodexSave = CodexSave {
    seen: std::collections::BTreeSet::new(),
    taken: std::collections::BTreeMap::new(),
};

pub(super) fn build(parent: &mut ChildSpawnerCommands, ctx: &TabCtx) {
    let codex = ctx.res::<Profile>().map_or(&EMPTY, |p| &p.codex);
    let picked = ctx.res::<CodexView>().and_then(|v| v.piece).and_then(piece);
    columns(parent, ctx, Some("Codex"), false, |col, p| match col {
        Column::Left => tally(p, ctx, codex),
        Column::Center => grid(p, ctx, codex, picked),
        Column::Details => details(p, ctx, codex, picked),
    });
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

fn gap(p: &mut ChildSpawnerCommands, h: f32) {
    p.spawn(Node {
        height: Val::Px(h),
        flex_shrink: 0.0,
        ..default()
    });
}

/// The left column: pieces seen in all, by slot type and by rarity, and what was taken.
fn tally(p: &mut ChildSpawnerCommands, ctx: &TabCtx, codex: &CodexSave) {
    let (seen, of) = codex.seen_of(pieces().iter());
    p.spawn(Node {
        justify_content: JustifyContent::Center,
        margin: UiRect::vertical(Val::Px(4.0)),
        ..default()
    })
    .with_children(|c| {
        text(c, ctx, &format!("Seen {seen} / {of}"), 20.0, Color::WHITE);
    });
    progress_bar(p, seen, of);
    gap(p, 10.0);
    for kind in Kind::ALL {
        let (seen, of) = codex.seen_of(pieces().iter().filter(|q| q.kind == kind));
        row(p, ctx, kind.plural(), GOLD, &format!("{seen} / {of}"));
    }
    gap(p, 6.0);
    for rarity in &RARITIES[1..] {
        let (seen, of) = codex.seen_of(pieces().iter().filter(|q| q.rarity == *rarity));
        row(
            p,
            ctx,
            rarity_label(*rarity),
            rarity.color(),
            &format!("{seen} / {of}"),
        );
    }
    gap(p, 6.0);
    let kinds = pieces()
        .iter()
        .filter(|q| codex.times_taken(q.key) > 0)
        .count();
    let times: u32 = pieces().iter().map(|q| codex.times_taken(q.key)).sum();
    row(p, ctx, "Pieces taken", GOLD, &kinds.to_string());
    row(p, ctx, "Times taken", GOLD, &times.to_string());
    p.spawn(Node {
        flex_grow: 1.0,
        ..default()
    });
    para(
        p,
        ctx,
        "Caches and quest turn-ins offer jewelry during a run. Every piece offered is logged here. \
         Press C in a run to see what you wear.",
        11.0,
        DIM,
    );
}

/// The center: a section per slot type, its pieces in a wrapping row, Uncommon first.
fn grid(p: &mut ChildSpawnerCommands, ctx: &TabCtx, codex: &CodexSave, picked: Option<&Piece>) {
    for kind in Kind::ALL {
        let (seen, of) = codex.seen_of(pieces().iter().filter(|q| q.kind == kind));
        p.spawn(Node {
            flex_shrink: 0.0,
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Baseline,
            margin: UiRect::new(Val::ZERO, Val::ZERO, Val::Px(8.0), Val::Px(4.0)),
            ..default()
        })
        .with_children(|h| {
            text(h, ctx, kind.plural(), 14.0, GOLD);
            text(h, ctx, &format!("{seen} / {of} seen"), 11.0, DIM);
        });
        p.spawn(Node {
            flex_shrink: 0.0,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(4.0),
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|r| {
            for q in pieces().iter().filter(|q| q.kind == kind) {
                cell(r, ctx, q, codex, picked.is_some_and(|p| p.key == q.key));
            }
        });
    }
}

/// One piece: its icon once seen, else a dim "?"; edged in its rarity's colour (gold while
/// picked), and badged with how often it was taken.
fn cell(
    r: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    q: &'static Piece,
    codex: &CodexSave,
    picked: bool,
) {
    let seen = codex.saw(q.key);
    let edge = if picked {
        GOLD
    } else if seen {
        q.rarity.color()
    } else {
        CELL_EDGE
    };
    r.spawn((
        Button,
        CodexAction::Pick(q.key),
        tip(q, codex),
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
        if seen {
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
                ItemIcon {
                    icon: q.icon,
                    tint: Color::WHITE,
                },
            ));
        } else {
            text(b, ctx, "?", 18.0, DIM);
        }
        let taken = codex.times_taken(q.key);
        if taken > 0 {
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
                    text: &taken.to_string(),
                    size: 11.0,
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

/// How a piece stands in the record.
fn standing(q: &Piece, codex: &CodexSave) -> (String, Color) {
    match (codex.saw(q.key), codex.times_taken(q.key)) {
        (false, _) => ("Not yet offered".to_string(), DIM),
        (true, 0) => ("Offered, never taken".to_string(), Color::WHITE),
        (true, 1) => ("Taken once".to_string(), TIP_GREEN),
        (true, n) => (format!("Taken {n} times"), TIP_GREEN),
    }
}

/// A piece's hover box: in full once seen, a mystery until then.
fn tip(q: &Piece, codex: &CodexSave) -> ArmoryTip {
    let status = standing(q, codex);
    if !codex.saw(q.key) {
        return ArmoryTip {
            title: "Unknown piece".to_string(),
            lines: vec![(q.kind.label().to_string(), Color::WHITE), status],
        };
    }
    let lines = std::iter::once((
        format!("{} {}", rarity_label(q.rarity), q.kind.label()),
        q.rarity.color(),
    ))
    .chain(q.bonuses.iter().map(|b| (b.describe(), TIP_GREEN)))
    .chain(std::iter::once(status))
    .collect();
    ArmoryTip {
        title: q.name.to_string(),
        lines,
    }
}

/// The Details column: the picked piece, in full once seen; else a hint.
fn details(p: &mut ChildSpawnerCommands, ctx: &TabCtx, codex: &CodexSave, picked: Option<&Piece>) {
    let Some(q) = picked else {
        para(
            p,
            ctx,
            "Click a piece to see what it grants and how often you took it.",
            13.0,
            DIM,
        );
        return;
    };
    let seen = codex.saw(q.key);
    p.spawn(Node {
        column_gap: Val::Px(8.0),
        align_items: AlignItems::Center,
        flex_shrink: 0.0,
        margin: UiRect::bottom(Val::Px(6.0)),
        ..default()
    })
    .with_children(|h| {
        if seen {
            super::collection::item_icon(h, q.icon, 40.0, Color::WHITE);
        } else {
            h.spawn((
                Node {
                    width: Val::Px(40.0),
                    height: Val::Px(40.0),
                    flex_shrink: 0.0,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(CELL_EDGE),
                BackgroundColor(CELL_FILL),
            ))
            .with_children(|b| {
                text(b, ctx, "?", 20.0, DIM);
            });
        }
        let (name, color) = if seen {
            (q.name, q.rarity.color())
        } else {
            ("Unknown piece", DIM)
        };
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
                text: name,
                size: 15.0,
                color,
                wrap: true,
            },
            ctx.font,
            1.0,
        );
    });
    let (status, color) = standing(q, codex);
    if !seen {
        para(p, ctx, q.kind.label(), 12.0, Color::WHITE);
        para(p, ctx, &status, 12.0, color);
        gap(p, 6.0);
        para(
            p,
            ctx,
            "Open caches and turn in quests during a run to be offered it.",
            12.0,
            DIM,
        );
        return;
    }
    para(
        p,
        ctx,
        &format!("{} {}", rarity_label(q.rarity), q.kind.label()),
        12.0,
        Color::WHITE,
    );
    para(p, ctx, &status, 12.0, color);
    gap(p, 6.0);
    for b in q.bonuses {
        para(p, ctx, &b.describe(), 12.0, TIP_GREEN);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tips_keep_unseen_pieces_a_mystery() {
        let q = piece("hand_of_justice").unwrap();
        let mut codex = CodexSave::default();
        let unseen = tip(q, &codex);
        assert_eq!(unseen.title, "Unknown piece");
        assert!(unseen.lines.iter().all(|(l, _)| !l.contains('%')));
        assert_eq!(standing(q, &codex).0, "Not yet offered");
        codex.see(q.key);
        let seen = tip(q, &codex);
        assert_eq!(seen.title, "Hand of Justice");
        assert_eq!(seen.lines[0].0, "Rare Trinket");
        assert_eq!(seen.lines.len(), q.bonuses.len() + 2);
        assert_eq!(seen.lines.last().unwrap().0, "Offered, never taken");
        codex.take(q.key);
        codex.take(q.key);
        assert_eq!(standing(q, &codex).0, "Taken 2 times");
    }
}
