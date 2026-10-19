//! The authored item templates. WoW keeps item templates on the server, and survivors has none, so
//! each row here is what `SMSG_ITEM_QUERY_SINGLE_RESPONSE` would carry for a real 1.12 entry: its
//! name, quality, class and icon, with our own numbers (stack sizes, prices, effects).
//!
//! **P2's items** (Alchemy's, Cooking's, First Aid's and Enchanting's, below) follow the same rules:
//! each entry but the recipe items' and the rare drops' is a reagent or product of a curated
//! recipe's `Spell.dbc` row, which `recipes`' data test checks; names, prices, stacks and icons are
//! from memory, each icon checked to ship with an `ItemDisplayInfo.dbc` row.
//!
//! **What is verified.** Each icon ships in the client (`Interface\Icons\<icon>.blp`), and each
//! item's display id (held by the test until something draws the item in the world) is an
//! `ItemDisplayInfo.dbc` row with that icon: `tests::displays_and_icons_exist_in_the_client`,
//! data-gated. The ores, bars and Coal are also the entries the real Smelting spells consume and
//! create (`recipes::tests::smelting_recipes_resolve_in_the_client`), and Empty Vial, Peacebloom,
//! Silverleaf and the Minor Healing Potion those of the real Minor Healing Potion recipe.
//! **Not verified:** the other entries, the names, and which of the rows sharing an icon the
//! cores' `item_template` names. The 1.12 client ships no item names and the repo has no
//! `Item.dbc` parser, so those are from memory and want a check against a core's `item_template`
//! (docs/PROFESSIONS.md, ground rules).

use benilla_ui::script::ItemTemplateView;

/// Copper Ore. Entry unverified (no `Item.dbc` parser in the repo).
pub(crate) const COPPER_ORE: u32 = 2770;
/// Tin Ore. Entry unverified.
pub(crate) const TIN_ORE: u32 = 2771;
/// Copper Bar. Entry unverified.
pub(crate) const COPPER_BAR: u32 = 2840;
/// Peacebloom. Entry unverified.
pub(crate) const PEACEBLOOM: u32 = 2447;
/// Silverleaf. Entry unverified.
pub(crate) const SILVERLEAF: u32 = 765;
/// Linen Cloth. Entry unverified.
pub(crate) const LINEN_CLOTH: u32 = 2589;
/// Minor Healing Potion. Entry unverified.
pub(crate) const MINOR_HEALING_POTION: u32 = 118;
/// Linen Bag. Entry unverified.
pub(crate) const LINEN_BAG: u32 = 4238;
/// Helm of Valor, a duplicate boss drop as a Soulbound bag item (section 3.4). Entry unverified;
/// display and icon are the gear catalog's (`gear/catalog.rs`, id 120).
pub(crate) const HELM_OF_VALOR: u32 = 16731;

// The gathered materials (section 3.3), each a real 1.12 entry from memory.
/// Iron Ore. Entry unverified.
pub(crate) const IRON_ORE: u32 = 2772;
/// Mithril Ore. Entry unverified.
pub(crate) const MITHRIL_ORE: u32 = 3858;
/// Truesilver Ore. Entry unverified.
pub(crate) const TRUESILVER_ORE: u32 = 7911;
/// Thorium Ore. Entry unverified.
pub(crate) const THORIUM_ORE: u32 = 10620;
/// Rough Stone. Entry unverified.
pub(crate) const ROUGH_STONE: u32 = 2835;
/// Coarse Stone. Entry unverified.
pub(crate) const COARSE_STONE: u32 = 2836;
/// Heavy Stone. Entry unverified.
pub(crate) const HEAVY_STONE: u32 = 2838;
/// Solid Stone. Entry unverified.
pub(crate) const SOLID_STONE: u32 = 7912;
/// Dense Stone. Entry unverified.
pub(crate) const DENSE_STONE: u32 = 12365;
/// Mageroyal. Entry unverified.
pub(crate) const MAGEROYAL: u32 = 785;
/// Bruiseweed. Entry unverified.
pub(crate) const BRUISEWEED: u32 = 2453;
/// Kingsblood. Entry unverified.
pub(crate) const KINGSBLOOD: u32 = 3356;
/// Liferoot. Entry unverified.
pub(crate) const LIFEROOT: u32 = 3357;
/// Fadeleaf. Entry unverified.
pub(crate) const FADELEAF: u32 = 3818;
/// Goldthorn. Entry unverified.
pub(crate) const GOLDTHORN: u32 = 3821;
/// Khadgar's Whisker. Entry unverified.
pub(crate) const KHADGARS_WHISKER: u32 = 3358;
/// Purple Lotus. Entry unverified.
pub(crate) const PURPLE_LOTUS: u32 = 8831;
/// Sungrass. Entry unverified.
pub(crate) const SUNGRASS: u32 = 8838;
/// Gromsblood. Entry unverified.
pub(crate) const GROMSBLOOD: u32 = 8846;
/// Golden Sansam. Entry unverified.
pub(crate) const GOLDEN_SANSAM: u32 = 13464;
/// Dreamfoil. Entry unverified.
pub(crate) const DREAMFOIL: u32 = 13463;
/// Mountain Silversage. Entry unverified.
pub(crate) const MOUNTAIN_SILVERSAGE: u32 = 13465;
/// Plaguebloom. Entry unverified.
pub(crate) const PLAGUEBLOOM: u32 = 13466;
/// Icecap. Entry unverified.
pub(crate) const ICECAP: u32 = 13467;
/// Black Lotus. Entry unverified.
pub(crate) const BLACK_LOTUS: u32 = 13468;
/// Ruined Leather Scraps. Entry unverified.
pub(crate) const RUINED_LEATHER_SCRAPS: u32 = 2934;
/// Light Leather. Entry unverified.
pub(crate) const LIGHT_LEATHER: u32 = 2318;
/// Medium Leather. Entry unverified.
pub(crate) const MEDIUM_LEATHER: u32 = 2319;
/// Heavy Leather. Entry unverified.
pub(crate) const HEAVY_LEATHER: u32 = 4234;
/// Thick Leather. Entry unverified.
pub(crate) const THICK_LEATHER: u32 = 4304;
/// Rugged Leather. Entry unverified.
pub(crate) const RUGGED_LEATHER: u32 = 8170;
/// Raw Brilliant Smallfish. Entry unverified.
pub(crate) const RAW_BRILLIANT_SMALLFISH: u32 = 6291;
/// Raw Longjaw Mud Snapper. Entry unverified.
pub(crate) const RAW_LONGJAW_MUD_SNAPPER: u32 = 6289;
/// Deviate Fish. Entry unverified.
pub(crate) const DEVIATE_FISH: u32 = 6522;
/// Oily Blackmouth. Entry unverified.
pub(crate) const OILY_BLACKMOUTH: u32 = 6358;
/// Stonescale Eel. Entry unverified.
pub(crate) const STONESCALE_EEL: u32 = 13422;

// The bars Smelting makes (section 3.3), and what the camp goblin sells for it; every entry is
// a reagent or product of a real 1.12 spell (`recipes`).
/// Tin Bar.
pub(crate) const TIN_BAR: u32 = 3576;
/// Bronze Bar.
pub(crate) const BRONZE_BAR: u32 = 2841;
/// Iron Bar.
pub(crate) const IRON_BAR: u32 = 3575;
/// Steel Bar.
pub(crate) const STEEL_BAR: u32 = 3859;
/// Mithril Bar.
pub(crate) const MITHRIL_BAR: u32 = 3860;
/// Truesilver Bar.
pub(crate) const TRUESILVER_BAR: u32 = 6037;
/// Thorium Bar.
pub(crate) const THORIUM_BAR: u32 = 12359;
/// Coal, Smelt Steel's second reagent: vendor-bought.
pub(crate) const COAL: u32 = 3857;
/// Empty Vial, the potion recipes' vial: vendor-bought.
pub(crate) const EMPTY_VIAL: u32 = 3371;

