//! The Armory Wardrobe: account-wide Faire cosmetics and completed card decks.

use bevy::prelude::*;

use super::{columns, para, text, ArmoryState, Column, TabCtx};
use crate::glue::art::{DIM, GOLD};
use crate::glue::GlueClicks;
use crate::survivors::faire::catalog::{self, Kind};
use crate::survivors::profile::Profile;

#[derive(Component)]
pub(super) struct WardrobeAction {
    kind: Kind,
    key: String,
}

pub(super) fn build(parent: &mut ChildSpawnerCommands, ctx: &TabCtx) {
    let profile = ctx.res::<Profile>();
    columns(parent, ctx, Some("Wardrobe"), false, |col, p| match col {
        Column::Left => {
            text(p, ctx, "Faire Collection", 18.0, GOLD);
            para(p, ctx, "Choose owned tabards, titles and up to three toys. Locked prizes show their Darkmoon Faire source.", 12.0, DIM);
            for suit in catalog::deck_keys() {
                let have = profile.map_or(0, |save| {
                    (1..=8)
                        .filter(|rank| {
                            save.faire
                                .cards
                                .contains(&crate::survivors::profile::slug(&format!(
                                    "{} of {suit}",
                                    [
                                        "Ace", "Two", "Three", "Four", "Five", "Six", "Seven",
                                        "Eight"
                                    ][rank - 1]
                                )))
                        })
                        .count()
                });
                let (name, _, _) = catalog::deck(suit).unwrap();
                text(
                    p,
                    ctx,
                    &format!("{suit}: {have}/8 → {name}"),
                    12.0,
                    if have == 8 { GOLD } else { DIM },
                );
            }
        }
        Column::Center => shelf(
            p,
            ctx,
            profile,
            Kind::Tabard,
            "Tabard (one equipped, or none)",
        ),
        Column::Details => {
            shelf(
                p,
                ctx,
                profile,
                Kind::Title,
                "Title (one equipped, or none)",
            );
            shelf(
                p,
                ctx,
                profile,
                Kind::Toy,
                "Toy bar (up to 3; keys 7 / 8 / 9)",
            );
        }
    });
}

fn shelf(
    p: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    profile: Option<&Profile>,
    kind: Kind,
    heading: &str,
) {
    text(p, ctx, heading, 15.0, GOLD);
    for prize in catalog::catalog().iter().filter(|prize| prize.kind == kind) {
        let owned = profile.is_some_and(|save| catalog::owned(save, prize));
        let selected = profile.is_some_and(|save| match kind {
            Kind::Tabard => save.faire.wardrobe.tabard.as_deref() == Some(&prize.key),
            Kind::Title => save.faire.wardrobe.title.as_deref() == Some(&prize.key),
            Kind::Toy => save.faire.wardrobe.toys.contains(&prize.key),
            _ => false,
        });
        p.spawn((
            Button,
            WardrobeAction {
                kind,
                key: prize.key.clone(),
            },
            Node {
                min_height: Val::Px(20.0),
                padding: UiRect::horizontal(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(if selected {
                Color::srgba(0.3, 0.2, 0.02, 0.8)
            } else {
                Color::NONE
            }),
        ))
        .with_children(|row| {
            let label = if owned {
                prize.name.clone()
            } else {
                format!("{} — locked: Darkmoon Faire", prize.name)
            };
            text(
                row,
                ctx,
                &label,
                12.0,
                if owned { prize.rarity.color() } else { DIM },
            );
        });
    }
}

/// Save a click and rebuild the shelf.  Locked prizes deliberately do nothing.
pub(super) fn input(
    clicks: Res<GlueClicks>,
    actions: Query<(Entity, &WardrobeAction)>,
    mut profile: ResMut<Profile>,
    mut armory: ResMut<ArmoryState>,
) {
    let Some(action) = actions
        .iter()
        .find(|(entity, _)| clicks.hit(*entity))
        .map(|(_, action)| action)
    else {
        return;
    };
    let owned = match action.kind {
        Kind::Tabard => profile.quests.tabards.contains(&action.key),
        Kind::Title => profile.faire.titles.contains(&action.key),
        Kind::Toy => profile.faire.toys.contains(&action.key),
        _ => false,
    };
    if !owned {
        return;
    }
    match action.kind {
        Kind::Tabard => {
            profile.faire.wardrobe.tabard = (profile.faire.wardrobe.tabard.as_deref()
                != Some(&action.key))
            .then(|| action.key.clone())
        }
        Kind::Title => {
            profile.faire.wardrobe.title = (profile.faire.wardrobe.title.as_deref()
                != Some(&action.key))
            .then(|| action.key.clone())
        }
        Kind::Toy => {
            let mut toys = profile.faire.wardrobe.toys.clone();
            if let Some(i) = toys.iter().position(|key| key == &action.key) {
                toys.remove(i);
            } else if toys.len() < 3 {
                toys.push(action.key.clone());
            }
            profile.faire.wardrobe.set_toys(toys);
        }
        _ => {}
    }
    profile.mark_dirty();
    armory.refresh();
}
