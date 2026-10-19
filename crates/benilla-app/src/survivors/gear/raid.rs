//! Molten Core's loot (`docs/RAIDS.md` section 4): the raid's own pieces, apart from the class
//! catalog's 45 a class, and what each of the ten bosses drops.
//!
//! - **Tier 1.** Each class's 1.12 set, 8 pieces in the 8 armour slots, at Epic. The item ids are
//!   the sets' real ones: the data test checks each set's 8 against its `ItemSet.dbc` row. Which
//!   id is which slot is from memory (the client has no item templates). Each display id is an
//!   `ItemDisplayInfo.dbc` row painted with the set's own raid textures
//!   (`Plate_RaidWarrior_A_01...`), chosen by texture name; the data test holds every display to
//!   its icon, but the link from item to display can't be checked against the client. Pants of
//!   Prophecy have no legs-only row in the client, so they use a robe-skirt row.
//! - **Set bonuses** at 3, 5 and 8 pieces ([`TierSet`]), the thresholds `ItemSet.dbc` gives.
//!   They are modest survivors echoes of the 1.12 bonuses: the 3 and 5 add to the gear's
//!   [`Bonuses`] ([`set_stats`]), and the 8 grants a proc from the mechanics library, or Might's
//!   reflect ([`apply_tier`]).
//! - **Named drops**: the bosses' 1.12 weapons and armour, each listing the classes that may
//!   Need it ([`RaidPiece::classes`]). Rings, trinkets and necklaces are **Bound** jewelry: the
//!   account keeps them, and they are worn from the start of every run. Their numbers are the
//!   jewelry catalog's. Their real ids are from memory, and the display ids were picked from
//!   `ItemDisplayInfo.dbc` by icon and model; neither can be checked against the client.
//! - **Sulfuras.** The Eye of Sulfuras and the Sulfuron Ingot are quest items, counted in the
//!   account's [`RaidVault`]. With one of each, the forge turns them into Sulfuras, Hand of
//!   Ragnaros, for the Warrior, the Paladin and the Shaman at once ([`forge`]).
//!
//! Ids are a block of their own from [`FIRST_ID`]: stable save keys, never computed.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::super::bonus::{Bonus, Bonuses, School, StatKey};
use super::super::mechanics::{self, Effect, MechanicId, MechanicSpec, Of, Trigger};
use super::super::{HeroState, Rng};
use super::Rarity::{self, *};
use super::Slot::{self, *};
use super::{GearSave, Item};

/// The first id of the raid's block, well clear of the class catalog's.
pub(in crate::survivors) const FIRST_ID: u16 = 2000;
/// The classes Sulfuras is for, and who may Need its Eye and Ingot: Warrior, Paladin, Shaman.
pub(in crate::survivors) const SULFURAS_CLASSES: &[usize] = &[0, 1, 5];
/// Sulfuras, Hand of Ragnaros, as the forge makes it.
pub(in crate::survivors) const SULFURAS: u16 = 2160;
/// The mechanics a full Tier 1 set grants, under one id.
const TIER_MECHANIC: MechanicId = MechanicId::Other("tier1");

/// A quest item the account counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum Token {
    Eye,
    Ingot,
}

/// What a raid piece is to the one who keeps it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum RaidKind {
    /// Worn in its slot: kept in a class's gear.
    Gear,
    /// A ring, trinket or necklace, by its jewelry key: kept for the account, worn from the start
    /// of every run.
    Bound(&'static str),
    /// A quest item, counted in the vault.
    Token(Token),
}

/// One of the raid's pieces. `item.class` is the first of `classes`, where a Greed win of a piece
/// for no class the winner plays is kept.
#[derive(Debug)]
pub(in crate::survivors) struct RaidPiece {
    pub item: Item,
    /// Its 1.12 item id.
    pub real: u32,
    /// The classes that may Need it, and whose gear may keep it.
    pub classes: &'static [usize],
    pub kind: RaidKind,
}

impl RaidPiece {
    /// The tooltip's second line: "Epic Head", "Legendary Ring", "Quest Item".
    pub(in crate::survivors) fn label(&self) -> String {
        match self.kind {
            RaidKind::Gear => format!(
                "{} {}",
                super::rarity_label(self.item.rarity),
                self.item.slot.label()
            ),
            RaidKind::Bound(key) => format!(
                "Bound {} {}",
                super::rarity_label(self.item.rarity),
                super::super::jewelry::piece(key).map_or("Jewelry", |p| p.kind.label())
            ),
            RaidKind::Token(_) => "Quest Item".to_string(),
        }
    }
}

const ONE: [&[usize]; 9] = [&[0], &[1], &[2], &[3], &[4], &[5], &[6], &[7], &[8]];
const EVERYONE: &[usize] = &[0, 1, 2, 3, 4, 5, 6, 7, 8];

const fn piece(
    id: u16,
    real: u32,
    classes: &'static [usize],
    slot: Slot,
    rarity: Rarity,
    name: &'static str,
    display_id: u32,
    icon: &'static str,
    bonuses: &'static [Bonus],
    kind: RaidKind,
) -> RaidPiece {
    RaidPiece {
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
        kind,
    }
}

/// A Tier 1 piece of class `class`.
const fn t1(
    id: u16,
    real: u32,
    class: usize,
    slot: Slot,
    name: &'static str,
    display_id: u32,
    icon: &'static str,
    bonuses: &'static [Bonus],
) -> RaidPiece {
    piece(
        id,
        real,
        ONE[class],
        slot,
        Epic,
        name,
        display_id,
        icon,
        bonuses,
        RaidKind::Gear,
    )
}

/// A named piece for `classes`.
const fn named(
    id: u16,
    real: u32,
    classes: &'static [usize],
    slot: Slot,
    rarity: Rarity,
    name: &'static str,
    display_id: u32,
    icon: &'static str,
    bonuses: &'static [Bonus],
) -> RaidPiece {
    piece(
        id,
        real,
        classes,
        slot,
        rarity,
        name,
        display_id,
        icon,
        bonuses,
        RaidKind::Gear,
    )
}

/// Bound jewelry: the jewelry catalog's piece `key`, repeated here (the tests hold the two
/// together). Its slot is no gear slot; it is never worn as gear.
const fn bound(
    id: u16,
    real: u32,
    key: &'static str,
    rarity: Rarity,
    name: &'static str,
    display_id: u32,
    icon: &'static str,
    bonuses: &'static [Bonus],
) -> RaidPiece {
    piece(
        id,
        real,
        EVERYONE,
        MainHand,
        rarity,
        name,
        display_id,
        icon,
        bonuses,
        RaidKind::Bound(key),
    )
}