// P2's items (docs/PROFESSIONS.md, 5.2): two more herbs, the vials, Alchemy's potions, elixirs
// and flasks, Cooking's raw goods and dishes, cloth and First Aid's bandages, Enchanting's dusts,
// essences and shards, two rare reagents, and the recipes that drop. Every entry but the recipe
// items' and the drops' is a reagent or product of a curated recipe's real `Spell.dbc` row
// (`recipes::tests::every_recipe_resolves_in_the_client`); the recipe items' entries are from
// memory. Names, prices, stack sizes and effects are authored.
/// Stranglekelp.
pub(crate) const STRANGLEKELP: u32 = 3820;
/// Wild Steelbloom.
pub(crate) const WILD_STEELBLOOM: u32 = 3355;
/// Leaded Vial.
pub(crate) const LEADED_VIAL: u32 = 3372;
/// Crystal Vial.
pub(crate) const CRYSTAL_VIAL: u32 = 8925;
/// Minor Mana Potion.
pub(crate) const MINOR_MANA_POTION: u32 = 2455;
/// Minor Rejuvenation Potion.
pub(crate) const MINOR_REJUVENATION_POTION: u32 = 2456;
/// Elixir of Minor Defense.
pub(crate) const ELIXIR_OF_MINOR_DEFENSE: u32 = 5997;
/// Blackmouth Oil.
pub(crate) const BLACKMOUTH_OIL: u32 = 6370;
/// Minor Magic Resistance Potion.
pub(crate) const MINOR_MAGIC_RESISTANCE_POTION: u32 = 3384;
/// Lesser Mana Potion.
pub(crate) const LESSER_MANA_POTION: u32 = 3385;
/// Elixir of Defense.
pub(crate) const ELIXIR_OF_DEFENSE: u32 = 3389;
/// Greater Healing Potion.
pub(crate) const GREATER_HEALING_POTION: u32 = 1710;
/// Free Action Potion.
pub(crate) const FREE_ACTION_POTION: u32 = 5634;
/// Mana Potion.
pub(crate) const MANA_POTION: u32 = 3827;
/// Elixir of Fortitude.
pub(crate) const ELIXIR_OF_FORTITUDE: u32 = 3825;
/// Mighty Troll's Blood Potion.
pub(crate) const MIGHTY_TROLLS_BLOOD_POTION: u32 = 3826;
/// Elixir of Agility.
pub(crate) const ELIXIR_OF_AGILITY: u32 = 8949;
/// Elixir of Greater Defense.
pub(crate) const ELIXIR_OF_GREATER_DEFENSE: u32 = 8951;
/// Greater Mana Potion.
pub(crate) const GREATER_MANA_POTION: u32 = 6149;
/// Superior Healing Potion.
pub(crate) const SUPERIOR_HEALING_POTION: u32 = 3928;
/// Stonescale Oil.
pub(crate) const STONESCALE_OIL: u32 = 13423;
/// Elixir of Greater Agility.
pub(crate) const ELIXIR_OF_GREATER_AGILITY: u32 = 9187;
/// Elixir of Giants.
pub(crate) const ELIXIR_OF_GIANTS: u32 = 9206;
/// Mighty Rage Potion.
pub(crate) const MIGHTY_RAGE_POTION: u32 = 13442;
/// Elixir of Superior Defense.
pub(crate) const ELIXIR_OF_SUPERIOR_DEFENSE: u32 = 13445;
/// Major Healing Potion.
pub(crate) const MAJOR_HEALING_POTION: u32 = 13446;
/// Elixir of the Mongoose.
pub(crate) const ELIXIR_OF_THE_MONGOOSE: u32 = 13452;
/// Elixir of Brute Force.
pub(crate) const ELIXIR_OF_BRUTE_FORCE: u32 = 13453;
/// Elixir of the Sages.
pub(crate) const ELIXIR_OF_THE_SAGES: u32 = 13447;
/// Greater Arcane Elixir.
pub(crate) const GREATER_ARCANE_ELIXIR: u32 = 13454;
/// Greater Stoneshield Potion.
pub(crate) const GREATER_STONESHIELD_POTION: u32 = 13455;
/// Major Mana Potion.
pub(crate) const MAJOR_MANA_POTION: u32 = 13444;
/// Flask of the Titans.
pub(crate) const FLASK_OF_THE_TITANS: u32 = 13510;
/// Flask of Supreme Power.
pub(crate) const FLASK_OF_SUPREME_POWER: u32 = 13512;
/// Flask of Distilled Wisdom.
pub(crate) const FLASK_OF_DISTILLED_WISDOM: u32 = 13511;
/// Stringy Wolf Meat.
pub(crate) const STRINGY_WOLF_MEAT: u32 = 2672;
/// Coyote Meat.
pub(crate) const COYOTE_MEAT: u32 = 2673;
/// Bear Meat.
pub(crate) const BEAR_MEAT: u32 = 3173;
/// Big Bear Meat.
pub(crate) const BIG_BEAR_MEAT: u32 = 3730;
/// Lion Meat.
pub(crate) const LION_MEAT: u32 = 3731;
/// Raptor Flesh.
pub(crate) const RAPTOR_FLESH: u32 = 12184;
/// Red Wolf Meat.
pub(crate) const RED_WOLF_MEAT: u32 = 12203;
/// Tender Wolf Meat.
pub(crate) const TENDER_WOLF_MEAT: u32 = 12208;
/// Giant Egg.
pub(crate) const GIANT_EGG: u32 = 12207;
/// Raw Sagefish.
pub(crate) const RAW_SAGEFISH: u32 = 21071;
/// Raw Spotted Yellowtail.
pub(crate) const RAW_SPOTTED_YELLOWTAIL: u32 = 4603;
/// Winter Squid.
pub(crate) const WINTER_SQUID: u32 = 13755;
/// Raw Nightfin Snapper.
pub(crate) const RAW_NIGHTFIN_SNAPPER: u32 = 13759;
/// Mild Spices.
pub(crate) const MILD_SPICES: u32 = 2678;
/// Hot Spices.
pub(crate) const HOT_SPICES: u32 = 2692;
/// Soothing Spices.
pub(crate) const SOOTHING_SPICES: u32 = 3713;
/// Refreshing Spring Water.
pub(crate) const REFRESHING_SPRING_WATER: u32 = 159;
/// Spiced Wolf Meat.
pub(crate) const SPICED_WOLF_MEAT: u32 = 2680;
/// Brilliant Smallfish.
pub(crate) const BRILLIANT_SMALLFISH: u32 = 6290;
/// Coyote Steak.
pub(crate) const COYOTE_STEAK: u32 = 2684;
/// Longjaw Mud Snapper.
pub(crate) const LONGJAW_MUD_SNAPPER: u32 = 4592;
/// Smoked Bear Meat.
pub(crate) const SMOKED_BEAR_MEAT: u32 = 6890;
/// Smoked Sagefish.
pub(crate) const SMOKED_SAGEFISH: u32 = 21072;
/// Savory Deviate Delight.
pub(crate) const SAVORY_DEVIATE_DELIGHT: u32 = 6657;
/// Big Bear Steak.
pub(crate) const BIG_BEAR_STEAK: u32 = 3726;
/// Hot Lion Chops.
pub(crate) const HOT_LION_CHOPS: u32 = 3727;
/// Roast Raptor.
pub(crate) const ROAST_RAPTOR: u32 = 12210;
/// Hot Wolf Ribs.
pub(crate) const HOT_WOLF_RIBS: u32 = 13851;
/// Spotted Yellowtail.
pub(crate) const SPOTTED_YELLOWTAIL: u32 = 6887;
/// Monster Omelet.
pub(crate) const MONSTER_OMELET: u32 = 12218;
/// Tender Wolf Steak.
pub(crate) const TENDER_WOLF_STEAK: u32 = 18045;
/// Grilled Squid.
pub(crate) const GRILLED_SQUID: u32 = 13928;
/// Nightfin Soup.
pub(crate) const NIGHTFIN_SOUP: u32 = 13931;
/// Wool Cloth.
pub(crate) const WOOL_CLOTH: u32 = 2592;
/// Silk Cloth.
pub(crate) const SILK_CLOTH: u32 = 4306;
/// Mageweave Cloth.
pub(crate) const MAGEWEAVE_CLOTH: u32 = 4338;
/// Runecloth.
pub(crate) const RUNECLOTH: u32 = 14047;
/// Linen Bandage.
pub(crate) const LINEN_BANDAGE: u32 = 1251;
/// Heavy Linen Bandage.
pub(crate) const HEAVY_LINEN_BANDAGE: u32 = 2581;
/// Wool Bandage.
pub(crate) const WOOL_BANDAGE: u32 = 3530;
/// Heavy Wool Bandage.
pub(crate) const HEAVY_WOOL_BANDAGE: u32 = 3531;
/// Silk Bandage.
pub(crate) const SILK_BANDAGE: u32 = 6450;
/// Heavy Silk Bandage.
pub(crate) const HEAVY_SILK_BANDAGE: u32 = 6451;
/// Mageweave Bandage.
pub(crate) const MAGEWEAVE_BANDAGE: u32 = 8544;
/// Heavy Mageweave Bandage.
pub(crate) const HEAVY_MAGEWEAVE_BANDAGE: u32 = 8545;
/// Runecloth Bandage.
pub(crate) const RUNECLOTH_BANDAGE: u32 = 14529;
/// Heavy Runecloth Bandage.
pub(crate) const HEAVY_RUNECLOTH_BANDAGE: u32 = 14530;
/// Strange Dust.
pub(crate) const STRANGE_DUST: u32 = 10940;
/// Lesser Magic Essence.
pub(crate) const LESSER_MAGIC_ESSENCE: u32 = 10938;
/// Greater Magic Essence.
pub(crate) const GREATER_MAGIC_ESSENCE: u32 = 10939;
/// Small Glimmering Shard.
pub(crate) const SMALL_GLIMMERING_SHARD: u32 = 10978;
/// Soul Dust.
pub(crate) const SOUL_DUST: u32 = 11083;
/// Lesser Astral Essence.
pub(crate) const LESSER_ASTRAL_ESSENCE: u32 = 10998;
/// Greater Astral Essence.
pub(crate) const GREATER_ASTRAL_ESSENCE: u32 = 11082;
/// Large Glimmering Shard.
pub(crate) const LARGE_GLIMMERING_SHARD: u32 = 11084;
/// Vision Dust.
pub(crate) const VISION_DUST: u32 = 11137;
/// Lesser Mystic Essence.
pub(crate) const LESSER_MYSTIC_ESSENCE: u32 = 11134;
/// Greater Mystic Essence.
pub(crate) const GREATER_MYSTIC_ESSENCE: u32 = 11135;
/// Large Glowing Shard.
pub(crate) const LARGE_GLOWING_SHARD: u32 = 11139;
/// Dream Dust.
pub(crate) const DREAM_DUST: u32 = 11176;
/// Lesser Nether Essence.
pub(crate) const LESSER_NETHER_ESSENCE: u32 = 11174;
/// Greater Nether Essence.
pub(crate) const GREATER_NETHER_ESSENCE: u32 = 11175;
/// Small Radiant Shard.
pub(crate) const SMALL_RADIANT_SHARD: u32 = 11177;
/// Large Radiant Shard.
pub(crate) const LARGE_RADIANT_SHARD: u32 = 11178;
/// Illusion Dust.
pub(crate) const ILLUSION_DUST: u32 = 16204;
/// Lesser Eternal Essence.
pub(crate) const LESSER_ETERNAL_ESSENCE: u32 = 16202;
/// Greater Eternal Essence.
pub(crate) const GREATER_ETERNAL_ESSENCE: u32 = 16203;
/// Small Brilliant Shard.
pub(crate) const SMALL_BRILLIANT_SHARD: u32 = 14343;
/// Large Brilliant Shard.
pub(crate) const LARGE_BRILLIANT_SHARD: u32 = 14344;
/// Essence of Fire.
pub(crate) const ESSENCE_OF_FIRE: u32 = 7078;
/// Righteous Orb.
pub(crate) const RIGHTEOUS_ORB: u32 = 12811;
/// Recipe: Flask of the Titans.
pub(crate) const RECIPE_FLASK_OF_THE_TITANS: u32 = 13519;
/// Recipe: Flask of Supreme Power.
pub(crate) const RECIPE_FLASK_OF_SUPREME_POWER: u32 = 13520;
/// Recipe: Flask of Distilled Wisdom.
pub(crate) const RECIPE_FLASK_OF_DISTILLED_WISDOM: u32 = 13521;
/// Recipe: Savory Deviate Delight.
pub(crate) const RECIPE_SAVORY_DEVIATE_DELIGHT: u32 = 6661;
/// Formula: Enchant Weapon - Crusader.
pub(crate) const FORMULA_ENCHANT_WEAPON_CRUSADER: u32 = 16252;

