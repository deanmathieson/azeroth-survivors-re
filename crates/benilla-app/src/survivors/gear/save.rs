//! The gear's section of each class's profile record, `ClassRecord::gear` in
//! [`crate::survivors::profile`]. Only the gear workstream edits this file.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::looks::HeroLooks;
use super::{item, Item, Slot};

/// One class's saved gear. Every field must read a save without it as its default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(in crate::survivors) struct GearSave {
    /// Every piece the class has collected, by [`Item::id`].
    pub collected: BTreeSet<u16>,
    /// What each slot wears: a [`Slot::key`] to an [`Item::id`]. Keyed by string, so a slot this
    /// build does not know is skipped rather than failing the whole profile.
    pub equipped: BTreeMap<String, u16>,
    /// The class hero's name and features ([`super::looks`]); a save from before them reads as no
    /// name and the class trainer's features.
    pub looks: HeroLooks,
}

/// What collecting a piece came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum Collected {
    /// A new piece, and whether it went straight on ([`upgrades`]).
    New { equipped: bool },
    /// Already had it.
    Duplicate,
}

/// Whether a newly collected `new` goes on over `worn`: always into an empty slot, otherwise only
/// over a lower rarity.
pub(in crate::survivors) fn upgrades(worn: Option<&Item>, new: &Item) -> bool {
    worn.is_none_or(|w| (w.rarity as u8) < (new.rarity as u8))
}

impl GearSave {
    /// No pieces and nothing worn, for a class without a record.
    pub const EMPTY: Self = Self {
        collected: BTreeSet::new(),
        equipped: BTreeMap::new(),
        looks: HeroLooks::EMPTY,
    };

    pub fn has(&self, id: u16) -> bool {
        self.collected.contains(&id)
    }

    /// The piece `slot` wears: a known, collected piece of that slot, else nothing.
    pub fn worn(&self, slot: Slot) -> Option<&'static Item> {
        let id = *self.equipped.get(slot.key())?;
        item(id).filter(|i| i.slot == slot && self.has(id))
    }

    /// Whether `id` is what its slot wears.
    pub fn wears(&self, id: u16) -> bool {
        item(id).is_some_and(|i| self.worn(i.slot).is_some_and(|w| w.id == id))
    }

    /// The worn pieces' ids, in [`Slot::ALL`] order.
    pub fn equipped_ids(&self) -> Vec<u16> {
        Slot::ALL
            .iter()
            .filter_map(|&s| self.worn(s))
            .map(|i| i.id)
            .collect()
    }

    /// Put `piece` on in its slot.
    pub fn equip(&mut self, piece: &Item) {
        self.equipped.insert(piece.slot.key().to_string(), piece.id);
    }

    /// Add `piece` to the collection; a new one goes on when it [`upgrades`] its slot.
    pub fn collect(&mut self, piece: &Item) -> Collected {
        if !self.collected.insert(piece.id) {
            return Collected::Duplicate;
        }
        let equipped = upgrades(self.worn(piece.slot), piece);
        if equipped {
            self.equip(piece);
        }
        Collected::New { equipped }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{items_for, piece, Rarity};
    use super::*;

    #[test]
    fn toml_round_trip() {
        let mut gear = GearSave::default();
        for p in &items_for(0)[..12] {
            gear.collect(p);
        }
        let text = toml::to_string_pretty(&gear).unwrap();
        assert!(text.contains("main_hand"), "{text}");
        assert_eq!(toml::from_str::<GearSave>(&text).unwrap(), gear);
    }

    #[test]
    fn old_and_odd_saves_read() {
        // A save from before gear, and one with only part of the section.
        assert_eq!(toml::from_str::<GearSave>("").unwrap(), GearSave::default());
        let part: GearSave = toml::from_str("collected = [118, 108]").unwrap();
        assert!(part.has(108) && part.has(118));
        assert!(part.equipped_ids().is_empty());
        // A slot this build does not know, a piece that is not in that slot or not collected, and
        // an unknown id are all skipped.
        let odd: GearSave = toml::from_str(
            "collected = [108, 118, 9999]\n[equipped]\nback = 108\nhead = 108\n\
             main_hand = 118\nchest = 9999\nlegs = 104\n",
        )
        .unwrap();
        assert_eq!(odd.equipped_ids(), vec![118]);
        assert!(odd.wears(118) && !odd.wears(108));
    }

    #[test]
    fn new_pieces_go_on_over_lower_rarities_only() {
        let head = |r| piece(0, Slot::Head, r).unwrap();
        let mut gear = GearSave::default();
        // Into an empty slot.
        assert_eq!(
            gear.collect(head(Rarity::Rare)),
            Collected::New { equipped: true }
        );
        // Not over a higher one, though it is collected.
        assert_eq!(
            gear.collect(head(Rarity::Uncommon)),
            Collected::New { equipped: false }
        );
        assert_eq!(gear.worn(Slot::Head).unwrap().rarity, Rarity::Rare);
        // Over a lower one.
        assert_eq!(
            gear.collect(head(Rarity::Epic)),
            Collected::New { equipped: true }
        );
        assert_eq!(gear.worn(Slot::Head).unwrap().rarity, Rarity::Epic);
        // Twice is a duplicate, and changes nothing.
        let before = gear.clone();
        assert_eq!(gear.collect(head(Rarity::Rare)), Collected::Duplicate);
        assert_eq!(gear, before);
        // A hand-picked lower piece stays on until something better drops.
        gear.equip(head(Rarity::Uncommon));
        assert_eq!(gear.worn(Slot::Head).unwrap().rarity, Rarity::Uncommon);
        assert!(upgrades(gear.worn(Slot::Head), head(Rarity::Rare)));
        assert!(!upgrades(gear.worn(Slot::Head), head(Rarity::Common)));
        assert!(upgrades(None, head(Rarity::Common)));
    }
}
