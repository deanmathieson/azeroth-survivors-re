//! Crafted gear (docs/PROFESSIONS.md, 3.4, 5.2, 6.1): the pieces Blacksmithing, Leatherworking and
//! Tailoring make, from the copper ladder to the Lionheart Helm, the Devilsaur pair, the Robe of
//! the Archmage and Molten Core's plate. Each is first a bag item (its real 1.12 entry,
//! [`CraftedPiece::real`], **Binds when equipped**), traded or sold until it is put on: "Equip"
//! (a right-click in the bags) checks the picked class may wear it, adds it to that class's
//! collection under its own id and puts it on, and takes the bag item away ([`equip`]). From then
//! on it is ordinary collected gear, shown in the Armory's Crafted rows and on the hero.
//!
//! **Ids** are a block of their own from [`FIRST_ID`], clear of the class catalog's and the raid's:
//! stable save keys, never computed.
//!
//! **Authored:** which classes may wear each piece (by its armour and what it is for: the robes
//! are their class's alone, as in 1.12), and its bonuses, scaled from the class catalog's piece
//! of the same slot and rarity so a crafted piece never outdoes the boss piece it sits beside
//! ([`tests::survivors_crafted_never_beats_the_boss_piece`], 6.1). **Not verifiable:** the display
//! ids and icons, from memory like the rest of the gear: each display is an `ItemDisplayInfo.dbc`
//! row carrying the piece's icon and the parts its slot draws
//! ([`tests::crafted_displays_and_icons_exist_in_the_client`]).

use super::super::bonus::{Bonus, School, StatKey};
use super::super::data::CLASSES;
use super::super::professions::items::*;
use super::save::GearSave;
use super::Item;
use super::Rarity::{self, *};
use super::Slot::{self, *};

/// The first id of the crafted block.
pub(in crate::survivors) const FIRST_ID: u16 = 3000;

/// One crafted piece.
#[derive(Debug)]
pub(in crate::survivors) struct CraftedPiece {
    pub item: Item,
    /// The bag item it is made as and equipped from: its 1.12 entry.
    pub real: u32,
    /// The classes that may wear it; `item.class` is the first.
    pub classes: &'static [usize],
}

const fn crafted(
    id: u16,
    real: u32,
    classes: &'static [usize],
    slot: Slot,
    rarity: Rarity,
    name: &'static str,
    display_id: u32,
    icon: &'static str,
    bonuses: &'static [Bonus],
) -> CraftedPiece {
    CraftedPiece {
        item: Item {
            id,
            class: classes[0],
            slot,
            rarity,
            name,
            display_id,
            icon,
            bonuses,
        },
        real,
        classes,
    }
}

// Shorthands, so each piece fits a line.
const fn dmg(v: f32) -> Bonus {
    Bonus::new(StatKey::Damage, v)
}
const fn holy(v: f32) -> Bonus {
    Bonus::school(School::Holy, v)
}
const fn fire(v: f32) -> Bonus {
    Bonus::school(School::Fire, v)
}
const fn frost(v: f32) -> Bonus {
    Bonus::school(School::Frost, v)
}
const fn arcane(v: f32) -> Bonus {
    Bonus::school(School::Arcane, v)
}
const fn crit(v: f32) -> Bonus {
    Bonus::new(StatKey::CritChance, v)
}
const fn critd(v: f32) -> Bonus {
    Bonus::new(StatKey::CritDamage, v)
}
const fn cdr(v: f32) -> Bonus {
    Bonus::new(StatKey::Cooldown, v)
}
const fn area(v: f32) -> Bonus {
    Bonus::new(StatKey::Area, v)
}
const fn hp(v: f32) -> Bonus {
    Bonus::new(StatKey::MaxHealth, v)
}
const fn armor(v: f32) -> Bonus {
    Bonus::new(StatKey::Armor, v)
}
const fn speed(v: f32) -> Bonus {
    Bonus::new(StatKey::MoveSpeed, v)
}
const fn regen(v: f32) -> Bonus {
    Bonus::new(StatKey::Regen, v)
}
const fn pickup(v: f32) -> Bonus {
    Bonus::new(StatKey::Pickup, v)
}