/// One authored template.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ItemDef {
    pub(crate) entry: u32,
    pub(crate) name: &'static str,
    /// 0 Poor … 5 Legendary.
    pub(crate) quality: u32,
    /// `ItemClass.dbc`: 0 Consumable, 1 Container, 4 Armor, 7 Trade Goods.
    pub(crate) class: u32,
    pub(crate) subclass: u32,
    /// 0 none, 1 Head, 18 Bag.
    pub(crate) inventory_type: u32,
    /// Under `Interface\Icons\`.
    pub(crate) icon: &'static str,
    /// Most per slot.
    pub(crate) stack: u32,
    /// A bag's slots; 0 for anything else.
    pub(crate) bag_slots: u32,
    /// `BagFamily` bits: a specialty bag's, or the families an item may go into; 0 general.
    pub(crate) family: u32,
    /// Copper per item.
    pub(crate) sell_price: u32,
    /// 1 binds on pickup, 2 on equip.
    pub(crate) bonding: u32,
    /// The green `Use:` or `Equip:` line: `(trigger, text)`, trigger 0 use, 1 equip.
    pub(crate) effect: Option<(u32, &'static str)>,
    pub(crate) armor: u32,
}

const fn goods(
    entry: u32,
    name: &'static str,
    quality: u32,
    icon: &'static str,
    stack: u32,
    sell_price: u32,
) -> ItemDef {
    ItemDef {
        entry,
        name,
        quality,
        class: 7,
        subclass: 0,
        inventory_type: 0,
        icon,
        stack,
        bag_slots: 0,
        family: 0,
        sell_price,
        bonding: 0,
        effect: None,
        armor: 0,
    }
}

/// A potion, elixir, flask, dish or bandage: `ItemClass` 0. What using it does, and the tooltip's
/// green `Use:` line, are [`super::effects::CONSUMABLES`]'.
const fn consumable(
    entry: u32,
    name: &'static str,
    quality: u32,
    icon: &'static str,
    stack: u32,
    sell_price: u32,
) -> ItemDef {
    ItemDef {
        class: 0,
        ..goods(entry, name, quality, icon, stack, sell_price)
    }
}

/// A recipe (`ItemClass` 9; subclass 5 Cooking, 6 Alchemy, 7 First Aid, 8 Enchanting): using it
/// teaches its recipe ([`super::effects`]).
const fn recipe_item(
    entry: u32,
    name: &'static str,
    quality: u32,
    subclass: u32,
    icon: &'static str,
    sell_price: u32,
) -> ItemDef {
    ItemDef {
        class: 9,
        subclass,
        ..goods(entry, name, quality, icon, 1, sell_price)
    }
}