/// A quest item toward Sulfuras.
const fn token(
    id: u16,
    real: u32,
    token: Token,
    rarity: Rarity,
    name: &'static str,
    display_id: u32,
    icon: &'static str,
) -> RaidPiece {
    piece(
        id,
        real,
        SULFURAS_CLASSES,
        MainHand,
        rarity,
        name,
        display_id,
        icon,
        &[],
        RaidKind::Token(token),
    )
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
const fn nature(v: f32) -> Bonus {
    Bonus::school(School::Nature, v)
}
const fn shadow(v: f32) -> Bonus {
    Bonus::school(School::Shadow, v)
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
const fn leech(v: f32) -> Bonus {
    Bonus::new(StatKey::Lifesteal, v)
}
const fn pickup(v: f32) -> Bonus {
    Bonus::new(StatKey::Pickup, v)
}
const fn targets(v: f32) -> Bonus {
    Bonus::new(StatKey::Targets, v)
}
const fn xp(v: f32) -> Bonus {
    Bonus::new(StatKey::Experience, v)
}

/// Every raid piece: the nine Tier 1 sets (ids `2000 + class * 10 + slot`, in [`Slot::ALL`]
/// order), the named drops from 2100, Bound jewelry from 2130, the quest items and Sulfuras. The
/// bonuses of a Tier 1 piece are 85% of its class's Tier 2 piece for the slot: the set bonuses
/// make up the rest.
#[rustfmt::skip]
pub(super) static RAID_ITEMS: &[RaidPiece] = &[
    // Battlegear of Might.
    t1(2000, 16866, 0, Head, "Helm of Might", 31259, "INV_Helmet_09", &[dmg(0.034), hp(10.2), critd(0.043)]),
    t1(2001, 16868, 0, Shoulder, "Pauldrons of Might", 31024, "INV_Shoulder_15", &[dmg(0.034), armor(0.013), hp(9.3)]),
    t1(2002, 16865, 0, Chest, "Breastplate of Might", 31021, "INV_Chest_Plate16", &[hp(11.9), dmg(0.038), armor(0.017)]),
    t1(2003, 16864, 0, Waist, "Belt of Might", 31019, "INV_Belt_09", &[dmg(0.025), speed(0.009), hp(7.6)]),
    t1(2004, 16867, 0, Legs, "Legplates of Might", 31023, "INV_Pants_04", &[dmg(0.034), hp(10.2), armor(0.013)]),
    t1(2005, 16862, 0, Feet, "Sabatons of Might", 31025, "INV_Boots_Plate_04", &[speed(0.013), dmg(0.03), hp(8.5)]),
    t1(2006, 16861, 0, Wrist, "Bracers of Might", 31020, "INV_Bracer_19", &[critd(0.025), dmg(0.021), armor(0.009)]),
    t1(2007, 16863, 0, Hands, "Gauntlets of Might", 31022, "INV_Gauntlets_10", &[dmg(0.03), critd(0.034), crit(0.009)]),
    // Lawbringer Armor.
    t1(2010, 16854, 1, Head, "Lawbringer Helm", 31506, "INV_Helmet_05", &[holy(0.051), hp(10.2), armor(0.013)]),
    t1(2011, 16856, 1, Shoulder, "Lawbringer Spaulders", 31510, "INV_Shoulder_20", &[holy(0.047), armor(0.013), hp(9.3)]),
    t1(2012, 16853, 1, Chest, "Lawbringer Chestguard", 31505, "INV_Chest_Plate03", &[hp(11.9), holy(0.055), armor(0.017)]),
    t1(2013, 16858, 1, Waist, "Lawbringer Belt", 31353, "INV_Belt_27", &[holy(0.034), regen(0.2), hp(7.6)]),
    t1(2014, 16855, 1, Legs, "Lawbringer Legplates", 31352, "INV_Pants_04", &[holy(0.051), hp(10.2), armor(0.013)]),
    t1(2015, 16859, 1, Feet, "Lawbringer Boots", 31354, "INV_Boots_Plate_09", &[speed(0.013), holy(0.038), hp(8.5)]),
    t1(2016, 16857, 1, Wrist, "Lawbringer Bracers", 31509, "INV_Bracer_18", &[armor(0.009), holy(0.03), regen(0.2)]),
    t1(2017, 16860, 1, Hands, "Lawbringer Gauntlets", 31507, "INV_Gauntlets_29", &[holy(0.038), crit(0.009), area(0.021)]),
    // Giantstalker Armor.
    t1(2020, 16846, 2, Head, "Giantstalker's Helmet", 32017, "INV_Helmet_05", &[dmg(0.034), crit(0.009), pickup(0.034)]),
    t1(2021, 16848, 2, Shoulder, "Giantstalker's Epaulets", 32030, "INV_Shoulder_10", &[dmg(0.034), crit(0.009), hp(9.3)]),
    t1(2022, 16845, 2, Chest, "Giantstalker's Breastplate", 32022, "INV_Chest_Chain_03", &[dmg(0.038), hp(11.9), crit(0.013)]),
    t1(2023, 16851, 2, Waist, "Giantstalker's Belt", 32019, "INV_Belt_28", &[dmg(0.025), speed(0.009), pickup(0.025)]),
    t1(2024, 16847, 2, Legs, "Giantstalker's Leggings", 32029, "INV_Pants_Mail_03", &[dmg(0.034), crit(0.009), hp(10.2)]),
    t1(2025, 16849, 2, Feet, "Giantstalker's Boots", 32027, "INV_Boots_Chain_13", &[speed(0.013), dmg(0.03), pickup(0.03)]),
    t1(2026, 16850, 2, Wrist, "Giantstalker's Bracers", 32018, "INV_Bracer_17", &[crit(0.007), dmg(0.021), area(0.017)]),
    t1(2027, 16852, 2, Hands, "Giantstalker's Gloves", 32024, "INV_Gauntlets_10", &[dmg(0.03), crit(0.009), critd(0.034)]),
    // Nightslayer Armor.
    t1(2030, 16821, 3, Head, "Nightslayer Cover", 31342, "INV_Helmet_41", &[dmg(0.034), crit(0.009), critd(0.043)]),
    t1(2031, 16823, 3, Shoulder, "Nightslayer Shoulder Pads", 31343, "INV_Shoulder_23", &[dmg(0.034), critd(0.038), crit(0.009)]),
    t1(2032, 16820, 3, Chest, "Nightslayer Chestpiece", 31105, "INV_Chest_Cloth_07", &[dmg(0.038), hp(11.9), crit(0.013)]),
    t1(2033, 16827, 3, Waist, "Nightslayer Belt", 31339, "INV_Belt_23", &[dmg(0.025), speed(0.009), crit(0.008)]),
    t1(2034, 16822, 3, Legs, "Nightslayer Pants", 31340, "INV_Pants_06", &[dmg(0.034), crit(0.009), hp(10.2)]),
    t1(2035, 16824, 3, Feet, "Nightslayer Boots", 31109, "INV_Boots_08", &[speed(0.013), dmg(0.03), crit(0.009)]),
    t1(2036, 16825, 3, Wrist, "Nightslayer Bracelets", 31106, "INV_Bracer_02", &[crit(0.007), dmg(0.021), critd(0.025)]),
    t1(2037, 16826, 3, Hands, "Nightslayer Gloves", 31107, "INV_Gauntlets_21", &[crit(0.009), dmg(0.03), critd(0.034)]),
    // Vestments of Prophecy.
    t1(2040, 16813, 4, Head, "Circlet of Prophecy", 31206, "INV_Helmet_34", &[holy(0.051), cdr(0.009), regen(0.3)]),
    t1(2041, 16816, 4, Shoulder, "Mantle of Prophecy", 30623, "INV_Shoulder_02", &[shadow(0.047), area(0.021), hp(9.3)]),
    t1(2042, 16815, 4, Chest, "Robes of Prophecy", 30614, "INV_Chest_Cloth_03", &[holy(0.055), hp(11.9), regen(0.3)]),
    t1(2043, 16817, 4, Waist, "Girdle of Prophecy", 30621, "INV_Belt_22", &[shadow(0.034), cdr(0.008), regen(0.2)]),
    t1(2044, 16814, 4, Legs, "Pants of Prophecy", 30613, "INV_Chest_Cloth_03", &[holy(0.051), hp(10.2), area(0.025)]),
    t1(2045, 16811, 4, Feet, "Boots of Prophecy", 30622, "INV_Boots_07", &[speed(0.013), shadow(0.038), regen(0.2)]),
    t1(2046, 16819, 4, Wrist, "Vambraces of Prophecy", 30617, "INV_Bracer_09", &[holy(0.03), regen(0.2), cdr(0.007)]),
    t1(2047, 16812, 4, Hands, "Gloves of Prophecy", 30619, "INV_Gauntlets_14", &[shadow(0.038), crit(0.009), cdr(0.009)]),
    // The Earthfury.
    t1(2050, 16842, 5, Head, "Earthfury Helmet", 31835, "INV_Helmet_09", &[nature(0.051), crit(0.009), hp(10.2)]),
    t1(2051, 16844, 5, Shoulder, "Earthfury Epaulets", 31833, "INV_Shoulder_29", &[nature(0.047), armor(0.013), crit(0.009)]),
    t1(2052, 16841, 5, Chest, "Earthfury Vestments", 31832, "INV_Chest_Chain_11", &[hp(11.9), nature(0.055), armor(0.017)]),
    t1(2053, 16838, 5, Waist, "Earthfury Belt", 31829, "INV_Belt_14", &[nature(0.034), area(0.017), regen(0.2)]),
    t1(2054, 16843, 5, Legs, "Earthfury Legguards", 31836, "INV_Pants_03", &[nature(0.051), hp(10.2), crit(0.009)]),
    t1(2055, 16837, 5, Feet, "Earthfury Boots", 31830, "INV_Boots_Plate_06", &[speed(0.013), nature(0.038), hp(8.5)]),
    t1(2056, 16840, 5, Wrist, "Earthfury Bracers", 31831, "INV_Bracer_16", &[crit(0.007), nature(0.03), armor(0.009)]),
    t1(2057, 16839, 5, Hands, "Earthfury Gauntlets", 31834, "INV_Gauntlets_11", &[nature(0.038), crit(0.009), critd(0.034)]),
    // Arcanist Regalia.
    t1(2060, 16795, 6, Head, "Arcanist Crown", 31512, "INV_Helmet_53", &[fire(0.051), crit(0.009), cdr(0.009)]),
    t1(2061, 16797, 6, Shoulder, "Arcanist Mantle", 30586, "INV_Shoulder_02", &[arcane(0.047), area(0.021), crit(0.009)]),
    t1(2062, 16798, 6, Chest, "Arcanist Robes", 30581, "INV_Chest_Cloth_03", &[fire(0.055), hp(11.9), crit(0.013)]),
    t1(2063, 16802, 6, Waist, "Arcanist Belt", 30583, "INV_Belt_30", &[arcane(0.034), cdr(0.008), area(0.017)]),
    t1(2064, 16796, 6, Legs, "Arcanist Leggings", 30582, "INV_Pants_08", &[fire(0.051), crit(0.009), hp(10.2)]),
    t1(2065, 16800, 6, Feet, "Arcanist Boots", 30587, "INV_Boots_07", &[speed(0.013), arcane(0.038), cdr(0.009)]),
    t1(2066, 16799, 6, Wrist, "Arcanist Bindings", 30584, "INV_Belt_29", &[cdr(0.007), fire(0.03), crit(0.007)]),
    t1(2067, 16801, 6, Hands, "Arcanist Gloves", 30585, "INV_Gauntlets_14", &[crit(0.009), arcane(0.038), critd(0.034)]),
    // Felheart Raiment.
    t1(2070, 16808, 7, Head, "Felheart Horns", 31987, "INV_Helmet_08", &[shadow(0.051), cdr(0.009), leech(0.003)]),
    t1(2071, 16807, 7, Shoulder, "Felheart Shoulder Pads", 31974, "INV_Shoulder_23", &[fire(0.047), area(0.021), hp(9.3)]),
    t1(2072, 16809, 7, Chest, "Felheart Robes", 31973, "INV_Chest_Cloth_09", &[shadow(0.055), hp(11.9), leech(0.004)]),
    t1(2073, 16806, 7, Waist, "Felheart Belt", 31969, "INV_Belt_13", &[fire(0.034), cdr(0.008), area(0.017)]),
    t1(2074, 16810, 7, Legs, "Felheart Pants", 31972, "INV_Pants_Cloth_14", &[shadow(0.051), hp(10.2), area(0.025)]),
    t1(2075, 16803, 7, Feet, "Felheart Slippers", 31975, "INV_Boots_Cloth_05", &[speed(0.013), fire(0.038), hp(8.5)]),
    t1(2076, 16804, 7, Wrist, "Felheart Bracers", 31970, "INV_Bracer_07", &[shadow(0.03), leech(0.003), cdr(0.007)]),
    t1(2077, 16805, 7, Hands, "Felheart Gloves", 31971, "INV_Gauntlets_19", &[fire(0.038), crit(0.009), cdr(0.009)]),
    // Cenarion Raiment.
    t1(2080, 16834, 8, Head, "Cenarion Helm", 31727, "INV_Helmet_09", &[arcane(0.051), regen(0.3), crit(0.009)]),
    t1(2081, 16836, 8, Shoulder, "Cenarion Spaulders", 31730, "INV_Shoulder_07", &[nature(0.047), area(0.021), hp(9.3)]),
    t1(2082, 16833, 8, Chest, "Cenarion Vestments", 31731, "INV_Chest_Cloth_06", &[hp(11.9), arcane(0.055), regen(0.3)]),
    t1(2083, 16828, 8, Waist, "Cenarion Belt", 31722, "INV_Belt_06", &[nature(0.034), speed(0.009), regen(0.2)]),
    t1(2084, 16835, 8, Legs, "Cenarion Leggings", 31729, "INV_Pants_06", &[arcane(0.051), hp(10.2), area(0.025)]),
    t1(2085, 16829, 8, Feet, "Cenarion Boots", 31723, "INV_Boots_08", &[speed(0.013), nature(0.038), regen(0.2)]),
    t1(2086, 16830, 8, Wrist, "Cenarion Bracers", 31725, "INV_Bracer_03", &[arcane(0.03), regen(0.2), crit(0.007)]),
    t1(2087, 16831, 8, Hands, "Cenarion Gloves", 31726, "INV_Gauntlets_07", &[nature(0.038), crit(0.009), cdr(0.009)]),
    // The named drops, Lucifron to Ragnaros.
    named(2100, 18875, &[8], Legs, Epic, "Salamander Scale Pants", 28646, "INV_Pants_06", &[nature(0.05), regen(0.3), hp(10.0)]),
    named(2101, 17069, &[2], MainHand, Epic, "Striker's Mark", 30926, "INV_Weapon_Bow_08", &[dmg(0.045), crit(0.015), pickup(0.03)]),
    named(2102, 17073, &[0, 1, 5], MainHand, Epic, "Earthshaker", 32162, "INV_Hammer_04", &[dmg(0.045), critd(0.05), area(0.03)]),
    named(2103, 18878, &[4, 6, 7], MainHand, Epic, "Sorcerous Dagger", 31337, "INV_Weapon_ShortBlade_07", &[dmg(0.045), crit(0.012), cdr(0.012)]),
    named(2104, 19146, &[3, 8], Wrist, Epic, "Wristguards of Stability", 28156, "INV_Bracer_04", &[dmg(0.025), hp(8.0), critd(0.03)]),
    named(2105, 18832, &[0, 1, 3], MainHand, Epic, "Brutality Blade", 23454, "INV_Sword_43", &[dmg(0.05), critd(0.055), crit(0.012)]),
    named(2106, 17105, &[1, 4, 5, 8], MainHand, Epic, "Aurastone Hammer", 34109, "INV_Hammer_05", &[dmg(0.04), regen(0.4), area(0.03)]),
    named(2107, 17071, &[0, 3], MainHand, Epic, "Gutgore Ripper", 34111, "INV_Weapon_ShortBlade_18", &[dmg(0.045), crit(0.015), leech(0.005)]),
    named(2108, 19143, &[0, 1], Hands, Epic, "Flameguard Gauntlets", 31539, "INV_Gauntlets_26", &[dmg(0.03), critd(0.035), fire(0.02)]),
    named(2109, 17074, &[0, 1, 2], MainHand, Epic, "Shadowstrike", 22404, "INV_Spear_08", &[dmg(0.045), leech(0.006), area(0.025)]),
    named(2110, 17103, &[6, 7, 1], MainHand, Epic, "Azuresong Mageblade", 22028, "INV_Sword_39", &[dmg(0.045), crit(0.015), cdr(0.015)]),
    named(2111, 18842, &[4, 5, 6, 7, 8], MainHand, Epic, "Staff of Dominance", 34114, "INV_Staff_13", &[dmg(0.05), area(0.035), cdr(0.015)]),
    named(2112, 17072, &[2], MainHand, Epic, "Blastershot Launcher", 31225, "INV_Weapon_Rifle_07", &[dmg(0.05), crit(0.015), critd(0.04)]),
    named(2113, 18805, &[0, 2, 3], MainHand, Epic, "Core Hound Tooth", 20276, "INV_Sword_12", &[dmg(0.04), crit(0.02), critd(0.05)]),
    named(2114, 18803, &[1, 5, 8], MainHand, Epic, "Finkle's Lava Dredger", 23240, "INV_Hammer_05", &[dmg(0.045), regen(0.4), fire(0.03)]),
    named(2115, 17076, &[0, 1, 2], MainHand, Epic, "Bonereaver's Edge", 29170, "INV_Sword_12", &[dmg(0.06), critd(0.07), crit(0.015)]),
    named(2116, 18816, &[3, 0], MainHand, Epic, "Perdition's Blade", 33626, "INV_Weapon_ShortBlade_11", &[dmg(0.055), fire(0.04), crit(0.015)]),
    named(2117, 19137, &[0, 1], Waist, Epic, "Onslaught Girdle", 31654, "INV_Belt_29", &[dmg(0.04), crit(0.012), armor(0.012)]),
    named(2118, 17104, &[0, 2, 5], MainHand, Epic, "Spinal Reaper", 34450, "INV_Axe_09", &[dmg(0.055), critd(0.065), leech(0.005)]),
    named(2119, 18817, &[2, 5], Head, Epic, "Crown of Destruction", 26274, "INV_Helmet_08", &[dmg(0.045), crit(0.012), hp(10.0)]),
    // Bound jewelry, as the jewelry catalog has it.
    bound(2130, 17109, "choker_of_enlightenment", Epic, "Choker of Enlightenment", 9858, "INV_Jewelry_Necklace_10", &[cdr(0.08), xp(0.10), regen(0.4)]),
    bound(2131, 18821, "quick_strike_ring", Epic, "Quick Strike Ring", 9835, "INV_Jewelry_Ring_07", &[dmg(0.10), crit(0.04), speed(0.04)]),
    bound(2132, 18820, "talisman_of_ephemeral_power", Epic, "Talisman of Ephemeral Power", 31287, "INV_Misc_StoneTablet_11", &[dmg(0.15), cdr(0.07)]),
    bound(2133, 19147, "ring_of_spell_power", Epic, "Ring of Spell Power", 31664, "INV_Jewelry_Ring_38", &[dmg(0.16), area(0.07)]),
    bound(2134, 17110, "seal_of_the_archmagus", Epic, "Seal of the Archmagus", 29697, "INV_Jewelry_Ring_21", &[cdr(0.08), area(0.10), hp(12.0)]),
    bound(2135, 17063, "band_of_accuria", Legendary, "Band of Accuria", 9840, "INV_Jewelry_Ring_15", &[dmg(0.14), critd(0.25), crit(0.04)]),
    bound(2136, 18814, "choker_of_the_fire_lord", Epic, "Choker of the Fire Lord", 1399, "INV_Jewelry_Amulet_05", &[dmg(0.12), area(0.08), cdr(0.03)]),
    // Toward Sulfuras.
    token(2150, 17204, Token::Eye, Legendary, "Eye of Sulfuras", 28291, "INV_Misc_Orb_05"),
    token(2151, 17203, Token::Ingot, Epic, "Sulfuron Ingot", 20656, "INV_Ingot_08"),
    named(SULFURAS, 17182, SULFURAS_CLASSES, MainHand, Legendary, "Sulfuras, Hand of Ragnaros", 29698, "INV_Hammer_Unique_Sulfuras", &[dmg(0.065), fire(0.09), critd(0.08), targets(1.0)]),
];

/// The Eye and the Ingot, by id.
const EYE: u16 = 2150;
const INGOT: u16 = 2151;

/// The raid piece with id `id`.
pub(in crate::survivors) fn raid_piece(id: u16) -> Option<&'static RaidPiece> {
    RAID_ITEMS.iter().find(|p| p.item.id == id)
}

/// Class `class`'s Tier 1 piece for `slot`; none for the main hand.
pub(in crate::survivors) fn tier_piece(class: usize, slot: Slot) -> Option<&'static RaidPiece> {
    let at = Slot::ALL
        .iter()
        .position(|&s| s == slot)
        .filter(|&i| i < 8)?;
    let id = FIRST_ID + u16::try_from(class).ok()? * 10 + at as u16;
    raid_piece(id).filter(|p| p.classes == [class])
}