/// Every crafted piece, by bag item entry. Plate is the Warrior's and the Paladin's; mail adds the
/// Hunter and the Shaman; leather is the Hunter's, the Rogue's and the Druid's; cloth the
/// casters'; one-hand weapons every melee class's, two-handers all but the Rogue's.
#[rustfmt::skip]
pub(in crate::survivors) static CRAFTED: &[CraftedPiece] = &[
    crafted(3000, EMBOSSED_LEATHER_VEST, &[2, 3, 8], Chest, Common, "Embossed Leather Vest", 8664, "INV_Chest_Leather_07", &[dmg(0.0085), hp(3.4)]),
    crafted(3001, HANDSTITCHED_LEATHER_BOOTS, &[2, 3, 8], Feet, Common, "Handstitched Leather Boots", 2984, "INV_Boots_01", &[speed(0.0034), dmg(0.0076)]),
    crafted(3002, HANDSTITCHED_LEATHER_PANTS, &[2, 3, 8], Legs, Common, "Handstitched Leather Pants", 683, "INV_Pants_02", &[dmg(0.0085), crit(0.0026)]),
    crafted(3003, DARK_LEATHER_BOOTS, &[2, 3, 8], Feet, Uncommon, "Dark Leather Boots", 2094, "INV_Boots_05", &[speed(0.0051), dmg(0.0127)]),
    crafted(3004, DARK_LEATHER_TUNIC, &[2, 3, 8], Chest, Uncommon, "Dark Leather Tunic", 14781, "INV_Chest_Leather_03", &[dmg(0.017), hp(5.1)]),
    crafted(3005, BROWN_LINEN_VEST, &[4, 6, 7, 8], Chest, Common, "Brown Linen Vest", 34726, "INV_Chest_Cloth_24", &[frost(0.012), hp(3.2)]),
    crafted(3006, WOOLEN_BOOTS, &[4, 6, 7, 8], Feet, Common, "Woolen Boots", 34262, "INV_Boots_Cloth_01", &[speed(0.0032), frost(0.012)]),
    crafted(3007, COPPER_MACE, &[0, 1, 2, 3, 5], MainHand, Common, "Copper Mace", 860, "INV_Mace_01", &[dmg(0.0127), critd(0.0127)]),
    crafted(3008, BRONZE_AXE, &[0, 1, 2, 3, 5], MainHand, Common, "Bronze Axe", 1415, "INV_Axe_02", &[dmg(0.0127), critd(0.0127)]),
    crafted(3009, COPPER_CHAIN_BELT, &[0, 1, 2, 5], Waist, Common, "Copper Chain Belt", 4501, "INV_Belt_03", &[dmg(0.0068), speed(0.0026)]),
    crafted(3010, COPPER_CHAIN_PANTS, &[0, 1, 2, 5], Legs, Common, "Copper Chain Pants", 685, "INV_Pants_03", &[dmg(0.0085), crit(0.0026)]),
    crafted(3011, COPPER_BRACERS, &[0, 1, 2, 5], Wrist, Common, "Copper Bracers", 3620, "INV_Bracer_07", &[crit(0.0017), dmg(0.0059)]),
    crafted(3012, ROUGH_BRONZE_LEGGINGS, &[0, 1, 2, 5], Legs, Common, "Rough Bronze Leggings", 685, "INV_Pants_03", &[dmg(0.0085), crit(0.0026)]),
    crafted(3013, GREEN_IRON_BRACERS, &[0, 1, 2, 5], Wrist, Uncommon, "Green Iron Bracers", 3621, "INV_Bracer_03", &[crit(0.0034), dmg(0.0085)]),
    crafted(3014, GREEN_IRON_HELM, &[0, 1, 2, 5], Head, Uncommon, "Green Iron Helm", 16083, "INV_Helmet_25", &[dmg(0.017), crit(0.0051)]),
    crafted(3015, HARDENED_IRON_SHORTSWORD, &[0, 1, 2, 3, 5], MainHand, Uncommon, "Hardened Iron Shortsword", 5149, "INV_Sword_23", &[dmg(0.0213), critd(0.0213)]),
    crafted(3016, HANDSTITCHED_LEATHER_BELT, &[2, 3, 8], Waist, Common, "Handstitched Leather Belt", 4520, "INV_Belt_04", &[dmg(0.0068), speed(0.0026)]),
    crafted(3017, EMBOSSED_LEATHER_GLOVES, &[2, 3, 8], Hands, Common, "Embossed Leather Gloves", 972, "INV_Gauntlets_05", &[crit(0.0026), dmg(0.0076)]),
    crafted(3018, HEAVY_LINEN_GLOVES, &[4, 6, 7, 8], Hands, Common, "Heavy Linen Gloves", 1063, "INV_Gauntlets_04", &[crit(0.0024), frost(0.012)]),
    crafted(3019, HEAVY_WOOLEN_GLOVES, &[4, 6, 7, 8], Hands, Common, "Heavy Woolen Gloves", 1063, "INV_Gauntlets_04", &[crit(0.0024), frost(0.012)]),
    crafted(3020, DOUBLE_STITCHED_WOOLEN_SHOULDERS, &[4, 6, 7, 8], Shoulder, Common, "Double-stitched Woolen Shoulders", 4146, "INV_Shoulder_09", &[frost(0.012), area(0.0056)]),
    crafted(3021, HEAVY_WOOLEN_PANTS, &[4, 6, 7, 8], Legs, Common, "Heavy Woolen Pants", 32924, "INV_Pants_Cloth_05", &[frost(0.012), crit(0.0024)]),
    crafted(3022, BARBARIC_HARNESS, &[2, 3, 8], Chest, Uncommon, "Barbaric Harness", 8649, "INV_Chest_Leather_08", &[dmg(0.017), hp(5.1)]),
    crafted(3023, HANDSTITCHED_LEATHER_VEST, &[2, 3, 8], Chest, Common, "Handstitched Leather Vest", 8646, "INV_Chest_Leather_09", &[dmg(0.0085), hp(3.4)]),
    crafted(3024, DARK_LEATHER_PANTS, &[2, 3, 8], Legs, Uncommon, "Dark Leather Pants", 14338, "INV_Pants_06", &[dmg(0.017), crit(0.0051)]),
    crafted(3025, BARBARIC_LEGGINGS, &[2, 3, 8], Legs, Uncommon, "Barbaric Leggings", 33678, "INV_Pants_Leather_07", &[dmg(0.017), crit(0.0051)]),
    crafted(3026, LINEN_BELT, &[4, 6, 7, 8], Waist, Common, "Linen Belt", 4905, "INV_Belt_10", &[frost(0.008), cdr(0.0016)]),
    crafted(3027, SILK_HEADBAND, &[4, 6, 7, 8], Head, Uncommon, "Silk Headband", 15863, "INV_Misc_Bandana_01", &[arcane(0.0213), crit(0.0051)]),
    crafted(3028, GREEN_SILKEN_SHOULDERS, &[4, 6, 7, 8], Shoulder, Uncommon, "Green Silken Shoulders", 1060, "INV_Shoulder_02", &[arcane(0.0213), area(0.0085)]),
    crafted(3029, GREEN_SILK_ARMOR, &[4, 6, 7, 8], Chest, Uncommon, "Green Silk Armor", 12696, "INV_Chest_Cloth_31", &[arcane(0.0255), hp(5.1)]),
    crafted(3030, HANDSTITCHED_LEATHER_BRACERS, &[2, 3, 8], Wrist, Common, "Handstitched Leather Bracers", 3598, "INV_Bracer_02", &[crit(0.0017), dmg(0.0059)]),
    crafted(3031, BARBARIC_IRON_BREASTPLATE, &[0, 1, 2, 5], Chest, Uncommon, "Barbaric Iron Breastplate", 13011, "INV_Chest_Chain_07", &[dmg(0.017), hp(5.1)]),
    crafted(3032, HEAVY_MITHRIL_BREASTPLATE, &[0, 1], Chest, Uncommon, "Heavy Mithril Breastplate", 19787, "INV_Chest_Plate08", &[hp(5.1), dmg(0.017)]),
    crafted(3033, HEAVY_MITHRIL_BOOTS, &[0, 1], Feet, Uncommon, "Heavy Mithril Boots", 19719, "INV_Boots_Plate_03", &[speed(0.0051), dmg(0.0127)]),
    crafted(3034, WICKED_MITHRIL_BLADE, &[0, 1, 2, 3, 5], MainHand, Uncommon, "Wicked Mithril Blade", 18409, "INV_Sword_37", &[dmg(0.0213), critd(0.0213)]),
    crafted(3035, NIGHTSCAPE_TUNIC, &[2, 3, 8], Chest, Uncommon, "Nightscape Tunic", 8640, "INV_Chest_Leather_06", &[dmg(0.017), hp(5.1)]),
    crafted(3036, NIGHTSCAPE_HEADBAND, &[2, 3, 8], Head, Uncommon, "Nightscape Headband", 8266, "INV_Misc_Bandana_03", &[dmg(0.017), crit(0.0051)]),
    crafted(3037, NIGHTSCAPE_PANTS, &[2, 3, 8], Legs, Uncommon, "Nightscape Pants", 32341, "INV_Pants_Leather_09", &[dmg(0.017), crit(0.0051)]),
    crafted(3038, NIGHTSCAPE_BOOTS, &[2, 3, 8], Feet, Uncommon, "Nightscape Boots", 2366, "INV_Boots_08", &[speed(0.0051), dmg(0.0127)]),
    crafted(3039, BLACK_MAGEWEAVE_VEST, &[4, 6, 7, 8], Chest, Uncommon, "Black Mageweave Vest", 24598, "INV_Chest_Cloth_04", &[arcane(0.0255), hp(5.1)]),
    crafted(3040, BLACK_MAGEWEAVE_LEGGINGS, &[4, 6, 7, 8], Legs, Uncommon, "Black Mageweave Leggings", 33652, "INV_Pants_Cloth_07", &[arcane(0.0213), crit(0.0051)]),
    crafted(3041, BLACK_MAGEWEAVE_GLOVES, &[4, 6, 7, 8], Hands, Uncommon, "Black Mageweave Gloves", 22755, "INV_Gauntlets_14", &[crit(0.0043), arcane(0.017)]),
    crafted(3042, BLACK_MAGEWEAVE_BOOTS, &[4, 6, 7, 8], Feet, Uncommon, "Black Mageweave Boots", 33048, "INV_Boots_Cloth_05", &[speed(0.0051), arcane(0.017)]),
    crafted(3043, CINDERCLOTH_ROBE, &[6, 7], Chest, Uncommon, "Cindercloth Robe", 12673, "INV_Chest_Cloth_25", &[fire(0.0255), hp(5.1)]),
    crafted(3044, SIMPLE_LINEN_PANTS, &[4, 6, 7, 8], Legs, Common, "Simple Linen Pants", 33618, "INV_Pants_Cloth_02", &[frost(0.012), crit(0.0024)]),
    crafted(3045, DARK_IRON_PLATE, &[0, 1], Chest, Rare, "Dark Iron Plate", 22510, "INV_Chest_Plate07", &[hp(8.5), dmg(0.0298), armor(0.0127)]),
    crafted(3046, IMPERIAL_PLATE_CHEST, &[0, 1], Chest, Uncommon, "Imperial Plate Chest", 9387, "INV_Chest_Plate03", &[hp(5.1), dmg(0.017)]),
    crafted(3047, IMPERIAL_PLATE_BELT, &[0, 1], Waist, Uncommon, "Imperial Plate Belt", 25740, "INV_Belt_11", &[dmg(0.0127), speed(0.0051)]),
    crafted(3048, IMPERIAL_PLATE_BRACERS, &[0, 1], Wrist, Uncommon, "Imperial Plate Bracers", 18378, "INV_Bracer_14", &[critd(0.0127), dmg(0.0085)]),
    crafted(3049, IMPERIAL_PLATE_BOOTS, &[0, 1], Feet, Uncommon, "Imperial Plate Boots", 19701, "INV_Boots_Plate_04", &[speed(0.0051), dmg(0.0127)]),
    crafted(3050, IMPERIAL_PLATE_HELM, &[0, 1], Head, Uncommon, "Imperial Plate Helm", 15811, "INV_Helmet_25", &[dmg(0.017), hp(5.1)]),
    crafted(3051, IMPERIAL_PLATE_SHOULDERS, &[0, 1], Shoulder, Uncommon, "Imperial Plate Shoulders", 29971, "INV_Shoulder_10", &[dmg(0.0127), armor(0.0059)]),
    crafted(3052, IMPERIAL_PLATE_LEGGINGS, &[0, 1], Legs, Uncommon, "Imperial Plate Leggings", 3414, "INV_Pants_04", &[dmg(0.017), hp(5.1)]),
    crafted(3053, LIONHEART_HELM, &[0, 1], Head, Epic, "Lionheart Helm", 22565, "INV_Helmet_36", &[crit(0.0102), dmg(0.0255), hp(7.6)]),
    crafted(3054, ARCANITE_REAPER, &[0, 1, 2, 5], MainHand, Epic, "Arcanite Reaper", 19238, "INV_Axe_09", &[dmg(0.0425), critd(0.051), crit(0.0127)]),
    crafted(3055, RUNECLOTH_BELT, &[4, 6, 7, 8], Waist, Uncommon, "Runecloth Belt", 15012, "INV_Belt_08", &[arcane(0.017), cdr(0.0034)]),
    crafted(3056, RUNECLOTH_GLOVES, &[4, 6, 7, 8], Hands, Uncommon, "Runecloth Gloves", 14045, "INV_Gauntlets_18", &[crit(0.0043), arcane(0.017)]),
    crafted(3057, CINDERCLOTH_VEST, &[6, 7], Chest, Uncommon, "Cindercloth Vest", 34725, "INV_Chest_Cloth_23", &[fire(0.0255), hp(5.1)]),
    crafted(3058, MOONCLOTH_LEGGINGS, &[4, 6, 7, 8], Legs, Rare, "Mooncloth Leggings", 32989, "INV_Pants_Cloth_09", &[arcane(0.034), crit(0.0076), hp(7.6)]),
    crafted(3059, MOONCLOTH_VEST, &[4, 6, 7, 8], Chest, Rare, "Mooncloth Vest", 14584, "INV_Chest_Cloth_08", &[arcane(0.0382), hp(8.5), crit(0.0085)]),
    crafted(3060, ROBE_OF_THE_ARCHMAGE, &[6], Chest, Epic, "Robe of the Archmage", 12673, "INV_Chest_Cloth_25", &[arcane(0.0414), cdr(0.0096), crit(0.0096)]),
    crafted(3061, TRUEFAITH_VESTMENTS, &[4], Chest, Epic, "Truefaith Vestments", 34730, "INV_Chest_Cloth_11", &[holy(0.0446), regen(0.2), hp(7.1)]),
    crafted(3062, VOLCANIC_BREASTPLATE, &[2, 3, 8], Chest, Uncommon, "Volcanic Breastplate", 8647, "INV_Chest_Leather_10", &[armor(0.0054), hp(5.1)]),
    crafted(3063, VOLCANIC_LEGGINGS, &[2, 3, 8], Legs, Uncommon, "Volcanic Leggings", 33119, "INV_Pants_Leather_05", &[armor(0.0054), crit(0.0051)]),
    crafted(3064, VOLCANIC_SHOULDERS, &[2, 3, 8], Shoulder, Uncommon, "Volcanic Shoulders", 14696, "INV_Shoulder_23", &[armor(0.0044), critd(0.014)]),
    crafted(3065, DEVILSAUR_LEGGINGS, &[2, 3, 8], Legs, Rare, "Devilsaur Leggings", 34710, "INV_Pants_Leather_21", &[crit(0.0068), dmg(0.0255), hp(7.6)]),
    crafted(3066, DEVILSAUR_GAUNTLETS, &[2, 3, 8], Hands, Rare, "Devilsaur Gauntlets", 17187, "INV_Gauntlets_26", &[crit(0.0068), dmg(0.0213), critd(0.0255)]),
    crafted(3067, FLARECORE_GLOVES, &[6, 7], Hands, Epic, "Flarecore Gloves", 12398, "INV_Gauntlets_19", &[fire(0.0085), arcane(0.0382), critd(0.034)]),
    crafted(3068, FLARECORE_MANTLE, &[6, 7], Shoulder, Epic, "Flarecore Mantle", 31823, "INV_Shoulder_19", &[fire(0.0467), area(0.0213), crit(0.0085)]),
    crafted(3069, COREHOUND_BOOTS, &[2, 3, 8], Feet, Epic, "Corehound Boots", 34976, "INV_Boots_Chain_05", &[speed(0.0127), dmg(0.0298), crit(0.0085)]),
    crafted(3070, MOLTEN_HELM, &[2, 3, 8], Head, Epic, "Molten Helm", 15550, "INV_Helmet_08", &[dmg(0.034), crit(0.0085), critd(0.0425)]),
    crafted(3071, FIERY_CHAIN_GIRDLE, &[0, 1, 2, 5], Waist, Epic, "Fiery Chain Girdle", 26953, "INV_Belt_13", &[dmg(0.0255), speed(0.0085), pickup(0.0255)]),
    crafted(3072, SULFURON_HAMMER, &[0, 1, 5], MainHand, Epic, "Sulfuron Hammer", 29446, "INV_Hammer_Unique_Sulfuras", &[dmg(0.0425), critd(0.051), crit(0.0127)]),
];