/// Every authored item. Stack sizes are 1.12's as remembered (ore and leather 10; bars, stone,
/// herbs, cloth and fish 20; potions 5); prices and effects are ours.
pub(crate) static ITEMS: &[ItemDef] = &[
    goods(COPPER_ORE, "Copper Ore", 1, "INV_Ore_Copper_01", 10, 5),
    goods(TIN_ORE, "Tin Ore", 1, "INV_Ore_Tin_01", 10, 25),
    goods(COPPER_BAR, "Copper Bar", 1, "INV_Ingot_02", 20, 10),
    goods(PEACEBLOOM, "Peacebloom", 1, "INV_Misc_Flower_02", 20, 5),
    goods(SILVERLEAF, "Silverleaf", 1, "INV_Misc_Herb_10", 20, 5),
    goods(LINEN_CLOTH, "Linen Cloth", 1, "INV_Fabric_Linen_01", 20, 13),
    goods(IRON_ORE, "Iron Ore", 1, "INV_Ore_Iron_01", 10, 150),
    goods(MITHRIL_ORE, "Mithril Ore", 1, "INV_Ore_Mithril_02", 10, 250),
    goods(
        TRUESILVER_ORE,
        "Truesilver Ore",
        2,
        "INV_Ore_TrueSilver_01",
        10,
        500,
    ),
    goods(THORIUM_ORE, "Thorium Ore", 1, "INV_Ore_Thorium_02", 10, 250),
    goods(ROUGH_STONE, "Rough Stone", 1, "INV_Stone_04", 20, 2),
    goods(COARSE_STONE, "Coarse Stone", 1, "INV_Stone_02", 20, 7),
    goods(HEAVY_STONE, "Heavy Stone", 1, "INV_Stone_05", 20, 15),
    goods(SOLID_STONE, "Solid Stone", 1, "INV_Stone_10", 20, 25),
    goods(DENSE_STONE, "Dense Stone", 1, "INV_Stone_08", 20, 250),
    goods(MAGEROYAL, "Mageroyal", 1, "INV_Jewelry_Talisman_03", 20, 5),
    goods(BRUISEWEED, "Bruiseweed", 1, "INV_Misc_Herb_01", 20, 10),
    goods(KINGSBLOOD, "Kingsblood", 1, "INV_Misc_Herb_03", 20, 30),
    goods(LIFEROOT, "Liferoot", 1, "INV_Misc_Root_02", 20, 30),
    goods(FADELEAF, "Fadeleaf", 1, "INV_Misc_Herb_12", 20, 50),
    goods(GOLDTHORN, "Goldthorn", 1, "INV_Misc_Herb_15", 20, 60),
    goods(
        KHADGARS_WHISKER,
        "Khadgar's Whisker",
        1,
        "INV_Misc_Herb_08",
        20,
        60,
    ),
    goods(PURPLE_LOTUS, "Purple Lotus", 1, "INV_Misc_Herb_17", 20, 30),
    goods(SUNGRASS, "Sungrass", 1, "INV_Misc_Herb_18", 20, 60),
    goods(GROMSBLOOD, "Gromsblood", 1, "INV_Misc_Herb_16", 20, 100),
    goods(
        GOLDEN_SANSAM,
        "Golden Sansam",
        1,
        "INV_Misc_Herb_SansamRoot",
        20,
        200,
    ),
    goods(
        DREAMFOIL,
        "Dreamfoil",
        1,
        "INV_Misc_Herb_DreamFoil",
        20,
        200,
    ),
    goods(
        MOUNTAIN_SILVERSAGE,
        "Mountain Silversage",
        1,
        "INV_Misc_Herb_MountainSilverSage",
        20,
        250,
    ),
    goods(
        PLAGUEBLOOM,
        "Plaguebloom",
        1,
        "INV_Misc_Herb_PlagueBloom",
        20,
        250,
    ),
    goods(ICECAP, "Icecap", 1, "INV_Misc_Herb_IceCap", 20, 250),
    goods(
        BLACK_LOTUS,
        "Black Lotus",
        1,
        "INV_Misc_Herb_BlackLotus",
        20,
        1000,
    ),
    goods(
        RUINED_LEATHER_SCRAPS,
        "Ruined Leather Scraps",
        0,
        "INV_Misc_LeatherScrap_02",
        10,
        1,
    ),
    goods(
        LIGHT_LEATHER,
        "Light Leather",
        1,
        "INV_Misc_LeatherScrap_03",
        10,
        15,
    ),
    goods(
        MEDIUM_LEATHER,
        "Medium Leather",
        1,
        "INV_Misc_LeatherScrap_05",
        10,
        25,
    ),
    goods(
        HEAVY_LEATHER,
        "Heavy Leather",
        1,
        "INV_Misc_LeatherScrap_07",
        10,
        60,
    ),
    goods(
        THICK_LEATHER,
        "Thick Leather",
        1,
        "INV_Misc_LeatherScrap_08",
        10,
        150,
    ),
    goods(
        RUGGED_LEATHER,
        "Rugged Leather",
        1,
        "INV_Misc_LeatherScrap_02",
        10,
        250,
    ),
    goods(
        RAW_BRILLIANT_SMALLFISH,
        "Raw Brilliant Smallfish",
        1,
        "INV_Misc_Fish_02",
        20,
        1,
    ),
    goods(
        RAW_LONGJAW_MUD_SNAPPER,
        "Raw Longjaw Mud Snapper",
        1,
        "INV_Misc_Fish_05",
        20,
        2,
    ),
    goods(
        DEVIATE_FISH,
        "Deviate Fish",
        1,
        "INV_Misc_MonsterTail_03",
        20,
        10,
    ),
    goods(
        OILY_BLACKMOUTH,
        "Oily Blackmouth",
        1,
        "INV_Misc_Fish_07",
        20,
        4,
    ),
    goods(
        STONESCALE_EEL,
        "Stonescale Eel",
        1,
        "INV_Misc_Fish_18",
        20,
        10,
    ),
    goods(TIN_BAR, "Tin Bar", 1, "INV_Ingot_05", 20, 25),
    goods(BRONZE_BAR, "Bronze Bar", 1, "INV_Ingot_Bronze", 20, 50),
    goods(IRON_BAR, "Iron Bar", 1, "INV_Ingot_Iron", 20, 200),
    goods(STEEL_BAR, "Steel Bar", 1, "INV_Ingot_Steel", 20, 250),
    goods(MITHRIL_BAR, "Mithril Bar", 1, "INV_Ingot_06", 20, 400),
    goods(
        TRUESILVER_BAR,
        "Truesilver Bar",
        2,
        "INV_Ingot_08",
        20,
        1_000,
    ),
    goods(THORIUM_BAR, "Thorium Bar", 1, "INV_Ingot_07", 20, 600),
    goods(COAL, "Coal", 1, "INV_Stone_15", 20, 125),
    goods(EMPTY_VIAL, "Empty Vial", 1, "INV_Drink_06", 20, 1),
    goods(STRANGLEKELP, "Stranglekelp", 1, "INV_Misc_Herb_11", 20, 15),
    goods(
        WILD_STEELBLOOM,
        "Wild Steelbloom",
        1,
        "INV_Misc_Flower_01",
        20,
        20,
    ),
    goods(LEADED_VIAL, "Leaded Vial", 1, "INV_Drink_06", 20, 10),
    goods(CRYSTAL_VIAL, "Crystal Vial", 1, "INV_Drink_06", 20, 50),
    consumable(
        MINOR_MANA_POTION,
        "Minor Mana Potion",
        1,
        "INV_Potion_70",
        5,
        5,
    ),
    consumable(
        MINOR_REJUVENATION_POTION,
        "Minor Rejuvenation Potion",
        1,
        "INV_Potion_02",
        5,
        10,
    ),
    consumable(
        ELIXIR_OF_MINOR_DEFENSE,
        "Elixir of Minor Defense",
        1,
        "INV_Potion_66",
        5,
        5,
    ),
    goods(BLACKMOUTH_OIL, "Blackmouth Oil", 1, "INV_Drink_12", 20, 15),
    consumable(
        MINOR_MAGIC_RESISTANCE_POTION,
        "Minor Magic Resistance Potion",
        1,
        "INV_Potion_08",
        5,
        30,
    ),
    consumable(
        LESSER_MANA_POTION,
        "Lesser Mana Potion",
        1,
        "INV_Potion_71",
        5,
        30,
    ),
    consumable(
        ELIXIR_OF_DEFENSE,
        "Elixir of Defense",
        1,
        "INV_Potion_66",
        5,
        40,
    ),
    consumable(
        GREATER_HEALING_POTION,
        "Greater Healing Potion",
        1,
        "INV_Potion_52",
        5,
        100,
    ),
    consumable(
        FREE_ACTION_POTION,
        "Free Action Potion",
        1,
        "INV_Potion_04",
        5,
        75,
    ),
    consumable(MANA_POTION, "Mana Potion", 1, "INV_Potion_72", 5, 100),
    consumable(
        ELIXIR_OF_FORTITUDE,
        "Elixir of Fortitude",
        1,
        "INV_Potion_44",
        5,
        110,
    ),
    consumable(
        MIGHTY_TROLLS_BLOOD_POTION,
        "Mighty Troll's Blood Potion",
        1,
        "INV_Potion_79",
        5,
        120,
    ),
    consumable(
        ELIXIR_OF_AGILITY,
        "Elixir of Agility",
        1,
        "INV_Potion_93",
        5,
        120,
    ),
    consumable(
        ELIXIR_OF_GREATER_DEFENSE,
        "Elixir of Greater Defense",
        1,
        "INV_Potion_65",
        5,
        125,
    ),
    consumable(
        GREATER_MANA_POTION,
        "Greater Mana Potion",
        1,
        "INV_Potion_73",
        5,
        150,
    ),
    consumable(
        SUPERIOR_HEALING_POTION,
        "Superior Healing Potion",
        1,
        "INV_Potion_53",
        5,
        250,
    ),
    goods(
        STONESCALE_OIL,
        "Stonescale Oil",
        1,
        "INV_Potion_39",
        20,
        100,
    ),
    consumable(
        ELIXIR_OF_GREATER_AGILITY,
        "Elixir of Greater Agility",
        1,
        "INV_Potion_94",
        5,
        300,
    ),
    consumable(
        ELIXIR_OF_GIANTS,
        "Elixir of Giants",
        1,
        "INV_Potion_61",
        5,
        300,
    ),
    consumable(
        MIGHTY_RAGE_POTION,
        "Mighty Rage Potion",
        1,
        "INV_Potion_41",
        5,
        400,
    ),
    consumable(
        ELIXIR_OF_SUPERIOR_DEFENSE,
        "Elixir of Superior Defense",
        1,
        "INV_Potion_43",
        5,
        400,
    ),
    consumable(
        MAJOR_HEALING_POTION,
        "Major Healing Potion",
        1,
        "INV_Potion_54",
        5,
        1000,
    ),
    consumable(
        ELIXIR_OF_THE_MONGOOSE,
        "Elixir of the Mongoose",
        1,
        "INV_Potion_32",
        5,
        1000,
    ),
    consumable(
        ELIXIR_OF_BRUTE_FORCE,
        "Elixir of Brute Force",
        1,
        "INV_Potion_40",
        5,
        1000,
    ),
    consumable(
        ELIXIR_OF_THE_SAGES,
        "Elixir of the Sages",
        1,
        "INV_Potion_29",
        5,
        1000,
    ),
    consumable(
        GREATER_ARCANE_ELIXIR,
        "Greater Arcane Elixir",
        1,
        "INV_Potion_25",
        5,
        1000,
    ),
    consumable(
        GREATER_STONESHIELD_POTION,
        "Greater Stoneshield Potion",
        1,
        "INV_Potion_69",
        5,
        1000,
    ),
    consumable(
        MAJOR_MANA_POTION,
        "Major Mana Potion",
        1,
        "INV_Potion_76",
        5,
        1500,
    ),
    consumable(
        FLASK_OF_THE_TITANS,
        "Flask of the Titans",
        1,
        "INV_Potion_62",
        1,
        3000,
    ),
    consumable(
        FLASK_OF_SUPREME_POWER,
        "Flask of Supreme Power",
        1,
        "INV_Potion_41",
        1,
        3000,
    ),
    consumable(
        FLASK_OF_DISTILLED_WISDOM,
        "Flask of Distilled Wisdom",
        1,
        "INV_Potion_97",
        1,
        3000,
    ),
    goods(
        STRINGY_WOLF_MEAT,
        "Stringy Wolf Meat",
        1,
        "INV_Misc_Food_14",
        10,
        4,
    ),
    goods(COYOTE_MEAT, "Coyote Meat", 1, "INV_Misc_Food_14", 10, 10),
    goods(BEAR_MEAT, "Bear Meat", 1, "INV_Misc_Food_14", 10, 15),
    goods(
        BIG_BEAR_MEAT,
        "Big Bear Meat",
        1,
        "INV_Misc_Food_14",
        10,
        30,
    ),
    goods(LION_MEAT, "Lion Meat", 1, "INV_Misc_Food_14", 10, 35),
    goods(RAPTOR_FLESH, "Raptor Flesh", 1, "INV_Misc_Food_14", 10, 85),
    goods(
        RED_WOLF_MEAT,
        "Red Wolf Meat",
        1,
        "INV_Misc_Food_14",
        10,
        85,
    ),
    goods(
        TENDER_WOLF_MEAT,
        "Tender Wolf Meat",
        1,
        "INV_Misc_Food_14",
        10,
        150,
    ),
    goods(GIANT_EGG, "Giant Egg", 1, "INV_Egg_02", 20, 150),
    goods(RAW_SAGEFISH, "Raw Sagefish", 1, "INV_Misc_Fish_20", 20, 4),
    goods(
        RAW_SPOTTED_YELLOWTAIL,
        "Raw Spotted Yellowtail",
        1,
        "INV_Misc_Fish_10",
        20,
        6,
    ),
    goods(WINTER_SQUID, "Winter Squid", 1, "INV_Misc_Fish_13", 20, 8),
    goods(
        RAW_NIGHTFIN_SNAPPER,
        "Raw Nightfin Snapper",
        1,
        "INV_Misc_Fish_03",
        20,
        8,
    ),
    goods(MILD_SPICES, "Mild Spices", 1, "INV_Misc_Dust_02", 20, 1),
    goods(HOT_SPICES, "Hot Spices", 1, "INV_Misc_Dust_01", 20, 10),
    goods(
        SOOTHING_SPICES,
        "Soothing Spices",
        1,
        "INV_Misc_Dust_06",
        20,
        40,
    ),
    goods(
        REFRESHING_SPRING_WATER,
        "Refreshing Spring Water",
        1,
        "INV_Drink_07",
        20,
        1,
    ),
    consumable(
        SPICED_WOLF_MEAT,
        "Spiced Wolf Meat",
        1,
        "INV_Misc_Food_09",
        20,
        6,
    ),
    consumable(
        BRILLIANT_SMALLFISH,
        "Brilliant Smallfish",
        1,
        "INV_Misc_Fish_02",
        20,
        1,
    ),
    consumable(COYOTE_STEAK, "Coyote Steak", 1, "INV_Misc_Food_16", 20, 20),
    consumable(
        LONGJAW_MUD_SNAPPER,
        "Longjaw Mud Snapper",
        1,
        "INV_Misc_Fish_05",
        20,
        3,
    ),
    consumable(
        SMOKED_BEAR_MEAT,
        "Smoked Bear Meat",
        1,
        "INV_Misc_Food_14",
        20,
        10,
    ),
    consumable(
        SMOKED_SAGEFISH,
        "Smoked Sagefish",
        1,
        "INV_Misc_Fish_20",
        20,
        6,
    ),
    consumable(
        SAVORY_DEVIATE_DELIGHT,
        "Savory Deviate Delight",
        1,
        "INV_Misc_MonsterTail_03",
        20,
        30,
    ),
    consumable(
        BIG_BEAR_STEAK,
        "Big Bear Steak",
        1,
        "INV_Misc_Food_17",
        20,
        40,
    ),
    consumable(
        HOT_LION_CHOPS,
        "Hot Lion Chops",
        1,
        "INV_Misc_Food_13",
        20,
        50,
    ),
    consumable(ROAST_RAPTOR, "Roast Raptor", 1, "INV_Misc_Food_19", 20, 75),
    consumable(
        HOT_WOLF_RIBS,
        "Hot Wolf Ribs",
        1,
        "INV_Misc_Food_18",
        20,
        75,
    ),
    consumable(
        SPOTTED_YELLOWTAIL,
        "Spotted Yellowtail",
        1,
        "INV_Misc_Fish_10",
        20,
        4,
    ),
    consumable(
        MONSTER_OMELET,
        "Monster Omelet",
        1,
        "INV_Misc_Food_06",
        20,
        100,
    ),
    consumable(
        TENDER_WOLF_STEAK,
        "Tender Wolf Steak",
        1,
        "INV_Misc_Food_08",
        20,
        100,
    ),
    consumable(
        GRILLED_SQUID,
        "Grilled Squid",
        1,
        "INV_Misc_Fish_13",
        20,
        120,
    ),
    consumable(NIGHTFIN_SOUP, "Nightfin Soup", 1, "INV_Drink_17", 20, 120),
    goods(WOOL_CLOTH, "Wool Cloth", 1, "INV_Fabric_Wool_01", 20, 33),
    goods(SILK_CLOTH, "Silk Cloth", 1, "INV_Fabric_Silk_01", 20, 38),
    goods(
        MAGEWEAVE_CLOTH,
        "Mageweave Cloth",
        1,
        "INV_Fabric_Mageweave_01",
        20,
        25,
    ),
    goods(
        RUNECLOTH,
        "Runecloth",
        1,
        "INV_Fabric_PurpleFire_01",
        20,
        40,
    ),
    consumable(
        LINEN_BANDAGE,
        "Linen Bandage",
        1,
        "INV_Misc_Bandage_15",
        20,
        10,
    ),
    consumable(
        HEAVY_LINEN_BANDAGE,
        "Heavy Linen Bandage",
        1,
        "INV_Misc_Bandage_18",
        20,
        20,
    ),
    consumable(
        WOOL_BANDAGE,
        "Wool Bandage",
        1,
        "INV_Misc_Bandage_14",
        20,
        28,
    ),
    consumable(
        HEAVY_WOOL_BANDAGE,
        "Heavy Wool Bandage",
        1,
        "INV_Misc_Bandage_17",
        20,
        50,
    ),
    consumable(
        SILK_BANDAGE,
        "Silk Bandage",
        1,
        "INV_Misc_Bandage_01",
        20,
        100,
    ),
    consumable(
        HEAVY_SILK_BANDAGE,
        "Heavy Silk Bandage",
        1,
        "INV_Misc_Bandage_02",
        20,
        200,
    ),
    consumable(
        MAGEWEAVE_BANDAGE,
        "Mageweave Bandage",
        1,
        "INV_Misc_Bandage_19",
        20,
        250,
    ),
    consumable(
        HEAVY_MAGEWEAVE_BANDAGE,
        "Heavy Mageweave Bandage",
        1,
        "INV_Misc_Bandage_20",
        20,
        500,
    ),
    consumable(
        RUNECLOTH_BANDAGE,
        "Runecloth Bandage",
        1,
        "INV_Misc_Bandage_11",
        20,
        400,
    ),
    consumable(
        HEAVY_RUNECLOTH_BANDAGE,
        "Heavy Runecloth Bandage",
        1,
        "INV_Misc_Bandage_12",
        20,
        1000,
    ),
    goods(
        STRANGE_DUST,
        "Strange Dust",
        1,
        "INV_Enchant_DustStrange",
        20,
        25,
    ),
    goods(
        LESSER_MAGIC_ESSENCE,
        "Lesser Magic Essence",
        2,
        "INV_Enchant_EssenceMagicSmall",
        10,
        30,
    ),
    goods(
        GREATER_MAGIC_ESSENCE,
        "Greater Magic Essence",
        2,
        "INV_Enchant_EssenceMagicLarge",
        10,
        90,
    ),
    goods(
        SMALL_GLIMMERING_SHARD,
        "Small Glimmering Shard",
        2,
        "INV_Enchant_ShardGlimmeringSmall",
        20,
        100,
    ),
    goods(SOUL_DUST, "Soul Dust", 1, "INV_Enchant_DustSoul", 20, 75),
    goods(
        LESSER_ASTRAL_ESSENCE,
        "Lesser Astral Essence",
        2,
        "INV_Enchant_EssenceAstralSmall",
        10,
        100,
    ),
    goods(
        GREATER_ASTRAL_ESSENCE,
        "Greater Astral Essence",
        2,
        "INV_Enchant_EssenceAstralLarge",
        10,
        300,
    ),
    goods(
        LARGE_GLIMMERING_SHARD,
        "Large Glimmering Shard",
        2,
        "INV_Enchant_ShardGlimmeringLarge",
        20,
        200,
    ),
    goods(
        VISION_DUST,
        "Vision Dust",
        1,
        "INV_Enchant_DustVision",
        20,
        150,
    ),
    goods(
        LESSER_MYSTIC_ESSENCE,
        "Lesser Mystic Essence",
        2,
        "INV_Enchant_EssenceMysticalSmall",
        10,
        200,
    ),
    goods(
        GREATER_MYSTIC_ESSENCE,
        "Greater Mystic Essence",
        2,
        "INV_Enchant_EssenceMysticalLarge",
        10,
        600,
    ),
    goods(
        LARGE_GLOWING_SHARD,
        "Large Glowing Shard",
        2,
        "INV_Enchant_ShardGlowingLarge",
        20,
        500,
    ),
    goods(
        DREAM_DUST,
        "Dream Dust",
        1,
        "INV_Enchant_DustDream",
        20,
        250,
    ),
    goods(
        LESSER_NETHER_ESSENCE,
        "Lesser Nether Essence",
        2,
        "INV_Enchant_EssenceNetherSmall",
        10,
        400,
    ),
    goods(
        GREATER_NETHER_ESSENCE,
        "Greater Nether Essence",
        2,
        "INV_Enchant_EssenceNetherLarge",
        10,
        1200,
    ),
    goods(
        SMALL_RADIANT_SHARD,
        "Small Radiant Shard",
        2,
        "INV_Enchant_ShardRadientSmall",
        20,
        800,
    ),
    goods(
        LARGE_RADIANT_SHARD,
        "Large Radiant Shard",
        2,
        "INV_Enchant_ShardRadientLarge",
        20,
        1500,
    ),
    goods(
        ILLUSION_DUST,
        "Illusion Dust",
        1,
        "INV_Enchant_DustIllusion",
        20,
        500,
    ),
    goods(
        LESSER_ETERNAL_ESSENCE,
        "Lesser Eternal Essence",
        2,
        "INV_Enchant_EssenceEternalSmall",
        10,
        800,
    ),
    goods(
        GREATER_ETERNAL_ESSENCE,
        "Greater Eternal Essence",
        2,
        "INV_Enchant_EssenceEternalLarge",
        10,
        2000,
    ),
    goods(
        SMALL_BRILLIANT_SHARD,
        "Small Brilliant Shard",
        3,
        "INV_Enchant_ShardBrilliantSmall",
        20,
        1500,
    ),
    goods(
        LARGE_BRILLIANT_SHARD,
        "Large Brilliant Shard",
        3,
        "INV_Enchant_ShardBrilliantLarge",
        20,
        3000,
    ),
    goods(
        ESSENCE_OF_FIRE,
        "Essence of Fire",
        2,
        "Spell_Fire_Volcano",
        10,
        1000,
    ),
    goods(
        RIGHTEOUS_ORB,
        "Righteous Orb",
        2,
        "INV_Misc_Gem_Pearl_05",
        20,
        2000,
    ),
    recipe_item(
        RECIPE_FLASK_OF_THE_TITANS,
        "Recipe: Flask of the Titans",
        2,
        6,
        "INV_Scroll_06",
        2500,
    ),
    recipe_item(
        RECIPE_FLASK_OF_SUPREME_POWER,
        "Recipe: Flask of Supreme Power",
        2,
        6,
        "INV_Scroll_06",
        2500,
    ),
    recipe_item(
        RECIPE_FLASK_OF_DISTILLED_WISDOM,
        "Recipe: Flask of Distilled Wisdom",
        2,
        6,
        "INV_Scroll_06",
        2500,
    ),
    recipe_item(
        RECIPE_SAVORY_DEVIATE_DELIGHT,
        "Recipe: Savory Deviate Delight",
        2,
        5,
        "INV_Scroll_03",
        375,
    ),
    recipe_item(
        FORMULA_ENCHANT_WEAPON_CRUSADER,
        "Formula: Enchant Weapon - Crusader",
        3,
        8,
        "INV_Scroll_05",
        3000,
    ),
    ItemDef {
        entry: MINOR_HEALING_POTION,
        name: "Minor Healing Potion",
        quality: 1,
        class: 0,
        subclass: 0,
        inventory_type: 0,
        icon: "INV_Potion_49",
        stack: 5,
        bag_slots: 0,
        family: 0,
        sell_price: 5,
        bonding: 0,
        effect: Some((0, "Restores 15% of your health.")),
        armor: 0,
    },
    ItemDef {
        entry: LINEN_BAG,
        name: "Linen Bag",
        quality: 1,
        class: 1,
        subclass: 0,
        inventory_type: 18,
        icon: "INV_Misc_Bag_10",
        stack: 1,
        bag_slots: 6,
        family: 0,
        sell_price: 100,
        bonding: 0,
        effect: None,
        armor: 0,
    },
    ItemDef {
        entry: HELM_OF_VALOR,
        name: "Helm of Valor",
        quality: 3,
        class: 4,
        subclass: 4,
        inventory_type: 1,
        icon: "INV_Helmet_02",
        stack: 1,
        bag_slots: 0,
        family: 0,
        sell_price: 9_870,
        bonding: 1,
        effect: Some((1, "Increases damage done by 3%.")),
        armor: 548,
    },
];