/// The raid's gear (not jewelry or quest items) that class `class` may wear, in id order: its
/// Tier 1 set, then the named drops for it.
pub(in crate::survivors) fn gear_for(class: usize) -> impl Iterator<Item = &'static RaidPiece> {
    RAID_ITEMS
        .iter()
        .filter(move |p| p.kind == RaidKind::Gear && p.classes.contains(&class))
}

// ---------------------------------------------------------------------------------------------
// Tier 1 set bonuses
// ---------------------------------------------------------------------------------------------

/// One step of a set: what `pieces` worn add.
#[derive(Debug)]
pub(in crate::survivors) struct SetBonus {
    pub pieces: u8,
    /// The tooltip line after "(n) Set: ".
    pub text: &'static str,
    /// Added to the gear's bonuses.
    pub stats: &'static [Bonus],
    /// Granted to the hero for the run ([`apply_tier`]).
    pub procs: &'static [MechanicSpec],
    /// A share of each melee blow the hero takes, struck back at the attacker.
    pub reflect: f32,
}

/// A class's Tier 1 set.
#[derive(Debug)]
pub(in crate::survivors) struct TierSet {
    pub class: usize,
    /// Its name, as its `ItemSet.dbc` row has it.
    pub name: &'static str,
    pub bonuses: [SetBonus; 3],
}

