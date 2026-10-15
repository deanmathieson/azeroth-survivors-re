//! The catalog: 45 pieces of jewelry, fifteen each of rings, trinkets and necklaces, from Uncommon
//! to Legendary. Each is a real 1.12 item under its own name, drawn with its own 1.12 display id
//! (as the `item_template` tables of the open-source cores give it, cmangos and mangoszero
//! agreeing) and that display's `ItemDisplayInfo.dbc` icon; the real quality is kept where it can
//! be (green, blue and purple), and the Legendaries are the era's last raid pieces, since 1.12 has
//! no orange jewelry.
//!
//! What each grants is the mode's own: two or three [`Bonus`]es, sized to its rarity's budget (the
//! tests hold every piece to its band), keeping the real item's character where it has one: Carrot
//! on a Stick runs, Diamond Flask heals, The 1 Ring is lucky, Band of Unnatural Forces strikes one
//! more target and Mark of the Champion brings a hero back once.
//!
//! Keys are stable save keys ([`super::CodexSave`]): never rename one.

use super::super::bonus::{Bonus, StatKey};
use super::Kind::{self, *};
use super::Piece;
use super::Rarity::{self, *};

const fn pc(
    key: &'static str,
    kind: Kind,
    rarity: Rarity,
    name: &'static str,
    display_id: u32,
    icon: &'static str,
    bonuses: &'static [Bonus],
) -> Piece {
    Piece {
        key,
        kind,
        rarity,
        name,
        display_id,
        icon,
        bonuses,
        proc: None,
    }
}

