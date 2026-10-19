//! Where each class's [`GearSave`] lives: its profile record, saved with the profile, or, while a
//! [`Shadow`] is up, an in-memory copy that is never saved. Every read and write of gear goes
//! through [`GearStore`], [`GearRead`] or [`class_gear`], so the two never mix. The account's
//! [`RaidVault`] lives beside it the same way.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::super::data::CLASSES;
use super::super::profile::{slug, Profile};
use super::raid::{self, Kept, RaidKind, RaidPiece, RaidVault};
use super::{items_for, GearSave};

/// Gear kept in memory only: the `WOW_SURVIVORS_GEAR` screenshot aid, or an auto-runner run the
/// profile does not record. Filled from the profile on first use; from then on all gear reads and
/// writes come here, and nothing of it is saved.
#[derive(Resource, Default)]
pub(super) struct Shadow {
    /// How many of each class's pieces, in catalog order, are added on filling.
    extra: usize,
    /// Per class, in [`CLASSES`] order; `None` until filled.
    classes: Option<Vec<GearSave>>,
    /// The account's raid vault, filled with the classes.
    vault: Option<RaidVault>,
}

impl Shadow {
    pub(super) fn with_extra(extra: usize) -> Self {
        Self {
            extra,
            classes: None,
            vault: None,
        }
    }

    /// Copy the saved gear in, plus the first `extra` pieces of each class, each collected (and
    /// so worn by the auto-equip rule) in catalog order: Common first, Legendary last.
    fn fill(&mut self, profile: Option<&Profile>) {
        if self.classes.is_some() {
            return;
        }
        let classes = (0..CLASSES.len())
            .map(|class| {
                let mut gear = saved(profile, class).clone();
                for piece in items_for(class).iter().take(self.extra) {
                    gear.collect(piece);
                }
                gear
            })
            .collect();
        self.classes = Some(classes);
        self.vault = Some(profile.map_or_else(RaidVault::default, |p| p.raid.clone()));
    }
}

/// `WOW_SURVIVORS_GEAR`'s value: `all`, or how many of each class's pieces to add.
pub(super) fn dev_count(text: &str) -> Option<usize> {
    let text = text.trim();
    if text.eq_ignore_ascii_case("all") {
        return Some(usize::MAX);
    }
    text.parse().ok()
}

/// Nothing collected, for a class with no record.
static NONE: GearSave = GearSave::EMPTY;

/// Class `class`'s gear as saved in `profile`.
fn saved(profile: Option<&Profile>, class: usize) -> &GearSave {
    CLASSES
        .get(class)
        .and_then(|c| profile?.classes.get(&slug(c.name)))
        .map_or(&NONE, |r| &r.gear)
}

/// Class `class`'s gear: the shadow's once filled, else the profile's.
fn gear_of<'a>(
    profile: Option<&'a Profile>,
    shadow: Option<&'a Shadow>,
    class: usize,
) -> &'a GearSave {
    match shadow.and_then(|s| s.classes.as_ref()) {
        Some(classes) => classes.get(class).unwrap_or(&NONE),
        None => saved(profile, class),
    }
}

/// Nothing kept, for a world without a profile.
static NO_VAULT: RaidVault = RaidVault::EMPTY;

/// The account's raid vault: the shadow's once filled, else the profile's.
fn vault_of<'a>(profile: Option<&'a Profile>, shadow: Option<&'a Shadow>) -> &'a RaidVault {
    match shadow.and_then(|s| s.vault.as_ref()) {
        Some(vault) => vault,
        None => profile.map_or(&NO_VAULT, |p| &p.raid),
    }
}

/// The account's raid vault, read straight from the world.
pub(in crate::survivors) fn raid_vault(world: &World) -> &RaidVault {
    vault_of(
        world.get_resource::<Profile>(),
        world.get_resource::<Shadow>(),
    )
}

/// Class `class`'s gear, read straight from the world (the Armory's tab builders).
pub(in crate::survivors) fn class_gear(world: &World, class: usize) -> &GearSave {
    gear_of(
        world.get_resource::<Profile>(),
        world.get_resource::<Shadow>(),
        class,
    )
}

/// Read access to every class's gear.
#[derive(SystemParam)]
pub(in crate::survivors) struct GearRead<'w> {
    profile: Option<Res<'w, Profile>>,
    shadow: Option<Res<'w, Shadow>>,
}

impl GearRead<'_> {
    pub(in crate::survivors) fn get(&self, class: usize) -> &GearSave {
        gear_of(self.profile.as_deref(), self.shadow.as_deref(), class)
    }

    /// Whether any class's gear may have changed since the system last ran.
    pub(super) fn is_changed(&self) -> bool {
        self.profile.as_ref().is_some_and(|p| p.is_changed())
            || self.shadow.as_ref().is_some_and(|s| s.is_changed())
    }
}

/// Read and write access to every class's gear.
#[derive(SystemParam)]
pub(super) struct GearStore<'w> {
    profile: Option<ResMut<'w, Profile>>,
    shadow: Option<ResMut<'w, Shadow>>,
}