/// The crafted piece with id `id`.
pub(in crate::survivors) fn crafted_piece(id: u16) -> Option<&'static CraftedPiece> {
    if id < FIRST_ID {
        return None;
    }
    CRAFTED.iter().find(|p| p.item.id == id)
}

/// The crafted piece the bag item `entry` is.
pub(in crate::survivors) fn by_item(entry: u32) -> Option<&'static CraftedPiece> {
    CRAFTED.iter().find(|p| p.real == entry)
}

/// The template's `AllowableClass` for bag item `entry`: its classes' bits, `1 << (id - 1)`; -1
/// (every class) for anything that is not crafted gear.
pub(in crate::survivors) fn class_mask(entry: u32) -> i32 {
    by_item(entry).map_or(-1, |p| {
        p.classes
            .iter()
            .filter_map(|&c| CLASSES.get(c))
            .fold(0u32, |mask, c| mask | 1 << (u32::from(c.id) - 1)) as i32
    })
}

/// The crafted pieces class `class` may wear, in id order.
pub(in crate::survivors) fn for_class(class: usize) -> impl Iterator<Item = &'static CraftedPiece> {
    CRAFTED.iter().filter(move |p| p.classes.contains(&class))
}

/// Why a crafted bag item did not go on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum EquipError {
    /// No class is picked.
    NoClass,
    /// The picked class may not wear it (its tooltip's class line is red).
    WrongClass,
    /// The class has it already.
    Duplicate,
}

