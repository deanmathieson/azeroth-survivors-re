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

// P3's items (docs/PROFESSIONS.md, 5.2): Blacksmithing's, Leatherworking's, Tailoring's and
// Engineering's reagents and products, the ores, bars, gems and elementals they need, the goblin's
// threads, fluxes and dyes, Molten Core's materials, and the recipes that drop. Every entry but
// the recipe items', the gems', the elementals' and the vendor goods' is a reagent or product of a
// curated recipe's real `Spell.dbc` row; each recipe item's name is its teaching spell's
// (`recipes::tests::recipe_items_teach_their_recipes_in_the_client`). Names of the rest, prices,
// stack sizes and icons are from memory, each icon checked to ship with an `ItemDisplayInfo.dbc`
// row.
/// Flask of Oil.
pub(crate) const FLASK_OF_OIL: u32 = 814;
/// Moss Agate.
pub(crate) const MOSS_AGATE: u32 = 1206;
/// Shadowgem.
pub(crate) const SHADOWGEM: u32 = 1210;
/// Jade.
pub(crate) const JADE: u32 = 1529;
/// Lesser Moonstone.
pub(crate) const LESSER_MOONSTONE: u32 = 1705;
/// Embossed Leather Vest.
pub(crate) const EMBOSSED_LEATHER_VEST: u32 = 2300;
/// Handstitched Leather Boots.
pub(crate) const HANDSTITCHED_LEATHER_BOOTS: u32 = 2302;
/// Handstitched Leather Pants.
pub(crate) const HANDSTITCHED_LEATHER_PANTS: u32 = 2303;
/// Light Armor Kit.
pub(crate) const LIGHT_ARMOR_KIT: u32 = 2304;
/// Medium Armor Kit.
pub(crate) const MEDIUM_ARMOR_KIT: u32 = 2313;
/// Dark Leather Boots.
pub(crate) const DARK_LEATHER_BOOTS: u32 = 2315;
/// Dark Leather Tunic.
pub(crate) const DARK_LEATHER_TUNIC: u32 = 2317;
/// Coarse Thread.
pub(crate) const COARSE_THREAD: u32 = 2320;
/// Fine Thread.
pub(crate) const FINE_THREAD: u32 = 2321;
/// Brown Linen Vest.
pub(crate) const BROWN_LINEN_VEST: u32 = 2568;
/// Woolen Boots.
pub(crate) const WOOLEN_BOOTS: u32 = 2583;
/// Green Dye.
pub(crate) const GREEN_DYE: u32 = 2605;
/// Silver Ore.
pub(crate) const SILVER_ORE: u32 = 2775;
/// Gold Ore.
pub(crate) const GOLD_ORE: u32 = 2776;
/// Silver Bar.
pub(crate) const SILVER_BAR: u32 = 2842;
/// Copper Mace.
pub(crate) const COPPER_MACE: u32 = 2844;
/// Bronze Axe.
pub(crate) const BRONZE_AXE: u32 = 2849;
/// Copper Chain Belt.
pub(crate) const COPPER_CHAIN_BELT: u32 = 2851;
/// Copper Chain Pants.
pub(crate) const COPPER_CHAIN_PANTS: u32 = 2852;
/// Copper Bracers.
pub(crate) const COPPER_BRACERS: u32 = 2853;
/// Rough Sharpening Stone.
pub(crate) const ROUGH_SHARPENING_STONE: u32 = 2862;
/// Coarse Sharpening Stone.
pub(crate) const COARSE_SHARPENING_STONE: u32 = 2863;
/// Rough Bronze Leggings.
pub(crate) const ROUGH_BRONZE_LEGGINGS: u32 = 2865;
/// Heavy Sharpening Stone.
pub(crate) const HEAVY_SHARPENING_STONE: u32 = 2871;
/// Weak Flux.
pub(crate) const WEAK_FLUX: u32 = 2880;
/// Bolt of Linen Cloth.
pub(crate) const BOLT_OF_LINEN_CLOTH: u32 = 2996;
/// Bolt of Woolen Cloth.
pub(crate) const BOLT_OF_WOOLEN_CLOTH: u32 = 2997;
/// Rough Weightstone.
pub(crate) const ROUGH_WEIGHTSTONE: u32 = 3239;
/// Coarse Weightstone.
pub(crate) const COARSE_WEIGHTSTONE: u32 = 3240;
/// Heavy Weightstone.
pub(crate) const HEAVY_WEIGHTSTONE: u32 = 3241;
/// Strong Flux.
pub(crate) const STRONG_FLUX: u32 = 3466;
/// Rough Grinding Stone.
pub(crate) const ROUGH_GRINDING_STONE: u32 = 3470;
/// Coarse Grinding Stone.
pub(crate) const COARSE_GRINDING_STONE: u32 = 3478;
/// Heavy Grinding Stone.
pub(crate) const HEAVY_GRINDING_STONE: u32 = 3486;
/// Gold Bar.
pub(crate) const GOLD_BAR: u32 = 3577;
/// Green Iron Bracers.
pub(crate) const GREEN_IRON_BRACERS: u32 = 3835;
/// Green Iron Helm.
pub(crate) const GREEN_IRON_HELM: u32 = 3836;
/// Hardened Iron Shortsword.
pub(crate) const HARDENED_IRON_SHORTSWORD: u32 = 3849;
/// Citrine.
pub(crate) const CITRINE: u32 = 3864;
/// Handstitched Leather Belt.
pub(crate) const HANDSTITCHED_LEATHER_BELT: u32 = 4237;
/// Embossed Leather Gloves.
pub(crate) const EMBOSSED_LEATHER_GLOVES: u32 = 4239;
/// Woolen Bag.
pub(crate) const WOOLEN_BAG: u32 = 4240;
/// Small Silk Pack.
pub(crate) const SMALL_SILK_PACK: u32 = 4245;
/// Heavy Armor Kit.
pub(crate) const HEAVY_ARMOR_KIT: u32 = 4265;
/// Silken Thread.
pub(crate) const SILKEN_THREAD: u32 = 4291;
/// Bolt of Silk Cloth.
pub(crate) const BOLT_OF_SILK_CLOTH: u32 = 4305;
/// Heavy Linen Gloves.
pub(crate) const HEAVY_LINEN_GLOVES: u32 = 4307;
/// Heavy Woolen Gloves.
pub(crate) const HEAVY_WOOLEN_GLOVES: u32 = 4310;
/// Double-stitched Woolen Shoulders.
pub(crate) const DOUBLE_STITCHED_WOOLEN_SHOULDERS: u32 = 4314;
/// Heavy Woolen Pants.
pub(crate) const HEAVY_WOOLEN_PANTS: u32 = 4316;
/// Thick Spider's Silk.
pub(crate) const THICK_SPIDERS_SILK: u32 = 4337;
/// Bolt of Mageweave.
pub(crate) const BOLT_OF_MAGEWEAVE: u32 = 4339;
/// Gray Dye.
pub(crate) const GRAY_DYE: u32 = 4340;
/// Rough Blasting Powder.
pub(crate) const ROUGH_BLASTING_POWDER: u32 = 4357;
/// Handful of Copper Bolts.
pub(crate) const HANDFUL_OF_COPPER_BOLTS: u32 = 4359;
/// Rough Copper Bomb.
pub(crate) const ROUGH_COPPER_BOMB: u32 = 4360;
/// Copper Modulator.
pub(crate) const COPPER_MODULATOR: u32 = 4363;
/// Coarse Blasting Powder.
pub(crate) const COARSE_BLASTING_POWDER: u32 = 4364;
/// Target Dummy.
pub(crate) const TARGET_DUMMY: u32 = 4366;
/// Whirring Bronze Gizmo.
pub(crate) const WHIRRING_BRONZE_GIZMO: u32 = 4375;
/// Heavy Blasting Powder.
pub(crate) const HEAVY_BLASTING_POWDER: u32 = 4377;
/// Bronze Framework.
pub(crate) const BRONZE_FRAMEWORK: u32 = 4382;
/// Explosive Sheep.
pub(crate) const EXPLOSIVE_SHEEP: u32 = 4384;
/// Iron Strut.
pub(crate) const IRON_STRUT: u32 = 4387;
/// Gyrochronatom.
pub(crate) const GYROCHRONATOM: u32 = 4389;
/// Iron Grenade.
pub(crate) const IRON_GRENADE: u32 = 4390;
/// Big Iron Bomb.
pub(crate) const BIG_IRON_BOMB: u32 = 4394;
/// Mechanical Dragonling.
pub(crate) const MECHANICAL_DRAGONLING: u32 = 4396;
/// Silver Contact.
pub(crate) const SILVER_CONTACT: u32 = 4404;
/// Barbaric Harness.
pub(crate) const BARBARIC_HARNESS: u32 = 5739;
/// Handstitched Leather Vest.
pub(crate) const HANDSTITCHED_LEATHER_VEST: u32 = 5957;
/// Dark Leather Pants.
pub(crate) const DARK_LEATHER_PANTS: u32 = 5961;
/// Barbaric Leggings.
pub(crate) const BARBARIC_LEGGINGS: u32 = 5963;
/// Linen Belt.
pub(crate) const LINEN_BELT: u32 = 7026;
/// Silk Headband.
pub(crate) const SILK_HEADBAND: u32 = 7050;
/// Green Silken Shoulders.
pub(crate) const GREEN_SILKEN_SHOULDERS: u32 = 7057;
/// Green Silk Armor.
pub(crate) const GREEN_SILK_ARMOR: u32 = 7065;
/// Elemental Earth.
pub(crate) const ELEMENTAL_EARTH: u32 = 7067;
/// Elemental Fire.
pub(crate) const ELEMENTAL_FIRE: u32 = 7068;
/// Iron Buckle.
pub(crate) const IRON_BUCKLE: u32 = 7071;
/// Core of Earth.
pub(crate) const CORE_OF_EARTH: u32 = 7075;
/// Essence of Earth.
pub(crate) const ESSENCE_OF_EARTH: u32 = 7076;
/// Heart of Fire.
pub(crate) const HEART_OF_FIRE: u32 = 7077;
/// Essence of Water.
pub(crate) const ESSENCE_OF_WATER: u32 = 7080;
/// Essence of Air.
pub(crate) const ESSENCE_OF_AIR: u32 = 7082;
/// Goblin Jumper Cables.
pub(crate) const GOBLIN_JUMPER_CABLES: u32 = 7148;
/// Goblin Rocket Boots.
pub(crate) const GOBLIN_ROCKET_BOOTS: u32 = 7189;
/// Fused Wiring.
pub(crate) const FUSED_WIRING: u32 = 7191;
/// Handstitched Leather Bracers.
pub(crate) const HANDSTITCHED_LEATHER_BRACERS: u32 = 7277;
/// Aquamarine.
pub(crate) const AQUAMARINE: u32 = 7909;
/// Star Ruby.
pub(crate) const STAR_RUBY: u32 = 7910;
/// Barbaric Iron Breastplate.
pub(crate) const BARBARIC_IRON_BREASTPLATE: u32 = 7914;
/// Heavy Mithril Breastplate.
pub(crate) const HEAVY_MITHRIL_BREASTPLATE: u32 = 7930;
/// Heavy Mithril Boots.
pub(crate) const HEAVY_MITHRIL_BOOTS: u32 = 7933;
/// Wicked Mithril Blade.
pub(crate) const WICKED_MITHRIL_BLADE: u32 = 7943;
/// Solid Sharpening Stone.
pub(crate) const SOLID_SHARPENING_STONE: u32 = 7964;
/// Solid Weightstone.
pub(crate) const SOLID_WEIGHTSTONE: u32 = 7965;
/// Solid Grinding Stone.
pub(crate) const SOLID_GRINDING_STONE: u32 = 7966;
/// Ichor of Undeath.
pub(crate) const ICHOR_OF_UNDEATH: u32 = 7972;
/// Wicked Claw.
pub(crate) const WICKED_CLAW: u32 = 8146;
/// Rugged Hide.
pub(crate) const RUGGED_HIDE: u32 = 8171;
/// Thick Armor Kit.
pub(crate) const THICK_ARMOR_KIT: u32 = 8173;
/// Nightscape Tunic.
pub(crate) const NIGHTSCAPE_TUNIC: u32 = 8175;
/// Nightscape Headband.
pub(crate) const NIGHTSCAPE_HEADBAND: u32 = 8176;
/// Nightscape Pants.
pub(crate) const NIGHTSCAPE_PANTS: u32 = 8193;
/// Nightscape Boots.
pub(crate) const NIGHTSCAPE_BOOTS: u32 = 8197;
/// Heavy Silken Thread.
pub(crate) const HEAVY_SILKEN_THREAD: u32 = 8343;
/// Inlaid Mithril Cylinder.
pub(crate) const INLAID_MITHRIL_CYLINDER: u32 = 9060;
/// Goblin Rocket Fuel.
pub(crate) const GOBLIN_ROCKET_FUEL: u32 = 9061;
/// Ghost Dye.
pub(crate) const GHOST_DYE: u32 = 9210;
/// Black Mageweave Vest.
pub(crate) const BLACK_MAGEWEAVE_VEST: u32 = 9998;
/// Black Mageweave Leggings.
pub(crate) const BLACK_MAGEWEAVE_LEGGINGS: u32 = 9999;
/// Black Mageweave Gloves.
pub(crate) const BLACK_MAGEWEAVE_GLOVES: u32 = 10003;
/// Black Mageweave Boots.
pub(crate) const BLACK_MAGEWEAVE_BOOTS: u32 = 10026;
/// Cindercloth Robe.
pub(crate) const CINDERCLOTH_ROBE: u32 = 10042;
/// Simple Linen Pants.
pub(crate) const SIMPLE_LINEN_PANTS: u32 = 10045;
/// Mageweave Bag.
pub(crate) const MAGEWEAVE_BAG: u32 = 10050;
/// Shadow Silk.
pub(crate) const SHADOW_SILK: u32 = 10285;
/// Solid Blasting Powder.
pub(crate) const SOLID_BLASTING_POWDER: u32 = 10505;
/// Gold Power Core.
pub(crate) const GOLD_POWER_CORE: u32 = 10558;
/// Mithril Tube.
pub(crate) const MITHRIL_TUBE: u32 = 10559;
/// Unstable Trigger.
pub(crate) const UNSTABLE_TRIGGER: u32 = 10560;
/// Mithril Casing.
pub(crate) const MITHRIL_CASING: u32 = 10561;
/// Mithril Mechanical Dragonling.
pub(crate) const MITHRIL_MECHANICAL_DRAGONLING: u32 = 10576;
/// Goblin Mortar.
pub(crate) const GOBLIN_MORTAR: u32 = 10577;
/// Schematic: Mithril Mechanical Dragonling.
pub(crate) const SCHEMATIC_MITHRIL_MECHANICAL_DRAGONLING: u32 = 10601;
/// Gnomish Death Ray.
pub(crate) const GNOMISH_DEATH_RAY: u32 = 10645;
/// Goblin Sapper Charge.
pub(crate) const GOBLIN_SAPPER_CHARGE: u32 = 10646;
/// Gnomish Net-o-Matic Projector.
pub(crate) const GNOMISH_NET_O_MATIC_PROJECTOR: u32 = 10720;
/// Gnomish Battle Chicken.
pub(crate) const GNOMISH_BATTLE_CHICKEN: u32 = 10725;
/// Goblin Dragon Gun.
pub(crate) const GOBLIN_DRAGON_GUN: u32 = 10727;
/// Dark Iron Ore.
pub(crate) const DARK_IRON_ORE: u32 = 11370;
/// Dark Iron Bar.
pub(crate) const DARK_IRON_BAR: u32 = 11371;
/// Blood of the Mountain.
pub(crate) const BLOOD_OF_THE_MOUNTAIN: u32 = 11382;
/// Dark Iron Plate.
pub(crate) const DARK_IRON_PLATE: u32 = 11604;
/// Plans: Dark Iron Plate.
pub(crate) const PLANS_DARK_IRON_PLATE: u32 = 11614;
/// Arcanite Bar.
pub(crate) const ARCANITE_BAR: u32 = 12360;
/// Blue Sapphire.
pub(crate) const BLUE_SAPPHIRE: u32 = 12361;
/// Arcane Crystal.
pub(crate) const ARCANE_CRYSTAL: u32 = 12363;
/// Dense Sharpening Stone.
pub(crate) const DENSE_SHARPENING_STONE: u32 = 12404;
/// Imperial Plate Chest.
pub(crate) const IMPERIAL_PLATE_CHEST: u32 = 12422;
/// Imperial Plate Belt.
pub(crate) const IMPERIAL_PLATE_BELT: u32 = 12424;
/// Imperial Plate Bracers.
pub(crate) const IMPERIAL_PLATE_BRACERS: u32 = 12425;
/// Imperial Plate Boots.
pub(crate) const IMPERIAL_PLATE_BOOTS: u32 = 12426;
/// Imperial Plate Helm.
pub(crate) const IMPERIAL_PLATE_HELM: u32 = 12427;
/// Imperial Plate Shoulders.
pub(crate) const IMPERIAL_PLATE_SHOULDERS: u32 = 12428;
/// Imperial Plate Leggings.
pub(crate) const IMPERIAL_PLATE_LEGGINGS: u32 = 12429;
/// Lionheart Helm.
pub(crate) const LIONHEART_HELM: u32 = 12640;
/// Dense Weightstone.
pub(crate) const DENSE_WEIGHTSTONE: u32 = 12643;
/// Dense Grinding Stone.
pub(crate) const DENSE_GRINDING_STONE: u32 = 12644;
/// Enchanted Thorium Bar.
pub(crate) const ENCHANTED_THORIUM_BAR: u32 = 12655;
/// Plans: Imperial Plate Shoulders.
pub(crate) const PLANS_IMPERIAL_PLATE_SHOULDERS: u32 = 12687;
/// Plans: Imperial Plate Belt.
pub(crate) const PLANS_IMPERIAL_PLATE_BELT: u32 = 12688;
/// Plans: Imperial Plate Bracers.
pub(crate) const PLANS_IMPERIAL_PLATE_BRACERS: u32 = 12690;
/// Plans: Imperial Plate Boots.
pub(crate) const PLANS_IMPERIAL_PLATE_BOOTS: u32 = 12700;
/// Plans: Imperial Plate Helm.
pub(crate) const PLANS_IMPERIAL_PLATE_HELM: u32 = 12701;
/// Plans: Imperial Plate Chest.
pub(crate) const PLANS_IMPERIAL_PLATE_CHEST: u32 = 12705;
/// Plans: Imperial Plate Leggings.
pub(crate) const PLANS_IMPERIAL_PLATE_LEGGINGS: u32 = 12711;
/// Plans: Lionheart Helm.
pub(crate) const PLANS_LIONHEART_HELM: u32 = 12717;
/// Arcanite Reaper.
pub(crate) const ARCANITE_REAPER: u32 = 12784;
/// Azerothian Diamond.
pub(crate) const AZEROTHIAN_DIAMOND: u32 = 12800;
/// Essence of Undeath.
pub(crate) const ESSENCE_OF_UNDEATH: u32 = 12808;
/// Enchanted Leather.
pub(crate) const ENCHANTED_LEATHER: u32 = 12810;
/// Plans: Arcanite Reaper.
pub(crate) const PLANS_ARCANITE_REAPER: u32 = 12838;
/// Recipe: Transmute Arcanite.
pub(crate) const RECIPE_TRANSMUTE_ARCANITE: u32 = 12958;
/// Runecloth Belt.
pub(crate) const RUNECLOTH_BELT: u32 = 13856;
/// Runecloth Gloves.
pub(crate) const RUNECLOTH_GLOVES: u32 = 13863;
/// Golden Pearl.
pub(crate) const GOLDEN_PEARL: u32 = 13926;
/// Cindercloth Vest.
pub(crate) const CINDERCLOTH_VEST: u32 = 14042;
/// Runecloth Bag.
pub(crate) const RUNECLOTH_BAG: u32 = 14046;
/// Bolt of Runecloth.
pub(crate) const BOLT_OF_RUNECLOTH: u32 = 14048;
/// Mooncloth Leggings.
pub(crate) const MOONCLOTH_LEGGINGS: u32 = 14137;
/// Mooncloth Vest.
pub(crate) const MOONCLOTH_VEST: u32 = 14138;
/// Robe of the Archmage.
pub(crate) const ROBE_OF_THE_ARCHMAGE: u32 = 14152;
/// Truefaith Vestments.
pub(crate) const TRUEFAITH_VESTMENTS: u32 = 14154;
/// Mooncloth Bag.
pub(crate) const MOONCLOTH_BAG: u32 = 14155;
/// Bottomless Bag.
pub(crate) const BOTTOMLESS_BAG: u32 = 14156;
/// Ironweb Spider Silk.
pub(crate) const IRONWEB_SPIDER_SILK: u32 = 14227;
/// Felcloth.
pub(crate) const FELCLOTH: u32 = 14256;
/// Rune Thread.
pub(crate) const RUNE_THREAD: u32 = 14341;
/// Mooncloth.
pub(crate) const MOONCLOTH: u32 = 14342;
/// Pattern: Mooncloth Leggings.
pub(crate) const PATTERN_MOONCLOTH_LEGGINGS: u32 = 14497;
/// Pattern: Mooncloth Bag.
pub(crate) const PATTERN_MOONCLOTH_BAG: u32 = 14499;
/// Pattern: Mooncloth Vest.
pub(crate) const PATTERN_MOONCLOTH_VEST: u32 = 14501;
/// Pattern: Bottomless Bag.
pub(crate) const PATTERN_BOTTOMLESS_BAG: u32 = 14510;
/// Pattern: Robe of the Archmage.
pub(crate) const PATTERN_ROBE_OF_THE_ARCHMAGE: u32 = 14513;
/// Pattern: Truefaith Vestments.
pub(crate) const PATTERN_TRUEFAITH_VESTMENTS: u32 = 14514;
/// Volcanic Breastplate.
pub(crate) const VOLCANIC_BREASTPLATE: u32 = 15053;
/// Volcanic Leggings.
pub(crate) const VOLCANIC_LEGGINGS: u32 = 15054;
/// Volcanic Shoulders.
pub(crate) const VOLCANIC_SHOULDERS: u32 = 15055;
/// Devilsaur Leggings.
pub(crate) const DEVILSAUR_LEGGINGS: u32 = 15062;
/// Devilsaur Gauntlets.
pub(crate) const DEVILSAUR_GAUNTLETS: u32 = 15063;
/// Cured Rugged Hide.
pub(crate) const CURED_RUGGED_HIDE: u32 = 15407;
/// Refined Deeprock Salt.
pub(crate) const REFINED_DEEPROCK_SALT: u32 = 15409;
/// Devilsaur Leather.
pub(crate) const DEVILSAUR_LEATHER: u32 = 15417;
/// Rugged Armor Kit.
pub(crate) const RUGGED_ARMOR_KIT: u32 = 15564;
/// Pattern: Volcanic Leggings.
pub(crate) const PATTERN_VOLCANIC_LEGGINGS: u32 = 15732;
/// Pattern: Volcanic Breastplate.
pub(crate) const PATTERN_VOLCANIC_BREASTPLATE: u32 = 15749;
/// Pattern: Devilsaur Gauntlets.
pub(crate) const PATTERN_DEVILSAUR_GAUNTLETS: u32 = 15758;
/// Pattern: Devilsaur Leggings.
pub(crate) const PATTERN_DEVILSAUR_LEGGINGS: u32 = 15772;
/// Pattern: Volcanic Shoulders.
pub(crate) const PATTERN_VOLCANIC_SHOULDERS: u32 = 15775;
/// Dense Blasting Powder.
pub(crate) const DENSE_BLASTING_POWDER: u32 = 15992;
/// Thorium Grenade.
pub(crate) const THORIUM_GRENADE: u32 = 15993;
/// Thorium Widget.
pub(crate) const THORIUM_WIDGET: u32 = 15994;
/// Thorium Tube.
pub(crate) const THORIUM_TUBE: u32 = 16000;
/// Delicate Arcanite Converter.
pub(crate) const DELICATE_ARCANITE_CONVERTER: u32 = 16006;
/// Arcanite Dragonling.
pub(crate) const ARCANITE_DRAGONLING: u32 = 16022;
/// Masterwork Target Dummy.
pub(crate) const MASTERWORK_TARGET_DUMMY: u32 = 16023;
/// Schematic: Thorium Grenade.
pub(crate) const SCHEMATIC_THORIUM_GRENADE: u32 = 16043;
/// Schematic: Masterwork Target Dummy.
pub(crate) const SCHEMATIC_MASTERWORK_TARGET_DUMMY: u32 = 16050;
/// Schematic: Arcanite Dragonling.
pub(crate) const SCHEMATIC_ARCANITE_DRAGONLING: u32 = 16054;
/// Flarecore Gloves.
pub(crate) const FLARECORE_GLOVES: u32 = 16979;
/// Flarecore Mantle.
pub(crate) const FLARECORE_MANTLE: u32 = 16980;
/// Corehound Boots.
pub(crate) const COREHOUND_BOOTS: u32 = 16982;
/// Molten Helm.
pub(crate) const MOLTEN_HELM: u32 = 16983;
/// Fiery Chain Girdle.
pub(crate) const FIERY_CHAIN_GIRDLE: u32 = 16989;
/// Fiery Core.
pub(crate) const FIERY_CORE: u32 = 17010;
/// Lava Core.
pub(crate) const LAVA_CORE: u32 = 17011;
/// Core Leather.
pub(crate) const CORE_LEATHER: u32 = 17012;
/// Pattern: Flarecore Mantle.
pub(crate) const PATTERN_FLARECORE_MANTLE: u32 = 17017;
/// Pattern: Flarecore Gloves.
pub(crate) const PATTERN_FLARECORE_GLOVES: u32 = 17018;
/// Plans: Fiery Chain Girdle.
pub(crate) const PLANS_FIERY_CHAIN_GIRDLE: u32 = 17049;
/// Pattern: Molten Helm.
pub(crate) const PATTERN_MOLTEN_HELM: u32 = 17052;
/// Pattern: Corehound Boots.
pub(crate) const PATTERN_COREHOUND_BOOTS: u32 = 17053;
/// Sulfuron Hammer.
pub(crate) const SULFURON_HAMMER: u32 = 17193;
/// Sulfuron Ingot.
pub(crate) const SULFURON_INGOT: u32 = 17203;
/// Core Armor Kit.
pub(crate) const CORE_ARMOR_KIT: u32 = 18251;
/// Pattern: Core Armor Kit.
pub(crate) const PATTERN_CORE_ARMOR_KIT: u32 = 18252;
/// Elemental Sharpening Stone.
pub(crate) const ELEMENTAL_SHARPENING_STONE: u32 = 18262;
/// Plans: Elemental Sharpening Stone.
pub(crate) const PLANS_ELEMENTAL_SHARPENING_STONE: u32 = 18264;
/// Plans: Sulfuron Hammer.
pub(crate) const PLANS_SULFURON_HAMMER: u32 = 18592;
/// Dense Dynamite.
pub(crate) const DENSE_DYNAMITE: u32 = 18641;
/// Bloodvine.
pub(crate) const BLOODVINE: u32 = 19726;
/// Core Felcloth Bag.
pub(crate) const CORE_FELCLOTH_BAG: u32 = 21342;
/// Pattern: Core Felcloth Bag.
pub(crate) const PATTERN_CORE_FELCLOTH_BAG: u32 = 21371;

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