// Shorthands, so each piece fits a line.
const fn dmg(v: f32) -> Bonus {
    Bonus::new(StatKey::Damage, v)
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
const fn xp(v: f32) -> Bonus {
    Bonus::new(StatKey::Experience, v)
}
const fn gold(v: f32) -> Bonus {
    Bonus::new(StatKey::Gold, v)
}
const fn luck(v: f32) -> Bonus {
    Bonus::new(StatKey::Luck, v)
}
const fn targets(v: f32) -> Bonus {
    Bonus::new(StatKey::Targets, v)
}
const fn revives(v: f32) -> Bonus {
    Bonus::new(StatKey::Revives, v)
}

/// Every piece: rings, then trinkets, then necklaces, each Uncommon first.
#[rustfmt::skip]
pub(super) static CATALOG: &[Piece] = &[
    // Rings: low-level greens, the Dire Maul, Scholomance and Blackrock blues, Molten Core and
    // Zul'Gurub epics, and Ragnaros's and Naxxramas's bands.
    pc("the_1_ring", Ring, Uncommon, "The 1 Ring", 224, "INV_Jewelry_Ring_03", &[luck(0.10), gold(0.06)]),
    pc("monkey_ring", Ring, Uncommon, "Monkey Ring", 9836, "INV_Jewelry_Ring_14", &[crit(0.03), speed(0.04)]),
    pc("sustaining_ring", Ring, Uncommon, "Sustaining Ring", 12984, "INV_Jewelry_Ring_08", &[regen(0.6), hp(12.0)]),
    pc("blood_ring", Ring, Uncommon, "Blood Ring", 9834, "INV_Jewelry_Ring_04", &[leech(0.015), dmg(0.04)]),
    pc("band_of_rumination", Ring, Rare, "Band of Rumination", 24022, "INV_Jewelry_Ring_16", &[cdr(0.06), xp(0.08)]),
    pc("painweaver_band", Ring, Rare, "Painweaver Band", 23608, "INV_Jewelry_Ring_18", &[crit(0.04), critd(0.15)]),
    pc("naglering", Ring, Rare, "Naglering", 9837, "INV_Jewelry_Ring_05", &[armor(0.05), hp(20.0)]),
    pc("tarnished_elven_ring", Ring, Rare, "Tarnished Elven Ring", 9823, "INV_Jewelry_Ring_13", &[dmg(0.08), crit(0.03)]),
    pc("band_of_the_ogre_king", Ring, Rare, "Band of the Ogre King", 23897, "INV_Jewelry_Ring_32", &[hp(24.0), dmg(0.06)]),
    pc("quick_strike_ring", Ring, Epic, "Quick Strike Ring", 9835, "INV_Jewelry_Ring_07", &[dmg(0.10), crit(0.04), speed(0.04)]),
    pc("don_julios_band", Ring, Epic, "Don Julio's Band", 31616, "INV_Jewelry_Ring_35", &[crit(0.06), dmg(0.09)]),
    pc("seal_of_the_archmagus", Ring, Epic, "Seal of the Archmagus", 29697, "INV_Jewelry_Ring_21", &[cdr(0.08), area(0.10), hp(12.0)]),
    pc("ring_of_spell_power", Ring, Epic, "Ring of Spell Power", 31664, "INV_Jewelry_Ring_38", &[dmg(0.16), area(0.07)]),
    pc("band_of_accuria", Ring, Legendary, "Band of Accuria", 9840, "INV_Jewelry_Ring_15", &[dmg(0.14), critd(0.25), crit(0.04)]),
    pc("band_of_unnatural_forces", Ring, Legendary, "Band of Unnatural Forces", 35423, "INV_Jewelry_Ring_48Naxxramas", &[targets(1.0), dmg(0.10), crit(0.04)]),
    // Trinkets: quest and world greens, the Blackrock and Dire Maul blues, Blackwing Lair,
    // Zul'Gurub and Ahn'Qiraj epics, and Naxxramas's two best.
    pc("carrot_on_a_stick", Trinket, Uncommon, "Carrot on a Stick", 21115, "INV_Misc_Food_54", &[speed(0.06), pickup(0.12)]),
    pc("tidal_charm", Trinket, Uncommon, "Tidal Charm", 6499, "INV_Misc_Rune_01", &[area(0.06), armor(0.03)]),
    pc("arena_master", Trinket, Uncommon, "Arena Master", 31147, "INV_Misc_ArmorKit_14", &[hp(18.0), armor(0.03)]),
    pc("linkens_boomerang", Trinket, Uncommon, "Linken's Boomerang", 22753, "INV_Weapon_ShortBlade_10", &[dmg(0.06), area(0.05)]),
    pc("hand_of_justice", Trinket, Rare, "Hand of Justice", 6337, "INV_Jewelry_Talisman_01", &[dmg(0.07), crit(0.03), armor(0.02)]),
    pc("blackhands_breadth", Trinket, Rare, "Blackhand's Breadth", 24776, "INV_Misc_ArmorKit_09", &[crit(0.05), critd(0.10)]),
    pc("royal_seal_of_eldre_thalas", Trinket, Rare, "Royal Seal of Eldre'Thalas", 29712, "INV_Jewelry_Talisman_10", &[cdr(0.05), dmg(0.06), area(0.04)]),
    pc("diamond_flask", Trinket, Rare, "Diamond Flask", 18059, "INV_Drink_01", &[hp(24.0), regen(0.8)]),
    pc("briarwood_reed", Trinket, Rare, "Briarwood Reed", 8232, "INV_Misc_Root_02", &[dmg(0.09), area(0.07)]),
    pc("talisman_of_ephemeral_power", Trinket, Epic, "Talisman of Ephemeral Power", 31287, "INV_Misc_StoneTablet_11", &[dmg(0.15), cdr(0.07)]),
    pc("zandalarian_hero_charm", Trinket, Epic, "Zandalarian Hero Charm", 32431, "INV_Jewelry_Necklace_13", &[dmg(0.13), area(0.10)]),
    pc("drake_fang_talisman", Trinket, Epic, "Drake Fang Talisman", 31936, "INV_Misc_Bone_06", &[dmg(0.10), crit(0.04), armor(0.03)]),
    pc("earthstrike", Trinket, Epic, "Earthstrike", 33520, "Spell_Nature_AbolishMagic", &[dmg(0.12), speed(0.05), critd(0.12)]),
    pc("kiss_of_the_spider", Trinket, Legendary, "Kiss of the Spider", 35361, "INV_Trinket_Naxxramas04", &[cdr(0.12), dmg(0.10), crit(0.04)]),
    pc("mark_of_the_champion", Trinket, Legendary, "Mark of the Champion", 35649, "INV_Misc_Token_ArgentDawn2", &[revives(1.0), dmg(0.10), armor(0.05)]),
    // Necklaces: quest greens, the Scholomance, Stratholme and Dire Maul blues, Onyxia, Molten
    // Core and Darkmoon epics, and Blackwing Lair's and Naxxramas's last.
    pc("medal_of_courage", Necklace, Uncommon, "Medal of Courage", 4841, "INV_Jewelry_Amulet_03", &[hp(15.0), dmg(0.04)]),
    pc("pendant_of_myzrael", Necklace, Uncommon, "Pendant of Myzrael", 9859, "INV_Jewelry_Necklace_07", &[area(0.06), xp(0.06)]),
    pc("felstone_good_luck_charm", Necklace, Uncommon, "Felstone Good Luck Charm", 23715, "INV_Jewelry_Talisman_11", &[luck(0.12), gold(0.04)]),
    pc("fire_opal_necklace", Necklace, Uncommon, "Fire Opal Necklace", 9854, "INV_Jewelry_Necklace_02", &[dmg(0.06), regen(0.4)]),
    pc("mark_of_fordring", Necklace, Rare, "Mark of Fordring", 23716, "INV_Jewelry_Talisman_07", &[crit(0.04), dmg(0.06)]),
    pc("star_of_mystaria", Necklace, Rare, "Star of Mystaria", 23717, "INV_Jewelry_Talisman_08", &[xp(0.10), cdr(0.05)]),
    pc("imperial_jewel", Necklace, Rare, "Imperial Jewel", 28784, "INV_Misc_Gem_Sapphire_01", &[pickup(0.20), xp(0.08)]),
    pc("talisman_of_evasion", Necklace, Rare, "Talisman of Evasion", 6494, "INV_Jewelry_Amulet_04", &[armor(0.05), speed(0.06)]),
    pc("evil_eye_pendant", Necklace, Rare, "Evil Eye Pendant", 30738, "INV_Misc_Eye_01", &[dmg(0.06), leech(0.02)]),
    pc("onyxia_tooth_pendant", Necklace, Epic, "Onyxia Tooth Pendant", 9860, "INV_Jewelry_Necklace_09", &[crit(0.05), dmg(0.08), speed(0.04)]),
    pc("choker_of_the_fire_lord", Necklace, Epic, "Choker of the Fire Lord", 1399, "INV_Jewelry_Amulet_05", &[dmg(0.12), area(0.08), cdr(0.03)]),
    pc("amulet_of_the_darkmoon", Necklace, Epic, "Amulet of the Darkmoon", 32008, "INV_Jewelry_Necklace_14", &[luck(0.15), gold(0.10), hp(12.0)]),
    pc("choker_of_enlightenment", Necklace, Epic, "Choker of Enlightenment", 9858, "INV_Jewelry_Necklace_10", &[cdr(0.08), xp(0.10), regen(0.4)]),
    pc("prestors_talisman_of_connivery", Necklace, Legendary, "Prestor's Talisman of Connivery", 31899, "INV_Jewelry_Necklace_17", &[crit(0.06), dmg(0.12), speed(0.06)]),
    pc("gem_of_trapped_innocents", Necklace, Legendary, "Gem of Trapped Innocents", 35358, "INV_Jewelry_Necklace_29Naxxramas", &[dmg(0.12), critd(0.25), leech(0.02)]),
];

/// Completed Darkmoon decks add their trinkets to the run pool, rather than the account codex.
#[rustfmt::skip]
pub(super) static DARKMOON: &[Piece] = &[
    pc("darkmoon_blue_dragon", Trinket, Legendary, "Darkmoon Card: Blue Dragon", 19288, "INV_Misc_Ticket_Tarot_BlueDragon_01", &[]),
    pc("darkmoon_maelstrom", Trinket, Legendary, "Darkmoon Card: Maelstrom", 19289, "INV_Misc_Ticket_Tarot_Maelstrom_01", &[]),
    pc("darkmoon_twisting_nether", Trinket, Legendary, "Darkmoon Card: Twisting Nether", 19290, "INV_Misc_Ticket_Tarot_TwistingNether_01", &[]),
    pc("darkmoon_heroism", Trinket, Legendary, "Darkmoon Card: Heroism", 19287, "INV_Misc_Ticket_Tarot_Heroism_01", &[]),
];
