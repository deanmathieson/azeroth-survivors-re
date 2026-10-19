//! The gear's section of each class's profile record, `ClassRecord::gear` in
//! [`crate::survivors::profile`]. Only the gear workstream edits this file.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::super::bonus::Bonuses;
use super::super::professions::recipes::{self, Enchant};
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
    /// Each enchanted piece's enchant: an [`Item::id`] (as a string, TOML's keys) to its
    /// `SpellItemEnchantment` id ([`Enchant`]). A piece or an enchant this build does not know
    /// adds nothing.
    pub enchants: BTreeMap<String, u32>,
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
        enchants: BTreeMap::new(),
    };

    /// The enchant on piece `id`, if it has one this build knows.
    pub fn enchant_of(&self, id: u16) -> Option<&'static Enchant> {
        self.enchants
            .get(&id.to_string())
            .and_then(|&e| recipes::enchant_by_id(e))
    }

    /// Put enchant `enchant` (a `SpellItemEnchantment` id) on piece `id`, over any it had.
    pub fn enchant(&mut self, id: u16, enchant: u32) {
        self.enchants.insert(id.to_string(), enchant);
    }

    /// What the worn pieces' enchants grant, each where its piece is worn in its own slot.
    pub fn enchant_bonuses(&self) -> Bonuses {
        let mut total = Bonuses::default();
        for id in self.equipped_ids() {
            if let Some(e) = self.enchant_of(id) {
                if item(id).is_some_and(|i| i.slot == e.slot) {
                    total.add_all(e.bonuses);
                }
            }
        }
        total
    }

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

    /// An enchant joins the worn piece's bonuses, survives a save, and adds nothing on a piece in
    /// another slot or off.
    #[test]
    fn enchants_join_the_worn_bonuses_and_save() {
        use super::super::super::bonus::StatKey;
        let chest = piece(0, Slot::Chest, Rarity::Rare).unwrap();
        let mut gear = GearSave::default();
        gear.collect(chest);
        assert_eq!(gear.enchant_bonuses(), Bonuses::default());
        // Enchant Chest - Major Health.
        let major = recipes::recipe(20026).and_then(|r| r.enchant).unwrap();
        gear.enchant(chest.id, major.id);
        assert!(gear.enchant_bonuses().get(StatKey::MaxHealth) > 0.0);
        let text = toml::to_string_pretty(&gear).unwrap();
        assert!(text.contains("[enchants]"), "{text}");
        let back: GearSave = toml::from_str(&text).unwrap();
        assert_eq!(back, gear);
        assert_eq!(back.enchant_of(chest.id), Some(major));
        // A wrist enchant on the chest does nothing; an unknown one neither.
        let wrist = recipes::recipe(7418).and_then(|r| r.enchant).unwrap();
        gear.enchant(chest.id, wrist.id);
        assert_eq!(gear.enchant_bonuses(), Bonuses::default());
        gear.enchant(chest.id, 999_999);
        assert_eq!(gear.enchant_of(chest.id), None);
        // Off, it adds nothing.
        gear.enchant(chest.id, major.id);
        gear.equipped.clear();
        assert_eq!(gear.enchant_bonuses(), Bonuses::default());
        // A save from before enchants reads as none.
        let old: GearSave = toml::from_str("collected = [108]").unwrap();
        assert!(old.enchants.is_empty());
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