/// A recipe (`ItemClass` 9; subclass 1 Leatherworking, 2 Tailoring, 3 Engineering, 4
/// Blacksmithing, 5 Cooking, 6 Alchemy, 7 First Aid, 8 Enchanting): using it teaches its recipe
/// ([`super::effects`]).
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

/// Trade goods of a subclass: 1 Parts, 2 Explosives, 3 Devices (Engineering's).
const fn trade(
    entry: u32,
    name: &'static str,
    quality: u32,
    icon: &'static str,
    stack: u32,
    sell_price: u32,
    subclass: u32,
) -> ItemDef {
    ItemDef {
        subclass,
        ..goods(entry, name, quality, icon, stack, sell_price)
    }
}

/// A bag (`ItemClass` 1, inventory type 18) of `slots` slots.
const fn bag(
    entry: u32,
    name: &'static str,
    quality: u32,
    icon: &'static str,
    slots: u32,
    sell_price: u32,
) -> ItemDef {
    ItemDef {
        class: 1,
        inventory_type: 18,
        bag_slots: slots,
        ..goods(entry, name, quality, icon, 1, sell_price)
    }
}

/// An engineered device worn in 1.12 (a trinket, inventory type 12; the rocket boots, 8): here it
/// is used from the bags or the satchel ([`super::gadgets`]).
const fn device(
    entry: u32,
    name: &'static str,
    quality: u32,
    icon: &'static str,
    stack: u32,
    sell_price: u32,
    inventory_type: u32,
) -> ItemDef {
    ItemDef {
        class: 4,
        subclass: if inventory_type == 8 { 1 } else { 0 },
        inventory_type,
        bonding: 2,
        ..goods(entry, name, quality, icon, stack, sell_price)
    }
}