/// The test inventory, `(entry, count)`: ore, a bar, herbs, cloth, potions, a spare bag and a
/// helm (P0), then a bandage, an elixir, a dish and Strange Dust (P2).
pub(crate) static TEST_INVENTORY: &[(u32, u32)] = &[
    (COPPER_ORE, 14),
    (TIN_ORE, 6),
    (COPPER_BAR, 8),
    (PEACEBLOOM, 9),
    (SILVERLEAF, 17),
    (LINEN_CLOTH, 20),
    (MINOR_HEALING_POTION, 4),
    (LINEN_BAG, 1),
    (HELM_OF_VALOR, 1),
    (LINEN_BANDAGE, 5),
    (ELIXIR_OF_GIANTS, 2),
    (SPICED_WOLF_MEAT, 4),
    (STRANGE_DUST, 6),
];

pub(crate) fn template(entry: u32) -> Option<&'static ItemDef> {
    ITEMS.iter().find(|d| d.entry == entry)
}

/// `Interface\Icons\<icon>`, the path a slot and a template carry.
pub(crate) fn icon_path(def: &ItemDef) -> String {
    format!("Interface\\Icons\\{}", def.icon)
}

/// `ITEM_QUALITY_COLORS`' hex for `quality`, the colour a link wears.
fn quality_hex(quality: u32) -> &'static str {
    match quality {
        0 => "ff9d9d9d",
        1 => "ffffffff",
        2 => "ff1eff00",
        3 => "ff0070dd",
        4 => "ffa335ee",
        _ => "ffff8000",
    }
}