impl TierSet {
    /// Its pieces, in [`Slot::ALL`] order.
    pub(in crate::survivors) fn pieces(&self) -> impl Iterator<Item = &'static RaidPiece> {
        let class = self.class;
        Slot::ALL
            .into_iter()
            .filter_map(move |s| tier_piece(class, s))
    }

    /// How many of its pieces `ids` hold.
    pub(in crate::survivors) fn worn(&self, ids: &[u16]) -> u8 {
        self.pieces().filter(|p| ids.contains(&p.item.id)).count() as u8
    }

    /// Its steps that `pieces` worn reach.
    pub(in crate::survivors) fn active(&self, pieces: u8) -> impl Iterator<Item = &SetBonus> {
        self.bonuses.iter().filter(move |b| pieces >= b.pieces)
    }
}

const fn stats(pieces: u8, text: &'static str, stats: &'static [Bonus]) -> SetBonus {
    SetBonus {
        pieces,
        text,
        stats,
        procs: &[],
        reflect: 0.0,
    }
}

const fn procs(text: &'static str, procs: &'static [MechanicSpec]) -> SetBonus {
    SetBonus {
        pieces: 8,
        text,
        stats: &[],
        procs,
        reflect: 0.0,
    }
}

/// Every class's set, in [`CLASSES`](super::super::data::CLASSES) order.
pub(in crate::survivors) static TIER_SETS: [TierSet; 9] = [
    TierSet {
        class: 0,
        name: "Battlegear of Might",
        bonuses: [
            stats(3, "+2% damage reduction.", &[armor(0.02)]),
            stats(5, "+20 max health.", &[hp(20.0)]),
            SetBonus {
                pieces: 8,
                text: "A quarter of each melee blow you take is struck back at the attacker.",
                stats: &[],
                procs: &[],
                reflect: 0.25,
            },
        ],
    },
    TierSet {
        class: 1,
        name: "Lawbringer Armor",
        bonuses: [
            stats(3, "+3% Holy damage.", &[holy(0.03)]),
            stats(5, "+2% critical strike.", &[crit(0.02)]),
            procs(
                "Your hits have a 5% chance to heal you and allies within 15 yd for 25.",
                &[MechanicSpec::new(
                    Trigger::OnHit {
                        of: Of::Any,
                        chance: 0.05,
                    },
                    Effect::HealPulse {
                        radius: 15.0,
                        amount: 25.0,
                    },
                )
                .visual(2054)
                .cooldown(4.0)],
            ),
        ],
    },
    TierSet {
        class: 2,
        name: "Giantstalker Armor",
        bonuses: [
            stats(3, "+3% damage.", &[dmg(0.03)]),
            stats(5, "+2% critical strike.", &[crit(0.02)]),
            procs(
                "Your hits have a 6% chance to split into 2 more shots (Multi-Shot).",
                &[MechanicSpec::new(
                    Trigger::OnHit {
                        of: Of::Any,
                        chance: 0.06,
                    },
                    Effect::Split {
                        count: 2,
                        spread: 8.0,
                    },
                )
                .cooldown(1.5)],
            ),
        ],
    },
    TierSet {
        class: 3,
        name: "Nightslayer Armor",
        bonuses: [
            stats(3, "+2% critical strike.", &[crit(0.02)]),
            stats(5, "+5% critical damage.", &[critd(0.05)]),
            procs(
                "Falling below 25% health heals you for 40 (Vanish); once a minute.",
                &[MechanicSpec::new(
                    Trigger::OnLowHealth { threshold: 0.25 },
                    Effect::HealPulse {
                        radius: 1.0,
                        amount: 40.0,
                    },
                )
                .visual(2054)
                .cooldown(60.0)],
            ),
        ],
    },
    TierSet {
        class: 4,
        name: "Vestments of Prophecy",
        bonuses: [
            stats(3, "+3% Holy damage.", &[holy(0.03)]),
            stats(5, "+0.5 health per second.", &[regen(0.5)]),
            procs(
                "Every 15 sec, heals you and allies within 15 yd for 20 (Prayer of Healing).",
                &[MechanicSpec::new(
                    Trigger::Periodic { every: 15.0 },
                    Effect::HealPulse {
                        radius: 15.0,
                        amount: 20.0,
                    },
                )
                .visual(2054)
                .cooldown(15.0)],
            ),
        ],
    },
    TierSet {
        class: 5,
        name: "The Earthfury",
        bonuses: [
            stats(3, "+3% Nature damage.", &[nature(0.03)]),
            stats(5, "+5% area of effect.", &[area(0.05)]),
            procs(
                "A killing blow is turned away: back up at 50% health (Reincarnation); once every \
                 10 min.",
                &[MechanicSpec::new(
                    Trigger::OnLethal,
                    Effect::CheatDeath {
                        immune: 2.0,
                        heal_frac: 0.5,
                    },
                )
                .visual(20707)
                .cooldown(600.0)],
            ),
        ],
    },
    TierSet {
        class: 6,
        name: "Arcanist Regalia",
        bonuses: [
            stats(3, "+3% damage.", &[dmg(0.03)]),
            stats(5, "-3% cooldowns.", &[cdr(0.03)]),
            procs(
                "Your hits have a 2% chance to make every ability ready at once; once every 30 \
                 sec.",
                &[MechanicSpec::new(
                    Trigger::OnHit {
                        of: Of::Any,
                        chance: 0.02,
                    },
                    Effect::ResetCooldowns,
                )
                .visual(26166)
                .cooldown(30.0)],
            ),
        ],
    },
    TierSet {
        class: 7,
        name: "Felheart Raiment",
        bonuses: [
            stats(3, "+3% Shadow damage.", &[shadow(0.03)]),
            stats(5, "+1% lifesteal.", &[leech(0.01)]),
            procs(
                "Your kills have a 30% chance to set Shadow burns on 3 enemies within 10 yd.",
                &[MechanicSpec::new(
                    Trigger::OnKill {
                        of: Of::Any,
                        chance: 0.30,
                    },
                    Effect::Dot {
                        school: School::Shadow,
                        dps: 6.0,
                        secs: 6.0,
                    },
                )
                .at(mechanics::Aim::Spread { radius: 10.0, n: 3 })
                .cooldown(2.0)],
            ),
        ],
    },
    TierSet {
        class: 8,
        name: "Cenarion Raiment",
        bonuses: [
            stats(3, "+3% Nature damage.", &[nature(0.03)]),
            stats(5, "+0.5 health per second.", &[regen(0.5)]),
            procs(
                "Falling below 50% health heals you and allies within 20 yd for 30 \
                 (Tranquility); once a minute.",
                &[MechanicSpec::new(
                    Trigger::OnLowHealth { threshold: 0.5 },
                    Effect::HealPulse {
                        radius: 20.0,
                        amount: 30.0,
                    },
                )
                .visual(2054)
                .cooldown(60.0)],
            ),
        ],
    },
];