/// A crafted piece of gear: `(class, subclass, inventory type)` (4 armour, 2 weapon), bind on
/// equip. What it grants is its gear piece's ([`crate::survivors::gear::crafted`]).
const fn gear_item(
    entry: u32,
    name: &'static str,
    quality: u32,
    kind: (u32, u32, u32),
    icon: &'static str,
    sell_price: u32,
) -> ItemDef {
    ItemDef {
        class: kind.0,
        subclass: kind.1,
        inventory_type: kind.2,
        bonding: 2,
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
    // P3.
    goods(FLASK_OF_OIL, "Flask of Oil", 1, "INV_Drink_01", 20, 75),
    goods(
        MOSS_AGATE,
        "Moss Agate",
        2,
        "INV_Misc_Gem_Emerald_03",
        20,
        100,
    ),
    goods(
        SHADOWGEM,
        "Shadowgem",
        2,
        "INV_Misc_Gem_Amethyst_01",
        20,
        100,
    ),
    goods(JADE, "Jade", 2, "INV_Misc_Gem_Emerald_02", 20, 200),
    goods(
        LESSER_MOONSTONE,
        "Lesser Moonstone",
        2,
        "INV_Misc_Gem_Pearl_03",
        20,
        150,
    ),
    consumable(
        LIGHT_ARMOR_KIT,
        "Light Armor Kit",
        1,
        "INV_Misc_ArmorKit_02",
        10,
        10,
    ),
    consumable(
        MEDIUM_ARMOR_KIT,
        "Medium Armor Kit",
        1,
        "INV_Misc_ArmorKit_04",
        10,
        25,
    ),
    goods(
        COARSE_THREAD,
        "Coarse Thread",
        1,
        "INV_Fabric_Linen_03",
        20,
        2,
    ),
    goods(FINE_THREAD, "Fine Thread", 1, "INV_Fabric_Wool_03", 20, 25),
    goods(GREEN_DYE, "Green Dye", 1, "INV_Drink_11", 20, 25),
    goods(SILVER_ORE, "Silver Ore", 1, "INV_Stone_16", 10, 75),
    goods(GOLD_ORE, "Gold Ore", 2, "INV_Ore_Copper_01", 10, 600),
    goods(SILVER_BAR, "Silver Bar", 1, "INV_Ingot_01", 20, 100),
    consumable(
        ROUGH_SHARPENING_STONE,
        "Rough Sharpening Stone",
        1,
        "INV_Stone_SharpeningStone_01",
        20,
        2,
    ),
    consumable(
        COARSE_SHARPENING_STONE,
        "Coarse Sharpening Stone",
        1,
        "INV_Stone_SharpeningStone_02",
        20,
        7,
    ),
    consumable(
        HEAVY_SHARPENING_STONE,
        "Heavy Sharpening Stone",
        1,
        "INV_Stone_SharpeningStone_03",
        20,
        15,
    ),
    goods(WEAK_FLUX, "Weak Flux", 1, "INV_Misc_Dust_04", 20, 25),
    goods(
        BOLT_OF_LINEN_CLOTH,
        "Bolt of Linen Cloth",
        1,
        "INV_Fabric_Linen_02",
        10,
        40,
    ),
    goods(
        BOLT_OF_WOOLEN_CLOTH,
        "Bolt of Woolen Cloth",
        1,
        "INV_Fabric_Wool_02",
        10,
        100,
    ),
    consumable(
        ROUGH_WEIGHTSTONE,
        "Rough Weightstone",
        1,
        "INV_Stone_WeightStone_01",
        20,
        2,
    ),
    consumable(
        COARSE_WEIGHTSTONE,
        "Coarse Weightstone",
        1,
        "INV_Stone_WeightStone_02",
        20,
        7,
    ),
    consumable(
        HEAVY_WEIGHTSTONE,
        "Heavy Weightstone",
        1,
        "INV_Stone_WeightStone_03",
        20,
        15,
    ),
    goods(STRONG_FLUX, "Strong Flux", 1, "INV_Misc_Dust_04", 20, 50),
    goods(
        ROUGH_GRINDING_STONE,
        "Rough Grinding Stone",
        1,
        "INV_Stone_GrindingStone_01",
        20,
        2,
    ),
    goods(
        COARSE_GRINDING_STONE,
        "Coarse Grinding Stone",
        1,
        "INV_Stone_GrindingStone_02",
        20,
        7,
    ),
    goods(
        HEAVY_GRINDING_STONE,
        "Heavy Grinding Stone",
        1,
        "INV_Stone_GrindingStone_03",
        20,
        30,
    ),
    goods(GOLD_BAR, "Gold Bar", 2, "INV_Ingot_03", 20, 600),
    goods(CITRINE, "Citrine", 2, "INV_Misc_Gem_Opal_02", 20, 250),
    bag(WOOLEN_BAG, "Woolen Bag", 1, "INV_Misc_Bag_09", 8, 200),
    bag(
        SMALL_SILK_PACK,
        "Small Silk Pack",
        1,
        "INV_Misc_Bag_02",
        10,
        500,
    ),
    consumable(
        HEAVY_ARMOR_KIT,
        "Heavy Armor Kit",
        1,
        "INV_Misc_ArmorKit_05",
        10,
        50,
    ),
    goods(
        SILKEN_THREAD,
        "Silken Thread",
        1,
        "INV_Fabric_Silk_03",
        20,
        125,
    ),
    goods(
        BOLT_OF_SILK_CLOTH,
        "Bolt of Silk Cloth",
        1,
        "INV_Fabric_Silk_02",
        10,
        150,
    ),
    goods(
        THICK_SPIDERS_SILK,
        "Thick Spider's Silk",
        1,
        "INV_Fabric_Wool_03",
        20,
        150,
    ),
    goods(
        BOLT_OF_MAGEWEAVE,
        "Bolt of Mageweave",
        1,
        "INV_Fabric_Mageweave_02",
        10,
        400,
    ),
    goods(GRAY_DYE, "Gray Dye", 1, "INV_Drink_10", 20, 87),
    trade(
        ROUGH_BLASTING_POWDER,
        "Rough Blasting Powder",
        1,
        "INV_Misc_Ammo_Gunpowder_02",
        20,
        4,
        2,
    ),
    trade(
        HANDFUL_OF_COPPER_BOLTS,
        "Handful of Copper Bolts",
        1,
        "INV_Misc_Gear_06",
        20,
        12,
        1,
    ),
    trade(
        ROUGH_COPPER_BOMB,
        "Rough Copper Bomb",
        1,
        "INV_Misc_Bomb_09",
        20,
        15,
        2,
    ),
    trade(
        COPPER_MODULATOR,
        "Copper Modulator",
        1,
        "INV_Gizmo_03",
        20,
        50,
        1,
    ),
    trade(
        COARSE_BLASTING_POWDER,
        "Coarse Blasting Powder",
        1,
        "INV_Misc_Ammo_Gunpowder_01",
        20,
        12,
        2,
    ),
    trade(
        TARGET_DUMMY,
        "Target Dummy",
        1,
        "INV_Misc_EngGizmos_07",
        5,
        100,
        3,
    ),
    trade(
        WHIRRING_BRONZE_GIZMO,
        "Whirring Bronze Gizmo",
        1,
        "INV_Gizmo_08",
        20,
        115,
        1,
    ),
    trade(
        HEAVY_BLASTING_POWDER,
        "Heavy Blasting Powder",
        1,
        "INV_Misc_Powder_Purple",
        20,
        25,
        2,
    ),
    trade(
        BRONZE_FRAMEWORK,
        "Bronze Framework",
        1,
        "INV_Gizmo_BronzeFramework_01",
        20,
        145,
        1,
    ),
    trade(
        EXPLOSIVE_SHEEP,
        "Explosive Sheep",
        1,
        "Spell_Nature_Polymorph",
        5,
        200,
        3,
    ),
    trade(IRON_STRUT, "Iron Strut", 1, "INV_Gizmo_Pipe_02", 20, 100, 1),
    trade(
        GYROCHRONATOM,
        "Gyrochronatom",
        1,
        "INV_Gizmo_07",
        20,
        300,
        1,
    ),
    trade(
        IRON_GRENADE,
        "Iron Grenade",
        1,
        "INV_Misc_Bomb_08",
        20,
        50,
        2,
    ),
    trade(
        BIG_IRON_BOMB,
        "Big Iron Bomb",
        1,
        "INV_Misc_Bomb_01",
        20,
        75,
        2,
    ),
    device(
        MECHANICAL_DRAGONLING,
        "Mechanical Dragonling",
        2,
        "INV_Misc_Head_Dragon_01",
        1,
        3000,
        12,
    ),
    trade(
        SILVER_CONTACT,
        "Silver Contact",
        1,
        "INV_Misc_Gear_02",
        20,
        22,
        1,
    ),
    goods(
        ELEMENTAL_EARTH,
        "Elemental Earth",
        1,
        "Spell_Nature_Earthquake",
        10,
        400,
    ),
    goods(
        ELEMENTAL_FIRE,
        "Elemental Fire",
        1,
        "Spell_Fire_Fire",
        10,
        400,
    ),
    goods(IRON_BUCKLE, "Iron Buckle", 1, "INV_Misc_Gear_01", 20, 30),
    goods(
        CORE_OF_EARTH,
        "Core of Earth",
        1,
        "Spell_Nature_StoneSkinTotem",
        10,
        600,
    ),
    goods(
        ESSENCE_OF_EARTH,
        "Essence of Earth",
        2,
        "Spell_Nature_StrengthOfEarthTotem02",
        10,
        800,
    ),
    goods(
        HEART_OF_FIRE,
        "Heart of Fire",
        1,
        "Spell_Fire_LavaSpawn",
        10,
        600,
    ),
    goods(
        ESSENCE_OF_WATER,
        "Essence of Water",
        2,
        "Spell_Frost_SummonWaterElemental",
        10,
        800,
    ),
    goods(
        ESSENCE_OF_AIR,
        "Essence of Air",
        2,
        "Spell_Nature_Cyclone",
        10,
        800,
    ),
    trade(
        GOBLIN_JUMPER_CABLES,
        "Goblin Jumper Cables",
        1,
        "INV_Misc_EngGizmos_10",
        1,
        1500,
        3,
    ),
    device(
        GOBLIN_ROCKET_BOOTS,
        "Goblin Rocket Boots",
        2,
        "INV_Gizmo_RocketBoot_01",
        1,
        1500,
        8,
    ),
    trade(
        FUSED_WIRING,
        "Fused Wiring",
        1,
        "INV_Misc_Gear_07",
        10,
        750,
        1,
    ),
    goods(
        AQUAMARINE,
        "Aquamarine",
        2,
        "INV_Misc_Gem_Crystal_02",
        20,
        400,
    ),
    goods(STAR_RUBY, "Star Ruby", 2, "INV_Misc_Gem_Ruby_02", 20, 500),
    consumable(
        SOLID_SHARPENING_STONE,
        "Solid Sharpening Stone",
        1,
        "INV_Stone_SharpeningStone_04",
        20,
        25,
    ),
    consumable(
        SOLID_WEIGHTSTONE,
        "Solid Weightstone",
        1,
        "INV_Stone_WeightStone_04",
        20,
        25,
    ),
    goods(
        SOLID_GRINDING_STONE,
        "Solid Grinding Stone",
        1,
        "INV_Stone_GrindingStone_04",
        20,
        50,
    ),
    goods(
        ICHOR_OF_UNDEATH,
        "Ichor of Undeath",
        1,
        "INV_Potion_20",
        10,
        300,
    ),
    goods(
        WICKED_CLAW,
        "Wicked Claw",
        1,
        "INV_Misc_MonsterClaw_02",
        20,
        300,
    ),
    goods(
        RUGGED_HIDE,
        "Rugged Hide",
        1,
        "INV_Misc_Pelt_Bear_03",
        10,
        500,
    ),
    consumable(
        THICK_ARMOR_KIT,
        "Thick Armor Kit",
        1,
        "INV_Misc_ArmorKit_08",
        10,
        100,
    ),
    goods(
        HEAVY_SILKEN_THREAD,
        "Heavy Silken Thread",
        1,
        "INV_Fabric_Mageweave_03",
        20,
        500,
    ),
    trade(
        INLAID_MITHRIL_CYLINDER,
        "Inlaid Mithril Cylinder",
        1,
        "INV_Gizmo_MithrilCasing_02",
        20,
        1000,
        1,
    ),
    goods(
        GOBLIN_ROCKET_FUEL,
        "Goblin Rocket Fuel",
        1,
        "INV_Drink_13",
        20,
        500,
    ),
    goods(GHOST_DYE, "Ghost Dye", 1, "INV_Potion_75", 20, 400),
    bag(
        MAGEWEAVE_BAG,
        "Mageweave Bag",
        1,
        "INV_Misc_Bag_10_Red",
        12,
        1000,
    ),
    goods(
        SHADOW_SILK,
        "Shadow Silk",
        1,
        "INV_Fabric_Mageweave_03",
        20,
        250,
    ),
    trade(
        SOLID_BLASTING_POWDER,
        "Solid Blasting Powder",
        1,
        "INV_Misc_Powder_Black",
        20,
        50,
        2,
    ),
    trade(
        GOLD_POWER_CORE,
        "Gold Power Core",
        1,
        "INV_Misc_Orb_01",
        20,
        300,
        1,
    ),
    trade(
        MITHRIL_TUBE,
        "Mithril Tube",
        1,
        "INV_Gizmo_Pipe_04",
        20,
        300,
        1,
    ),
    trade(
        UNSTABLE_TRIGGER,
        "Unstable Trigger",
        1,
        "INV_Gizmo_05",
        20,
        300,
        1,
    ),
    trade(
        MITHRIL_CASING,
        "Mithril Casing",
        1,
        "INV_Gizmo_MithrilCasing_01",
        20,
        300,
        1,
    ),
    device(
        MITHRIL_MECHANICAL_DRAGONLING,
        "Mithril Mechanical Dragonling",
        3,
        "INV_Misc_Head_Dragon_01",
        1,
        6000,
        12,
    ),
    device(
        GOBLIN_MORTAR,
        "Goblin Mortar",
        2,
        "INV_Misc_EngGizmos_16",
        1,
        1500,
        12,
    ),
    recipe_item(
        SCHEMATIC_MITHRIL_MECHANICAL_DRAGONLING,
        "Schematic: Mithril Mechanical Dragonling",
        2,
        3,
        "INV_Scroll_03",
        2000,
    ),
    device(
        GNOMISH_DEATH_RAY,
        "Gnomish Death Ray",
        3,
        "INV_Misc_EngGizmos_13",
        1,
        2500,
        12,
    ),
    trade(
        GOBLIN_SAPPER_CHARGE,
        "Goblin Sapper Charge",
        1,
        "INV_Misc_Bomb_07",
        10,
        375,
        2,
    ),
    device(
        GNOMISH_NET_O_MATIC_PROJECTOR,
        "Gnomish Net-o-Matic Projector",
        2,
        "INV_Gizmo_06",
        1,
        1500,
        12,
    ),
    device(
        GNOMISH_BATTLE_CHICKEN,
        "Gnomish Battle Chicken",
        2,
        "Spell_Magic_PolymorphChicken",
        1,
        1500,
        12,
    ),
    device(
        GOBLIN_DRAGON_GUN,
        "Goblin Dragon Gun",
        3,
        "INV_Misc_EngGizmos_19",
        1,
        2500,
        12,
    ),
    goods(
        DARK_IRON_ORE,
        "Dark Iron Ore",
        1,
        "INV_Ore_Mithril_01",
        10,
        1000,
    ),
    goods(
        DARK_IRON_BAR,
        "Dark Iron Bar",
        1,
        "INV_Ingot_Mithril",
        20,
        1200,
    ),
    goods(
        BLOOD_OF_THE_MOUNTAIN,
        "Blood of the Mountain",
        2,
        "INV_Misc_Gem_Bloodstone_02",
        10,
        2000,
    ),
    recipe_item(
        PLANS_DARK_IRON_PLATE,
        "Plans: Dark Iron Plate",
        3,
        4,
        "INV_Scroll_06",
        5000,
    ),
    goods(ARCANITE_BAR, "Arcanite Bar", 2, "INV_Ingot_04", 20, 10000),
    goods(
        BLUE_SAPPHIRE,
        "Blue Sapphire",
        2,
        "INV_Misc_Gem_Sapphire_02",
        20,
        1000,
    ),
    goods(
        ARCANE_CRYSTAL,
        "Arcane Crystal",
        2,
        "INV_Misc_Gem_Crystal_01",
        20,
        1500,
    ),
    consumable(
        DENSE_SHARPENING_STONE,
        "Dense Sharpening Stone",
        1,
        "INV_Stone_SharpeningStone_05",
        20,
        100,
    ),
    consumable(
        DENSE_WEIGHTSTONE,
        "Dense Weightstone",
        1,
        "INV_Stone_WeightStone_05",
        20,
        100,
    ),
    goods(
        DENSE_GRINDING_STONE,
        "Dense Grinding Stone",
        1,
        "INV_Stone_GrindingStone_05",
        20,
        200,
    ),
    goods(
        ENCHANTED_THORIUM_BAR,
        "Enchanted Thorium Bar",
        1,
        "INV_Ingot_Eternium",
        20,
        1000,
    ),
    recipe_item(
        PLANS_IMPERIAL_PLATE_SHOULDERS,
        "Plans: Imperial Plate Shoulders",
        2,
        4,
        "INV_Scroll_03",
        1000,
    ),
    recipe_item(
        PLANS_IMPERIAL_PLATE_BELT,
        "Plans: Imperial Plate Belt",
        2,
        4,
        "INV_Scroll_03",
        1000,
    ),
    recipe_item(
        PLANS_IMPERIAL_PLATE_BRACERS,
        "Plans: Imperial Plate Bracers",
        2,
        4,
        "INV_Scroll_03",
        1000,
    ),
    recipe_item(
        PLANS_IMPERIAL_PLATE_BOOTS,
        "Plans: Imperial Plate Boots",
        2,
        4,
        "INV_Scroll_03",
        1000,
    ),
    recipe_item(
        PLANS_IMPERIAL_PLATE_HELM,
        "Plans: Imperial Plate Helm",
        2,
        4,
        "INV_Scroll_03",
        1000,
    ),
    recipe_item(
        PLANS_IMPERIAL_PLATE_CHEST,
        "Plans: Imperial Plate Chest",
        2,
        4,
        "INV_Scroll_03",
        1000,
    ),
    recipe_item(
        PLANS_IMPERIAL_PLATE_LEGGINGS,
        "Plans: Imperial Plate Leggings",
        2,
        4,
        "INV_Scroll_03",
        1000,
    ),
    recipe_item(
        PLANS_LIONHEART_HELM,
        "Plans: Lionheart Helm",
        3,
        4,
        "INV_Scroll_06",
        5000,
    ),
    goods(
        AZEROTHIAN_DIAMOND,
        "Azerothian Diamond",
        2,
        "INV_Misc_Gem_Diamond_01",
        20,
        1000,
    ),
    goods(
        ESSENCE_OF_UNDEATH,
        "Essence of Undeath",
        2,
        "Spell_Shadow_ShadeTrueSight",
        10,
        800,
    ),
    goods(
        ENCHANTED_LEATHER,
        "Enchanted Leather",
        1,
        "INV_Misc_Rune_05",
        20,
        1000,
    ),
    recipe_item(
        PLANS_ARCANITE_REAPER,
        "Plans: Arcanite Reaper",
        3,
        4,
        "INV_Scroll_06",
        5000,
    ),
    recipe_item(
        RECIPE_TRANSMUTE_ARCANITE,
        "Recipe: Transmute Arcanite",
        2,
        6,
        "INV_Scroll_06",
        3000,
    ),
    goods(
        GOLDEN_PEARL,
        "Golden Pearl",
        2,
        "INV_Misc_Gem_Pearl_04",
        20,
        1000,
    ),
    bag(
        RUNECLOTH_BAG,
        "Runecloth Bag",
        1,
        "INV_Misc_Bag_14",
        14,
        2500,
    ),
    goods(
        BOLT_OF_RUNECLOTH,
        "Bolt of Runecloth",
        1,
        "INV_Fabric_PurpleFire_02",
        10,
        800,
    ),
    bag(
        MOONCLOTH_BAG,
        "Mooncloth Bag",
        2,
        "INV_Misc_Bag_13",
        16,
        5000,
    ),
    bag(
        BOTTOMLESS_BAG,
        "Bottomless Bag",
        3,
        "INV_Misc_Bag_15",
        18,
        10000,
    ),
    goods(
        IRONWEB_SPIDER_SILK,
        "Ironweb Spider Silk",
        1,
        "INV_Fabric_Silk_03",
        20,
        400,
    ),
    goods(FELCLOTH, "Felcloth", 1, "INV_Fabric_FelRag", 20, 2000),
    goods(
        RUNE_THREAD,
        "Rune Thread",
        1,
        "INV_Fabric_MoonRag_02",
        20,
        1250,
    ),
    goods(MOONCLOTH, "Mooncloth", 2, "INV_Fabric_MoonRag_01", 10, 5000),
    recipe_item(
        PATTERN_MOONCLOTH_LEGGINGS,
        "Pattern: Mooncloth Leggings",
        2,
        2,
        "INV_Scroll_03",
        2000,
    ),
    recipe_item(
        PATTERN_MOONCLOTH_BAG,
        "Pattern: Mooncloth Bag",
        2,
        2,
        "INV_Scroll_03",
        2000,
    ),
    recipe_item(
        PATTERN_MOONCLOTH_VEST,
        "Pattern: Mooncloth Vest",
        2,
        2,
        "INV_Scroll_03",
        2000,
    ),
    recipe_item(
        PATTERN_BOTTOMLESS_BAG,
        "Pattern: Bottomless Bag",
        3,
        2,
        "INV_Scroll_06",
        5000,
    ),
    recipe_item(
        PATTERN_ROBE_OF_THE_ARCHMAGE,
        "Pattern: Robe of the Archmage",
        3,
        2,
        "INV_Scroll_06",
        5000,
    ),
    recipe_item(
        PATTERN_TRUEFAITH_VESTMENTS,
        "Pattern: Truefaith Vestments",
        3,
        2,
        "INV_Scroll_06",
        5000,
    ),
    goods(
        CURED_RUGGED_HIDE,
        "Cured Rugged Hide",
        1,
        "INV_Misc_Pelt_Bear_02",
        10,
        1000,
    ),
    goods(
        REFINED_DEEPROCK_SALT,
        "Refined Deeprock Salt",
        1,
        "INV_Misc_Dust_03",
        20,
        500,
    ),
    goods(
        DEVILSAUR_LEATHER,
        "Devilsaur Leather",
        2,
        "INV_Misc_LeatherScrap_01",
        10,
        600,
    ),
    consumable(
        RUGGED_ARMOR_KIT,
        "Rugged Armor Kit",
        1,
        "INV_Misc_ArmorKit_09",
        10,
        250,
    ),
    recipe_item(
        PATTERN_VOLCANIC_LEGGINGS,
        "Pattern: Volcanic Leggings",
        2,
        1,
        "INV_Scroll_03",
        2000,
    ),
    recipe_item(
        PATTERN_VOLCANIC_BREASTPLATE,
        "Pattern: Volcanic Breastplate",
        2,
        1,
        "INV_Scroll_03",
        2000,
    ),
    recipe_item(
        PATTERN_DEVILSAUR_GAUNTLETS,
        "Pattern: Devilsaur Gauntlets",
        3,
        1,
        "INV_Scroll_03",
        3000,
    ),
    recipe_item(
        PATTERN_DEVILSAUR_LEGGINGS,
        "Pattern: Devilsaur Leggings",
        3,
        1,
        "INV_Scroll_03",
        3000,
    ),
    recipe_item(
        PATTERN_VOLCANIC_SHOULDERS,
        "Pattern: Volcanic Shoulders",
        2,
        1,
        "INV_Scroll_03",
        2000,
    ),
    trade(
        DENSE_BLASTING_POWDER,
        "Dense Blasting Powder",
        1,
        "INV_Misc_Ammo_Gunpowder_02",
        20,
        250,
        2,
    ),
    trade(
        THORIUM_GRENADE,
        "Thorium Grenade",
        1,
        "INV_Misc_Bomb_06",
        20,
        250,
        2,
    ),
    trade(
        THORIUM_WIDGET,
        "Thorium Widget",
        1,
        "INV_Gizmo_09",
        20,
        600,
        1,
    ),
    trade(
        THORIUM_TUBE,
        "Thorium Tube",
        1,
        "INV_Gizmo_Pipe_01",
        20,
        1000,
        1,
    ),
    trade(
        DELICATE_ARCANITE_CONVERTER,
        "Delicate Arcanite Converter",
        2,
        "INV_Gizmo_04",
        20,
        1500,
        1,
    ),
    device(
        ARCANITE_DRAGONLING,
        "Arcanite Dragonling",
        4,
        "INV_Misc_Head_Dragon_Black",
        1,
        10000,
        12,
    ),
    trade(
        MASTERWORK_TARGET_DUMMY,
        "Masterwork Target Dummy",
        1,
        "INV_Misc_EngGizmos_08",
        5,
        2500,
        3,
    ),
    recipe_item(
        SCHEMATIC_THORIUM_GRENADE,
        "Schematic: Thorium Grenade",
        2,
        3,
        "INV_Scroll_03",
        3000,
    ),
    recipe_item(
        SCHEMATIC_MASTERWORK_TARGET_DUMMY,
        "Schematic: Masterwork Target Dummy",
        2,
        3,
        "INV_Scroll_03",
        3000,
    ),
    recipe_item(
        SCHEMATIC_ARCANITE_DRAGONLING,
        "Schematic: Arcanite Dragonling",
        3,
        3,
        "INV_Scroll_06",
        5000,
    ),
    goods(
        FIERY_CORE,
        "Fiery Core",
        2,
        "Spell_Fire_FlameShock",
        20,
        5000,
    ),
    goods(LAVA_CORE, "Lava Core", 2, "Spell_Fire_Immolation", 20, 5000),
    goods(
        CORE_LEATHER,
        "Core Leather",
        2,
        "INV_Misc_LeatherScrap_06",
        20,
        1500,
    ),
    recipe_item(
        PATTERN_FLARECORE_MANTLE,
        "Pattern: Flarecore Mantle",
        4,
        2,
        "INV_Scroll_05",
        10000,
    ),
    recipe_item(
        PATTERN_FLARECORE_GLOVES,
        "Pattern: Flarecore Gloves",
        4,
        2,
        "INV_Scroll_05",
        10000,
    ),
    recipe_item(
        PLANS_FIERY_CHAIN_GIRDLE,
        "Plans: Fiery Chain Girdle",
        4,
        4,
        "INV_Scroll_06",
        10000,
    ),
    recipe_item(
        PATTERN_MOLTEN_HELM,
        "Pattern: Molten Helm",
        4,
        1,
        "INV_Scroll_05",
        10000,
    ),
    recipe_item(
        PATTERN_COREHOUND_BOOTS,
        "Pattern: Corehound Boots",
        4,
        1,
        "INV_Scroll_05",
        10000,
    ),
    goods(SULFURON_INGOT, "Sulfuron Ingot", 4, "INV_Ingot_08", 20, 0),
    consumable(
        CORE_ARMOR_KIT,
        "Core Armor Kit",
        2,
        "INV_Misc_ArmorKit_18",
        10,
        1500,
    ),
    recipe_item(
        PATTERN_CORE_ARMOR_KIT,
        "Pattern: Core Armor Kit",
        4,
        1,
        "INV_Scroll_05",
        10000,
    ),
    consumable(
        ELEMENTAL_SHARPENING_STONE,
        "Elemental Sharpening Stone",
        2,
        "INV_Stone_12",
        20,
        500,
    ),
    recipe_item(
        PLANS_ELEMENTAL_SHARPENING_STONE,
        "Plans: Elemental Sharpening Stone",
        3,
        4,
        "INV_Scroll_06",
        5000,
    ),
    recipe_item(
        PLANS_SULFURON_HAMMER,
        "Plans: Sulfuron Hammer",
        4,
        4,
        "INV_Scroll_06",
        10000,
    ),
    trade(
        DENSE_DYNAMITE,
        "Dense Dynamite",
        1,
        "INV_Misc_Bomb_05",
        20,
        250,
        2,
    ),
    goods(BLOODVINE, "Bloodvine", 2, "INV_Misc_Herb_19", 20, 1000),
    bag(
        CORE_FELCLOTH_BAG,
        "Core Felcloth Bag",
        4,
        "INV_Misc_Bag_CoreFelclothBag",
        20,
        15000,
    ),
    recipe_item(
        PATTERN_CORE_FELCLOTH_BAG,
        "Pattern: Core Felcloth Bag",
        4,
        2,
        "INV_Scroll_05",
        10000,
    ),
    // P3's crafted gear: bind on equip, worn once equipped from the bags (`gear::crafted`).
    gear_item(
        EMBOSSED_LEATHER_VEST,
        "Embossed Leather Vest",
        1,
        (4, 2, 5),
        "INV_Chest_Leather_07",
        60,
    ),
    gear_item(
        HANDSTITCHED_LEATHER_BOOTS,
        "Handstitched Leather Boots",
        1,
        (4, 2, 8),
        "INV_Boots_01",
        60,
    ),
    gear_item(
        HANDSTITCHED_LEATHER_PANTS,
        "Handstitched Leather Pants",
        1,
        (4, 2, 7),
        "INV_Pants_02",
        60,
    ),
    gear_item(
        DARK_LEATHER_BOOTS,
        "Dark Leather Boots",
        2,
        (4, 2, 8),
        "INV_Boots_05",
        600,
    ),
    gear_item(
        DARK_LEATHER_TUNIC,
        "Dark Leather Tunic",
        2,
        (4, 2, 5),
        "INV_Chest_Leather_03",
        600,
    ),
    gear_item(
        BROWN_LINEN_VEST,
        "Brown Linen Vest",
        1,
        (4, 1, 5),
        "INV_Chest_Cloth_24",
        60,
    ),
    gear_item(
        WOOLEN_BOOTS,
        "Woolen Boots",
        1,
        (4, 1, 8),
        "INV_Boots_Cloth_01",
        60,
    ),
    gear_item(COPPER_MACE, "Copper Mace", 1, (2, 4, 13), "INV_Mace_01", 60),
    gear_item(BRONZE_AXE, "Bronze Axe", 1, (2, 0, 13), "INV_Axe_02", 60),
    gear_item(
        COPPER_CHAIN_BELT,
        "Copper Chain Belt",
        1,
        (4, 3, 6),
        "INV_Belt_03",
        60,
    ),
    gear_item(
        COPPER_CHAIN_PANTS,
        "Copper Chain Pants",
        1,
        (4, 3, 7),
        "INV_Pants_03",
        60,
    ),
    gear_item(
        COPPER_BRACERS,
        "Copper Bracers",
        1,
        (4, 3, 9),
        "INV_Bracer_07",
        60,
    ),
    gear_item(
        ROUGH_BRONZE_LEGGINGS,
        "Rough Bronze Leggings",
        1,
        (4, 3, 7),
        "INV_Pants_03",
        60,
    ),
    gear_item(
        GREEN_IRON_BRACERS,
        "Green Iron Bracers",
        2,
        (4, 3, 9),
        "INV_Bracer_03",
        600,
    ),
    gear_item(
        GREEN_IRON_HELM,
        "Green Iron Helm",
        2,
        (4, 3, 1),
        "INV_Helmet_25",
        600,
    ),
    gear_item(
        HARDENED_IRON_SHORTSWORD,
        "Hardened Iron Shortsword",
        2,
        (2, 7, 13),
        "INV_Sword_23",
        600,
    ),
    gear_item(
        HANDSTITCHED_LEATHER_BELT,
        "Handstitched Leather Belt",
        1,
        (4, 2, 6),
        "INV_Belt_04",
        60,
    ),
    gear_item(
        EMBOSSED_LEATHER_GLOVES,
        "Embossed Leather Gloves",
        1,
        (4, 2, 10),
        "INV_Gauntlets_05",
        60,
    ),
    gear_item(
        HEAVY_LINEN_GLOVES,
        "Heavy Linen Gloves",
        1,
        (4, 1, 10),
        "INV_Gauntlets_04",
        60,
    ),
    gear_item(
        HEAVY_WOOLEN_GLOVES,
        "Heavy Woolen Gloves",
        1,
        (4, 1, 10),
        "INV_Gauntlets_04",
        60,
    ),
    gear_item(
        DOUBLE_STITCHED_WOOLEN_SHOULDERS,
        "Double-stitched Woolen Shoulders",
        1,
        (4, 1, 3),
        "INV_Shoulder_09",
        60,
    ),
    gear_item(
        HEAVY_WOOLEN_PANTS,
        "Heavy Woolen Pants",
        1,
        (4, 1, 7),
        "INV_Pants_Cloth_05",
        60,
    ),
    gear_item(
        BARBARIC_HARNESS,
        "Barbaric Harness",
        2,
        (4, 2, 5),
        "INV_Chest_Leather_08",
        600,
    ),
    gear_item(
        HANDSTITCHED_LEATHER_VEST,
        "Handstitched Leather Vest",
        1,
        (4, 2, 5),
        "INV_Chest_Leather_09",
        60,
    ),
    gear_item(
        DARK_LEATHER_PANTS,
        "Dark Leather Pants",
        2,
        (4, 2, 7),
        "INV_Pants_06",
        600,
    ),
    gear_item(
        BARBARIC_LEGGINGS,
        "Barbaric Leggings",
        2,
        (4, 2, 7),
        "INV_Pants_Leather_07",
        600,
    ),
    gear_item(LINEN_BELT, "Linen Belt", 1, (4, 1, 6), "INV_Belt_10", 60),
    gear_item(
        SILK_HEADBAND,
        "Silk Headband",
        2,
        (4, 1, 1),
        "INV_Misc_Bandana_01",
        600,
    ),
    gear_item(
        GREEN_SILKEN_SHOULDERS,
        "Green Silken Shoulders",
        2,
        (4, 1, 3),
        "INV_Shoulder_02",
        600,
    ),
    gear_item(
        GREEN_SILK_ARMOR,
        "Green Silk Armor",
        2,
        (4, 1, 5),
        "INV_Chest_Cloth_31",
        600,
    ),
    gear_item(
        HANDSTITCHED_LEATHER_BRACERS,
        "Handstitched Leather Bracers",
        1,
        (4, 2, 9),
        "INV_Bracer_02",
        60,
    ),
    gear_item(
        BARBARIC_IRON_BREASTPLATE,
        "Barbaric Iron Breastplate",
        2,
        (4, 3, 5),
        "INV_Chest_Chain_07",
        600,
    ),
    gear_item(
        HEAVY_MITHRIL_BREASTPLATE,
        "Heavy Mithril Breastplate",
        2,
        (4, 4, 5),
        "INV_Chest_Plate08",
        600,
    ),
    gear_item(
        HEAVY_MITHRIL_BOOTS,
        "Heavy Mithril Boots",
        2,
        (4, 4, 8),
        "INV_Boots_Plate_03",
        600,
    ),
    gear_item(
        WICKED_MITHRIL_BLADE,
        "Wicked Mithril Blade",
        2,
        (2, 7, 13),
        "INV_Sword_37",
        600,
    ),
    gear_item(
        NIGHTSCAPE_TUNIC,
        "Nightscape Tunic",
        2,
        (4, 2, 5),
        "INV_Chest_Leather_06",
        600,
    ),
    gear_item(
        NIGHTSCAPE_HEADBAND,
        "Nightscape Headband",
        2,
        (4, 2, 1),
        "INV_Misc_Bandana_03",
        600,
    ),
    gear_item(
        NIGHTSCAPE_PANTS,
        "Nightscape Pants",
        2,
        (4, 2, 7),
        "INV_Pants_Leather_09",
        600,
    ),
    gear_item(
        NIGHTSCAPE_BOOTS,
        "Nightscape Boots",
        2,
        (4, 2, 8),
        "INV_Boots_08",
        600,
    ),
    gear_item(
        BLACK_MAGEWEAVE_VEST,
        "Black Mageweave Vest",
        2,
        (4, 1, 5),
        "INV_Chest_Cloth_04",
        600,
    ),
    gear_item(
        BLACK_MAGEWEAVE_LEGGINGS,
        "Black Mageweave Leggings",
        2,
        (4, 1, 7),
        "INV_Pants_Cloth_07",
        600,
    ),
    gear_item(
        BLACK_MAGEWEAVE_GLOVES,
        "Black Mageweave Gloves",
        2,
        (4, 1, 10),
        "INV_Gauntlets_14",
        600,
    ),
    gear_item(
        BLACK_MAGEWEAVE_BOOTS,
        "Black Mageweave Boots",
        2,
        (4, 1, 8),
        "INV_Boots_Cloth_05",
        600,
    ),
    gear_item(
        CINDERCLOTH_ROBE,
        "Cindercloth Robe",
        2,
        (4, 1, 20),
        "INV_Chest_Cloth_25",
        600,
    ),
    gear_item(
        SIMPLE_LINEN_PANTS,
        "Simple Linen Pants",
        1,
        (4, 1, 7),
        "INV_Pants_Cloth_02",
        60,
    ),
    gear_item(
        DARK_IRON_PLATE,
        "Dark Iron Plate",
        3,
        (4, 4, 5),
        "INV_Chest_Plate07",
        3000,
    ),
    gear_item(
        IMPERIAL_PLATE_CHEST,
        "Imperial Plate Chest",
        2,
        (4, 4, 5),
        "INV_Chest_Plate03",
        600,
    ),
    gear_item(
        IMPERIAL_PLATE_BELT,
        "Imperial Plate Belt",
        2,
        (4, 4, 6),
        "INV_Belt_11",
        600,
    ),
    gear_item(
        IMPERIAL_PLATE_BRACERS,
        "Imperial Plate Bracers",
        2,
        (4, 4, 9),
        "INV_Bracer_14",
        600,
    ),
    gear_item(
        IMPERIAL_PLATE_BOOTS,
        "Imperial Plate Boots",
        2,
        (4, 4, 8),
        "INV_Boots_Plate_04",
        600,
    ),
    gear_item(
        IMPERIAL_PLATE_HELM,
        "Imperial Plate Helm",
        2,
        (4, 4, 1),
        "INV_Helmet_25",
        600,
    ),
    gear_item(
        IMPERIAL_PLATE_SHOULDERS,
        "Imperial Plate Shoulders",
        2,
        (4, 4, 3),
        "INV_Shoulder_10",
        600,
    ),
    gear_item(
        IMPERIAL_PLATE_LEGGINGS,
        "Imperial Plate Leggings",
        2,
        (4, 4, 7),
        "INV_Pants_04",
        600,
    ),
    gear_item(
        LIONHEART_HELM,
        "Lionheart Helm",
        4,
        (4, 4, 1),
        "INV_Helmet_36",
        12000,
    ),
    gear_item(
        ARCANITE_REAPER,
        "Arcanite Reaper",
        4,
        (2, 1, 17),
        "INV_Axe_09",
        12000,
    ),
    gear_item(
        RUNECLOTH_BELT,
        "Runecloth Belt",
        2,
        (4, 1, 6),
        "INV_Belt_08",
        600,
    ),
    gear_item(
        RUNECLOTH_GLOVES,
        "Runecloth Gloves",
        2,
        (4, 1, 10),
        "INV_Gauntlets_18",
        600,
    ),
    gear_item(
        CINDERCLOTH_VEST,
        "Cindercloth Vest",
        2,
        (4, 1, 5),
        "INV_Chest_Cloth_23",
        600,
    ),
    gear_item(
        MOONCLOTH_LEGGINGS,
        "Mooncloth Leggings",
        3,
        (4, 1, 7),
        "INV_Pants_Cloth_09",
        3000,
    ),
    gear_item(
        MOONCLOTH_VEST,
        "Mooncloth Vest",
        3,
        (4, 1, 5),
        "INV_Chest_Cloth_08",
        3000,
    ),
    gear_item(
        ROBE_OF_THE_ARCHMAGE,
        "Robe of the Archmage",
        4,
        (4, 1, 20),
        "INV_Chest_Cloth_25",
        12000,
    ),
    gear_item(
        TRUEFAITH_VESTMENTS,
        "Truefaith Vestments",
        4,
        (4, 1, 20),
        "INV_Chest_Cloth_11",
        12000,
    ),
    gear_item(
        VOLCANIC_BREASTPLATE,
        "Volcanic Breastplate",
        2,
        (4, 2, 5),
        "INV_Chest_Leather_10",
        600,
    ),
    gear_item(
        VOLCANIC_LEGGINGS,
        "Volcanic Leggings",
        2,
        (4, 2, 7),
        "INV_Pants_Leather_05",
        600,
    ),
    gear_item(
        VOLCANIC_SHOULDERS,
        "Volcanic Shoulders",
        2,
        (4, 2, 3),
        "INV_Shoulder_23",
        600,
    ),
    gear_item(
        DEVILSAUR_LEGGINGS,
        "Devilsaur Leggings",
        3,
        (4, 2, 7),
        "INV_Pants_Leather_21",
        3000,
    ),
    gear_item(
        DEVILSAUR_GAUNTLETS,
        "Devilsaur Gauntlets",
        3,
        (4, 2, 10),
        "INV_Gauntlets_26",
        3000,
    ),
    gear_item(
        FLARECORE_GLOVES,
        "Flarecore Gloves",
        4,
        (4, 1, 10),
        "INV_Gauntlets_19",
        12000,
    ),
    gear_item(
        FLARECORE_MANTLE,
        "Flarecore Mantle",
        4,
        (4, 1, 3),
        "INV_Shoulder_19",
        12000,
    ),
    gear_item(
        COREHOUND_BOOTS,
        "Corehound Boots",
        4,
        (4, 2, 8),
        "INV_Boots_Chain_05",
        12000,
    ),
    gear_item(
        MOLTEN_HELM,
        "Molten Helm",
        4,
        (4, 2, 1),
        "INV_Helmet_08",
        12000,
    ),
    gear_item(
        FIERY_CHAIN_GIRDLE,
        "Fiery Chain Girdle",
        4,
        (4, 3, 6),
        "INV_Belt_13",
        12000,
    ),
    gear_item(
        SULFURON_HAMMER,
        "Sulfuron Hammer",
        4,
        (2, 5, 17),
        "INV_Hammer_Unique_Sulfuras",
        12000,
    ),
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
        2 => "Weapon",
        4 => "Armor",
        9 => "Recipe",
        _ => "Trade Goods",
    }
}