/// The 1.12 chat link, `|cff…|Hitem:<entry>:0:0:0|h[Name]|h|r`.
pub(crate) fn link(def: &ItemDef) -> String {
    format!(
        "|c{}|Hitem:{}:0:0:0|h[{}]|h|r",
        quality_hex(def.quality),
        def.entry,
        def.name
    )
}

/// `ItemClass.dbc`'s name for the classes authored here.
fn class_name(class: u32) -> &'static str {
    match class {
        0 => "Consumable",
        1 => "Container",
        4 => "Armor",
        9 => "Recipe",
        _ => "Trade Goods",
    }
}

/// `ItemSubClass.dbc`'s display name for the subclasses authored here.
fn subclass_name(class: u32, subclass: u32) -> &'static str {
    match (class, subclass) {
        (1, _) => "Bag",
        (4, 4) => "Plate",
        (0, _) => "Consumable",
        (9, 5) => "Cooking",
        (9, 6) => "Alchemy",
        (9, 7) => "First Aid",
        (9, 8) => "Enchanting",
        _ => "Trade Goods",
    }
}

/// The tooltip view the stock `GameTooltip` renders: what the server's template would say.
pub(crate) fn template_view(def: &ItemDef) -> ItemTemplateView {
    ItemTemplateView {
        name: def.name.to_string(),
        quality: def.quality,
        class: def.class,
        subclass: def.subclass,
        inventory_type: def.inventory_type,
        item_type: Some(class_name(def.class).to_string()),
        item_sub_type: Some(subclass_name(def.class, def.subclass).to_string()),
        sub_class_display: Some(subclass_name(def.class, def.subclass).to_string()),
        // Trade goods, consumables and bags print no type cell.
        hide_subclass: def.class != 4,
        bonding: def.bonding,
        stackable: def.stack,
        container_slots: def.bag_slots,
        armor: def.armor,
        // Every class and race may use them: `<= 0` prints no line.
        allowable_class: -1,
        allowable_race: -1,
        spell_triggers: super::effects::use_line(def.entry)
            .map(|text| (0, 0, text))
            .or_else(|| {
                def.effect
                    .map(|(trigger, text)| (trigger, 0, text.to_string()))
            })
            .into_iter()
            .collect(),
        sell_price: def.sell_price,
        icon: Some(icon_path(def)),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn survivors_items_entries_are_unique_and_sane() {
        let mut seen = std::collections::HashSet::new();
        for d in ITEMS {
            assert!(seen.insert(d.entry), "{} listed twice", d.entry);
            assert!(d.stack >= 1, "{}: a stack holds one at least", d.name);
            assert_eq!(
                d.bag_slots > 0,
                d.class == 1,
                "{}: only bags have slots",
                d.name
            );
        }
        for &(entry, count) in TEST_INVENTORY {
            let d = template(entry).expect("a test item is authored");
            assert!(count >= 1 && d.stack >= 1);
        }
    }

    #[test]
    fn survivors_items_link_and_view_read_like_the_server() {
        let ore = template(COPPER_ORE).unwrap();
        assert_eq!(link(ore), "|cffffffff|Hitem:2770:0:0:0|h[Copper Ore]|h|r");
        let v = template_view(ore);
        assert_eq!(v.stackable, 10);
        assert_eq!(
            v.icon.as_deref(),
            Some("Interface\\Icons\\INV_Ore_Copper_01")
        );
        let bag = template_view(template(LINEN_BAG).unwrap());
        assert_eq!(bag.container_slots, 6);
        let potion = template_view(template(MINOR_HEALING_POTION).unwrap());
        assert_eq!(potion.spell_triggers[0].0, 0, "a Use: line");
        let helm = template(HELM_OF_VALOR).unwrap();
        assert!(link(helm).starts_with("|cff0070dd"), "a Rare link is blue");
    }

    /// Each item's display, `(entry, ItemDisplayInfo.dbc id)`: a row carrying the item's icon. The
    /// helm's is the gear catalog's (`gear/catalog.rs`, id 120).
    const DISPLAYS: &[(u32, u32)] = &[
        (COPPER_ORE, 4681),
        (TIN_ORE, 7340),
        (COPPER_BAR, 7391),
        (PEACEBLOOM, 7396),
        (SILVERLEAF, 18088),
        (LINEN_CLOTH, 7383),
        (MINOR_HEALING_POTION, 15710),
        (LINEN_BAG, 1183),
        (HELM_OF_VALOR, 31284),
        (IRON_ORE, 4689),
        (MITHRIL_ORE, 20661),
        (TRUESILVER_ORE, 20657),
        (THORIUM_ORE, 20658),
        (ROUGH_STONE, 1364),
        (COARSE_STONE, 3131),
        (HEAVY_STONE, 1405),
        (SOLID_STONE, 4719),
        (DENSE_STONE, 4718),
        (MAGEROYAL, 942),
        (BRUISEWEED, 2025),
        (KINGSBLOOD, 2948),
        (LIFEROOT, 1500),
        (FADELEAF, 18169),
        (GOLDTHORN, 19497),
        (KHADGARS_WHISKER, 1264),
        (PURPLE_LOTUS, 19493),
        (SUNGRASS, 19492),
        (GROMSBLOOD, 19494),
        (GOLDEN_SANSAM, 24690),
        (DREAMFOIL, 24689),
        (MOUNTAIN_SILVERSAGE, 24692),
        (PLAGUEBLOOM, 24693),
        (ICECAP, 24691),
        (BLACK_LOTUS, 24688),
        (RUINED_LEATHER_SCRAPS, 4673),
        (LIGHT_LEATHER, 4674),
        (MEDIUM_LEATHER, 4675),
        (HEAVY_LEATHER, 4677),
        (THICK_LEATHER, 7594),
        (RUGGED_LEATHER, 20951),
        (RAW_BRILLIANT_SMALLFISH, 1208),
        (RAW_LONGJAW_MUD_SNAPPER, 4810),
        (DEVIATE_FISH, 18096),
        (OILY_BLACKMOUTH, 18536),
        (STONESCALE_EEL, 32520),
        (TIN_BAR, 18086),
        (BRONZE_BAR, 7390),
        (IRON_BAR, 7376),
        (STEEL_BAR, 7392),
        (MITHRIL_BAR, 20659),
        (TRUESILVER_BAR, 20656),
        (THORIUM_BAR, 22444),
        (COAL, 15805),
        (EMPTY_VIAL, 18076),
        (STRANGLEKELP, 18089),
        (WILD_STEELBLOOM, 1307),
        (LEADED_VIAL, 18076),
        (CRYSTAL_VIAL, 18076),
        (MINOR_MANA_POTION, 15715),
        (MINOR_REJUVENATION_POTION, 2345),
        (ELIXIR_OF_MINOR_DEFENSE, 16321),
        (BLACKMOUTH_OIL, 18113),
        (MINOR_MAGIC_RESISTANCE_POTION, 292),
        (LESSER_MANA_POTION, 15716),
        (ELIXIR_OF_DEFENSE, 16321),
        (GREATER_HEALING_POTION, 15713),
        (FREE_ACTION_POTION, 2348),
        (MANA_POTION, 15717),
        (ELIXIR_OF_FORTITUDE, 15774),
        (MIGHTY_TROLLS_BLOOD_POTION, 15793),
        (ELIXIR_OF_AGILITY, 17882),
        (ELIXIR_OF_GREATER_DEFENSE, 17883),
        (GREATER_MANA_POTION, 15718),
        (SUPERIOR_HEALING_POTION, 15714),
        (STONESCALE_OIL, 29352),
        (ELIXIR_OF_GREATER_AGILITY, 17902),
        (ELIXIR_OF_GIANTS, 17904),
        (MIGHTY_RAGE_POTION, 19547),
        (ELIXIR_OF_SUPERIOR_DEFENSE, 15790),
        (MAJOR_HEALING_POTION, 24152),
        (ELIXIR_OF_THE_MONGOOSE, 16836),
        (ELIXIR_OF_BRUTE_FORCE, 24212),
        (ELIXIR_OF_THE_SAGES, 21836),
        (GREATER_ARCANE_ELIXIR, 17898),
        (GREATER_STONESHIELD_POTION, 17974),
        (MAJOR_MANA_POTION, 21672),
        (FLASK_OF_THE_TITANS, 24213),
        (FLASK_OF_SUPREME_POWER, 19547),
        (FLASK_OF_DISTILLED_WISDOM, 21531),
        (STRINGY_WOLF_MEAT, 1117),
        (COYOTE_MEAT, 1117),
        (BEAR_MEAT, 1117),
        (BIG_BEAR_MEAT, 1117),
        (LION_MEAT, 1117),
        (RAPTOR_FLESH, 1117),
        (RED_WOLF_MEAT, 1117),
        (TENDER_WOLF_MEAT, 1117),
        (GIANT_EGG, 18047),
        (RAW_SAGEFISH, 24719),
        (RAW_SPOTTED_YELLOWTAIL, 24521),
        (WINTER_SQUID, 18537),
        (RAW_NIGHTFIN_SNAPPER, 4813),
        (MILD_SPICES, 2480),
        (HOT_SPICES, 1295),
        (SOOTHING_SPICES, 31325),
        (REFRESHING_SPRING_WATER, 18082),
        (SPICED_WOLF_MEAT, 1483),
        (BRILLIANT_SMALLFISH, 1208),
        (COYOTE_STEAK, 1116),
        (LONGJAW_MUD_SNAPPER, 4810),
        (SMOKED_BEAR_MEAT, 1117),
        (SMOKED_SAGEFISH, 24719),
        (SAVORY_DEVIATE_DELIGHT, 18096),
        (BIG_BEAR_STEAK, 3397),
        (HOT_LION_CHOPS, 1762),
        (ROAST_RAPTOR, 4779),
        (HOT_WOLF_RIBS, 2022),
        (SPOTTED_YELLOWTAIL, 24521),
        (MONSTER_OMELET, 993),
        (TENDER_WOLF_STEAK, 1144),
        (GRILLED_SQUID, 18537),
        (NIGHTFIN_SOUP, 21473),
        (WOOL_CLOTH, 4746),
        (SILK_CLOTH, 4747),
        (MAGEWEAVE_CLOTH, 4751),
        (RUNECLOTH, 24897),
        (LINEN_BANDAGE, 11907),
        (HEAVY_LINEN_BANDAGE, 7588),
        (WOOL_BANDAGE, 11909),
        (HEAVY_WOOL_BANDAGE, 7598),
        (SILK_BANDAGE, 8603),
        (HEAVY_SILK_BANDAGE, 11926),
        (MAGEWEAVE_BANDAGE, 17457),
        (HEAVY_MAGEWEAVE_BANDAGE, 17458),
        (RUNECLOTH_BANDAGE, 25146),
        (HEAVY_RUNECLOTH_BANDAGE, 25147),
        (STRANGE_DUST, 20611),
        (LESSER_MAGIC_ESSENCE, 20608),
        (GREATER_MAGIC_ESSENCE, 20609),
        (SMALL_GLIMMERING_SHARD, 20612),
        (SOUL_DUST, 20614),
        (LESSER_ASTRAL_ESSENCE, 20610),
        (GREATER_ASTRAL_ESSENCE, 20613),
        (LARGE_GLIMMERING_SHARD, 20615),
        (VISION_DUST, 20798),
        (LESSER_MYSTIC_ESSENCE, 20794),
        (GREATER_MYSTIC_ESSENCE, 20795),
        (LARGE_GLOWING_SHARD, 20784),
        (DREAM_DUST, 20899),
        (LESSER_NETHER_ESSENCE, 20895),
        (GREATER_NETHER_ESSENCE, 20896),
        (SMALL_RADIANT_SHARD, 20901),
        (LARGE_RADIANT_SHARD, 20902),
        (ILLUSION_DUST, 26773),
        (LESSER_ETERNAL_ESSENCE, 26771),
        (GREATER_ETERNAL_ESSENCE, 26772),
        (SMALL_BRILLIANT_SHARD, 25054),
        (LARGE_BRILLIANT_SHARD, 25055),
        (ESSENCE_OF_FIRE, 23287),
        (RIGHTEOUS_ORB, 29175),
        (RECIPE_FLASK_OF_THE_TITANS, 15274),
        (RECIPE_FLASK_OF_SUPREME_POWER, 15274),
        (RECIPE_FLASK_OF_DISTILLED_WISDOM, 15274),
        (RECIPE_SAVORY_DEVIATE_DELIGHT, 811),
        (FORMULA_ENCHANT_WEAPON_CRUSADER, 1097),
    ];

    /// The looks are real: each display id is an `ItemDisplayInfo.dbc` row whose inventory icon is
    /// the item's, and each icon ships as `Interface\Icons\<name>.blp`. Every mismatch is listed.
    #[test]
    fn displays_and_icons_exist_in_the_client() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open the MPQ chain");
        let cat = benilla_formats::load_item_display_catalog(&mut chain).expect("ItemDisplayInfo");
        let mut bad = Vec::new();
        for d in ITEMS {
            let path = icon_path(d);
            let Some(&(_, display_id)) = DISPLAYS.iter().find(|(e, _)| *e == d.entry) else {
                bad.push(format!("{}: no display listed", d.name));
                continue;
            };
            match cat.get(display_id) {
                None => bad.push(format!("{}: no display {display_id}", d.name)),
                Some(row)
                    if !row
                        .icon
                        .as_deref()
                        .is_some_and(|c| c.eq_ignore_ascii_case(&path)) =>
                {
                    bad.push(format!(
                        "{}: display {display_id} shows {:?}",
                        d.name, row.icon
                    ));
                }
                Some(_) => {}
            }
            if !chain.contains(&format!("{path}.blp")) {
                bad.push(format!("{}: no {path}.blp", d.name));
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }
}