/// Class `class`'s set.
pub(in crate::survivors) fn tier_set(class: usize) -> Option<&'static TierSet> {
    TIER_SETS.iter().find(|s| s.class == class)
}

/// The set `id` belongs to.
pub(in crate::survivors) fn set_of(id: u16) -> Option<&'static TierSet> {
    let p = raid_piece(id)?;
    tier_set(p.item.class).filter(|s| s.pieces().any(|q| q.item.id == id))
}

/// What the sets' steps reached by the pieces `ids` add to the gear's bonuses.
pub(in crate::survivors) fn set_stats(ids: &[u16]) -> Bonuses {
    let mut total = Bonuses::default();
    for set in &TIER_SETS {
        for step in set.active(set.worn(ids)) {
            total.add_all(step.stats);
        }
    }
    total
}

/// Puts class `class`'s set at `pieces` worn on `hero`: the 8-piece's procs and reflect, or none.
/// Idempotent; whether anything would change is [`tier_applied`].
pub(in crate::survivors) fn apply_tier(hero: &mut HeroState, class: usize, pieces: u8) {
    let Some(set) = tier_set(class) else { return };
    let procs: Vec<MechanicSpec> = set
        .active(pieces)
        .flat_map(|b| b.procs.iter().copied())
        .collect();
    hero.reflect = set.active(pieces).map(|b| b.reflect).sum();
    if procs.is_empty() {
        mechanics::revoke(hero, TIER_MECHANIC);
    } else {
        mechanics::grant(hero, TIER_MECHANIC, procs);
    }
}

/// Whether `hero` already carries class `class`'s set at `pieces` worn.
pub(in crate::survivors) fn tier_applied(hero: &HeroState, class: usize, pieces: u8) -> bool {
    let Some(set) = tier_set(class) else {
        return true;
    };
    let reflect: f32 = set.active(pieces).map(|b| b.reflect).sum();
    let procs = set.active(pieces).any(|b| !b.procs.is_empty());
    hero.reflect == reflect && mechanics::has(hero, TIER_MECHANIC) == procs
}

// ---------------------------------------------------------------------------------------------
// What each boss drops
// ---------------------------------------------------------------------------------------------

/// One boss's drop: a kill drops one Tier 1 piece of a slot in `tier` (two for a party of 3 or
/// more), `named` pieces drawn from `named` (those the party can use first), and each token at
/// its chance.
#[derive(Debug)]
pub(in crate::survivors) struct BossLoot {
    /// The boss's script name ([`super::super::raid::script_for`]).
    pub boss: &'static str,
    pub tier: &'static [Slot],
    pub named: &'static [u16],
    pub named_drops: u8,
    pub tokens: &'static [(u16, f32)],
}