/// `ItemSubClass.dbc`'s display name for the subclasses authored here.
fn subclass_name(class: u32, subclass: u32) -> &'static str {
    match (class, subclass) {
        (1, _) => "Bag",
        (2, 0 | 1) => "Axe",
        (2, 4 | 5) => "Mace",
        (2, 7 | 8) => "Sword",
        (4, 0) => "Miscellaneous",
        (4, 1) => "Cloth",
        (4, 2) => "Leather",
        (4, 3) => "Mail",
        (4, 4) => "Plate",
        (0, _) => "Consumable",
        (7, 1) => "Parts",
        (7, 2) => "Explosives",
        (7, 3) => "Devices",
        (9, 1) => "Leatherworking",
        (9, 2) => "Tailoring",
        (9, 3) => "Engineering",
        (9, 4) => "Blacksmithing",
        (9, 5) => "Cooking",
        (9, 6) => "Alchemy",
        (9, 7) => "First Aid",
        (9, 8) => "Enchanting",
        _ => "Trade Goods",
    }
}

/// The green lines a template prints: a consumable's, a recipe's or a gadget's `Use:`, a crafted
/// piece's `Equip:` bonuses, or the authored line.
fn spell_lines(def: &ItemDef) -> Vec<(u32, u32, String)> {
    if let Some(text) = super::effects::use_line(def.entry) {
        return vec![(0, 0, text)];
    }
    if let Some(piece) = crate::survivors::gear::crafted_by_item(def.entry) {
        return piece
            .item
            .bonuses
            .iter()
            .map(|b| (1, 0, format!("{}.", b.describe())))
            .collect();
    }
    def.effect
        .map(|(trigger, text)| (trigger, 0, text.to_string()))
        .into_iter()
        .collect()
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
        hide_subclass: !matches!(def.class, 2 | 4),
        bonding: def.bonding,
        stackable: def.stack,
        container_slots: def.bag_slots,
        armor: def.armor,
        // A crafted piece names the classes that may wear it (red for the others); anything else
        // every class may use: `<= 0` prints no line.
        allowable_class: crate::survivors::gear::crafted_class_mask(def.entry),
        allowable_race: -1,
        spell_triggers: spell_lines(def),
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

    /// A crafted piece reads as 1.12's: Binds when equipped, its armour type, its green `Equip:`
    /// lines, and a class line that is red for a class that may not wear it.
    #[test]
    fn survivors_items_crafted_gear_reads_boe_and_its_classes() {
        use benilla_ui::script::{item_usable, PlayerReqState};
        let v = template_view(template(LIONHEART_HELM).unwrap());
        assert_eq!(v.bonding, 2, "Binds when equipped");
        assert_eq!(v.allowable_class, 0b11, "the Warrior and the Paladin");
        assert!(!v.hide_subclass);
        assert_eq!(v.item_sub_type.as_deref(), Some("Plate"));
        assert!(!v.spell_triggers.is_empty() && v.spell_triggers.iter().all(|t| t.0 == 1));
        let mage = PlayerReqState {
            level: 60,
            class_id: 8,
            race_id: 1,
            ..Default::default()
        };
        assert!(!item_usable(&v, &mage, |_| false), "red for a mage");
        let warrior = PlayerReqState {
            class_id: 1,
            ..mage.clone()
        };
        assert!(item_usable(&v, &warrior, |_| false));
        // A gadget's `Use:` line, a bag's slots, a recipe's subclass.
        let chicken = template_view(template(GNOMISH_BATTLE_CHICKEN).unwrap());
        assert_eq!(chicken.spell_triggers[0].0, 0);
        assert_eq!(
            template_view(template(MOONCLOTH_BAG).unwrap()).container_slots,
            16
        );
        let plans = template_view(template(PLANS_LIONHEART_HELM).unwrap());
        assert_eq!(plans.item_sub_type.as_deref(), Some("Blacksmithing"));
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
        (FLASK_OF_OIL, 18057),
        (MOSS_AGATE, 1217),
        (SHADOWGEM, 3490),
        (JADE, 3491),
        (LESSER_MOONSTONE, 12309),
        (EMBOSSED_LEATHER_VEST, 8664),
        (HANDSTITCHED_LEATHER_BOOTS, 2984),
        (HANDSTITCHED_LEATHER_PANTS, 683),
        (LIGHT_ARMOR_KIT, 16189),
        (MEDIUM_ARMOR_KIT, 23311),
        (DARK_LEATHER_BOOTS, 2094),
        (DARK_LEATHER_TUNIC, 14781),
        (COARSE_THREAD, 4752),
        (FINE_THREAD, 10044),
        (BROWN_LINEN_VEST, 34726),
        (WOOLEN_BOOTS, 34262),
        (GREEN_DYE, 18080),
        (SILVER_ORE, 18107),
        (GOLD_ORE, 4681),
        (SILVER_BAR, 1754),
        (COPPER_MACE, 860),
        (BRONZE_AXE, 1415),
        (COPPER_CHAIN_BELT, 4501),
        (COPPER_CHAIN_PANTS, 685),
        (COPPER_BRACERS, 3620),
        (ROUGH_SHARPENING_STONE, 24673),
        (COARSE_SHARPENING_STONE, 24674),
        (ROUGH_BRONZE_LEGGINGS, 685),
        (HEAVY_SHARPENING_STONE, 24675),
        (WEAK_FLUX, 34561),
        (BOLT_OF_LINEN_CLOTH, 4748),
        (BOLT_OF_WOOLEN_CLOTH, 4749),
        (ROUGH_WEIGHTSTONE, 24683),
        (COARSE_WEIGHTSTONE, 24684),
        (HEAVY_WEIGHTSTONE, 24685),
        (STRONG_FLUX, 34561),
        (ROUGH_GRINDING_STONE, 24678),
        (COARSE_GRINDING_STONE, 24679),
        (HEAVY_GRINDING_STONE, 24680),
        (GOLD_BAR, 1663),
        (GREEN_IRON_BRACERS, 3621),
        (GREEN_IRON_HELM, 16083),
        (HARDENED_IRON_SHORTSWORD, 5149),
        (CITRINE, 4169),
        (HANDSTITCHED_LEATHER_BELT, 4520),
        (EMBOSSED_LEATHER_GLOVES, 972),
        (WOOLEN_BAG, 2585),
        (SMALL_SILK_PACK, 981),
        (HEAVY_ARMOR_KIT, 30647),
        (SILKEN_THREAD, 4815),
        (BOLT_OF_SILK_CLOTH, 4750),
        (HEAVY_LINEN_GLOVES, 1063),
        (HEAVY_WOOLEN_GLOVES, 1063),
        (DOUBLE_STITCHED_WOOLEN_SHOULDERS, 4146),
        (HEAVY_WOOLEN_PANTS, 32924),
        (THICK_SPIDERS_SILK, 10044),
        (BOLT_OF_MAGEWEAVE, 9445),
        (GRAY_DYE, 18079),
        (ROUGH_BLASTING_POWDER, 2027),
        (HANDFUL_OF_COPPER_BOLTS, 1235),
        (ROUGH_COPPER_BOMB, 25483),
        (COPPER_MODULATOR, 7839),
        (COARSE_BLASTING_POWDER, 3744),
        (TARGET_DUMMY, 31423),
        (WHIRRING_BRONZE_GIZMO, 20626),
        (HEAVY_BLASTING_POWDER, 32819),
        (BRONZE_FRAMEWORK, 20624),
        (EXPLOSIVE_SHEEP, 7361),
        (IRON_STRUT, 18044),
        (GYROCHRONATOM, 21189),
        (IRON_GRENADE, 25482),
        (BIG_IRON_BOMB, 7624),
        (MECHANICAL_DRAGONLING, 20655),
        (SILVER_CONTACT, 3257),
        (BARBARIC_HARNESS, 8649),
        (HANDSTITCHED_LEATHER_VEST, 8646),
        (DARK_LEATHER_PANTS, 14338),
        (BARBARIC_LEGGINGS, 33678),
        (LINEN_BELT, 4905),
        (SILK_HEADBAND, 15863),
        (GREEN_SILKEN_SHOULDERS, 1060),
        (GREEN_SILK_ARMOR, 12696),
        (ELEMENTAL_EARTH, 28841),
        (ELEMENTAL_FIRE, 5686),
        (IRON_BUCKLE, 1221),
        (CORE_OF_EARTH, 17919),
        (ESSENCE_OF_EARTH, 23754),
        (HEART_OF_FIRE, 21583),
        (ESSENCE_OF_WATER, 33283),
        (ESSENCE_OF_AIR, 13687),
        (GOBLIN_JUMPER_CABLES, 31201),
        (GOBLIN_ROCKET_BOOTS, 20622),
        (FUSED_WIRING, 4660),
        (HANDSTITCHED_LEATHER_BRACERS, 3598),
        (AQUAMARINE, 13496),
        (STAR_RUBY, 4777),
        (BARBARIC_IRON_BREASTPLATE, 13011),
        (HEAVY_MITHRIL_BREASTPLATE, 19787),
        (HEAVY_MITHRIL_BOOTS, 19719),
        (WICKED_MITHRIL_BLADE, 18409),
        (SOLID_SHARPENING_STONE, 24676),
        (SOLID_WEIGHTSTONE, 24686),
        (SOLID_GRINDING_STONE, 24681),
        (ICHOR_OF_UNDEATH, 15794),
        (WICKED_CLAW, 1498),
        (RUGGED_HIDE, 2890),
        (THICK_ARMOR_KIT, 17724),
        (NIGHTSCAPE_TUNIC, 8640),
        (NIGHTSCAPE_HEADBAND, 8266),
        (NIGHTSCAPE_PANTS, 32341),
        (NIGHTSCAPE_BOOTS, 2366),
        (HEAVY_SILKEN_THREAD, 4816),
        (INLAID_MITHRIL_CYLINDER, 33552),
        (GOBLIN_ROCKET_FUEL, 18117),
        (GHOST_DYE, 23731),
        (BLACK_MAGEWEAVE_VEST, 24598),
        (BLACK_MAGEWEAVE_LEGGINGS, 33652),
        (BLACK_MAGEWEAVE_GLOVES, 22755),
        (BLACK_MAGEWEAVE_BOOTS, 33048),
        (CINDERCLOTH_ROBE, 12673),
        (SIMPLE_LINEN_PANTS, 33618),
        (MAGEWEAVE_BAG, 4056),
        (SHADOW_SILK, 4816),
        (SOLID_BLASTING_POWDER, 31324),
        (GOLD_POWER_CORE, 3109),
        (MITHRIL_TUBE, 15507),
        (UNSTABLE_TRIGGER, 20628),
        (MITHRIL_CASING, 20620),
        (MITHRIL_MECHANICAL_DRAGONLING, 20655),
        (GOBLIN_MORTAR, 31200),
        (SCHEMATIC_MITHRIL_MECHANICAL_DRAGONLING, 811),
        (GNOMISH_DEATH_RAY, 31202),
        (GOBLIN_SAPPER_CHARGE, 25484),
        (GNOMISH_NET_O_MATIC_PROJECTOR, 20627),
        (GNOMISH_BATTLE_CHICKEN, 19115),
        (GOBLIN_DRAGON_GUN, 36338),
        (DARK_IRON_ORE, 4691),
        (DARK_IRON_BAR, 4693),
        (BLOOD_OF_THE_MOUNTAIN, 1263),
        (DARK_IRON_PLATE, 22510),
        (PLANS_DARK_IRON_PLATE, 15274),
        (ARCANITE_BAR, 1485),
        (BLUE_SAPPHIRE, 1659),
        (ARCANE_CRYSTAL, 3273),
        (DENSE_SHARPENING_STONE, 24677),
        (IMPERIAL_PLATE_CHEST, 9387),
        (IMPERIAL_PLATE_BELT, 25740),
        (IMPERIAL_PLATE_BRACERS, 18378),
        (IMPERIAL_PLATE_BOOTS, 19701),
        (IMPERIAL_PLATE_HELM, 15811),
        (IMPERIAL_PLATE_SHOULDERS, 29971),
        (IMPERIAL_PLATE_LEGGINGS, 3414),
        (LIONHEART_HELM, 22565),
        (DENSE_WEIGHTSTONE, 24687),
        (DENSE_GRINDING_STONE, 24682),
        (ENCHANTED_THORIUM_BAR, 22445),
        (PLANS_IMPERIAL_PLATE_SHOULDERS, 811),
        (PLANS_IMPERIAL_PLATE_BELT, 811),
        (PLANS_IMPERIAL_PLATE_BRACERS, 811),
        (PLANS_IMPERIAL_PLATE_BOOTS, 811),
        (PLANS_IMPERIAL_PLATE_HELM, 811),
        (PLANS_IMPERIAL_PLATE_CHEST, 811),
        (PLANS_IMPERIAL_PLATE_LEGGINGS, 811),
        (PLANS_LIONHEART_HELM, 15274),
        (ARCANITE_REAPER, 19238),
        (AZEROTHIAN_DIAMOND, 4775),
        (ESSENCE_OF_UNDEATH, 23291),
        (ENCHANTED_LEATHER, 23292),
        (PLANS_ARCANITE_REAPER, 15274),
        (RECIPE_TRANSMUTE_ARCANITE, 15274),
        (RUNECLOTH_BELT, 15012),
        (RUNECLOTH_GLOVES, 14045),
        (GOLDEN_PEARL, 24730),
        (CINDERCLOTH_VEST, 34725),
        (RUNECLOTH_BAG, 21585),
        (BOLT_OF_RUNECLOTH, 24898),
        (MOONCLOTH_LEGGINGS, 32989),
        (MOONCLOTH_VEST, 14584),
        (ROBE_OF_THE_ARCHMAGE, 12673),
        (TRUEFAITH_VESTMENTS, 34730),
        (MOONCLOTH_BAG, 20342),
        (BOTTOMLESS_BAG, 24496),
        (IRONWEB_SPIDER_SILK, 4815),
        (FELCLOTH, 25028),
        (RUNE_THREAD, 34846),
        (MOONCLOTH, 25052),
        (PATTERN_MOONCLOTH_LEGGINGS, 811),
        (PATTERN_MOONCLOTH_BAG, 811),
        (PATTERN_MOONCLOTH_VEST, 811),
        (PATTERN_BOTTOMLESS_BAG, 15274),
        (PATTERN_ROBE_OF_THE_ARCHMAGE, 15274),
        (PATTERN_TRUEFAITH_VESTMENTS, 15274),
        (VOLCANIC_BREASTPLATE, 8647),
        (VOLCANIC_LEGGINGS, 33119),
        (VOLCANIC_SHOULDERS, 14696),
        (DEVILSAUR_LEGGINGS, 34710),
        (DEVILSAUR_GAUNTLETS, 17187),
        (CURED_RUGGED_HIDE, 2107),
        (REFINED_DEEPROCK_SALT, 34578),
        (DEVILSAUR_LEATHER, 4676),
        (RUGGED_ARMOR_KIT, 24776),
        (PATTERN_VOLCANIC_LEGGINGS, 811),
        (PATTERN_VOLCANIC_BREASTPLATE, 811),
        (PATTERN_DEVILSAUR_GAUNTLETS, 811),
        (PATTERN_DEVILSAUR_LEGGINGS, 811),
        (PATTERN_VOLCANIC_SHOULDERS, 811),
        (DENSE_BLASTING_POWDER, 2027),
        (THORIUM_GRENADE, 18062),
        (THORIUM_WIDGET, 20625),
        (THORIUM_TUBE, 19482),
        (DELICATE_ARCANITE_CONVERTER, 24052),
        (ARCANITE_DRAGONLING, 31434),
        (MASTERWORK_TARGET_DUMMY, 31205),
        (SCHEMATIC_THORIUM_GRENADE, 811),
        (SCHEMATIC_MASTERWORK_TARGET_DUMMY, 811),
        (SCHEMATIC_ARCANITE_DRAGONLING, 15274),
        (FLARECORE_GLOVES, 12398),
        (FLARECORE_MANTLE, 31823),
        (COREHOUND_BOOTS, 34976),
        (MOLTEN_HELM, 15550),
        (FIERY_CHAIN_GIRDLE, 26953),
        (FIERY_CORE, 28258),
        (LAVA_CORE, 29317),
        (CORE_LEATHER, 5755),
        (PATTERN_FLARECORE_MANTLE, 1097),
        (PATTERN_FLARECORE_GLOVES, 1097),
        (PLANS_FIERY_CHAIN_GIRDLE, 15274),
        (PATTERN_MOLTEN_HELM, 1097),
        (PATTERN_COREHOUND_BOOTS, 1097),
        (SULFURON_HAMMER, 29446),
        (SULFURON_INGOT, 20656),
        (CORE_ARMOR_KIT, 23763),
        (PATTERN_CORE_ARMOR_KIT, 1097),
        (ELEMENTAL_SHARPENING_STONE, 4716),
        (PLANS_ELEMENTAL_SHARPENING_STONE, 15274),
        (PLANS_SULFURON_HAMMER, 15274),
        (DENSE_DYNAMITE, 7626),
        (BLOODVINE, 19495),
        (CORE_FELCLOTH_BAG, 33942),
        (PATTERN_CORE_FELCLOTH_BAG, 1097),
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