impl EquipError {
    /// The stock `GlobalStrings.lua` key and its words.
    pub(in crate::survivors) fn global_string(self) -> (&'static str, &'static str) {
        match self {
            EquipError::NoClass => ("SURVIVORS_PICK_A_CLASS", "Pick a class to wear it."),
            EquipError::WrongClass => (
                "ERR_CANT_EQUIP_SKILL",
                "You do not have the required proficiency for that item.",
            ),
            EquipError::Duplicate => ("SURVIVORS_ALREADY_COLLECTED", "You already have that."),
        }
    }
}

/// Equip `piece` from the bags on class `class`'s gear: collected under its id and put on in its
/// slot. The caller takes the bag item away on `Ok`.
pub(in crate::survivors) fn equip(
    gear: &mut GearSave,
    piece: &'static CraftedPiece,
    class: usize,
) -> Result<(), EquipError> {
    if !piece.classes.contains(&class) {
        return Err(EquipError::WrongClass);
    }
    if gear.has(piece.item.id) {
        return Err(EquipError::Duplicate);
    }
    gear.collected.insert(piece.item.id);
    gear.equip(&piece.item);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::super::{items_for, piece, usable_by, worn_bonuses};
    use super::*;

    /// Ids are their block's and unique; each piece is a crafted bag item that binds on equip,
    /// made by a recipe, for at least one class.
    #[test]
    fn survivors_crafted_pieces_are_bag_items_made_by_recipes() {
        use super::super::super::professions::recipes;
        let mut seen = std::collections::HashSet::new();
        for p in CRAFTED {
            assert!(
                p.item.id >= FIRST_ID && seen.insert(p.item.id),
                "{}",
                p.item.name
            );
            assert!(!p.classes.is_empty() && p.classes.iter().all(|&c| c < CLASSES.len()));
            let def = template(p.real).unwrap_or_else(|| panic!("{} has no bag item", p.item.name));
            assert_eq!(def.name, p.item.name);
            assert_eq!(def.bonding, 2, "{}: binds when equipped", p.item.name);
            assert!(matches!(def.class, 2 | 4), "{}", p.item.name);
            assert_eq!(def.quality, p.item.rarity as u32 + 1, "{}", p.item.name);
            assert!(def.icon.eq_ignore_ascii_case(p.item.icon));
            assert!(
                recipes::all().any(|r| r.product == p.real),
                "no recipe makes {}",
                p.item.name
            );
            assert!(!p.item.bonuses.is_empty(), "{} grants nothing", p.item.name);
            assert!(super::super::item(p.item.id).is_some_and(|i| std::ptr::eq(i, &p.item)));
        }
        assert!(CRAFTED.len() >= 60, "{} pieces", CRAFTED.len());
    }

    /// Each bonus's worth against the best of its kind at the rarity, over the class catalog.
    fn worth(rarity: Rarity) -> impl Fn(&[Bonus]) -> f32 {
        let mut best: HashMap<(StatKey, Option<School>), f32> = HashMap::new();
        for class in 0..CLASSES.len() {
            for i in items_for(class).iter().filter(|i| i.rarity == rarity) {
                for b in i.bonuses {
                    let e = best.entry((b.stat, b.school)).or_insert(0.0);
                    *e = e.max(b.value);
                }
            }
        }
        move |bs: &[Bonus]| {
            bs.iter()
                .map(|b| b.value / best.get(&(b.stat, b.school)).copied().unwrap_or(f32::MAX))
                .sum()
        }
    }

    /// 6.1's guardrail: a crafted piece never beats the boss piece of its slot and rarity, for
    /// any class that may wear it, nor tops out above Epic. Its bonuses' worth (each against the
    /// best of its kind at that rarity) is below the boss piece's.
    #[test]
    fn survivors_crafted_never_beats_the_boss_piece() {
        for p in CRAFTED {
            assert!((p.item.rarity as u8) <= (Epic as u8), "{}", p.item.name);
            let w = worth(p.item.rarity);
            let mine = w(p.item.bonuses);
            for &class in p.classes {
                let boss = piece(class, p.item.slot, p.item.rarity).expect("a boss piece");
                assert!(
                    mine < w(boss.bonuses),
                    "{} ({mine}) beats {}'s {} ({})",
                    p.item.name,
                    CLASSES[class].name,
                    boss.name,
                    w(boss.bonuses)
                );
            }
        }
    }

    /// The wrong class is refused; the right one collects and wears it, once.
    #[test]
    fn survivors_crafted_equip_checks_the_class() {
        let lionheart = by_item(LIONHEART_HELM).expect("the Lionheart Helm");
        assert_eq!(lionheart.item.slot, Head);
        let mut gear = GearSave::default();
        assert_eq!(
            equip(&mut gear, lionheart, 6),
            Err(EquipError::WrongClass),
            "a mage"
        );
        assert!(!gear.has(lionheart.item.id));
        assert_eq!(equip(&mut gear, lionheart, 0), Ok(()));
        assert!(gear.wears(lionheart.item.id), "on the warrior's head");
        assert_eq!(equip(&mut gear, lionheart, 0), Err(EquipError::Duplicate));
        assert!(usable_by(&lionheart.item, 1) && !usable_by(&lionheart.item, 6));
        assert!(worn_bonuses(&gear.equipped_ids()).get(StatKey::CritChance) > 0.0);
        // The tooltip's class line: the Warrior's (1) and the Paladin's (2) bits.
        assert_eq!(class_mask(LIONHEART_HELM), 0b11);
        assert_eq!(class_mask(COPPER_ORE), -1);
        let archmage = by_item(ROBE_OF_THE_ARCHMAGE).unwrap();
        assert_eq!(archmage.classes, &[6], "the Mage's alone");
        assert_eq!(
            for_class(6)
                .filter(|p| p.real == ROBE_OF_THE_ARCHMAGE)
                .count(),
            1
        );
    }

    /// The looks are real rows: each display is an `ItemDisplayInfo.dbc` row whose icon is the
    /// piece's and which draws its slot (a model for a head, shoulders or weapon, the body
    /// textures for the rest), and each icon ships. Every mismatch is listed.
    #[test]
    fn crafted_displays_and_icons_exist_in_the_client() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open the MPQ chain");
        let cat = benilla_formats::load_item_display_catalog(&mut chain).expect("ItemDisplayInfo");
        let mut bad = Vec::new();
        for p in CRAFTED {
            let path = format!("Interface\\Icons\\{}", p.item.icon);
            match cat.get(p.item.display_id) {
                None => bad.push(format!("{}: no display {}", p.item.name, p.item.display_id)),
                Some(d) => {
                    let r = &d.region_textures;
                    let draws = match p.item.slot {
                        Head | Shoulder | MainHand => d.model[0].is_some(),
                        Chest => r[3].is_some(),
                        Waist => r[4].is_some() || r[5].is_some(),
                        Legs => r[5].is_some(),
                        Feet => r[7].is_some() || r[6].is_some(),
                        Wrist => r[1].is_some(),
                        Hands => r[2].is_some(),
                    };
                    let icon = d
                        .icon
                        .as_deref()
                        .is_some_and(|c| c.eq_ignore_ascii_case(&path));
                    if !(draws && icon) {
                        bad.push(format!(
                            "{}: display {} shows {:?}, draws {draws}",
                            p.item.name, p.item.display_id, d.icon
                        ));
                    }
                }
            }
            if !chain.contains(&format!("{path}.blp")) {
                bad.push(format!("{}: no {path}.blp", p.item.name));
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }
}