impl GearStore<'_> {
    pub(super) fn get(&self, class: usize) -> &GearSave {
        gear_of(self.profile.as_deref(), self.shadow.as_deref(), class)
    }

    /// Change class `class`'s gear with `f`. On the profile the change is queued for saving; in
    /// a shadow it stays in memory. `None`, and `f` unrun, for an unknown class.
    pub(super) fn edit<R>(
        &mut self,
        class: usize,
        f: impl FnOnce(&mut GearSave) -> R,
    ) -> Option<R> {
        let name = CLASSES.get(class)?.name;
        if let Some(shadow) = self.shadow.as_deref_mut() {
            shadow.fill(self.profile.as_deref());
            return shadow.classes.as_mut()?.get_mut(class).map(f);
        }
        let profile = self.profile.as_deref_mut()?;
        let out = f(&mut profile.classes.entry(slug(name)).or_default().gear);
        profile.mark_dirty();
        Some(out)
    }

    /// Change the account's raid vault with `f`, as [`GearStore::edit`] changes gear. `None`, and
    /// `f` unrun, without a profile.
    pub(super) fn edit_vault<R>(&mut self, f: impl FnOnce(&mut RaidVault) -> R) -> Option<R> {
        if let Some(shadow) = self.shadow.as_deref_mut() {
            shadow.fill(self.profile.as_deref());
            return shadow.vault.as_mut().map(f);
        }
        let profile = self.profile.as_deref_mut()?;
        let out = f(&mut profile.raid);
        profile.mark_dirty();
        Some(out)
    }

    /// Keep raid piece `piece`, won by a player of class `class`: gear in its keeper's class
    /// ([`raid::keeper`]), jewelry and quest items in the vault. Then, with an Eye and an Ingot
    /// held, the forge lights and Sulfuras joins the gear of every class it is for: the second
    /// value says it did.
    pub(super) fn keep_raid(&mut self, piece: &RaidPiece, class: usize) -> (Kept, bool) {
        let kept = match piece.kind {
            RaidKind::Gear => {
                let to = raid::keeper(piece, class);
                if self.get(to).has(piece.item.id) {
                    Kept::Duplicate
                } else {
                    let mut unused = RaidVault::default();
                    self.edit(to, |g| raid::keep(piece, Some(g), &mut unused))
                        .unwrap_or(Kept::Duplicate)
                }
            }
            _ => self
                .edit_vault(|v| raid::keep(piece, None, v))
                .unwrap_or(Kept::Duplicate),
        };
        let forged =
            matches!(kept, Kept::Counted(_)) && self.edit_vault(raid::forge).unwrap_or(false);
        if forged {
            if let Some(sulfuras) = raid::raid_piece(raid::SULFURAS) {
                for &c in raid::SULFURAS_CLASSES {
                    self.edit(c, |g| g.collect(&sulfuras.item));
                }
            }
        }
        (kept, forged)
    }
}

/// A shadow fills as soon as the profile is in, so the screens show its extra pieces from the
/// start.
pub(super) fn fill_shadow(mut shadow: ResMut<Shadow>, profile: Option<Res<Profile>>) {
    if shadow.classes.is_none() {
        shadow.fill(profile.as_deref());
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::super::{piece, Rarity, Slot};
    use super::*;

    #[test]
    fn dev_counts_read() {
        assert_eq!(dev_count("all"), Some(usize::MAX));
        assert_eq!(dev_count(" ALL "), Some(usize::MAX));
        assert_eq!(dev_count("12"), Some(12));
        assert_eq!(dev_count("some"), None);
    }

    #[test]
    fn profile_edits_are_saved_and_shadow_edits_are_not() {
        let rare_head = piece(1, Slot::Head, Rarity::Rare).unwrap();
        let mut world = World::new();
        world.insert_resource(Profile::default());
        world
            .run_system_once(move |mut store: GearStore| {
                store.edit(1, |g| g.collect(rare_head));
            })
            .unwrap();
        let profile = world.resource::<Profile>();
        assert!(class_gear(&world, 1).has(rare_head.id));
        assert!(profile.classes[&slug(CLASSES[1].name)]
            .gear
            .has(rare_head.id));

        // Under a shadow of the first nine (the Commons), the profile's piece is kept, the
        // Commons are added and worn, and nothing reaches the profile.
        world.insert_resource(Shadow::with_extra(9));
        world.run_system_once(fill_shadow).unwrap();
        let epic_head = piece(1, Slot::Head, Rarity::Epic).unwrap();
        world
            .run_system_once(move |mut store: GearStore| {
                store.edit(1, |g| g.collect(epic_head));
            })
            .unwrap();
        let shown = class_gear(&world, 1);
        assert!(shown.has(rare_head.id) && shown.has(epic_head.id));
        assert_eq!(shown.collected.len(), 11);
        assert_eq!(shown.worn(Slot::Head).map(|i| i.id), Some(epic_head.id));
        assert_eq!(
            shown.worn(Slot::Chest).map(|i| i.rarity),
            Some(Rarity::Common)
        );
        let saved = &world.resource::<Profile>().classes[&slug(CLASSES[1].name)].gear;
        assert_eq!(saved.collected.len(), 1);
        // Every other class gets its Commons too.
        assert_eq!(class_gear(&world, 0).equipped_ids().len(), 9);
    }
}
