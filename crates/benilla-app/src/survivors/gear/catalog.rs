//! The catalog: for each of the nine classes one piece per slot and rarity, 45 a class, named
//! for real 1.12 items and drawn with their `ItemDisplayInfo.dbc` looks. Common is starter and
//! vendor gear, Uncommon leveling greens and early blues, Rare the Dungeon Set 1 pieces, Epic
//! Tier 2 and Legendary Tier 3 with the era's legendaries where the class has one.
//!
//! Ids read `(class + 1) * 100 + rarity * 10 + slot`, for readability only: they are stable save
//! keys and are never computed. Bonuses follow the class: casters stack their schools,
//! [`StatKey::CritChance`], [`StatKey::Cooldown`] and [`StatKey::Area`]; melee stacks damage,
//! [`StatKey::CritDamage`], health and armor; hybrids mix. Each piece's share of its rarity's
//! budget follows its slot (main hand most, wrists least), and every Legendary main hand adds a
//! target, so a full Legendary set about doubles a hero's output.

use super::super::bonus::{Bonus, School, StatKey};
use super::Item;
use super::Rarity::{self, *};
use super::Slot::{self, *};

/// Every class's pieces, in [`CLASSES`](super::super::data::CLASSES) order.
pub(super) static CLASS_ITEMS: &[&[Item]] = &[
    WARRIOR, PALADIN, HUNTER, ROGUE, PRIEST, SHAMAN, MAGE, WARLOCK, DRUID,
];