/// The ten bosses in order. The Tier 1 slots follow 1.12 where they dropped from a boss
/// (Lucifron's gloves, Magmadar's legs, Garr's helms, Geddon and Sulfuron's shoulders,
/// Golemagg's chests and legs); the wrists and belts, trash drops in 1.12, come from Majordomo's
/// cache here, as there is no trash loot.
#[rustfmt::skip]
pub(in crate::survivors) static MC_LOOT: &[BossLoot] = &[
    BossLoot { boss: "Lucifron", tier: &[Hands], named: &[2130, 2108], named_drops: 1, tokens: &[] },
    BossLoot { boss: "Magmadar", tier: &[Legs], named: &[2101, 2102, 2131], named_drops: 1, tokens: &[] },
    BossLoot { boss: "Gehennas", tier: &[Feet], named: &[2103, 2104], named_drops: 1, tokens: &[] },
    BossLoot { boss: "Garr", tier: &[Head], named: &[2105, 2106, 2107], named_drops: 1, tokens: &[] },
    BossLoot { boss: "Shazzrah", tier: &[Hands, Feet], named: &[2132, 2133, 2103], named_drops: 1, tokens: &[] },
    BossLoot { boss: "Baron Geddon", tier: &[Shoulder], named: &[2134, 2100], named_drops: 1, tokens: &[] },
    BossLoot { boss: "Sulfuron Harbinger", tier: &[Shoulder], named: &[2109, 2100], named_drops: 1, tokens: &[] },
    BossLoot { boss: "Golemagg the Incinerator", tier: &[Chest, Legs], named: &[2110, 2111, 2112], named_drops: 1, tokens: &[(INGOT, 0.25)] },
    BossLoot { boss: "Majordomo Executus", tier: &[Wrist, Waist], named: &[2113, 2114], named_drops: 1, tokens: &[] },
    BossLoot { boss: "Ragnaros", tier: &[], named: &[2115, 2116, 2117, 2118, 2119, 2135, 2136], named_drops: 3, tokens: &[(EYE, 0.10), (INGOT, 0.5)] },
];

/// What boss `name` drops, if it is one of the ten.
pub(in crate::survivors) fn loot_for(name: &str) -> Option<&'static BossLoot> {
    MC_LOOT.iter().find(|l| l.boss == name)
}

/// The bosses that drop piece `id`, each with its odds a solo kill: a Tier 1 slot's share of the
/// boss's slots, a named piece's share of its draws, a token's chance.
pub(in crate::survivors) fn sources(id: u16) -> Vec<(&'static str, f32)> {
    let Some(p) = raid_piece(id) else {
        return Vec::new();
    };
    let tier = set_of(id).is_some();
    MC_LOOT
        .iter()
        .filter_map(|l| {
            let chance = if tier && l.tier.contains(&p.item.slot) {
                1.0 / l.tier.len() as f32
            } else if l.named.contains(&id) {
                (f32::from(l.named_drops) / l.named.len() as f32).min(1.0)
            } else {
                l.tokens.iter().find(|t| t.0 == id)?.1
            };
            Some((l.boss, chance))
        })
        .collect()
}

/// One kill's drop, by piece id, for a party of `party`'s classes (one entry a hero; solo, just
/// the player's class). Each Tier 1 piece is the set of a class picked at random from the party.
pub(in crate::survivors) fn roll_kill(loot: &BossLoot, party: &[usize], rng: &mut Rng) -> Vec<u16> {
    let classes: &[usize] = if party.is_empty() { &[0] } else { party };
    let mut out = Vec::new();
    if !loot.tier.is_empty() {
        let n = if party.len() >= 3 { 2 } else { 1 };
        for _ in 0..n {
            let slot = loot.tier[rng.index(loot.tier.len())];
            let class = classes[rng.index(classes.len())];
            if let Some(p) = tier_piece(class, slot) {
                out.push(p.item.id);
            }
        }
    }
    let usable =
        |id: &u16| raid_piece(*id).is_some_and(|p| p.classes.iter().any(|c| classes.contains(c)));
    let mut first: Vec<u16> = loot.named.iter().copied().filter(usable).collect();
    let mut rest: Vec<u16> = loot
        .named
        .iter()
        .copied()
        .filter(|id| !usable(id))
        .collect();
    for _ in 0..loot.named_drops {
        let pool = if first.is_empty() {
            &mut rest
        } else {
            &mut first
        };
        if pool.is_empty() {
            break;
        }
        let i = rng.index(pool.len());
        out.push(pool.swap_remove(i));
    }
    for &(id, chance) in loot.tokens {
        if rng.f32() < chance {
            out.push(id);
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Keeping it
// ---------------------------------------------------------------------------------------------

/// The account's raid keepsakes, the profile's `raid` section. A save from before it reads as
/// empty, so it needs no version bump.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(in crate::survivors) struct RaidVault {
    /// Eyes of Sulfuras held.
    pub eyes: u32,
    /// Sulfuron Ingots held.
    pub ingots: u32,
    /// Bound jewelry kept, by raid id.
    pub bound: BTreeSet<u16>,
    /// Sulfuras has been forged.
    pub sulfuras: bool,
}

impl RaidVault {
    /// Nothing kept, for a world without a profile.
    pub const EMPTY: Self = Self {
        eyes: 0,
        ingots: 0,
        bound: BTreeSet::new(),
        sulfuras: false,
    };

    /// The Bound jewelry kept, as jewelry keys.
    pub(in crate::survivors) fn bound_keys(&self) -> Vec<&'static str> {
        self.bound
            .iter()
            .filter_map(|&id| match raid_piece(id)?.kind {
                RaidKind::Bound(key) => Some(key),
                _ => None,
            })
            .collect()
    }
}

/// The jewelry piece keyed `key`, when it is raid Bound jewelry: what another player's account may
/// put on its hero from the start, never any other piece of the catalog.
pub(in crate::survivors) fn bound_piece(
    key: &str,
) -> Option<&'static super::super::jewelry::Piece> {
    RAID_ITEMS
        .iter()
        .any(|p| matches!(p.kind, RaidKind::Bound(k) if k == key))
        .then(|| super::super::jewelry::piece(key))
        .flatten()
}

/// With an Eye and an Ingot in `vault` and no Sulfuras yet, both go to the forge: true when it
/// lit, and Sulfuras is then the Warrior's, Paladin's and Shaman's to collect.
pub(in crate::survivors) fn forge(vault: &mut RaidVault) -> bool {
    if vault.sulfuras || vault.eyes == 0 || vault.ingots == 0 {
        return false;
    }
    vault.eyes -= 1;
    vault.ingots -= 1;
    vault.sulfuras = true;
    true
}

/// Where a raid piece won by a player of class `class` goes: that class's gear when it may wear
/// it, else the piece's own first class (a Greed win).
pub(in crate::survivors) fn keeper(piece: &RaidPiece, class: usize) -> usize {
    if piece.classes.contains(&class) {
        class
    } else {
        piece.item.class
    }
}

/// What keeping a raid piece came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum Kept {
    /// New to its keeper's gear, and whether it went on.
    Gear { equipped: bool },
    /// New Bound jewelry for the account.
    Bound,
    /// One more of a quest item; this many now held.
    Counted(u32),
    /// Already had it.
    Duplicate,
}

