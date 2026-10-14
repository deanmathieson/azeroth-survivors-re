//! The moving parts behind the Armory's gear tabs, which only build nodes from the world: item
//! icons filled once the archives are readable, the tabs' clicks, and what each tab has picked.

use bevy::prelude::*;

use super::super::armory::ArmoryState;
use super::super::Selection;
use super::item;
use super::store::GearStore;
use crate::glue::GlueClicks;

/// An item icon slot: `Interface\Icons\<icon>.blp`, drawn in `tint` once it loads.
#[derive(Component)]
pub(in crate::survivors) struct ItemIcon {
    pub icon: &'static str,
    pub tint: Color,
}

#[derive(Component)]
pub(super) struct ItemIconDone;

/// A click in a gear tab.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::survivors) enum GearAction {
    /// Show this piece in the Collection's Details.
    Pick(u16),
    /// Wear this piece.
    Equip(u16),
    /// Show this battleground (an index into [`super::super::data::MAPS`]) in the Drop Tables.
    Zone(usize),
}

/// What the gear tabs have picked.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::survivors) struct GearView {
    /// The Collection's piece in Details.
    pub item: Option<u16>,
    /// The Drop Tables' battleground; `None` is the one picked at the menu.
    pub zone: Option<usize>,
}

/// Fill each [`ItemIcon`] from the client archives, as `fill_icons` fills the HUD's spell icons;
/// a missing file leaves the slot's dark placeholder.
pub(super) fn fill_item_icons(
    mut commands: Commands,
    mut slots: Query<(Entity, &mut ImageNode, &ItemIcon), Without<ItemIconDone>>,
    assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(mut assets) = assets else { return };
    for (e, mut img, icon) in &mut slots {
        commands.entity(e).insert(ItemIconDone);
        let path = format!("Interface\\Icons\\{}.blp", icon.icon);
        let Some(h) = assets.sprite_texture(&path, &mut images) else {
            warn!("survivors gear: no icon {path}");
            continue;
        };
        img.image = h;
        img.color = icon.tint;
    }
}

/// A gear tab's click: pick a piece or a battleground, or wear a collected piece; the open tab
/// is rebuilt to show it.
pub(super) fn gear_clicks(
    clicks: Res<GlueClicks>,
    actions: Query<(Entity, &GearAction)>,
    selection: Res<Selection>,
    mut view: ResMut<GearView>,
    mut store: GearStore,
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
    match action {
        GearAction::Pick(id) => view.item = Some(id),
        GearAction::Zone(map) => view.zone = Some(map),
        GearAction::Equip(id) => {
            let (Some(class), Some(piece)) = (selection.class, item(id)) else {
                return;
            };
            let gear = store.get(class);
            if piece.class != class || !gear.has(id) || gear.wears(id) {
                return;
            }
            store.edit(class, |g| g.equip(piece));
        }
    }
    armory.refresh();
}