const fn it(
    id: u16,
    class: usize,
    slot: Slot,
    rarity: Rarity,
    name: &'static str,
    display_id: u32,
    icon: &'static str,
    bonuses: &'static [Bonus],
) -> Item {
    Item {
        id,
        class,
        slot,
        rarity,
        name,
        display_id,
        icon,
        bonuses,
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
const fn nature(v: f32) -> Bonus {
    Bonus::school(School::Nature, v)
}
const fn frost(v: f32) -> Bonus {
    Bonus::school(School::Frost, v)
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

/// Warrior: vendor Chainmail, the Scarlet Monastery blues, Battlegear of Valor (Dungeon Set 1),
/// Battlegear of Wrath (Tier 2) and Dreadnaught's Battlegear (Tier 3), with Thunderfury.
#[rustfmt::skip]
static WARRIOR: &[Item] = &[
    it(100, 0, Head, Common, "Brigandine Helm", 27772, "INV_Helmet_09", &[dmg(0.01), hp(3.0)]),
    it(101, 0, Shoulder, Common, "Rough Bronze Shoulders", 25774, "INV_Shoulder_07", &[dmg(0.01), armor(0.004)]),
    it(102, 0, Chest, Common, "Chainmail Armor", 1019, "INV_Chest_Chain", &[hp(4.0), dmg(0.01)]),
    it(103, 0, Waist, Common, "Chainmail Belt", 13014, "INV_Belt_06", &[dmg(0.008), speed(0.003)]),
    it(104, 0, Legs, Common, "Chainmail Pants", 697, "INV_Pants_03", &[dmg(0.01), hp(3.0)]),
    it(105, 0, Feet, Common, "Chainmail Boots", 1020, "INV_Boots_01", &[speed(0.004), dmg(0.009)]),
    it(106, 0, Wrist, Common, "Chainmail Bracers", 3626, "INV_Bracer_07", &[critd(0.008), dmg(0.007)]),
    it(107, 0, Hands, Common, "Chainmail Gloves", 510, "INV_Gauntlets_04", &[dmg(0.009), critd(0.01)]),
    it(108, 0, MainHand, Common, "Worn Shortsword", 1542, "INV_Sword_04", &[dmg(0.015), critd(0.015)]),
    it(110, 0, Head, Uncommon, "Raging Berserker's Helm", 16083, "INV_Helmet_25", &[dmg(0.02), hp(6.0)]),
    it(111, 0, Shoulder, Uncommon, "Herod's Shoulder", 19851, "INV_Shoulder_23", &[dmg(0.015), armor(0.007)]),
    it(112, 0, Chest, Uncommon, "Scarlet Chestpiece", 4767, "INV_Chest_Chain", &[hp(6.0), dmg(0.02)]),
    it(113, 0, Waist, Uncommon, "Scarlet Belt", 29819, "INV_Belt_14", &[dmg(0.015), speed(0.006)]),
    it(114, 0, Legs, Uncommon, "Scarlet Leggings", 23199, "INV_Pants_05", &[dmg(0.02), hp(6.0)]),
    it(115, 0, Feet, Uncommon, "Scarlet Boots", 29820, "INV_Boots_Plate_06", &[speed(0.006), dmg(0.015)]),
    it(116, 0, Wrist, Uncommon, "Scarlet Wristguards", 29821, "INV_Bracer_16", &[critd(0.015), dmg(0.01)]),
    it(117, 0, Hands, Uncommon, "Scarlet Gauntlets", 16728, "INV_Gauntlets_10", &[dmg(0.015), critd(0.02)]),
    it(118, 0, MainHand, Uncommon, "Ravager", 19300, "INV_Axe_09", &[dmg(0.025), critd(0.025)]),
    it(120, 0, Head, Rare, "Helm of Valor", 31284, "INV_Helmet_02", &[dmg(0.03), hp(9.0), critd(0.035)]),
    it(121, 0, Shoulder, Rare, "Spaulders of Valor", 29964, "INV_Shoulder_30", &[dmg(0.03), armor(0.01), hp(8.0)]),
    it(122, 0, Chest, Rare, "Breastplate of Valor", 29958, "INV_Chest_Plate03", &[hp(10.0), dmg(0.035), armor(0.015)]),
    it(123, 0, Waist, Rare, "Belt of Valor", 29959, "INV_Belt_34", &[dmg(0.02), speed(0.009), hp(7.0)]),
    it(124, 0, Legs, Rare, "Legplates of Valor", 29963, "INV_Pants_04", &[dmg(0.03), hp(9.0), armor(0.01)]),
    it(125, 0, Feet, Rare, "Boots of Valor", 29960, "INV_Boots_Plate_03", &[speed(0.01), dmg(0.025), hp(7.0)]),
    it(126, 0, Wrist, Rare, "Bracers of Valor", 29961, "INV_Bracer_18", &[critd(0.025), dmg(0.02), armor(0.008)]),
    it(127, 0, Hands, Rare, "Gauntlets of Valor", 29962, "INV_Gauntlets_26", &[dmg(0.025), critd(0.03), crit(0.007)]),
    it(128, 0, MainHand, Rare, "Dal'Rend's Sacred Charge", 20073, "INV_Sword_43", &[dmg(0.035), critd(0.045), crit(0.01)]),
    it(130, 0, Head, Epic, "Helm of Wrath", 34215, "INV_Helmet_71", &[dmg(0.04), hp(12.0), critd(0.05)]),
    it(131, 0, Shoulder, Epic, "Pauldrons of Wrath", 34253, "INV_Shoulder_34", &[dmg(0.04), armor(0.015), hp(11.0)]),
    it(132, 0, Chest, Epic, "Breastplate of Wrath", 33983, "INV_Chest_Plate16", &[hp(14.0), dmg(0.045), armor(0.02)]),
    it(133, 0, Waist, Epic, "Waistband of Wrath", 33990, "INV_Belt_09", &[dmg(0.03), speed(0.01), hp(9.0)]),
    it(134, 0, Legs, Epic, "Legplates of Wrath", 33986, "INV_Pants_04", &[dmg(0.04), hp(12.0), armor(0.015)]),
    it(135, 0, Feet, Epic, "Sabatons of Wrath", 33989, "INV_Boots_Plate_04", &[speed(0.015), dmg(0.035), hp(10.0)]),
    it(136, 0, Wrist, Epic, "Bracelets of Wrath", 33982, "INV_Bracer_19", &[critd(0.03), dmg(0.025), armor(0.01)]),
    it(137, 0, Hands, Epic, "Gauntlets of Wrath", 33984, "INV_Gauntlets_10", &[dmg(0.035), critd(0.04), crit(0.01)]),
    it(138, 0, MainHand, Epic, "Ashkandi, Greatsword of the Brotherhood", 31821, "INV_Sword_50", &[dmg(0.05), critd(0.06), crit(0.015)]),
    it(140, 0, Head, Legendary, "Dreadnaught Helmet", 35060, "INV_Helmet_58", &[dmg(0.055), hp(16.0), critd(0.065), armor(0.02)]),
    it(141, 0, Shoulder, Legendary, "Dreadnaught Pauldrons", 35059, "INV_Shoulder_29", &[dmg(0.05), armor(0.02), hp(15.0), critd(0.06)]),
    it(142, 0, Chest, Legendary, "Dreadnaught Breastplate", 35049, "INV_Chest_Plate02", &[hp(18.0), dmg(0.06), armor(0.025), leech(0.006)]),
    it(143, 0, Waist, Legendary, "Dreadnaught Waistguard", 35058, "INV_Belt_27", &[dmg(0.04), speed(0.015), hp(12.0), critd(0.05)]),
    it(144, 0, Legs, Legendary, "Dreadnaught Legplates", 35051, "INV_Pants_Plate_05", &[dmg(0.055), hp(16.0), armor(0.02), critd(0.065)]),
    it(145, 0, Feet, Legendary, "Dreadnaught Sabatons", 35052, "INV_Boots_Plate_06", &[speed(0.02), dmg(0.045), hp(14.0), armor(0.02)]),
    it(146, 0, Wrist, Legendary, "Dreadnaught Bracers", 35044, "INV_Bracer_15", &[critd(0.04), dmg(0.035), armor(0.015), hp(10.0)]),
    it(147, 0, Hands, Legendary, "Dreadnaught Gauntlets", 35050, "INV_Gauntlets_28", &[dmg(0.045), critd(0.055), crit(0.015), hp(14.0)]),
    it(148, 0, MainHand, Legendary, "Thunderfury, Blessed Blade of the Windseeker", 30606, "INV_Sword_39", &[dmg(0.065), critd(0.08), crit(0.02), targets(1.0)]),
];

/// Paladin: vendor Banded mail, Jouster's greens, Lightforge Armor (Dungeon Set 1), Judgement Armor
/// (Tier 2) and Redemption Armor (Tier 3), with Sulfuras.
#[rustfmt::skip]
static PALADIN: &[Item] = &[
    it(200, 1, Head, Common, "Banded Helm", 2617, "INV_Helmet_06", &[holy(0.015), hp(3.0)]),
    it(201, 1, Shoulder, Common, "Banded Pauldrons", 4455, "INV_Shoulder_07", &[holy(0.015), armor(0.004)]),
    it(202, 1, Chest, Common, "Banded Armor", 2968, "INV_Chest_Plate01", &[hp(4.0), holy(0.015)]),
    it(203, 1, Waist, Common, "Banded Girdle", 2970, "INV_Belt_01", &[holy(0.01), regen(0.1)]),
    it(204, 1, Legs, Common, "Banded Leggings", 2969, "INV_Pants_03", &[holy(0.015), hp(3.0)]),
    it(205, 1, Feet, Common, "Banded Boots", 2971, "INV_Boots_01", &[speed(0.004), holy(0.015)]),
    it(206, 1, Wrist, Common, "Banded Bracers", 3620, "INV_Bracer_07", &[armor(0.003), holy(0.01)]),
    it(207, 1, Hands, Common, "Banded Gauntlets", 2973, "INV_Gauntlets_04", &[holy(0.015), crit(0.003)]),
    it(208, 1, MainHand, Common, "Battleworn Hammer", 1477, "INV_Hammer_03", &[dmg(0.015), holy(0.02)]),
    it(210, 1, Head, Uncommon, "Jouster's Visor", 1126, "INV_Helmet_03", &[holy(0.025), hp(6.0)]),
    it(211, 1, Shoulder, Uncommon, "Jouster's Pauldrons", 22849, "INV_Shoulder_02", &[holy(0.025), armor(0.007)]),
    it(212, 1, Chest, Uncommon, "Jouster's Chestplate", 2974, "INV_Chest_Plate01", &[hp(6.0), holy(0.03)]),
    it(213, 1, Waist, Uncommon, "Jouster's Girdle", 2975, "INV_Belt_01", &[holy(0.02), regen(0.1)]),
    it(214, 1, Legs, Uncommon, "Jouster's Legplates", 2976, "INV_Pants_03", &[holy(0.025), hp(6.0)]),
    it(215, 1, Feet, Uncommon, "Jouster's Greaves", 2977, "INV_Boots_01", &[speed(0.006), holy(0.02)]),
    it(216, 1, Wrist, Uncommon, "Jouster's Wristguards", 3624, "INV_Bracer_03", &[armor(0.005), holy(0.015)]),
    it(217, 1, Hands, Uncommon, "Jouster's Gauntlets", 2979, "INV_Gauntlets_04", &[holy(0.02), crit(0.005)]),
    it(218, 1, MainHand, Uncommon, "Mograine's Might", 8587, "INV_Hammer_09", &[dmg(0.025), holy(0.03)]),
    it(220, 1, Head, Rare, "Lightforge Helm", 31089, "INV_Helmet_08", &[holy(0.04), hp(9.0), armor(0.01)]),
    it(221, 1, Shoulder, Rare, "Lightforge Spaulders", 29971, "INV_Shoulder_10", &[holy(0.04), armor(0.01), hp(8.0)]),
    it(222, 1, Chest, Rare, "Lightforge Breastplate", 29969, "INV_Chest_Plate03", &[hp(10.0), holy(0.045), armor(0.015)]),
    it(223, 1, Waist, Rare, "Lightforge Belt", 29966, "INV_Belt_11", &[holy(0.03), regen(0.2), hp(7.0)]),
    it(224, 1, Legs, Rare, "Lightforge Legplates", 29972, "INV_Pants_04", &[holy(0.04), hp(9.0), armor(0.01)]),
    it(225, 1, Feet, Rare, "Lightforge Boots", 29967, "INV_Boots_Plate_03", &[speed(0.01), holy(0.035), hp(7.0)]),
    it(226, 1, Wrist, Rare, "Lightforge Bracers", 29968, "INV_Bracer_14", &[armor(0.008), holy(0.025), regen(0.1)]),
    it(227, 1, Hands, Rare, "Lightforge Gauntlets", 29970, "INV_Gauntlets_19", &[holy(0.035), crit(0.007), area(0.015)]),
    it(228, 1, MainHand, Rare, "Hammer of the Grand Crusader", 35251, "INV_Mace_26", &[dmg(0.035), holy(0.05), critd(0.045)]),
    it(230, 1, Head, Epic, "Judgement Crown", 34216, "INV_Helmet_74", &[holy(0.06), hp(12.0), armor(0.015)]),
    it(231, 1, Shoulder, Epic, "Judgement Spaulders", 34258, "INV_Shoulder_37", &[holy(0.055), armor(0.015), hp(11.0)]),
    it(232, 1, Chest, Epic, "Judgement Breastplate", 33635, "INV_Chest_Plate03", &[hp(14.0), holy(0.065), armor(0.02)]),
    it(233, 1, Waist, Epic, "Judgement Belt", 33633, "INV_Belt_27", &[holy(0.04), regen(0.2), hp(9.0)]),
    it(234, 1, Legs, Epic, "Judgement Legplates", 33637, "INV_Pants_04", &[holy(0.06), hp(12.0), armor(0.015)]),
    it(235, 1, Feet, Epic, "Judgement Sabatons", 33638, "INV_Boots_Plate_09", &[speed(0.015), holy(0.045), hp(10.0)]),
    it(236, 1, Wrist, Epic, "Judgement Bindings", 33634, "INV_Bracer_18", &[armor(0.01), holy(0.035), regen(0.2)]),
    it(237, 1, Hands, Epic, "Judgement Gauntlets", 33636, "INV_Gauntlets_29", &[holy(0.045), crit(0.01), area(0.025)]),
    it(238, 1, MainHand, Epic, "Lok'amir il Romathis", 33731, "INV_Mace22", &[dmg(0.05), holy(0.07), critd(0.06)]),
    it(240, 1, Head, Legendary, "Redemption Headpiece", 35612, "INV_Helmet_15", &[holy(0.075), hp(16.0), armor(0.02), regen(0.4)]),
    it(241, 1, Shoulder, Legendary, "Redemption Spaulders", 35617, "INV_Shoulder_14", &[holy(0.07), armor(0.02), hp(15.0), area(0.035)]),
    it(242, 1, Chest, Legendary, "Redemption Tunic", 35618, "INV_Chest_Chain_15", &[hp(18.0), holy(0.085), armor(0.025), regen(0.4)]),
    it(243, 1, Waist, Legendary, "Redemption Girdle", 35614, "INV_Belt_22", &[holy(0.055), regen(0.3), hp(12.0), area(0.03)]),
    it(244, 1, Legs, Legendary, "Redemption Legguards", 35616, "INV_Pants_Mail_15", &[holy(0.075), hp(16.0), armor(0.02), critd(0.065)]),
    it(245, 1, Feet, Legendary, "Redemption Boots", 35613, "INV_Boots_Chain_05", &[speed(0.02), holy(0.065), hp(14.0), armor(0.02)]),
    it(246, 1, Wrist, Legendary, "Redemption Wristguards", 35619, "INV_Bracer_02", &[armor(0.015), holy(0.05), regen(0.2), hp(10.0)]),
    it(247, 1, Hands, Legendary, "Redemption Handguards", 35615, "INV_Gauntlets_25", &[holy(0.065), crit(0.015), area(0.03), hp(14.0)]),
    it(248, 1, MainHand, Legendary, "Sulfuras, Hand of Ragnaros", 29698, "INV_Hammer_Unique_Sulfuras", &[dmg(0.065), holy(0.09), critd(0.08), targets(1.0)]),
];

/// Hunter: vendor Rawhide, Huntsman's greens, Beaststalker Armor (Dungeon Set 1), Dragonstalker
/// Armor (Tier 2) and Cryptstalker Armor (Tier 3).
#[rustfmt::skip]
static HUNTER: &[Item] = &[
    it(300, 2, Head, Common, "Studded Leather Cap", 4256, "INV_Helmet_04", &[dmg(0.01), crit(0.003)]),
    it(301, 2, Shoulder, Common, "Rawhide Shoulderpads", 14203, "INV_Shoulder_08", &[dmg(0.01), crit(0.003)]),
    it(302, 2, Chest, Common, "Rawhide Tunic", 12253, "INV_Chest_Fur", &[dmg(0.01), hp(4.0)]),
    it(303, 2, Waist, Common, "Rawhide Belt", 4852, "INV_Belt_06", &[dmg(0.008), speed(0.003)]),
    it(304, 2, Legs, Common, "Rugged Trapper's Pants", 9975, "INV_Pants_02", &[dmg(0.01), crit(0.003)]),
    it(305, 2, Feet, Common, "Rugged Trapper's Boots", 3738, "INV_Boots_03", &[speed(0.004), dmg(0.009)]),
    it(306, 2, Wrist, Common, "Rawhide Bracers", 3613, "INV_Bracer_04", &[crit(0.002), dmg(0.007)]),
    it(307, 2, Hands, Common, "Rawhide Gloves", 14200, "INV_Gauntlets_05", &[dmg(0.009), crit(0.003)]),
    it(308, 2, MainHand, Common, "Worn Axe", 1382, "INV_Axe_01", &[dmg(0.015), crit(0.004)]),
    it(310, 2, Head, Uncommon, "Huntsman's Cap", 1494, "INV_Helmet_04", &[dmg(0.02), crit(0.006)]),
    it(311, 2, Shoulder, Uncommon, "Huntsman's Shoulders", 4869, "INV_Shoulder_06", &[dmg(0.015), crit(0.005)]),
    it(312, 2, Chest, Uncommon, "Huntsman's Armor", 1995, "INV_Chest_Wolf", &[dmg(0.02), hp(6.0)]),
    it(313, 2, Waist, Uncommon, "Huntsman's Belt", 3335, "INV_Belt_01", &[dmg(0.015), speed(0.006)]),
    it(314, 2, Legs, Uncommon, "Huntsman's Leggings", 10450, "INV_Pants_04", &[dmg(0.02), crit(0.006)]),
    it(315, 2, Feet, Uncommon, "Huntsman's Boots", 14534, "INV_Boots_Wolf", &[speed(0.006), dmg(0.015)]),
    it(316, 2, Wrist, Uncommon, "Huntsman's Bands", 14535, "INV_Bracer_08", &[crit(0.004), dmg(0.01)]),
    it(317, 2, Hands, Uncommon, "Huntsman's Gloves", 14536, "INV_Gauntlets_05", &[dmg(0.015), crit(0.005)]),
    it(318, 2, MainHand, Uncommon, "Fiery War Axe", 8499, "INV_Axe_10", &[dmg(0.025), crit(0.007)]),
    it(320, 2, Head, Rare, "Beaststalker's Cap", 31410, "INV_Helmet_24", &[dmg(0.03), crit(0.009), pickup(0.03)]),
    it(321, 2, Shoulder, Rare, "Beaststalker's Mantle", 31409, "INV_Shoulder_10", &[dmg(0.03), crit(0.008), hp(8.0)]),
    it(322, 2, Chest, Rare, "Beaststalker's Tunic", 31402, "INV_Chest_Chain_03", &[dmg(0.035), hp(10.0), crit(0.01)]),
    it(323, 2, Waist, Rare, "Beaststalker's Belt", 31404, "INV_Belt_28", &[dmg(0.02), speed(0.009), pickup(0.02)]),
    it(324, 2, Legs, Rare, "Beaststalker's Pants", 31403, "INV_Pants_03", &[dmg(0.03), crit(0.009), hp(9.0)]),
    it(325, 2, Feet, Rare, "Beaststalker's Boots", 31407, "INV_Boots_Plate_07", &[speed(0.01), dmg(0.025), pickup(0.025)]),
    it(326, 2, Wrist, Rare, "Beaststalker's Bindings", 31405, "INV_Bracer_17", &[crit(0.006), dmg(0.02), area(0.015)]),
    it(327, 2, Hands, Rare, "Beaststalker's Gloves", 31406, "INV_Gauntlets_10", &[dmg(0.025), crit(0.007), critd(0.03)]),
    it(328, 2, MainHand, Rare, "Blackhand Doomsaw", 22210, "INV_Spear_01", &[dmg(0.035), crit(0.01), area(0.025)]),
    it(330, 2, Head, Epic, "Dragonstalker's Helm", 34085, "INV_Helmet_05", &[dmg(0.04), crit(0.01), pickup(0.04)]),
    it(331, 2, Shoulder, Epic, "Dragonstalker's Spaulders", 34091, "INV_Shoulder_10", &[dmg(0.04), crit(0.01), hp(11.0)]),
    it(332, 2, Chest, Epic, "Dragonstalker's Breastplate", 33667, "INV_Chest_Chain_03", &[dmg(0.045), hp(14.0), crit(0.015)]),
    it(333, 2, Waist, Epic, "Dragonstalker's Belt", 33665, "INV_Belt_28", &[dmg(0.03), speed(0.01), pickup(0.03)]),
    it(334, 2, Legs, Epic, "Dragonstalker's Legguards", 33672, "INV_Pants_03", &[dmg(0.04), crit(0.01), hp(12.0)]),
    it(335, 2, Feet, Epic, "Dragonstalker's Greaves", 33669, "INV_Boots_Plate_07", &[speed(0.015), dmg(0.035), pickup(0.035)]),
    it(336, 2, Wrist, Epic, "Dragonstalker's Bracers", 33666, "INV_Bracer_17", &[crit(0.008), dmg(0.025), area(0.02)]),
    it(337, 2, Hands, Epic, "Dragonstalker's Gauntlets", 33668, "INV_Gauntlets_10", &[dmg(0.035), crit(0.01), critd(0.04)]),
    it(338, 2, MainHand, Epic, "Barb of the Sand Reaver", 34250, "INV_Weapon_Halberd_AhnQiraj", &[dmg(0.05), crit(0.015), area(0.035)]),
    it(340, 2, Head, Legendary, "Cryptstalker Headpiece", 35412, "INV_Helmet_15", &[dmg(0.055), crit(0.015), pickup(0.055), speed(0.02)]),
    it(341, 2, Shoulder, Legendary, "Cryptstalker Spaulders", 35414, "INV_Shoulder_14", &[dmg(0.05), crit(0.015), hp(15.0), area(0.035)]),
    it(342, 2, Chest, Legendary, "Cryptstalker Tunic", 35415, "INV_Chest_Chain_15", &[dmg(0.06), hp(18.0), crit(0.02), armor(0.025)]),
    it(343, 2, Waist, Legendary, "Cryptstalker Girdle", 35410, "INV_Belt_22", &[dmg(0.04), speed(0.015), pickup(0.04), crit(0.01)]),
    it(344, 2, Legs, Legendary, "Cryptstalker Legguards", 35413, "INV_Pants_Mail_15", &[dmg(0.055), crit(0.015), hp(16.0), area(0.04)]),
    it(345, 2, Feet, Legendary, "Cryptstalker Boots", 35409, "INV_Boots_Chain_05", &[speed(0.02), dmg(0.045), pickup(0.045), crit(0.015)]),
    it(346, 2, Wrist, Legendary, "Cryptstalker Wristguards", 35416, "INV_Bracer_02", &[crit(0.01), dmg(0.035), area(0.025), hp(10.0)]),
    it(347, 2, Hands, Legendary, "Cryptstalker Handwraps", 35411, "INV_Gauntlets_25", &[dmg(0.045), crit(0.015), critd(0.055), speed(0.02)]),
    it(348, 2, MainHand, Legendary, "Brimstone Staff", 35242, "INV_Staff_31", &[dmg(0.065), crit(0.02), area(0.045), targets(1.0)]),
];

/// Rogue: vendor Cured Leather, the Deadmines' Defias blues, Shadowcraft Armor (Dungeon Set 1),
/// Bloodfang Armor (Tier 2) and Bonescythe Armor (Tier 3).
#[rustfmt::skip]
static ROGUE: &[Item] = &[
    it(400, 3, Head, Common, "Red Defias Mask", 8266, "INV_Misc_Bandana_03", &[dmg(0.01), crit(0.003)]),
    it(401, 3, Shoulder, Common, "Cured Leather Shoulderpads", 4902, "INV_Shoulder_03", &[dmg(0.01), critd(0.01)]),
    it(402, 3, Chest, Common, "Cured Leather Armor", 2106, "INV_Chest_Wolf", &[dmg(0.01), hp(4.0)]),
    it(403, 3, Waist, Common, "Cured Leather Belt", 3247, "INV_Belt_01", &[dmg(0.008), speed(0.003)]),
    it(404, 3, Legs, Common, "Footpad's Pants", 2176, "INV_Pants_Wolf", &[dmg(0.01), crit(0.003)]),
    it(405, 3, Feet, Common, "Footpad's Shoes", 2094, "INV_Boots_05", &[speed(0.004), dmg(0.009)]),
    it(406, 3, Wrist, Common, "Cured Leather Bracers", 3597, "INV_Bracer_04", &[crit(0.002), dmg(0.007)]),
    it(407, 3, Hands, Common, "Cured Leather Gloves", 2359, "INV_Gauntlets_05", &[crit(0.003), dmg(0.009)]),
    it(408, 3, MainHand, Common, "Worn Dagger", 2704, "INV_Weapon_ShortBlade_05", &[dmg(0.015), critd(0.015)]),
    it(410, 3, Head, Uncommon, "Nightscape Headband", 8753, "INV_Helmet_04", &[dmg(0.02), crit(0.006)]),
    it(411, 3, Shoulder, Uncommon, "Nightscape Shoulders", 14212, "INV_Shoulder_08", &[dmg(0.015), critd(0.02)]),
    it(412, 3, Chest, Uncommon, "Blackened Defias Armor", 8710, "INV_Chest_Leather_09", &[dmg(0.02), hp(6.0)]),
    it(413, 3, Waist, Uncommon, "Blackened Defias Belt", 9584, "INV_Belt_03", &[dmg(0.015), speed(0.006)]),
    it(414, 3, Legs, Uncommon, "Blackened Defias Leggings", 3248, "INV_Pants_02", &[dmg(0.02), crit(0.006)]),
    it(415, 3, Feet, Uncommon, "Blackened Defias Boots", 12197, "INV_Boots_07", &[speed(0.006), dmg(0.015)]),
    it(416, 3, Wrist, Uncommon, "Bandit Bracers", 3618, "INV_Bracer_04", &[crit(0.004), dmg(0.01)]),
    it(417, 3, Hands, Uncommon, "Blackened Defias Gloves", 13754, "INV_Gauntlets_05", &[crit(0.005), dmg(0.015)]),
    it(418, 3, MainHand, Uncommon, "Cruel Barb", 3764, "INV_Sword_09", &[dmg(0.025), critd(0.025)]),
    it(420, 3, Head, Rare, "Shadowcraft Cap", 31501, "INV_Helmet_41", &[dmg(0.03), crit(0.009), critd(0.035)]),
    it(421, 3, Shoulder, Rare, "Shadowcraft Spaulders", 31343, "INV_Shoulder_23", &[dmg(0.03), critd(0.035), crit(0.008)]),
    it(422, 3, Chest, Rare, "Shadowcraft Tunic", 31004, "INV_Chest_Cloth_07", &[dmg(0.035), hp(10.0), crit(0.01)]),
    it(423, 3, Waist, Rare, "Shadowcraft Belt", 31000, "INV_Belt_23", &[dmg(0.02), speed(0.009), crit(0.007)]),
    it(424, 3, Legs, Rare, "Shadowcraft Pants", 31008, "INV_Pants_06", &[dmg(0.03), crit(0.009), hp(9.0)]),
    it(425, 3, Feet, Rare, "Shadowcraft Boots", 31001, "INV_Boots_08", &[speed(0.01), dmg(0.025), crit(0.007)]),
    it(426, 3, Wrist, Rare, "Shadowcraft Bracers", 31003, "INV_Bracer_02", &[crit(0.006), dmg(0.02), critd(0.025)]),
    it(427, 3, Hands, Rare, "Shadowcraft Gloves", 31007, "INV_Gauntlets_21", &[crit(0.007), dmg(0.025), critd(0.03)]),
    it(428, 3, MainHand, Rare, "Shadowfang", 20089, "INV_Sword_13", &[dmg(0.035), critd(0.045), crit(0.01)]),
    it(430, 3, Head, Epic, "Bloodfang Hood", 33743, "INV_Helmet_41", &[dmg(0.04), crit(0.01), critd(0.05)]),
    it(431, 3, Shoulder, Epic, "Bloodfang Spaulders", 33653, "INV_Shoulder_23", &[dmg(0.04), critd(0.045), crit(0.01)]),
    it(432, 3, Chest, Epic, "Bloodfang Chestpiece", 31113, "INV_Chest_Cloth_07", &[dmg(0.045), hp(14.0), crit(0.015)]),
    it(433, 3, Waist, Epic, "Bloodfang Belt", 31110, "INV_Belt_23", &[dmg(0.03), speed(0.01), crit(0.009)]),
    it(434, 3, Legs, Epic, "Bloodfang Pants", 31115, "INV_Pants_06", &[dmg(0.04), crit(0.01), hp(12.0)]),
    it(435, 3, Feet, Epic, "Bloodfang Boots", 31111, "INV_Boots_08", &[speed(0.015), dmg(0.035), crit(0.01)]),
    it(436, 3, Wrist, Epic, "Bloodfang Bracers", 31127, "INV_Bracer_02", &[crit(0.008), dmg(0.025), critd(0.03)]),
    it(437, 3, Hands, Epic, "Bloodfang Gloves", 31114, "INV_Gauntlets_21", &[crit(0.01), dmg(0.035), critd(0.04)]),
    it(438, 3, MainHand, Epic, "Maladath, Runed Blade of the Black Flight", 31855, "INV_Sword_49", &[dmg(0.05), critd(0.06), crit(0.015)]),
    it(440, 3, Head, Legendary, "Bonescythe Helmet", 35061, "INV_Helmet_58", &[dmg(0.055), crit(0.015), critd(0.065), speed(0.02)]),
    it(441, 3, Shoulder, Legendary, "Bonescythe Pauldrons", 35064, "INV_Shoulder_29", &[dmg(0.05), critd(0.06), crit(0.015), hp(15.0)]),
    it(442, 3, Chest, Legendary, "Bonescythe Breastplate", 35054, "INV_Chest_Plate02", &[dmg(0.06), hp(18.0), crit(0.02), leech(0.006)]),
    it(443, 3, Waist, Legendary, "Bonescythe Waistguard", 36349, "INV_Belt_27", &[dmg(0.04), speed(0.015), crit(0.01), critd(0.05)]),
    it(444, 3, Legs, Legendary, "Bonescythe Legplates", 35065, "INV_Pants_Plate_05", &[dmg(0.055), crit(0.015), hp(16.0), critd(0.065)]),
    it(445, 3, Feet, Legendary, "Bonescythe Sabatons", 35062, "INV_Boots_Plate_06", &[speed(0.02), dmg(0.045), crit(0.015), critd(0.055)]),
    it(446, 3, Wrist, Legendary, "Bonescythe Bracers", 35053, "INV_Bracer_15", &[crit(0.01), dmg(0.035), critd(0.04), speed(0.015)]),
    it(447, 3, Hands, Legendary, "Bonescythe Gauntlets", 35055, "INV_Gauntlets_28", &[crit(0.015), dmg(0.045), critd(0.055), leech(0.005)]),
    it(448, 3, MainHand, Legendary, "The Hungering Cold", 35575, "INV_Sword_62", &[dmg(0.065), critd(0.08), crit(0.02), targets(1.0)]),
];

/// Priest: Neophyte's and Brocade whites, Celestial greens, Vestments of the Devout (Dungeon Set
/// 1), Vestments of Transcendence (Tier 2) and Vestments of Faith (Tier 3), with Atiesh.
#[rustfmt::skip]
static PRIEST: &[Item] = &[
    it(500, 4, Head, Common, "Embroidered Hat", 13220, "INV_Helmet_04", &[holy(0.015), cdr(0.003)]),
    it(501, 4, Shoulder, Common, "Brocade Shoulderpads", 10178, "INV_Shoulder_06", &[holy(0.015), area(0.007)]),
    it(502, 4, Chest, Common, "Neophyte's Robe", 24974, "INV_Chest_Cloth_13", &[holy(0.015), hp(4.0)]),
    it(503, 4, Waist, Common, "Brocade Cord", 10087, "INV_Belt_01", &[holy(0.01), cdr(0.002)]),
    it(504, 4, Legs, Common, "Neophyte's Pants", 11166, "INV_Pants_01", &[holy(0.015), hp(3.0)]),
    it(505, 4, Feet, Common, "Neophyte's Boots", 10099, "INV_Boots_01", &[speed(0.004), holy(0.015)]),
    it(506, 4, Wrist, Common, "Brocade Bracers", 16897, "INV_Bracer_16", &[holy(0.01), regen(0.1)]),
    it(507, 4, Hands, Common, "Brocade Gloves", 12985, "INV_Gauntlets_06", &[holy(0.015), crit(0.003)]),
    it(508, 4, MainHand, Common, "Worn Mace", 1161, "INV_Mace_01", &[dmg(0.015), cdr(0.004)]),
    it(510, 4, Head, Uncommon, "Celestial Crown", 27797, "INV_Helmet_13", &[holy(0.025), cdr(0.006)]),
    it(511, 4, Shoulder, Uncommon, "Celestial Pauldrons", 21771, "INV_Shoulder_02", &[shadow(0.025), area(0.01)]),
    it(512, 4, Chest, Uncommon, "Celestial Tunic", 26254, "INV_Chest_Cloth_28", &[holy(0.03), hp(6.0)]),
    it(513, 4, Waist, Uncommon, "Celestial Belt", 26252, "INV_Belt_29", &[shadow(0.02), cdr(0.004)]),
    it(514, 4, Legs, Uncommon, "Celestial Kilt", 26260, "INV_Pants_13", &[holy(0.025), hp(6.0)]),
    it(515, 4, Feet, Uncommon, "Celestial Slippers", 26261, "INV_Boots_05", &[speed(0.006), shadow(0.02)]),
    it(516, 4, Wrist, Uncommon, "Celestial Bindings", 26253, "INV_Bracer_17", &[holy(0.015), regen(0.1)]),
    it(517, 4, Hands, Uncommon, "Celestial Handwraps", 26258, "INV_Gauntlets_20", &[shadow(0.02), crit(0.005)]),
    it(518, 4, MainHand, Uncommon, "Staff of Westfall", 2509, "INV_Staff_03", &[dmg(0.025), cdr(0.007)]),
    it(520, 4, Head, Rare, "Devout Crown", 31104, "INV_Crown_01", &[holy(0.04), cdr(0.009), regen(0.2)]),
    it(521, 4, Shoulder, Rare, "Devout Mantle", 31103, "INV_Shoulder_02", &[holy(0.04), area(0.02), hp(8.0)]),
    it(522, 4, Chest, Rare, "Devout Robe", 30422, "INV_Chest_Cloth_11", &[holy(0.045), hp(10.0), regen(0.2)]),
    it(523, 4, Waist, Rare, "Devout Belt", 30425, "INV_Belt_10", &[holy(0.03), cdr(0.007), regen(0.2)]),
    it(524, 4, Legs, Rare, "Devout Skirt", 30423, "INV_Pants_08", &[holy(0.04), hp(9.0), area(0.02)]),
    it(525, 4, Feet, Rare, "Devout Sandals", 30429, "INV_Boots_05", &[speed(0.01), holy(0.035), regen(0.2)]),
    it(526, 4, Wrist, Rare, "Devout Bracers", 30426, "INV_Belt_31", &[holy(0.025), regen(0.1), cdr(0.006)]),
    it(527, 4, Hands, Rare, "Devout Gloves", 30427, "INV_Gauntlets_14", &[holy(0.035), crit(0.007), cdr(0.007)]),
    it(528, 4, MainHand, Rare, "Headmaster's Charge", 35241, "INV_Staff_13", &[dmg(0.035), cdr(0.01), area(0.025)]),
    it(530, 4, Head, Epic, "Halo of Transcendence", 34054, "INV_Helmet_24", &[holy(0.06), cdr(0.01), regen(0.3)]),
    it(531, 4, Shoulder, Epic, "Pauldrons of Transcendence", 34048, "INV_Shoulder_02", &[shadow(0.055), area(0.025), hp(11.0)]),
    it(532, 4, Chest, Epic, "Robes of Transcendence", 34047, "INV_Chest_Cloth_03", &[holy(0.065), hp(14.0), regen(0.3)]),
    it(533, 4, Waist, Epic, "Belt of Transcendence", 34053, "INV_Belt_22", &[shadow(0.04), cdr(0.009), regen(0.2)]),
    it(534, 4, Legs, Epic, "Leggings of Transcendence", 34049, "INV_Pants_08", &[holy(0.06), hp(12.0), area(0.03)]),
    it(535, 4, Feet, Epic, "Boots of Transcendence", 34055, "INV_Boots_07", &[speed(0.015), shadow(0.045), regen(0.2)]),
    it(536, 4, Wrist, Epic, "Bindings of Transcendence", 34052, "INV_Bracer_09", &[holy(0.035), regen(0.2), cdr(0.008)]),
    it(537, 4, Hands, Epic, "Handguards of Transcendence", 34051, "INV_Gauntlets_14", &[shadow(0.045), crit(0.01), cdr(0.01)]),
    it(538, 4, MainHand, Epic, "Benediction", 31347, "INV_Staff_30", &[dmg(0.05), cdr(0.015), area(0.035)]),
    it(540, 4, Head, Legendary, "Circlet of Faith", 35155, "INV_Crown_01", &[shadow(0.075), cdr(0.015), regen(0.4), area(0.04)]),
    it(541, 4, Shoulder, Legendary, "Shoulderpads of Faith", 35149, "INV_Shoulder_25", &[holy(0.07), area(0.035), hp(15.0), cdr(0.015)]),
    it(542, 4, Chest, Legendary, "Robe of Faith", 35142, "INV_Chest_Cloth_43", &[shadow(0.085), hp(18.0), regen(0.4), cdr(0.02)]),
    it(543, 4, Waist, Legendary, "Belt of Faith", 35143, "INV_Belt_08", &[holy(0.055), cdr(0.01), regen(0.3), speed(0.015)]),
    it(544, 4, Legs, Legendary, "Leggings of Faith", 35146, "INV_Pants_Cloth_05", &[shadow(0.075), hp(16.0), area(0.04), crit(0.015)]),
    it(545, 4, Feet, Legendary, "Sandals of Faith", 35148, "INV_Boots_Fabric_01", &[speed(0.02), holy(0.065), regen(0.3), cdr(0.015)]),
    it(546, 4, Wrist, Legendary, "Bindings of Faith", 35144, "INV_Bracer_13", &[shadow(0.05), regen(0.2), cdr(0.01), crit(0.01)]),
    it(547, 4, Hands, Legendary, "Gloves of Faith", 35145, "INV_Gauntlets_17", &[holy(0.065), crit(0.015), cdr(0.015), area(0.03)]),
    it(548, 4, MainHand, Legendary, "Atiesh, Greatstaff of the Guardian", 35634, "INV_Staff_Medivh", &[dmg(0.065), cdr(0.02), area(0.045), targets(1.0)]),
];

/// Shaman: Primitive and Scalemail whites, Wolf Rider's greens, The Elements (Dungeon Set 1), The
/// Ten Storms (Tier 2) and The Earthshatter (Tier 3).
#[rustfmt::skip]
static SHAMAN: &[Item] = &[
    it(600, 5, Head, Common, "Augmented Chain Helm", 1127, "INV_Helmet_03", &[nature(0.015), crit(0.003)]),
    it(601, 5, Shoulder, Common, "Rough Bronze Shoulders", 4483, "INV_Shoulder_05", &[nature(0.015), armor(0.004)]),
    it(602, 5, Chest, Common, "Primitive Mantle", 4492, "INV_Chest_Wolf", &[hp(4.0), nature(0.015)]),
    it(603, 5, Waist, Common, "Scalemail Belt", 18974, "INV_Belt_12", &[nature(0.01), area(0.006)]),
    it(604, 5, Legs, Common, "Primitive Kilt", 4497, "INV_Pants_02", &[nature(0.015), hp(3.0)]),
    it(605, 5, Feet, Common, "Scalemail Boots", 4493, "INV_Boots_05", &[speed(0.004), nature(0.015)]),
    it(606, 5, Wrist, Common, "Scalemail Bracers", 4494, "INV_Bracer_08", &[crit(0.002), nature(0.01)]),
    it(607, 5, Hands, Common, "Scalemail Gloves", 4495, "INV_Gauntlets_05", &[nature(0.015), crit(0.003)]),
    it(608, 5, MainHand, Common, "Spiked Club", 861, "INV_Mace_01", &[dmg(0.015), crit(0.004)]),
    it(610, 5, Head, Uncommon, "Wolf Rider's Headgear", 4176, "INV_Helmet_04", &[nature(0.025), crit(0.006)]),
    it(611, 5, Shoulder, Uncommon, "Wolf Rider's Shoulder Pads", 5414, "INV_Shoulder_08", &[nature(0.025), armor(0.007)]),
    it(612, 5, Chest, Uncommon, "Wolf Rider's Padded Armor", 2104, "INV_Chest_Wolf", &[hp(6.0), nature(0.03)]),
    it(613, 5, Waist, Uncommon, "Wolf Rider's Belt", 3264, "INV_Belt_01", &[nature(0.02), area(0.01)]),
    it(614, 5, Legs, Uncommon, "Wolf Rider's Leggings", 9500, "INV_Pants_Wolf", &[nature(0.025), hp(6.0)]),
    it(615, 5, Feet, Uncommon, "Wolf Rider's Boots", 2105, "INV_Boots_07", &[speed(0.006), nature(0.02)]),
    it(616, 5, Wrist, Uncommon, "Wolf Rider's Wristbands", 3704, "INV_Bracer_08", &[crit(0.004), nature(0.015)]),
    it(617, 5, Hands, Uncommon, "Wolf Rider's Gloves", 2101, "INV_Gauntlets_05", &[nature(0.02), crit(0.005)]),
    it(618, 5, MainHand, Uncommon, "Smite's Mighty Hammer", 8584, "INV_Hammer_16", &[dmg(0.025), crit(0.007)]),
    it(620, 5, Head, Rare, "Coif of Elements", 31117, "INV_Helmet_04", &[nature(0.04), crit(0.009), hp(9.0)]),
    it(621, 5, Shoulder, Rare, "Pauldrons of Elements", 30925, "INV_Shoulder_29", &[fire(0.04), armor(0.01), crit(0.008)]),
    it(622, 5, Chest, Rare, "Vest of Elements", 31416, "INV_Chest_Chain_11", &[hp(10.0), nature(0.045), armor(0.015)]),
    it(623, 5, Waist, Rare, "Cord of Elements", 31413, "INV_Belt_16", &[fire(0.03), area(0.015), regen(0.2)]),
    it(624, 5, Legs, Rare, "Kilt of Elements", 31415, "INV_Pants_03", &[nature(0.04), hp(9.0), crit(0.009)]),
    it(625, 5, Feet, Rare, "Boots of Elements", 31412, "INV_Boots_Wolf", &[speed(0.01), fire(0.035), hp(7.0)]),
    it(626, 5, Wrist, Rare, "Bindings of Elements", 31411, "INV_Bracer_02", &[crit(0.006), nature(0.025), armor(0.008)]),
    it(627, 5, Hands, Rare, "Gauntlets of Elements", 31414, "INV_Gauntlets_11", &[fire(0.035), crit(0.007), critd(0.03)]),
    it(628, 5, MainHand, Rare, "Hand of Edward the Odd", 8567, "INV_Hammer_06", &[dmg(0.035), crit(0.01), area(0.025)]),
    it(630, 5, Head, Epic, "Helmet of Ten Storms", 34217, "INV_Helmet_69", &[nature(0.06), crit(0.01), hp(12.0)]),
    it(631, 5, Shoulder, Epic, "Epaulets of Ten Storms", 34255, "INV_Shoulder_33", &[nature(0.055), armor(0.015), crit(0.01)]),
    it(632, 5, Chest, Epic, "Breastplate of Ten Storms", 34080, "INV_Chest_Chain_11", &[hp(14.0), nature(0.065), armor(0.02)]),
    it(633, 5, Waist, Epic, "Belt of Ten Storms", 34078, "INV_Belt_14", &[nature(0.04), area(0.02), regen(0.2)]),
    it(634, 5, Legs, Epic, "Legplates of Ten Storms", 34084, "INV_Pants_03", &[nature(0.06), hp(12.0), crit(0.01)]),
    it(635, 5, Feet, Epic, "Greaves of Ten Storms", 34083, "INV_Boots_Plate_06", &[speed(0.015), nature(0.045), hp(10.0)]),
    it(636, 5, Wrist, Epic, "Bracers of Ten Storms", 34079, "INV_Bracer_16", &[crit(0.008), nature(0.035), armor(0.01)]),
    it(637, 5, Hands, Epic, "Gauntlets of Ten Storms", 34082, "INV_Gauntlets_11", &[nature(0.045), crit(0.01), critd(0.04)]),
    it(638, 5, MainHand, Epic, "Earthshaker", 25046, "INV_Hammer_09", &[dmg(0.05), crit(0.015), area(0.035)]),
    it(640, 5, Head, Legendary, "Earthshatter Headpiece", 35749, "INV_Helmet_15", &[nature(0.075), crit(0.015), hp(16.0), area(0.04)]),
    it(641, 5, Shoulder, Legendary, "Earthshatter Spaulders", 35751, "INV_Shoulder_14", &[fire(0.07), armor(0.02), crit(0.015), area(0.035)]),
    it(642, 5, Chest, Legendary, "Earthshatter Tunic", 35752, "INV_Chest_Chain_15", &[hp(18.0), nature(0.085), armor(0.025), crit(0.02)]),
    it(643, 5, Waist, Legendary, "Earthshatter Girdle", 35747, "INV_Belt_22", &[fire(0.055), area(0.03), regen(0.3), speed(0.015)]),
    it(644, 5, Legs, Legendary, "Earthshatter Legguards", 35750, "INV_Pants_Mail_15", &[nature(0.075), hp(16.0), crit(0.015), armor(0.02)]),
    it(645, 5, Feet, Legendary, "Earthshatter Boots", 35746, "INV_Boots_Chain_05", &[speed(0.02), fire(0.065), hp(14.0), area(0.03)]),
    it(646, 5, Wrist, Legendary, "Earthshatter Wristguards", 35753, "INV_Bracer_02", &[crit(0.01), nature(0.05), armor(0.015), regen(0.2)]),
    it(647, 5, Hands, Legendary, "Earthshatter Handguards", 35748, "INV_Gauntlets_25", &[fire(0.065), crit(0.015), critd(0.055), area(0.03)]),
    it(648, 5, MainHand, Legendary, "Might of Menethil", 35239, "INV_Mace_25", &[dmg(0.065), crit(0.02), area(0.045), targets(1.0)]),
];

/// Mage: Apprentice's and Woven whites, Conjurer's greens, Magister's Regalia (Dungeon Set 1),
/// Netherwind Regalia (Tier 2) and Frostfire Regalia (Tier 3), with Atiesh.
#[rustfmt::skip]
static MAGE: &[Item] = &[
    it(700, 6, Head, Common, "Silk Headband", 11544, "INV_Helmet_04", &[frost(0.015), crit(0.003)]),
    it(701, 6, Shoulder, Common, "Reinforced Woolen Shoulders", 10597, "INV_Shoulder_09", &[frost(0.015), area(0.007)]),
    it(702, 6, Chest, Common, "Apprentice's Robe", 14584, "INV_Chest_Cloth_08", &[frost(0.015), hp(4.0)]),
    it(703, 6, Waist, Common, "Linen Belt", 10103, "INV_Belt_01", &[frost(0.01), cdr(0.002)]),
    it(704, 6, Legs, Common, "Apprentice's Pants", 8181, "INV_Pants_02", &[frost(0.015), crit(0.003)]),
    it(705, 6, Feet, Common, "Apprentice's Boots", 10104, "INV_Boots_01", &[speed(0.004), frost(0.015)]),
    it(706, 6, Wrist, Common, "Woven Bracers", 14586, "INV_Bracer_07", &[cdr(0.002), frost(0.01)]),
    it(707, 6, Hands, Common, "Heavy Linen Gloves", 14588, "INV_Gauntlets_27", &[crit(0.003), frost(0.015)]),
    it(708, 6, MainHand, Common, "Bent Staff", 472, "INV_Staff_08", &[dmg(0.015), crit(0.004)]),
    it(710, 6, Head, Uncommon, "Conjurer's Hood", 15006, "INV_Helmet_12", &[arcane(0.025), crit(0.006)]),
    it(711, 6, Shoulder, Uncommon, "Conjurer's Mantle", 18866, "INV_Shoulder_25", &[arcane(0.025), area(0.01)]),
    it(712, 6, Chest, Uncommon, "Conjurer's Robe", 18833, "INV_Chest_Cloth_38", &[arcane(0.03), hp(6.0)]),
    it(713, 6, Waist, Uncommon, "Conjurer's Cinch", 26280, "INV_Belt_30", &[arcane(0.02), cdr(0.004)]),
    it(714, 6, Legs, Uncommon, "Conjurer's Breeches", 19812, "INV_Pants_11", &[arcane(0.025), crit(0.006)]),
    it(715, 6, Feet, Uncommon, "Conjurer's Shoes", 26268, "INV_Boots_07", &[speed(0.006), arcane(0.02)]),
    it(716, 6, Wrist, Uncommon, "Conjurer's Bracers", 26266, "INV_Bracer_17", &[cdr(0.004), arcane(0.015)]),
    it(717, 6, Hands, Uncommon, "Conjurer's Gloves", 19813, "INV_Gauntlets_17", &[crit(0.005), arcane(0.02)]),
    it(718, 6, MainHand, Uncommon, "Staff of Jordan", 1376, "INV_Staff_07", &[dmg(0.025), crit(0.007)]),
    it(720, 6, Head, Rare, "Magister's Crown", 31087, "INV_Crown_02", &[arcane(0.04), crit(0.009), cdr(0.009)]),
    it(721, 6, Shoulder, Rare, "Magister's Mantle", 30211, "INV_Shoulder_23", &[frost(0.04), area(0.02), crit(0.008)]),
    it(722, 6, Chest, Rare, "Magister's Robes", 29591, "INV_Chest_Cloth_25", &[arcane(0.045), hp(10.0), crit(0.01)]),
    it(723, 6, Waist, Rare, "Magister's Belt", 29596, "INV_Belt_08", &[frost(0.03), cdr(0.007), area(0.015)]),
    it(724, 6, Legs, Rare, "Magister's Leggings", 34619, "INV_Pants_06", &[arcane(0.04), crit(0.009), hp(9.0)]),
    it(725, 6, Feet, Rare, "Magister's Boots", 29594, "INV_Boots_02", &[speed(0.01), frost(0.035), cdr(0.007)]),
    it(726, 6, Wrist, Rare, "Magister's Bindings", 29595, "INV_Bracer_13", &[cdr(0.006), arcane(0.025), crit(0.006)]),
    it(727, 6, Hands, Rare, "Magister's Gloves", 29593, "INV_Gauntlets_17", &[crit(0.007), frost(0.035), critd(0.03)]),
    it(728, 6, MainHand, Rare, "Witchblade", 35246, "INV_knife_1h_stratholme_d_03", &[dmg(0.035), crit(0.01), area(0.025)]),
    it(730, 6, Head, Epic, "Netherwind Crown", 34218, "INV_Helmet_70", &[fire(0.06), crit(0.01), cdr(0.01)]),
    it(731, 6, Shoulder, Epic, "Netherwind Mantle", 34254, "INV_Shoulder_32", &[arcane(0.055), area(0.025), crit(0.01)]),
    it(732, 6, Chest, Epic, "Netherwind Robes", 34038, "INV_Chest_Cloth_03", &[fire(0.065), hp(14.0), crit(0.015)]),
    it(733, 6, Waist, Epic, "Netherwind Belt", 34046, "INV_Belt_22", &[arcane(0.04), cdr(0.009), area(0.02)]),
    it(734, 6, Legs, Epic, "Netherwind Pants", 34039, "INV_Pants_08", &[fire(0.06), crit(0.01), hp(12.0)]),
    it(735, 6, Feet, Epic, "Netherwind Boots", 34043, "INV_Boots_07", &[speed(0.015), arcane(0.045), cdr(0.01)]),
    it(736, 6, Wrist, Epic, "Netherwind Bindings", 34045, "INV_Bracer_09", &[cdr(0.008), fire(0.035), crit(0.008)]),
    it(737, 6, Hands, Epic, "Netherwind Gloves", 34041, "INV_Gauntlets_14", &[crit(0.01), arcane(0.045), critd(0.04)]),
    it(738, 6, MainHand, Epic, "Staff of the Shadow Flame", 31959, "INV_Staff_06", &[dmg(0.05), crit(0.015), area(0.035)]),
    it(740, 6, Head, Legendary, "Frostfire Circlet", 35331, "INV_Crown_01", &[frost(0.075), crit(0.015), cdr(0.015), area(0.04)]),
    it(741, 6, Shoulder, Legendary, "Frostfire Shoulderpads", 35326, "INV_Shoulder_25", &[fire(0.07), area(0.035), crit(0.015), hp(15.0)]),
    it(742, 6, Chest, Legendary, "Frostfire Robe", 35328, "INV_Chest_Cloth_43", &[frost(0.085), hp(18.0), crit(0.02), cdr(0.02)]),
    it(743, 6, Waist, Legendary, "Frostfire Belt", 35333, "INV_Belt_03", &[fire(0.055), cdr(0.01), area(0.03), speed(0.015)]),
    it(744, 6, Legs, Legendary, "Frostfire Leggings", 35329, "INV_Pants_Cloth_05", &[frost(0.075), crit(0.015), hp(16.0), area(0.04)]),
    it(745, 6, Feet, Legendary, "Frostfire Sandals", 35327, "INV_Boots_Fabric_01", &[speed(0.02), fire(0.065), cdr(0.015), crit(0.015)]),
    it(746, 6, Wrist, Legendary, "Frostfire Bindings", 35332, "INV_Bracer_13", &[cdr(0.01), frost(0.05), crit(0.01), area(0.025)]),
    it(747, 6, Hands, Legendary, "Frostfire Gloves", 35330, "INV_Gauntlets_17", &[crit(0.015), fire(0.065), critd(0.055), area(0.03)]),
    it(748, 6, MainHand, Legendary, "Atiesh, Greatstaff of the Guardian", 35631, "INV_Staff_Medivh", &[dmg(0.065), crit(0.02), area(0.045), targets(1.0)]),
];

/// Warlock: Acolyte's and Thin Cloth whites, Bloodwoven greens, Dreadmist Raiment (Dungeon Set 1),
/// Nemesis Raiment (Tier 2) and Plagueheart Raiment (Tier 3), with Atiesh.
#[rustfmt::skip]
static WARLOCK: &[Item] = &[
    it(800, 7, Head, Common, "Embroidered Hat", 13225, "INV_Helmet_04", &[shadow(0.015), cdr(0.003)]),
    it(801, 7, Shoulder, Common, "Brocade Shoulderpads", 11472, "INV_Shoulder_06", &[shadow(0.015), area(0.007)]),
    it(802, 7, Chest, Common, "Acolyte's Robe", 14986, "INV_Chest_Cloth_05", &[shadow(0.015), hp(4.0)]),
    it(803, 7, Waist, Common, "Thin Cloth Belt", 14621, "INV_Belt_10", &[shadow(0.01), cdr(0.002)]),
    it(804, 7, Legs, Common, "Acolyte's Pants", 14620, "INV_Pants_01", &[shadow(0.015), hp(3.0)]),
    it(805, 7, Feet, Common, "Acolyte's Shoes", 7776, "INV_Boots_01", &[speed(0.004), shadow(0.015)]),
    it(806, 7, Wrist, Common, "Thin Cloth Bracers", 14618, "INV_Bracer_07", &[shadow(0.01), leech(0.001)]),
    it(807, 7, Hands, Common, "Thin Cloth Gloves", 14619, "INV_Gauntlets_05", &[shadow(0.015), crit(0.003)]),
    it(808, 7, MainHand, Common, "Gnarled Staff", 3569, "INV_Misc_Root_01", &[dmg(0.015), leech(0.001)]),
    it(810, 7, Head, Uncommon, "Bloodwoven Mask", 17235, "INV_Helmet_11", &[shadow(0.025), cdr(0.006)]),
    it(811, 7, Shoulder, Uncommon, "Bloodwoven Pads", 13672, "INV_Shoulder_23", &[fire(0.025), area(0.01)]),
    it(812, 7, Chest, Uncommon, "Bloodwoven Jerkin", 26288, "INV_Chest_Cloth_18", &[shadow(0.03), hp(6.0)]),
    it(813, 7, Waist, Uncommon, "Bloodwoven Cord", 26284, "INV_Belt_13", &[fire(0.02), cdr(0.004)]),
    it(814, 7, Legs, Uncommon, "Bloodwoven Pants", 26296, "INV_Pants_12", &[shadow(0.025), hp(6.0)]),
    it(815, 7, Feet, Uncommon, "Bloodwoven Boots", 26285, "INV_Boots_08", &[speed(0.006), fire(0.02)]),
    it(816, 7, Wrist, Uncommon, "Bloodwoven Bracers", 26287, "INV_Bracer_09", &[shadow(0.015), leech(0.001)]),
    it(817, 7, Hands, Uncommon, "Bloodwoven Wraps", 26290, "INV_Gauntlets_03", &[fire(0.02), crit(0.005)]),
    it(818, 7, MainHand, Uncommon, "Hypnotic Blade", 2693, "INV_Weapon_ShortBlade_02", &[dmg(0.025), leech(0.002)]),
    it(820, 7, Head, Rare, "Dreadmist Mask", 31261, "INV_Helmet_29", &[shadow(0.04), cdr(0.009), leech(0.003)]),
    it(821, 7, Shoulder, Rare, "Dreadmist Mantle", 29798, "INV_Misc_Bone_TaurenSkull_01", &[shadow(0.04), area(0.02), hp(8.0)]),
    it(822, 7, Chest, Rare, "Dreadmist Robe", 29792, "INV_Chest_Cloth_49", &[shadow(0.045), hp(10.0), leech(0.003)]),
    it(823, 7, Waist, Rare, "Dreadmist Belt", 29793, "INV_Belt_12", &[shadow(0.03), cdr(0.007), area(0.015)]),
    it(824, 7, Legs, Rare, "Dreadmist Leggings", 29797, "INV_Pants_08", &[shadow(0.04), hp(9.0), area(0.02)]),
    it(825, 7, Feet, Rare, "Dreadmist Sandals", 29799, "INV_Boots_05", &[speed(0.01), shadow(0.035), hp(7.0)]),
    it(826, 7, Wrist, Rare, "Dreadmist Bracers", 29795, "INV_Bracer_13", &[shadow(0.025), leech(0.002), cdr(0.006)]),
    it(827, 7, Hands, Rare, "Dreadmist Wraps", 29800, "INV_Gauntlets_32", &[shadow(0.035), crit(0.007), cdr(0.007)]),
    it(828, 7, MainHand, Rare, "Gravestone Scepter", 35710, "INV_Mace_1H_Stratholme_D_02", &[dmg(0.035), leech(0.004), area(0.025)]),
    it(830, 7, Head, Epic, "Nemesis Skullcap", 33644, "INV_Helmet_08", &[shadow(0.06), cdr(0.01), leech(0.004)]),
    it(831, 7, Shoulder, Epic, "Nemesis Spaulders", 34022, "INV_Shoulder_19", &[fire(0.055), area(0.025), hp(11.0)]),
    it(832, 7, Chest, Epic, "Nemesis Robes", 34014, "INV_Chest_Leather_01", &[shadow(0.065), hp(14.0), leech(0.005)]),
    it(833, 7, Waist, Epic, "Nemesis Belt", 34011, "INV_Belt_13", &[fire(0.04), cdr(0.009), area(0.02)]),
    it(834, 7, Legs, Epic, "Nemesis Leggings", 31972, "INV_Pants_Cloth_14", &[shadow(0.06), hp(12.0), area(0.03)]),
    it(835, 7, Feet, Epic, "Nemesis Boots", 34010, "INV_Boots_05", &[speed(0.015), fire(0.045), hp(10.0)]),
    it(836, 7, Wrist, Epic, "Nemesis Bracers", 34012, "INV_Bracer_07", &[shadow(0.035), leech(0.003), cdr(0.008)]),
    it(837, 7, Hands, Epic, "Nemesis Gloves", 34013, "INV_Gauntlets_19", &[fire(0.045), crit(0.01), cdr(0.01)]),
    it(838, 7, MainHand, Epic, "Claw of Chromaggus", 31849, "INV_Weapon_Shortblade_29", &[dmg(0.05), leech(0.005), area(0.035)]),
    it(840, 7, Head, Legendary, "Plagueheart Circlet", 35181, "INV_Crown_01", &[shadow(0.075), cdr(0.015), leech(0.006), area(0.04)]),
    it(841, 7, Shoulder, Legendary, "Plagueheart Shoulderpads", 35187, "INV_Shoulder_25", &[fire(0.07), area(0.035), hp(15.0), leech(0.005)]),
    it(842, 7, Chest, Legendary, "Plagueheart Robe", 35185, "INV_Chest_Cloth_43", &[shadow(0.085), hp(18.0), leech(0.006), cdr(0.02)]),
    it(843, 7, Waist, Legendary, "Plagueheart Belt", 35179, "INV_Belt_03", &[fire(0.055), cdr(0.01), area(0.03), speed(0.015)]),
    it(844, 7, Legs, Legendary, "Plagueheart Leggings", 35184, "INV_Pants_Cloth_05", &[shadow(0.075), hp(16.0), area(0.04), crit(0.015)]),
    it(845, 7, Feet, Legendary, "Plagueheart Sandals", 35186, "INV_Boots_Fabric_01", &[speed(0.02), fire(0.065), hp(14.0), cdr(0.015)]),
    it(846, 7, Wrist, Legendary, "Plagueheart Bindings", 35180, "INV_Bracer_13", &[shadow(0.05), leech(0.003), cdr(0.01), area(0.025)]),
    it(847, 7, Hands, Legendary, "Plagueheart Gloves", 35183, "INV_Gauntlets_17", &[fire(0.065), crit(0.015), cdr(0.015), area(0.03)]),
    it(848, 7, MainHand, Legendary, "Atiesh, Greatstaff of the Guardian", 35632, "INV_Staff_Medivh", &[dmg(0.065), leech(0.007), area(0.045), targets(1.0)]),
];

/// Druid: vendor Tanned Leather, the Wailing Caverns' Embrace of the Viper, Wildheart Raiment
/// (Dungeon Set 1), Stormrage Raiment (Tier 2) and Dreamwalker Raiment (Tier 3), with Atiesh.
#[rustfmt::skip]
static DRUID: &[Item] = &[
    it(900, 8, Head, Common, "Studded Leather Cap", 17202, "INV_Helmet_15", &[nature(0.015), regen(0.1)]),
    it(901, 8, Shoulder, Common, "Rawhide Shoulderpads", 15002, "INV_Shoulder_08", &[nature(0.015), area(0.007)]),
    it(902, 8, Chest, Common, "Tanned Leather Jerkin", 2082, "INV_Chest_Wolf", &[hp(4.0), nature(0.015)]),
    it(903, 8, Waist, Common, "Tanned Leather Belt", 3358, "INV_Belt_01", &[nature(0.01), speed(0.003)]),
    it(904, 8, Legs, Common, "Tanned Leather Pants", 16967, "INV_Pants_09", &[nature(0.015), hp(3.0)]),
    it(905, 8, Feet, Common, "Tanned Leather Boots", 2083, "INV_Boots_04", &[speed(0.004), nature(0.015)]),
    it(906, 8, Wrist, Common, "Tanned Leather Bracers", 17015, "INV_Bracer_02", &[nature(0.01), regen(0.1)]),
    it(907, 8, Hands, Common, "Tanned Leather Gloves", 17066, "INV_Gauntlets_18", &[nature(0.015), crit(0.003)]),
    it(908, 8, MainHand, Common, "Quarter Staff", 1371, "INV_Staff_02", &[dmg(0.015), area(0.009)]),
    it(910, 8, Head, Uncommon, "Wolfshead Helm", 17264, "INV_Helmet_08", &[nature(0.025), regen(0.1)]),
    it(911, 8, Shoulder, Uncommon, "Serpent's Shoulders", 17319, "INV_Shoulder_23", &[nature(0.025), area(0.01)]),
    it(912, 8, Chest, Uncommon, "Armor of the Fang", 3832, "INV_Chest_Fur", &[hp(6.0), nature(0.03)]),
    it(913, 8, Waist, Uncommon, "Belt of the Fang", 4460, "INV_Belt_01", &[nature(0.02), speed(0.006)]),
    it(914, 8, Legs, Uncommon, "Leggings of the Fang", 3834, "INV_Pants_02", &[nature(0.025), hp(6.0)]),
    it(915, 8, Feet, Uncommon, "Footpads of the Fang", 17317, "INV_Boots_01", &[speed(0.006), nature(0.02)]),
    it(916, 8, Wrist, Uncommon, "Scouting Bracers", 17316, "INV_Bracer_03", &[nature(0.015), regen(0.1)]),
    it(917, 8, Hands, Uncommon, "Gloves of the Fang", 17314, "INV_Gauntlets_15", &[nature(0.02), crit(0.005)]),
    it(918, 8, MainHand, Uncommon, "Crescent Staff", 18437, "INV_Staff_21", &[dmg(0.025), area(0.015)]),
    it(920, 8, Head, Rare, "Wildheart Cowl", 31228, "INV_Helmet_27", &[nature(0.04), regen(0.2), crit(0.009)]),
    it(921, 8, Shoulder, Rare, "Wildheart Spaulders", 30412, "INV_Shoulder_01", &[arcane(0.04), area(0.02), hp(8.0)]),
    it(922, 8, Chest, Rare, "Wildheart Vest", 29974, "INV_Chest_Plate06", &[hp(10.0), nature(0.045), regen(0.2)]),
    it(923, 8, Waist, Rare, "Wildheart Belt", 29976, "INV_Belt_15", &[arcane(0.03), speed(0.009), regen(0.2)]),
    it(924, 8, Legs, Rare, "Wildheart Kilt", 29975, "INV_Pants_08", &[nature(0.04), hp(9.0), area(0.02)]),
    it(925, 8, Feet, Rare, "Wildheart Boots", 34638, "INV_Boots_08", &[speed(0.01), arcane(0.035), regen(0.2)]),
    it(926, 8, Wrist, Rare, "Wildheart Bracers", 29977, "INV_Bracer_09", &[nature(0.025), regen(0.1), crit(0.006)]),
    it(927, 8, Hands, Rare, "Wildheart Gloves", 29978, "INV_Gauntlets_15", &[arcane(0.035), crit(0.007), cdr(0.007)]),
    it(928, 8, MainHand, Rare, "Princess Theradras' Scepter", 8592, "INV_Hammer_05", &[dmg(0.035), area(0.025), cdr(0.01)]),
    it(930, 8, Head, Epic, "Stormrage Cover", 33655, "INV_Helmet_09", &[arcane(0.06), regen(0.3), crit(0.01)]),
    it(931, 8, Shoulder, Epic, "Stormrage Pauldrons", 30546, "INV_Shoulder_07", &[nature(0.055), area(0.025), hp(11.0)]),
    it(932, 8, Chest, Epic, "Stormrage Chestguard", 30536, "INV_Chest_Chain_16", &[hp(14.0), arcane(0.065), regen(0.3)]),
    it(933, 8, Waist, Epic, "Stormrage Belt", 30541, "INV_Belt_06", &[nature(0.04), speed(0.01), regen(0.2)]),
    it(934, 8, Legs, Epic, "Stormrage Legguards", 30538, "INV_Pants_06", &[arcane(0.06), hp(12.0), area(0.03)]),
    it(935, 8, Feet, Epic, "Stormrage Boots", 30542, "INV_Boots_08", &[speed(0.015), nature(0.045), regen(0.2)]),
    it(936, 8, Wrist, Epic, "Stormrage Bracers", 30548, "INV_Bracer_03", &[arcane(0.035), regen(0.2), crit(0.008)]),
    it(937, 8, Hands, Epic, "Stormrage Handguards", 34016, "INV_Gauntlets_25", &[nature(0.045), crit(0.01), cdr(0.01)]),
    it(938, 8, MainHand, Epic, "Staff of Dominance", 19891, "INV_Staff_29", &[dmg(0.05), area(0.035), cdr(0.015)]),
    it(940, 8, Head, Legendary, "Dreamwalker Headpiece", 35162, "INV_Helmet_15", &[nature(0.075), regen(0.4), crit(0.015), area(0.04)]),
    it(941, 8, Shoulder, Legendary, "Dreamwalker Spaulders", 35160, "INV_Shoulder_14", &[arcane(0.07), area(0.035), hp(15.0), regen(0.4)]),
    it(942, 8, Chest, Legendary, "Dreamwalker Tunic", 35159, "INV_Chest_Chain_15", &[hp(18.0), nature(0.085), regen(0.4), area(0.04)]),
    it(943, 8, Waist, Legendary, "Dreamwalker Girdle", 35164, "INV_Belt_22", &[arcane(0.055), speed(0.015), regen(0.3), cdr(0.01)]),
    it(944, 8, Legs, Legendary, "Dreamwalker Legguards", 35161, "INV_Pants_Mail_15", &[nature(0.075), hp(16.0), area(0.04), crit(0.015)]),
    it(945, 8, Feet, Legendary, "Dreamwalker Boots", 35165, "INV_Boots_Chain_05", &[speed(0.02), arcane(0.065), regen(0.3), area(0.03)]),
    it(946, 8, Wrist, Legendary, "Dreamwalker Wristguards", 35158, "INV_Bracer_02", &[nature(0.05), regen(0.2), crit(0.01), cdr(0.01)]),
    it(947, 8, Hands, Legendary, "Dreamwalker Handguards", 35163, "INV_Gauntlets_25", &[arcane(0.065), crit(0.015), cdr(0.015), area(0.03)]),
    it(948, 8, MainHand, Legendary, "Atiesh, Greatstaff of the Guardian", 35633, "INV_Staff_Medivh", &[dmg(0.065), area(0.045), cdr(0.02), targets(1.0)]),
];