/// Keep `piece` in `gear` (its keeper's class's, [`keeper`]) or the account's `vault`.
pub(in crate::survivors) fn keep(
    piece: &RaidPiece,
    gear: Option<&mut GearSave>,
    vault: &mut RaidVault,
) -> Kept {
    match piece.kind {
        RaidKind::Gear => match gear.map(|g| g.collect(&piece.item)) {
            Some(super::save::Collected::New { equipped }) => Kept::Gear { equipped },
            _ => Kept::Duplicate,
        },
        RaidKind::Bound(_) => {
            if vault.bound.insert(piece.item.id) {
                Kept::Bound
            } else {
                Kept::Duplicate
            }
        }
        RaidKind::Token(Token::Eye) => {
            vault.eyes += 1;
            Kept::Counted(vault.eyes)
        }
        RaidKind::Token(Token::Ingot) => {
            vault.ingots += 1;
            Kept::Counted(vault.ingots)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::super::super::data::CLASSES;
    use super::super::super::raid::script_for;
    use super::super::{catalog, item};
    use super::*;

    fn named_of(boss: &str) -> Vec<&'static str> {
        loot_for(boss)
            .unwrap()
            .named
            .iter()
            .chain(loot_for(boss).unwrap().tokens.iter().map(|t| &t.0))
            .map(|&id| raid_piece(id).unwrap().item.name)
            .collect()
    }

    #[test]
    fn ids_are_unique_and_clear_of_the_class_catalog() {
        let mut seen = HashSet::new();
        for p in RAID_ITEMS {
            assert!(seen.insert(p.item.id), "id {} is used twice", p.item.id);
            assert!(p.item.id >= FIRST_ID, "{}", p.item.name);
            assert!(!p.classes.is_empty(), "{}", p.item.name);
            assert_eq!(p.item.class, p.classes[0], "{}", p.item.name);
            assert!(p.classes.iter().all(|&c| c < CLASSES.len()));
            assert!(!p.item.name.is_empty() && !p.item.icon.is_empty() && p.item.display_id > 0);
            if !matches!(p.kind, RaidKind::Token(_)) {
                assert!(!p.item.bonuses.is_empty(), "{} grants nothing", p.item.name);
            }
            // The catalog's lookup finds it, and nothing of the class catalog shares its id.
            assert!(std::ptr::eq(item(p.item.id).unwrap(), &p.item));
        }
        for items in catalog::CLASS_ITEMS {
            for i in *items {
                assert!(!seen.contains(&i.id), "{} clashes", i.name);
            }
        }
        let reals: HashSet<u32> = RAID_ITEMS.iter().map(|p| p.real).collect();
        assert_eq!(reals.len(), RAID_ITEMS.len(), "a 1.12 id is used twice");
        // ~25-35 named pieces beside the sets.
        let others = RAID_ITEMS.len() - 72;
        assert!((25..=35).contains(&others), "{others}");
    }

    #[test]
    fn every_class_has_a_full_tier_one_set() {
        for (class, def) in CLASSES.iter().enumerate() {
            let set = tier_set(class).unwrap();
            assert_eq!(set.class, class);
            let pieces: Vec<_> = set.pieces().collect();
            assert_eq!(pieces.len(), 8, "{}", def.name);
            for (p, slot) in pieces.iter().zip(Slot::ALL) {
                assert_eq!(p.item.slot, slot, "{}", p.item.name);
                assert_eq!(p.item.rarity, Rarity::Epic);
                assert_eq!(p.classes, [class]);
                assert_eq!(p.kind, RaidKind::Gear);
                assert!(std::ptr::eq(set_of(p.item.id).unwrap(), set));
            }
            assert!(tier_piece(class, MainHand).is_none());
            assert_eq!(
                set.bonuses.iter().map(|b| b.pieces).collect::<Vec<_>>(),
                [3, 5, 8]
            );
            for step in &set.bonuses {
                assert!(!step.text.is_empty());
                for spec in step.procs {
                    assert_eq!(spec.validate(), Ok(()), "{}", set.name);
                }
                let does = !step.stats.is_empty() || !step.procs.is_empty() || step.reflect > 0.0;
                assert!(does, "{} {} does nothing", set.name, step.pieces);
            }
        }
        assert!(set_of(2100).is_none(), "a named drop is in no set");
    }

    #[test]
    fn lucifron_and_magmadar_drop_gloves_and_legs_and_every_slot_drops() {
        assert_eq!(loot_for("Lucifron").unwrap().tier, [Hands]);
        assert_eq!(loot_for("Magmadar").unwrap().tier, [Legs]);
        for slot in &Slot::ALL[..8] {
            assert!(
                MC_LOOT.iter().any(|l| l.tier.contains(slot)),
                "no boss drops Tier 1 {slot:?}"
            );
        }
        let rag = named_of("Ragnaros");
        for name in [
            "Bonereaver's Edge",
            "Band of Accuria",
            "Eye of Sulfuras",
            "Sulfuron Ingot",
        ] {
            assert!(rag.contains(&name), "{name}");
        }
        let garr = named_of("Garr");
        assert!(garr.contains(&"Brutality Blade") && garr.contains(&"Aurastone Hammer"));
        let golemagg = named_of("Golemagg the Incinerator");
        assert!(
            golemagg.contains(&"Azuresong Mageblade") && golemagg.contains(&"Staff of Dominance")
        );
        assert!(golemagg.contains(&"Sulfuron Ingot"));
    }

    #[test]
    fn loot_rows_are_the_ten_scripted_bosses_and_name_real_pieces() {
        assert_eq!(MC_LOOT.len(), 10);
        for l in MC_LOOT {
            assert!(script_for(l.boss).is_some(), "{} has no script", l.boss);
            for &id in l.named.iter().chain(l.tokens.iter().map(|t| &t.0)) {
                let p = raid_piece(id).unwrap();
                assert_ne!(p.item.id, SULFURAS, "Sulfuras is forged, never dropped");
            }
            for &(_, chance) in l.tokens {
                assert!(chance > 0.0 && chance <= 1.0);
            }
        }
        // Every named piece but Sulfuras drops somewhere.
        for p in RAID_ITEMS.iter().filter(|p| set_of(p.item.id).is_none()) {
            if p.item.id == SULFURAS {
                continue;
            }
            assert!(
                MC_LOOT
                    .iter()
                    .any(|l| l.named.contains(&p.item.id)
                        || l.tokens.iter().any(|t| t.0 == p.item.id)),
                "nothing drops {}",
                p.item.name
            );
        }
        assert!(loot_for("Flamewaker").is_none());
    }

    #[test]
    fn a_kill_drops_two_or_three_pieces() {
        let mut rng = Rng(0xC0FFEE);
        for loot in MC_LOOT {
            for party in [&[6][..], &[0, 4], &[0, 1, 2, 3, 4]] {
                for _ in 0..50 {
                    let drop = roll_kill(loot, party, &mut rng);
                    let pieces = drop
                        .iter()
                        .filter(|&&id| !matches!(raid_piece(id).unwrap().kind, RaidKind::Token(_)))
                        .count();
                    assert!((2..=3).contains(&pieces), "{}: {drop:?}", loot.boss);
                    for &id in &drop {
                        if let Some(set) = set_of(id) {
                            assert!(party.contains(&set.class), "{} for {party:?}", id);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn solo_drops_are_for_the_player_and_a_big_party_gets_two_tier_pieces() {
        let lucifron = loot_for("Lucifron").unwrap();
        let mut rng = Rng(9);
        for _ in 0..40 {
            let drop = roll_kill(lucifron, &[6], &mut rng);
            assert!(drop.contains(&tier_piece(6, Hands).unwrap().item.id));
            let five = roll_kill(lucifron, &[0, 1, 2, 3, 4], &mut rng);
            assert_eq!(five.iter().filter(|&&id| set_of(id).is_some()).count(), 2);
        }
        // Named pieces the party can use come first: a lone hunter's Magmadar never drops
        // Earthshaker while Striker's Mark or the ring are left.
        let magmadar = loot_for("Magmadar").unwrap();
        for _ in 0..40 {
            let drop = roll_kill(magmadar, &[2], &mut rng);
            assert!(!drop.contains(&2102), "{drop:?}");
        }
        // The tokens come at their odds.
        let rag = loot_for("Ragnaros").unwrap();
        let (mut eyes, mut ingots) = (0, 0);
        for _ in 0..2000 {
            let drop = roll_kill(rag, &[0], &mut rng);
            eyes += drop.iter().filter(|&&id| id == EYE).count();
            ingots += drop.iter().filter(|&&id| id == INGOT).count();
        }
        assert!((120..300).contains(&eyes), "{eyes} eyes of 2000 at 10%");
        assert!(
            (850..1150).contains(&ingots),
            "{ingots} ingots of 2000 at 50%"
        );
    }

    #[test]
    fn sources_name_the_bosses() {
        let gloves = tier_piece(6, Hands).unwrap().item.id;
        assert_eq!(sources(gloves), [("Lucifron", 1.0), ("Shazzrah", 0.5)]);
        assert_eq!(sources(EYE), [("Ragnaros", 0.10)]);
        assert_eq!(
            sources(INGOT),
            [("Golemagg the Incinerator", 0.25), ("Ragnaros", 0.5)]
        );
        assert_eq!(sources(2105), [("Garr", 1.0 / 3.0)]);
        assert!(sources(SULFURAS).is_empty(), "forged, never dropped");
    }

    #[test]
    fn set_steps_add_at_three_five_and_eight() {
        let ids = |n: usize| -> Vec<u16> {
            tier_set(6)
                .unwrap()
                .pieces()
                .take(n)
                .map(|p| p.item.id)
                .collect()
        };
        assert_eq!(set_stats(&ids(2)), Bonuses::default());
        let three = set_stats(&ids(3));
        let mut want = Bonuses::default();
        want.add_all(&[dmg(0.03)]);
        assert_eq!(three, want);
        want.add_all(&[cdr(0.03)]);
        assert_eq!(set_stats(&ids(5)), want);
        assert_eq!(set_stats(&ids(8)), want, "the 8 adds a proc, not stats");
        assert_eq!(tier_set(6).unwrap().worn(&ids(8)), 8);
        // Another class's pieces count for nothing toward this set.
        assert_eq!(tier_set(0).unwrap().worn(&ids(8)), 0);
    }

    #[test]
    fn eight_pieces_grant_the_proc_and_mights_reflect() {
        let mut mage = HeroState::default();
        assert!(tier_applied(&mage, 6, 5));
        assert!(!tier_applied(&mage, 6, 8));
        apply_tier(&mut mage, 6, 8);
        assert!(mechanics::has(&mage, TIER_MECHANIC));
        assert!(tier_applied(&mage, 6, 8));
        apply_tier(&mut mage, 6, 7);
        assert!(!mechanics::has(&mage, TIER_MECHANIC));

        let mut warrior = HeroState::default();
        apply_tier(&mut warrior, 0, 8);
        assert_eq!(warrior.reflect, 0.25);
        assert!(!mechanics::has(&warrior, TIER_MECHANIC), "Might reflects");
        assert!(tier_applied(&warrior, 0, 8));
        apply_tier(&mut warrior, 0, 3);
        assert_eq!(warrior.reflect, 0.0);
    }

    #[test]
    fn keeping_gear_jewelry_and_tokens_and_forging_sulfuras() {
        let mut vault = RaidVault::default();
        let mut gear = GearSave::default();
        let helm = tier_piece(0, Head).unwrap();
        assert_eq!(
            keep(helm, Some(&mut gear), &mut vault),
            Kept::Gear { equipped: true }
        );
        assert!(gear.wears(helm.item.id));
        assert_eq!(keep(helm, Some(&mut gear), &mut vault), Kept::Duplicate);

        let band = raid_piece(2135).unwrap();
        assert_eq!(keep(band, None, &mut vault), Kept::Bound);
        assert_eq!(keep(band, None, &mut vault), Kept::Duplicate);
        assert_eq!(vault.bound_keys(), ["band_of_accuria"]);

        let eye = raid_piece(EYE).unwrap();
        let ingot = raid_piece(INGOT).unwrap();
        assert!(!forge(&mut vault), "nothing to forge");
        assert_eq!(keep(ingot, None, &mut vault), Kept::Counted(1));
        assert_eq!(keep(ingot, None, &mut vault), Kept::Counted(2));
        assert!(!forge(&mut vault), "an Ingot is not enough");
        assert_eq!(keep(eye, None, &mut vault), Kept::Counted(1));
        assert!(forge(&mut vault));
        assert_eq!((vault.eyes, vault.ingots, vault.sulfuras), (0, 1, true));
        assert_eq!(keep(eye, None, &mut vault), Kept::Counted(1));
        assert!(!forge(&mut vault), "forged once");

        // A Greed win of a piece for no class the winner plays goes to the piece's own class.
        let bonereaver = raid_piece(2115).unwrap();
        assert_eq!(keeper(bonereaver, 1), 1);
        assert_eq!(keeper(bonereaver, 6), 0);
        assert_eq!(keeper(raid_piece(SULFURAS).unwrap(), 5), 5);
    }

    #[test]
    fn the_vault_reads_old_saves_and_round_trips() {
        assert_eq!(
            toml::from_str::<RaidVault>("").unwrap(),
            RaidVault::default()
        );
        let vault = RaidVault {
            eyes: 1,
            ingots: 3,
            bound: [2135, 2136].into(),
            sulfuras: false,
        };
        let text = toml::to_string(&vault).unwrap();
        assert_eq!(toml::from_str::<RaidVault>(&text).unwrap(), vault);
    }

    #[test]
    fn bound_jewelry_is_the_jewelry_catalogs_piece() {
        for p in RAID_ITEMS {
            let RaidKind::Bound(key) = p.kind else {
                continue;
            };
            let j = super::super::super::jewelry::piece(key).unwrap();
            assert_eq!(j.name, p.item.name);
            assert_eq!(j.rarity, p.item.rarity);
            assert_eq!(j.display_id, p.item.display_id);
            assert_eq!(j.icon, p.item.icon);
            assert_eq!(j.bonuses, p.item.bonuses, "{}", p.item.name);
        }
    }

    /// The looks are real: every display id is an `ItemDisplayInfo.dbc` row whose inventory icon
    /// is the piece's, and every icon ships as `Interface\Icons\<name>.blp`.
    #[test]
    fn displays_and_icons_exist_in_the_client() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open the MPQ chain");
        let cat = benilla_formats::load_item_display_catalog(&mut chain).expect("ItemDisplayInfo");
        let mut bad = Vec::new();
        for p in RAID_ITEMS {
            let i = &p.item;
            let path = format!("Interface\\Icons\\{}", i.icon);
            match cat.get(i.display_id) {
                None => bad.push(format!("{}: no display {}", i.name, i.display_id)),
                Some(d)
                    if !d
                        .icon
                        .as_deref()
                        .is_some_and(|c| c.eq_ignore_ascii_case(&path)) =>
                {
                    bad.push(format!(
                        "{}: display {} shows {:?}",
                        i.name, i.display_id, d.icon
                    ))
                }
                Some(_) => {}
            }
            if !chain.contains(&format!("{path}.blp")) {
                bad.push(format!("{}: no {path}.blp", i.name));
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    /// The sets are real: each `ItemSet.dbc` row has the set's name, its 8 pieces' 1.12 ids, and
    /// bonuses at 3, 5 and 8 pieces.
    #[test]
    fn sets_match_item_set_dbc() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open the MPQ chain");
        let sets = benilla_formats::load_item_sets(&mut chain).expect("ItemSet");
        let mut bad = Vec::new();
        for set in &TIER_SETS {
            let Some(row) = (1..4096).find_map(|id| sets.set(id).filter(|r| r.name == set.name))
            else {
                bad.push(format!("{}: no ItemSet row of that name", set.name));
                continue;
            };
            let mut ours: Vec<u32> = set.pieces().map(|p| p.real).collect();
            let mut theirs: Vec<u32> = row.items.iter().copied().filter(|&i| i != 0).collect();
            ours.sort_unstable();
            theirs.sort_unstable();
            if ours != theirs {
                bad.push(format!("{}: ids {ours:?}, the client {theirs:?}", set.name));
            }
            let mut steps: Vec<u32> = row.bonuses.iter().map(|b| b.0).filter(|&n| n > 0).collect();
            steps.sort_unstable();
            steps.dedup();
            if steps != [3, 5, 8] {
                bad.push(format!("{}: thresholds {steps:?}", set.name));
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }
}
