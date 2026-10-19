//! The curated recipe books (docs/PROFESSIONS.md, 5.1): Mining's Smelting, and P2's Alchemy,
//! Cooking, First Aid and Enchanting, done at the camp's fire. Each row is a real 1.12 tradeskill
//! spell: its `Reagent[8]`, its `SPELL_EFFECT_CREATE_ITEM` product and count (or, for an enchant,
//! its `SPELL_EFFECT_ENCHANT_ITEM_PERMANENT` and the `SpellItemEnchantment` row it names), its
//! `SkillLineAbility` line and trivial bands, its spell focus and its cast time are checked against
//! the client by [`tests::every_recipe_resolves_in_the_client`].
//!
//! **Authored:** the skill each is learned at (the trainer's `reqskillvalue`, from memory: the
//! client does not carry it), how it is learned ([`Learn`]: at that skill, bought from the camp's
//! trainer, or read off a recipe that drops in runs), the trainer's prices, and each enchant's
//! survivors bonuses ([`Enchant`]).
//!
//! **The camp's fire** stands in for every focus a book names: Smelting's Forge, Cooking's Cooking
//! Fire and the flasks' Alchemy Lab.

use benilla_ui::script::{
    CraftReagent, CraftRecipe, CraftState, CraftTooltip, TradeSkillReagent, TradeSkillRecipe,
    TradeSkillState,
};

use super::super::bonus::{Bonus, School, StatKey};
use super::super::gear::Slot;
use super::inventory::{BagError, Bags};
use super::items::{self, *};
use super::skills::{Line, Skills};

/// Every Smelt spell's cast: `SpellCastTimes.dbc` row 14.
#[cfg(test)]
pub(crate) const SMELT_SECS: f32 = 3.0;
/// A `SpellFocusObject.dbc` row: its id and name, the tradeskill window's "Requires:" line.
pub(crate) type Focus = (u32, &'static str);
/// Smelting's focus.
pub(crate) const FORGE: Focus = (3, "Forge");
/// Cooking's focus.
pub(crate) const COOKING_FIRE: Focus = (4, "Cooking Fire");
/// The flasks' focus.
pub(crate) const ALCHEMY_LAB: Focus = (663, "Alchemy Lab");
/// Every enchant spell's icon (`SpellIcon.dbc`), which the craft window shows for the row.
pub(crate) const ENCHANT_ICON: &str = "Interface\\Icons\\Spell_Holy_GreaterHeal";
/// The craft window's type for Enchanting: the opener's `EffectMiscValue[0]`.
const CRAFT_TYPE_ENCHANTING: u32 = 3;

/// How a recipe is learned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Learn {
    /// Known once the skill reaches the recipe's, as if its trainer had taught it.
    Skill,
    /// Bought from the camp's trainer for this many copper, once the skill reaches the recipe's.
    Trainer(u32),
    /// Read off this recipe item, which drops in runs.
    Drop(u32),
}

/// What an enchant does to the piece it is on: its `SpellItemEnchantment` row (the tooltip's
/// green line is its name), the survivors slot it goes on, and its bonuses, which join the worn
/// piece's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Enchant {
    pub(crate) id: u32,
    pub(crate) name: &'static str,
    pub(in crate::survivors) slot: Slot,
    pub(in crate::survivors) bonuses: &'static [Bonus],
}

/// One recipe.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Recipe {
    pub(crate) spell: u32,
    pub(crate) name: &'static str,
    pub(crate) line: Line,
    /// `(item, count)`, in `Reagent[8]`'s order.
    pub(crate) reagents: &'static [(u32, u32)],
    /// The created item, 0 for an enchant.
    pub(crate) product: u32,
    /// Made per cast: `EffectBasePoints` plus its one die.
    pub(crate) made: u32,
    /// The skill it is learned at.
    pub(crate) req: u16,
    /// `SkillLineAbility`'s `min_value` and `max_value`: yellow from the first, grey from the
    /// second.
    pub(crate) trivial: (u16, u16),
    /// `SpellCastTimes.dbc`'s base, in milliseconds.
    pub(crate) cast_ms: u32,
    pub(crate) focus: Option<Focus>,
    pub(crate) learn: Learn,
    pub(crate) enchant: Option<&'static Enchant>,
}

impl Recipe {
    const fn makes(self, made: u32) -> Self {
        Self { made, ..self }
    }

    const fn casts(self, cast_ms: u32) -> Self {
        Self { cast_ms, ..self }
    }

    const fn at(self, focus: Focus) -> Self {
        Self {
            focus: Some(focus),
            ..self
        }
    }

    const fn bought(self, copper: u32) -> Self {
        Self {
            learn: Learn::Trainer(copper),
            ..self
        }
    }

    const fn dropped(self, item: u32) -> Self {
        Self {
            learn: Learn::Drop(item),
            ..self
        }
    }

    /// The cast, in seconds.
    pub(crate) fn cast_secs(&self) -> f32 {
        self.cast_ms as f32 / 1000.0
    }
}

const fn base(
    line: Line,
    spell: u32,
    name: &'static str,
    reagents: &'static [(u32, u32)],
    product: u32,
    req: u16,
    trivial: (u16, u16),
) -> Recipe {
    Recipe {
        spell,
        name,
        line,
        reagents,
        product,
        made: 1,
        req,
        trivial,
        cast_ms: 3000,
        focus: None,
        learn: Learn::Skill,
        enchant: None,
    }
}

const fn smelt(
    spell: u32,
    name: &'static str,
    reagents: &'static [(u32, u32)],
    product: u32,
    req: u16,
    trivial: (u16, u16),
) -> Recipe {
    base(Line::Mining, spell, name, reagents, product, req, trivial).at(FORGE)
}

const fn brew(
    spell: u32,
    name: &'static str,
    reagents: &'static [(u32, u32)],
    product: u32,
    req: u16,
    trivial: (u16, u16),
) -> Recipe {
    base(Line::Alchemy, spell, name, reagents, product, req, trivial)
}

const fn cook(
    spell: u32,
    name: &'static str,
    reagents: &'static [(u32, u32)],
    product: u32,
    req: u16,
    trivial: (u16, u16),
) -> Recipe {
    base(Line::Cooking, spell, name, reagents, product, req, trivial).at(COOKING_FIRE)
}

const fn bandage(
    spell: u32,
    name: &'static str,
    reagents: &'static [(u32, u32)],
    product: u32,
    req: u16,
    trivial: (u16, u16),
) -> Recipe {
    base(Line::FirstAid, spell, name, reagents, product, req, trivial)
}

const fn enchant(
    spell: u32,
    name: &'static str,
    reagents: &'static [(u32, u32)],
    req: u16,
    trivial: (u16, u16),
    enchant: &'static Enchant,
) -> Recipe {
    Recipe {
        made: 0,
        cast_ms: 5000,
        enchant: Some(enchant),
        ..base(Line::Enchanting, spell, name, reagents, 0, req, trivial)
    }
}

/// Smelting, in the order the trainer teaches it: the ores the run's veins give (no Silver, Gold
/// or Dark Iron yet).
static SMELTING: &[Recipe] = &[
    smelt(
        2657,
        "Smelt Copper",
        &[(COPPER_ORE, 1)],
        COPPER_BAR,
        1,
        (25, 70),
    ),
    smelt(3304, "Smelt Tin", &[(TIN_ORE, 1)], TIN_BAR, 65, (50, 75)),
    smelt(
        2659,
        "Smelt Bronze",
        &[(COPPER_BAR, 1), (TIN_BAR, 1)],
        BRONZE_BAR,
        65,
        (65, 115),
    )
    .makes(2),
    smelt(
        3307,
        "Smelt Iron",
        &[(IRON_ORE, 1)],
        IRON_BAR,
        125,
        (130, 140),
    ),
    smelt(
        3569,
        "Smelt Steel",
        &[(IRON_BAR, 1), (COAL, 1)],
        STEEL_BAR,
        165,
        (165, 165),
    ),
    smelt(
        10097,
        "Smelt Mithril",
        &[(MITHRIL_ORE, 1)],
        MITHRIL_BAR,
        175,
        (175, 175),
    ),
    smelt(
        10098,
        "Smelt Truesilver",
        &[(TRUESILVER_ORE, 1)],
        TRUESILVER_BAR,
        230,
        (230, 230),
    ),
    smelt(
        16153,
        "Smelt Thorium",
        &[(THORIUM_ORE, 1)],
        THORIUM_BAR,
        250,
        (250, 250),
    ),
];

/// Alchemy.
static ALCHEMY: &[Recipe] = &[
    brew(
        2330,
        "Minor Healing Potion",
        &[(PEACEBLOOM, 1), (SILVERLEAF, 1), (EMPTY_VIAL, 1)],
        MINOR_HEALING_POTION,
        1,
        (55, 95),
    ),
    brew(
        7183,
        "Elixir of Minor Defense",
        &[(SILVERLEAF, 2), (EMPTY_VIAL, 1)],
        ELIXIR_OF_MINOR_DEFENSE,
        1,
        (55, 95),
    ),
    brew(
        2331,
        "Minor Mana Potion",
        &[(MAGEROYAL, 1), (SILVERLEAF, 1), (EMPTY_VIAL, 1)],
        MINOR_MANA_POTION,
        25,
        (65, 105),
    ),
    brew(
        2332,
        "Minor Rejuvenation Potion",
        &[(MAGEROYAL, 2), (PEACEBLOOM, 1), (EMPTY_VIAL, 1)],
        MINOR_REJUVENATION_POTION,
        40,
        (70, 110),
    ),
    brew(
        7836,
        "Blackmouth Oil",
        &[(OILY_BLACKMOUTH, 2), (EMPTY_VIAL, 1)],
        BLACKMOUTH_OIL,
        80,
        (80, 100),
    )
    .casts(5000)
    .bought(400),
    brew(
        3172,
        "Minor Magic Resistance Potion",
        &[(MAGEROYAL, 3), (WILD_STEELBLOOM, 1), (EMPTY_VIAL, 1)],
        MINOR_MAGIC_RESISTANCE_POTION,
        110,
        (135, 175),
    ),
    brew(
        3173,
        "Lesser Mana Potion",
        &[(MAGEROYAL, 1), (STRANGLEKELP, 1), (EMPTY_VIAL, 1)],
        LESSER_MANA_POTION,
        120,
        (145, 185),
    ),
    brew(
        3177,
        "Elixir of Defense",
        &[(WILD_STEELBLOOM, 1), (STRANGLEKELP, 1), (LEADED_VIAL, 1)],
        ELIXIR_OF_DEFENSE,
        130,
        (155, 195),
    ),
    brew(
        6624,
        "Free Action Potion",
        &[(BLACKMOUTH_OIL, 2), (STRANGLEKELP, 1), (LEADED_VIAL, 1)],
        FREE_ACTION_POTION,
        150,
        (175, 215),
    )
    .bought(2000),
    brew(
        7181,
        "Greater Healing Potion",
        &[(LIFEROOT, 1), (KINGSBLOOD, 1), (LEADED_VIAL, 1)],
        GREATER_HEALING_POTION,
        155,
        (175, 215),
    ),
    brew(
        3452,
        "Mana Potion",
        &[(STRANGLEKELP, 1), (KINGSBLOOD, 1), (LEADED_VIAL, 1)],
        MANA_POTION,
        160,
        (180, 220),
    )
    .casts(5000),
    brew(
        3450,
        "Elixir of Fortitude",
        &[(WILD_STEELBLOOM, 1), (GOLDTHORN, 1), (LEADED_VIAL, 1)],
        ELIXIR_OF_FORTITUDE,
        175,
        (195, 235),
    )
    .casts(5000),
    brew(
        3451,
        "Mighty Troll's Blood Potion",
        &[(LIFEROOT, 1), (BRUISEWEED, 1), (LEADED_VIAL, 1)],
        MIGHTY_TROLLS_BLOOD_POTION,
        180,
        (200, 240),
    )
    .casts(5000),
    brew(
        11449,
        "Elixir of Agility",
        &[(STRANGLEKELP, 1), (GOLDTHORN, 1), (LEADED_VIAL, 1)],
        ELIXIR_OF_AGILITY,
        185,
        (205, 245),
    ),
    brew(
        11450,
        "Elixir of Greater Defense",
        &[(WILD_STEELBLOOM, 1), (GOLDTHORN, 1), (LEADED_VIAL, 1)],
        ELIXIR_OF_GREATER_DEFENSE,
        195,
        (215, 255),
    ),
    brew(
        11448,
        "Greater Mana Potion",
        &[(KHADGARS_WHISKER, 1), (GOLDTHORN, 1), (LEADED_VIAL, 1)],
        GREATER_MANA_POTION,
        205,
        (220, 260),
    ),
    brew(
        11457,
        "Superior Healing Potion",
        &[(SUNGRASS, 1), (KHADGARS_WHISKER, 1), (CRYSTAL_VIAL, 1)],
        SUPERIOR_HEALING_POTION,
        215,
        (230, 270),
    ),
    brew(
        11467,
        "Elixir of Greater Agility",
        &[(SUNGRASS, 1), (GOLDTHORN, 1), (CRYSTAL_VIAL, 1)],
        ELIXIR_OF_GREATER_AGILITY,
        240,
        (255, 295),
    ),
    brew(
        11472,
        "Elixir of Giants",
        &[(SUNGRASS, 1), (GROMSBLOOD, 1), (CRYSTAL_VIAL, 1)],
        ELIXIR_OF_GIANTS,
        245,
        (260, 300),
    )
    .bought(10000),
    brew(
        17551,
        "Stonescale Oil",
        &[(STONESCALE_EEL, 1), (LEADED_VIAL, 1)],
        STONESCALE_OIL,
        250,
        (250, 260),
    ),
    brew(
        17552,
        "Mighty Rage Potion",
        &[(GROMSBLOOD, 3), (CRYSTAL_VIAL, 1)],
        MIGHTY_RAGE_POTION,
        255,
        (270, 310),
    )
    .bought(12000),
    brew(
        17554,
        "Elixir of Superior Defense",
        &[(STONESCALE_OIL, 2), (SUNGRASS, 1), (CRYSTAL_VIAL, 1)],
        ELIXIR_OF_SUPERIOR_DEFENSE,
        265,
        (280, 320),
    ),
    brew(
        17555,
        "Elixir of the Sages",
        &[(DREAMFOIL, 1), (PLAGUEBLOOM, 2), (CRYSTAL_VIAL, 1)],
        ELIXIR_OF_THE_SAGES,
        270,
        (285, 325),
    )
    .bought(15000),
    brew(
        17556,
        "Major Healing Potion",
        &[
            (GOLDEN_SANSAM, 2),
            (MOUNTAIN_SILVERSAGE, 1),
            (CRYSTAL_VIAL, 1),
        ],
        MAJOR_HEALING_POTION,
        275,
        (290, 330),
    )
    .bought(15000),
    brew(
        17557,
        "Elixir of Brute Force",
        &[(GROMSBLOOD, 2), (PLAGUEBLOOM, 2), (CRYSTAL_VIAL, 1)],
        ELIXIR_OF_BRUTE_FORCE,
        275,
        (290, 330),
    )
    .bought(15000),
    brew(
        17570,
        "Greater Stoneshield Potion",
        &[(STONESCALE_OIL, 3), (THORIUM_ORE, 1), (CRYSTAL_VIAL, 1)],
        GREATER_STONESHIELD_POTION,
        280,
        (295, 335),
    )
    .bought(15000),
    brew(
        17571,
        "Elixir of the Mongoose",
        &[
            (MOUNTAIN_SILVERSAGE, 2),
            (PLAGUEBLOOM, 2),
            (CRYSTAL_VIAL, 1),
        ],
        ELIXIR_OF_THE_MONGOOSE,
        280,
        (295, 335),
    )
    .bought(20000),
    brew(
        17573,
        "Greater Arcane Elixir",
        &[(DREAMFOIL, 3), (MOUNTAIN_SILVERSAGE, 1), (CRYSTAL_VIAL, 1)],
        GREATER_ARCANE_ELIXIR,
        285,
        (300, 340),
    )
    .bought(20000),
    brew(
        17580,
        "Major Mana Potion",
        &[(DREAMFOIL, 3), (ICECAP, 2), (CRYSTAL_VIAL, 1)],
        MAJOR_MANA_POTION,
        295,
        (310, 350),
    )
    .bought(20000),
    brew(
        17635,
        "Flask of the Titans",
        &[
            (GROMSBLOOD, 30),
            (STONESCALE_OIL, 10),
            (BLACK_LOTUS, 1),
            (CRYSTAL_VIAL, 1),
        ],
        FLASK_OF_THE_TITANS,
        300,
        (315, 330),
    )
    .casts(25000)
    .at(ALCHEMY_LAB)
    .dropped(RECIPE_FLASK_OF_THE_TITANS),
    brew(
        17637,
        "Flask of Supreme Power",
        &[
            (DREAMFOIL, 30),
            (MOUNTAIN_SILVERSAGE, 10),
            (BLACK_LOTUS, 1),
            (CRYSTAL_VIAL, 1),
        ],
        FLASK_OF_SUPREME_POWER,
        300,
        (315, 330),
    )
    .casts(25000)
    .at(ALCHEMY_LAB)
    .dropped(RECIPE_FLASK_OF_SUPREME_POWER),
    brew(
        17636,
        "Flask of Distilled Wisdom",
        &[
            (DREAMFOIL, 30),
            (ICECAP, 10),
            (BLACK_LOTUS, 1),
            (CRYSTAL_VIAL, 1),
        ],
        FLASK_OF_DISTILLED_WISDOM,
        300,
        (315, 330),
    )
    .casts(25000)
    .at(ALCHEMY_LAB)
    .dropped(RECIPE_FLASK_OF_DISTILLED_WISDOM),
];

/// Cooking.
static COOKING: &[Recipe] = &[
    cook(
        7751,
        "Brilliant Smallfish",
        &[(RAW_BRILLIANT_SMALLFISH, 1)],
        BRILLIANT_SMALLFISH,
        1,
        (45, 85),
    ),
    cook(
        2539,
        "Spiced Wolf Meat",
        &[(STRINGY_WOLF_MEAT, 1), (MILD_SPICES, 1)],
        SPICED_WOLF_MEAT,
        10,
        (50, 90),
    ),
    cook(
        8607,
        "Smoked Bear Meat",
        &[(BEAR_MEAT, 1)],
        SMOKED_BEAR_MEAT,
        40,
        (80, 120),
    )
    .bought(300),
    cook(
        2541,
        "Coyote Steak",
        &[(COYOTE_MEAT, 1)],
        COYOTE_STEAK,
        50,
        (90, 130),
    ),
    cook(
        7753,
        "Longjaw Mud Snapper",
        &[(RAW_LONGJAW_MUD_SNAPPER, 1)],
        LONGJAW_MUD_SNAPPER,
        50,
        (90, 130),
    ),
    cook(
        25704,
        "Smoked Sagefish",
        &[(RAW_SAGEFISH, 1), (MILD_SPICES, 1)],
        SMOKED_SAGEFISH,
        80,
        (120, 160),
    )
    .bought(1000),
    cook(
        8238,
        "Savory Deviate Delight",
        &[(DEVIATE_FISH, 1), (MILD_SPICES, 1)],
        SAVORY_DEVIATE_DELIGHT,
        85,
        (125, 165),
    )
    .dropped(RECIPE_SAVORY_DEVIATE_DELIGHT),
    cook(
        3397,
        "Big Bear Steak",
        &[(BIG_BEAR_MEAT, 1), (HOT_SPICES, 1)],
        BIG_BEAR_STEAK,
        110,
        (150, 190),
    )
    .bought(2000),
    cook(
        3398,
        "Hot Lion Chops",
        &[(LION_MEAT, 1), (HOT_SPICES, 1)],
        HOT_LION_CHOPS,
        125,
        (175, 215),
    )
    .bought(2500),
    cook(
        15855,
        "Roast Raptor",
        &[(RAPTOR_FLESH, 1), (HOT_SPICES, 1)],
        ROAST_RAPTOR,
        175,
        (215, 255),
    )
    .bought(5000),
    cook(
        15856,
        "Hot Wolf Ribs",
        &[(RED_WOLF_MEAT, 1), (HOT_SPICES, 1)],
        HOT_WOLF_RIBS,
        175,
        (215, 255),
    )
    .bought(5000),
    cook(
        18238,
        "Spotted Yellowtail",
        &[(RAW_SPOTTED_YELLOWTAIL, 1)],
        SPOTTED_YELLOWTAIL,
        225,
        (265, 305),
    ),
    cook(
        15933,
        "Monster Omelet",
        &[(GIANT_EGG, 1), (SOOTHING_SPICES, 2)],
        MONSTER_OMELET,
        225,
        (265, 305),
    )
    .bought(10000),
    cook(
        22480,
        "Tender Wolf Steak",
        &[(TENDER_WOLF_MEAT, 1), (SOOTHING_SPICES, 1)],
        TENDER_WOLF_STEAK,
        225,
        (265, 305),
    )
    .bought(10000),
    cook(
        18240,
        "Grilled Squid",
        &[(WINTER_SQUID, 1), (SOOTHING_SPICES, 1)],
        GRILLED_SQUID,
        240,
        (280, 320),
    )
    .bought(12000),
    cook(
        18243,
        "Nightfin Soup",
        &[(RAW_NIGHTFIN_SNAPPER, 1), (REFRESHING_SPRING_WATER, 1)],
        NIGHTFIN_SOUP,
        250,
        (290, 330),
    )
    .bought(15000),
];

/// First aid.
static FIRST_AID: &[Recipe] = &[
    bandage(
        3275,
        "Linen Bandage",
        &[(LINEN_CLOTH, 1)],
        LINEN_BANDAGE,
        1,
        (30, 60),
    ),
    bandage(
        3276,
        "Heavy Linen Bandage",
        &[(LINEN_CLOTH, 2)],
        HEAVY_LINEN_BANDAGE,
        40,
        (50, 100),
    ),
    bandage(
        3277,
        "Wool Bandage",
        &[(WOOL_CLOTH, 1)],
        WOOL_BANDAGE,
        80,
        (80, 150),
    ),
    bandage(
        3278,
        "Heavy Wool Bandage",
        &[(WOOL_CLOTH, 2)],
        HEAVY_WOOL_BANDAGE,
        115,
        (115, 185),
    ),
    bandage(
        7928,
        "Silk Bandage",
        &[(SILK_CLOTH, 1)],
        SILK_BANDAGE,
        150,
        (150, 210),
    ),
    bandage(
        7929,
        "Heavy Silk Bandage",
        &[(SILK_CLOTH, 2)],
        HEAVY_SILK_BANDAGE,
        180,
        (180, 240),
    ),
    bandage(
        10840,
        "Mageweave Bandage",
        &[(MAGEWEAVE_CLOTH, 1)],
        MAGEWEAVE_BANDAGE,
        210,
        (210, 270),
    ),
    bandage(
        10841,
        "Heavy Mageweave Bandage",
        &[(MAGEWEAVE_CLOTH, 2)],
        HEAVY_MAGEWEAVE_BANDAGE,
        240,
        (240, 300),
    ),
    bandage(
        18629,
        "Runecloth Bandage",
        &[(RUNECLOTH, 1)],
        RUNECLOTH_BANDAGE,
        260,
        (260, 320),
    ),
    bandage(
        18630,
        "Heavy Runecloth Bandage",
        &[(RUNECLOTH, 2)],
        HEAVY_RUNECLOTH_BANDAGE,
        290,
        (290, 350),
    ),
];

/// Enchanting: each enchant a real `SpellItemEnchantment` row, its survivors bonuses ours.
static ENCHANTING: &[Recipe] = &[
    enchant(
        7418,
        "Enchant Bracer - Minor Health",
        &[(STRANGE_DUST, 1)],
        1,
        (70, 110),
        &Enchant {
            id: 41,
            name: "Health +5",
            slot: Slot::Wrist,
            bonuses: &[Bonus {
                stat: StatKey::MaxHealth,
                value: 2.0,
                school: None,
            }],
        },
    ),
    enchant(
        7420,
        "Enchant Chest - Minor Health",
        &[(STRANGE_DUST, 1)],
        1,
        (70, 110),
        &Enchant {
            id: 41,
            name: "Health +5",
            slot: Slot::Chest,
            bonuses: &[Bonus {
                stat: StatKey::MaxHealth,
                value: 2.0,
                school: None,
            }],
        },
    ),
    enchant(
        7426,
        "Enchant Chest - Minor Absorption",
        &[(STRANGE_DUST, 2), (LESSER_MAGIC_ESSENCE, 1)],
        70,
        (90, 130),
        &Enchant {
            id: 44,
            name: "Absorption (10)",
            slot: Slot::Chest,
            bonuses: &[Bonus {
                stat: StatKey::Armor,
                value: 0.01,
                school: None,
            }],
        },
    ),
    enchant(
        7457,
        "Enchant Bracer - Minor Stamina",
        &[(STRANGE_DUST, 3)],
        80,
        (100, 140),
        &Enchant {
            id: 66,
            name: "Stamina +1",
            slot: Slot::Wrist,
            bonuses: &[Bonus {
                stat: StatKey::MaxHealth,
                value: 2.0,
                school: None,
            }],
        },
    ),
    enchant(
        7748,
        "Enchant Chest - Lesser Health",
        &[(STRANGE_DUST, 2), (LESSER_MAGIC_ESSENCE, 2)],
        85,
        (105, 145),
        &Enchant {
            id: 242,
            name: "Health +15",
            slot: Slot::Chest,
            bonuses: &[Bonus {
                stat: StatKey::MaxHealth,
                value: 5.0,
                school: None,
            }],
        },
    ),
    enchant(
        7779,
        "Enchant Bracer - Minor Agility",
        &[(STRANGE_DUST, 2), (GREATER_MAGIC_ESSENCE, 1)],
        95,
        (115, 155),
        &Enchant {
            id: 247,
            name: "Agility +1",
            slot: Slot::Wrist,
            bonuses: &[Bonus {
                stat: StatKey::CritChance,
                value: 0.005,
                school: None,
            }],
        },
    ),
    enchant(
        7782,
        "Enchant Bracer - Minor Strength",
        &[(STRANGE_DUST, 5)],
        95,
        (115, 155),
        &Enchant {
            id: 248,
            name: "Strength +1",
            slot: Slot::Wrist,
            bonuses: &[Bonus {
                stat: StatKey::Damage,
                value: 0.005,
                school: None,
            }],
        },
    ),
    enchant(
        7788,
        "Enchant Weapon - Minor Striking",
        &[
            (STRANGE_DUST, 2),
            (GREATER_MAGIC_ESSENCE, 1),
            (SMALL_GLIMMERING_SHARD, 1),
        ],
        100,
        (120, 160),
        &Enchant {
            id: 250,
            name: "Weapon Damage +1 ",
            slot: Slot::MainHand,
            bonuses: &[Bonus {
                stat: StatKey::Damage,
                value: 0.01,
                school: None,
            }],
        },
    ),
    enchant(
        7857,
        "Enchant Chest - Health",
        &[(STRANGE_DUST, 4), (LESSER_ASTRAL_ESSENCE, 1)],
        125,
        (145, 185),
        &Enchant {
            id: 254,
            name: "Health +25",
            slot: Slot::Chest,
            bonuses: &[Bonus {
                stat: StatKey::MaxHealth,
                value: 8.0,
                school: None,
            }],
        },
    ),
    enchant(
        7863,
        "Enchant Boots - Minor Stamina",
        &[(STRANGE_DUST, 8)],
        125,
        (150, 190),
        &Enchant {
            id: 66,
            name: "Stamina +1",
            slot: Slot::Feet,
            bonuses: &[Bonus {
                stat: StatKey::MaxHealth,
                value: 2.0,
                school: None,
            }],
        },
    ),
    enchant(
        7867,
        "Enchant Boots - Minor Agility",
        &[(STRANGE_DUST, 6), (LESSER_ASTRAL_ESSENCE, 2)],
        125,
        (150, 190),
        &Enchant {
            id: 247,
            name: "Agility +1",
            slot: Slot::Feet,
            bonuses: &[Bonus {
                stat: StatKey::CritChance,
                value: 0.005,
                school: None,
            }],
        },
    ),
    enchant(
        13501,
        "Enchant Bracer - Lesser Stamina",
        &[(SOUL_DUST, 2)],
        135,
        (155, 195),
        &Enchant {
            id: 724,
            name: "Stamina +3",
            slot: Slot::Wrist,
            bonuses: &[Bonus {
                stat: StatKey::MaxHealth,
                value: 5.0,
                school: None,
            }],
        },
    ),
    enchant(
        13503,
        "Enchant Weapon - Lesser Striking",
        &[(SOUL_DUST, 2), (LARGE_GLIMMERING_SHARD, 1)],
        145,
        (165, 205),
        &Enchant {
            id: 241,
            name: "Weapon Damage +2",
            slot: Slot::MainHand,
            bonuses: &[Bonus {
                stat: StatKey::Damage,
                value: 0.015,
                school: None,
            }],
        },
    ),
    enchant(
        13536,
        "Enchant Bracer - Lesser Strength",
        &[(SOUL_DUST, 2)],
        145,
        (165, 205),
        &Enchant {
            id: 823,
            name: "Strength +3",
            slot: Slot::Wrist,
            bonuses: &[Bonus {
                stat: StatKey::Damage,
                value: 0.01,
                school: None,
            }],
        },
    ),
    enchant(
        13626,
        "Enchant Chest - Minor Stats",
        &[
            (GREATER_ASTRAL_ESSENCE, 1),
            (SOUL_DUST, 1),
            (LARGE_GLIMMERING_SHARD, 1),
        ],
        150,
        (175, 215),
        &Enchant {
            id: 847,
            name: "All Stats +1",
            slot: Slot::Chest,
            bonuses: &[
                Bonus {
                    stat: StatKey::Damage,
                    value: 0.005,
                    school: None,
                },
                Bonus {
                    stat: StatKey::CritChance,
                    value: 0.005,
                    school: None,
                },
                Bonus {
                    stat: StatKey::MaxHealth,
                    value: 2.0,
                    school: None,
                },
            ],
        },
    )
    .bought(2500),
    enchant(
        13637,
        "Enchant Boots - Lesser Agility",
        &[(SOUL_DUST, 1), (LESSER_MYSTIC_ESSENCE, 1)],
        160,
        (180, 220),
        &Enchant {
            id: 849,
            name: "Agility +3",
            slot: Slot::Feet,
            bonuses: &[Bonus {
                stat: StatKey::CritChance,
                value: 0.01,
                school: None,
            }],
        },
    ),
    enchant(
        13640,
        "Enchant Chest - Greater Health",
        &[(SOUL_DUST, 3)],
        160,
        (180, 220),
        &Enchant {
            id: 850,
            name: "Health +35",
            slot: Slot::Chest,
            bonuses: &[Bonus {
                stat: StatKey::MaxHealth,
                value: 12.0,
                school: None,
            }],
        },
    ),
    enchant(
        13644,
        "Enchant Boots - Lesser Stamina",
        &[(SOUL_DUST, 4)],
        170,
        (190, 230),
        &Enchant {
            id: 724,
            name: "Stamina +3",
            slot: Slot::Feet,
            bonuses: &[Bonus {
                stat: StatKey::MaxHealth,
                value: 5.0,
                school: None,
            }],
        },
    ),
    enchant(
        13648,
        "Enchant Bracer - Stamina",
        &[(SOUL_DUST, 6)],
        170,
        (190, 230),
        &Enchant {
            id: 852,
            name: "Stamina +5",
            slot: Slot::Wrist,
            bonuses: &[Bonus {
                stat: StatKey::MaxHealth,
                value: 8.0,
                school: None,
            }],
        },
    ),
    enchant(
        13661,
        "Enchant Bracer - Strength",
        &[(VISION_DUST, 1)],
        180,
        (200, 240),
        &Enchant {
            id: 856,
            name: "Strength +5",
            slot: Slot::Wrist,
            bonuses: &[Bonus {
                stat: StatKey::Damage,
                value: 0.015,
                school: None,
            }],
        },
    ),
    enchant(
        13693,
        "Enchant Weapon - Striking",
        &[(GREATER_MYSTIC_ESSENCE, 2), (LARGE_GLOWING_SHARD, 1)],
        195,
        (215, 255),
        &Enchant {
            id: 943,
            name: "Weapon Damage +3",
            slot: Slot::MainHand,
            bonuses: &[Bonus {
                stat: StatKey::Damage,
                value: 0.02,
                school: None,
            }],
        },
    ),
    enchant(
        13700,
        "Enchant Chest - Lesser Stats",
        &[
            (GREATER_MYSTIC_ESSENCE, 2),
            (VISION_DUST, 2),
            (LARGE_GLOWING_SHARD, 1),
        ],
        200,
        (220, 260),
        &Enchant {
            id: 866,
            name: "All Stats +2",
            slot: Slot::Chest,
            bonuses: &[
                Bonus {
                    stat: StatKey::Damage,
                    value: 0.01,
                    school: None,
                },
                Bonus {
                    stat: StatKey::CritChance,
                    value: 0.01,
                    school: None,
                },
                Bonus {
                    stat: StatKey::MaxHealth,
                    value: 4.0,
                    school: None,
                },
            ],
        },
    )
    .bought(5000),
    enchant(
        13815,
        "Enchant Gloves - Agility",
        &[(LESSER_NETHER_ESSENCE, 1), (VISION_DUST, 1)],
        210,
        (230, 270),
        &Enchant {
            id: 904,
            name: "Agility +5",
            slot: Slot::Hands,
            bonuses: &[Bonus {
                stat: StatKey::CritChance,
                value: 0.015,
                school: None,
            }],
        },
    ),
    enchant(
        13836,
        "Enchant Boots - Stamina",
        &[(VISION_DUST, 5)],
        215,
        (235, 275),
        &Enchant {
            id: 852,
            name: "Stamina +5",
            slot: Slot::Feet,
            bonuses: &[Bonus {
                stat: StatKey::MaxHealth,
                value: 8.0,
                school: None,
            }],
        },
    ),
    enchant(
        13858,
        "Enchant Chest - Superior Health",
        &[(VISION_DUST, 6)],
        220,
        (240, 280),
        &Enchant {
            id: 908,
            name: "Health +50",
            slot: Slot::Chest,
            bonuses: &[Bonus {
                stat: StatKey::MaxHealth,
                value: 16.0,
                school: None,
            }],
        },
    ),
    enchant(
        13887,
        "Enchant Gloves - Strength",
        &[(LESSER_NETHER_ESSENCE, 2), (VISION_DUST, 3)],
        225,
        (245, 285),
        &Enchant {
            id: 856,
            name: "Strength +5",
            slot: Slot::Hands,
            bonuses: &[Bonus {
                stat: StatKey::Damage,
                value: 0.015,
                school: None,
            }],
        },
    ),
    enchant(
        13935,
        "Enchant Boots - Agility",
        &[(GREATER_NETHER_ESSENCE, 2)],
        235,
        (255, 295),
        &Enchant {
            id: 904,
            name: "Agility +5",
            slot: Slot::Feet,
            bonuses: &[Bonus {
                stat: StatKey::CritChance,
                value: 0.015,
                school: None,
            }],
        },
    ),
    enchant(
        13939,
        "Enchant Bracer - Greater Strength",
        &[(DREAM_DUST, 2), (GREATER_NETHER_ESSENCE, 1)],
        240,
        (260, 300),
        &Enchant {
            id: 927,
            name: "Strength +7",
            slot: Slot::Wrist,
            bonuses: &[Bonus {
                stat: StatKey::Damage,
                value: 0.02,
                school: None,
            }],
        },
    ),
    enchant(
        13941,
        "Enchant Chest - Stats",
        &[
            (LARGE_RADIANT_SHARD, 1),
            (DREAM_DUST, 3),
            (GREATER_NETHER_ESSENCE, 2),
        ],
        245,
        (265, 305),
        &Enchant {
            id: 928,
            name: "All Stats +3",
            slot: Slot::Chest,
            bonuses: &[
                Bonus {
                    stat: StatKey::Damage,
                    value: 0.015,
                    school: None,
                },
                Bonus {
                    stat: StatKey::CritChance,
                    value: 0.015,
                    school: None,
                },
                Bonus {
                    stat: StatKey::MaxHealth,
                    value: 6.0,
                    school: None,
                },
            ],
        },
    )
    .bought(10000),
    enchant(
        13943,
        "Enchant Weapon - Greater Striking",
        &[(LARGE_RADIANT_SHARD, 2), (GREATER_NETHER_ESSENCE, 2)],
        245,
        (265, 305),
        &Enchant {
            id: 805,
            name: "Weapon Damage +4",
            slot: Slot::MainHand,
            bonuses: &[Bonus {
                stat: StatKey::Damage,
                value: 0.03,
                school: None,
            }],
        },
    ),
    enchant(
        13945,
        "Enchant Bracer - Greater Stamina",
        &[(DREAM_DUST, 5)],
        245,
        (265, 305),
        &Enchant {
            id: 929,
            name: "Stamina +7",
            slot: Slot::Wrist,
            bonuses: &[Bonus {
                stat: StatKey::MaxHealth,
                value: 12.0,
                school: None,
            }],
        },
    ),
    enchant(
        20020,
        "Enchant Boots - Greater Stamina",
        &[(DREAM_DUST, 10)],
        260,
        (280, 320),
        &Enchant {
            id: 929,
            name: "Stamina +7",
            slot: Slot::Feet,
            bonuses: &[Bonus {
                stat: StatKey::MaxHealth,
                value: 12.0,
                school: None,
            }],
        },
    ),
    enchant(
        13898,
        "Enchant Weapon - Fiery Weapon",
        &[(SMALL_RADIANT_SHARD, 4), (ESSENCE_OF_FIRE, 1)],
        265,
        (285, 325),
        &Enchant {
            id: 803,
            name: "Fiery Weapon",
            slot: Slot::MainHand,
            bonuses: &[
                Bonus {
                    stat: StatKey::Damage,
                    value: 0.02,
                    school: None,
                },
                Bonus {
                    stat: StatKey::Damage,
                    value: 0.05,
                    school: Some(School::Fire),
                },
            ],
        },
    )
    .bought(20000),
    enchant(
        20012,
        "Enchant Gloves - Greater Agility",
        &[(LESSER_ETERNAL_ESSENCE, 3), (ILLUSION_DUST, 3)],
        270,
        (290, 330),
        &Enchant {
            id: 1887,
            name: "Agility +7",
            slot: Slot::Hands,
            bonuses: &[Bonus {
                stat: StatKey::CritChance,
                value: 0.02,
                school: None,
            }],
        },
    )
    .bought(15000),
    enchant(
        20026,
        "Enchant Chest - Major Health",
        &[(ILLUSION_DUST, 6), (SMALL_BRILLIANT_SHARD, 1)],
        275,
        (295, 335),
        &Enchant {
            id: 1892,
            name: "Health +100",
            slot: Slot::Chest,
            bonuses: &[Bonus {
                stat: StatKey::MaxHealth,
                value: 24.0,
                school: None,
            }],
        },
    )
    .bought(15000),
    enchant(
        20010,
        "Enchant Bracer - Superior Strength",
        &[(ILLUSION_DUST, 6), (GREATER_ETERNAL_ESSENCE, 6)],
        295,
        (315, 355),
        &Enchant {
            id: 1885,
            name: "Strength +9",
            slot: Slot::Wrist,
            bonuses: &[Bonus {
                stat: StatKey::Damage,
                value: 0.03,
                school: None,
            }],
        },
    )
    .bought(20000),
    enchant(
        20013,
        "Enchant Gloves - Greater Strength",
        &[(GREATER_ETERNAL_ESSENCE, 4), (ILLUSION_DUST, 4)],
        295,
        (315, 355),
        &Enchant {
            id: 927,
            name: "Strength +7",
            slot: Slot::Hands,
            bonuses: &[Bonus {
                stat: StatKey::Damage,
                value: 0.02,
                school: None,
            }],
        },
    )
    .bought(20000),
    enchant(
        20023,
        "Enchant Boots - Greater Agility",
        &[(GREATER_ETERNAL_ESSENCE, 8)],
        295,
        (315, 355),
        &Enchant {
            id: 1887,
            name: "Agility +7",
            slot: Slot::Feet,
            bonuses: &[Bonus {
                stat: StatKey::CritChance,
                value: 0.02,
                school: None,
            }],
        },
    )
    .bought(20000),
    enchant(
        20031,
        "Enchant Weapon - Superior Striking",
        &[(LARGE_BRILLIANT_SHARD, 2), (GREATER_ETERNAL_ESSENCE, 10)],
        300,
        (320, 360),
        &Enchant {
            id: 1897,
            name: "Weapon Damage +5",
            slot: Slot::MainHand,
            bonuses: &[Bonus {
                stat: StatKey::Damage,
                value: 0.04,
                school: None,
            }],
        },
    )
    .bought(25000),
    enchant(
        20025,
        "Enchant Chest - Greater Stats",
        &[
            (LARGE_BRILLIANT_SHARD, 4),
            (ILLUSION_DUST, 15),
            (GREATER_ETERNAL_ESSENCE, 10),
        ],
        300,
        (320, 360),
        &Enchant {
            id: 1891,
            name: "All Stats +4",
            slot: Slot::Chest,
            bonuses: &[
                Bonus {
                    stat: StatKey::Damage,
                    value: 0.02,
                    school: None,
                },
                Bonus {
                    stat: StatKey::CritChance,
                    value: 0.02,
                    school: None,
                },
                Bonus {
                    stat: StatKey::MaxHealth,
                    value: 8.0,
                    school: None,
                },
            ],
        },
    )
    .bought(30000),
    enchant(
        20034,
        "Enchant Weapon - Crusader",
        &[(LARGE_BRILLIANT_SHARD, 4), (RIGHTEOUS_ORB, 2)],
        300,
        (320, 360),
        &Enchant {
            id: 1900,
            name: "Crusader",
            slot: Slot::MainHand,
            bonuses: &[
                Bonus {
                    stat: StatKey::Damage,
                    value: 0.05,
                    school: None,
                },
                Bonus {
                    stat: StatKey::Regen,
                    value: 1.0,
                    school: None,
                },
            ],
        },
    )
    .dropped(FORMULA_ENCHANT_WEAPON_CRUSADER),
];

/// `line`'s book, in the order its trainer teaches it; empty for a gathering line without one.
pub(crate) fn book(line: Line) -> &'static [Recipe] {
    match line {
        Line::Mining => SMELTING,
        Line::Alchemy => ALCHEMY,
        Line::Cooking => COOKING,
        Line::FirstAid => FIRST_AID,
        Line::Enchanting => ENCHANTING,
        Line::Herbalism | Line::Skinning | Line::Fishing => &[],
    }
}

/// Every recipe of every book.
pub(crate) fn all() -> impl Iterator<Item = &'static Recipe> {
    Line::ALL.into_iter().flat_map(book)
}

/// The recipe a `DoTradeSkill` or `DoCraft` names.
pub(crate) fn recipe(spell: u32) -> Option<&'static Recipe> {
    all().find(|r| r.spell == spell)
}

/// The recipe a recipe item teaches.
pub(crate) fn taught_by(item: u32) -> Option<&'static Recipe> {
    all().find(|r| r.learn == Learn::Drop(item))
}

/// The enchant with `SpellItemEnchantment` id `id`.
pub(crate) fn enchant_by_id(id: u32) -> Option<&'static Enchant> {
    ENCHANTING
        .iter()
        .filter_map(|r| r.enchant)
        .find(|e| e.id == id)
}

/// An enchant as the tooltip's green line reads: its name, and what it does here.
pub(crate) fn enchant_line(e: &Enchant) -> String {
    let what: Vec<String> = e.bonuses.iter().map(Bonus::describe).collect();
    format!("{} ({})", e.name, what.join(", "))
}

/// Whether the player knows `r`: a skill recipe once the skill reaches it, a bought or dropped one
/// once learned.
pub(crate) fn known(r: &Recipe, skills: &Skills) -> bool {
    match r.learn {
        Learn::Skill => skills.rank(r.line).rank >= r.req,
        Learn::Trainer(_) | Learn::Drop(_) => skills.learned(r.spell),
    }
}

/// Whether the player may learn `r` now: unknown, and the skill there.
pub(crate) fn learnable(r: &Recipe, skills: &Skills) -> bool {
    !known(r, skills) && skills.rank(r.line).rank >= r.req
}

/// One cast: the reagents out of the bags and the product in, or nothing at all.
pub(crate) fn craft_once(bags: &mut Bags, r: &Recipe) -> Result<(), BagError> {
    if r.product == 0 {
        return bags.exchange(r.reagents, &[]);
    }
    bags.exchange(r.reagents, &[(r.product, r.made)])
}

/// How many casts the bags can feed.
pub(crate) fn available(bags: &Bags, r: &Recipe) -> u32 {
    r.reagents
        .iter()
        .map(|&(item, need)| bags.count(item) / need.max(1))
        .min()
        .unwrap_or(0)
}

/// The stock tradeskill window's snapshot of `line`'s book: every recipe the player knows, its
/// colour on `ui_tradeskill::difficulty`'s bands, its reagents counted in the bags, and the
/// focus the camp's fire meets.
pub(crate) fn trade_skill_state(
    bags: &Bags,
    skills: &Skills,
    line: Line,
    repeat: u32,
) -> TradeSkillState {
    let r = skills.rank(line);
    let recipes = book(line)
        .iter()
        .filter(|rec| known(rec, skills))
        .map(|rec| {
            let product = items::template(rec.product);
            TradeSkillRecipe {
                // The product's class is the header: Consumable or Trade Goods.
                group: product.map(|p| {
                    let name = if p.class == 0 {
                        "Consumable"
                    } else {
                        "Trade Goods"
                    };
                    (p.class, 0, name.to_string())
                }),
                spell_id: rec.spell,
                name: rec.name.to_string(),
                difficulty: crate::ui_tradeskill::difficulty(
                    u32::from(r.rank),
                    u32::from(rec.trivial.0),
                    u32::from(rec.trivial.1),
                ),
                num_available: available(bags, rec),
                icon: product.map(items::icon_path),
                min_made: rec.made,
                max_made: rec.made,
                cooldown_secs: None,
                product_item: rec.product,
                product_inv_type: 0,
                // No item levels are authored: the skill orders the list.
                product_item_level: u32::from(rec.req),
                reagents: rec
                    .reagents
                    .iter()
                    .map(|&(item, need)| {
                        let def = items::template(item);
                        TradeSkillReagent {
                            item,
                            name: def.map(|d| d.name.to_string()),
                            icon: def.map(items::icon_path),
                            need,
                            have: bags.count(item),
                        }
                    })
                    .collect(),
                tools: rec
                    .focus
                    .map(|(_, name)| vec![(name.to_string(), true)])
                    .unwrap_or_default(),
            }
        })
        .collect();
    TradeSkillState {
        line: line.skill_line(),
        line_name: line.name().to_string(),
        rank: u32::from(r.rank),
        max_rank: u32::from(r.max),
        recipes,
        repeat_count: repeat,
    }
}

/// The stock craft window's snapshot of Enchanting: every enchant the player knows, its colour,
/// its reagents, and what it does to the piece it goes on.
pub(crate) fn craft_state(bags: &Bags, skills: &Skills) -> CraftState {
    let r = skills.rank(Line::Enchanting);
    let recipes = ENCHANTING
        .iter()
        .filter(|rec| known(rec, skills))
        .filter_map(|rec| {
            let e = rec.enchant?;
            Some(CraftRecipe {
                spell_id: rec.spell,
                name: rec.name.to_string(),
                sub_name: String::new(),
                difficulty: crate::ui_tradeskill::difficulty(
                    u32::from(r.rank),
                    u32::from(rec.trivial.0),
                    u32::from(rec.trivial.1),
                ),
                num_available: available(bags, rec),
                icon: Some(ENCHANT_ICON.to_string()),
                description: Some(format!(
                    "Permanently enchant the {} piece you wear: {}. It takes hold from your next \
                     run.",
                    e.slot.label().to_lowercase(),
                    enchant_line(e)
                )),
                needs_item_target: false,
                reagents: rec
                    .reagents
                    .iter()
                    .map(|&(item, need)| {
                        let def = items::template(item);
                        CraftReagent {
                            item,
                            name: def.map(|d| d.name.to_string()),
                            icon: def.map(items::icon_path),
                            need,
                            have: bags.count(item),
                        }
                    })
                    .collect(),
                tools: Vec::new(),
                tooltip: CraftTooltip::Spell(rec.spell),
                spell_level: 0,
            })
        })
        .collect();
    CraftState {
        name: Line::Enchanting.name().to_string(),
        rank: u32::from(r.rank),
        max_rank: u32::from(r.max),
        craft_type: CRAFT_TYPE_ENCHANTING,
        recipes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Smelting` (2656), the tradeskill opener whose window lists Mining's book.
    const SMELTING_SPELL: u32 = 2656;
    /// `Disenchant` (13262), Enchanting's.
    const DISENCHANT_SPELL: u32 = 13262;
    /// `SPELL_EFFECT_CREATE_ITEM` and `SPELL_EFFECT_ENCHANT_ITEM` (permanent).
    const CREATE_ITEM: u32 = 24;
    const ENCHANT_ITEM: u32 = 53;

    #[test]
    fn survivors_recipes_every_item_is_authored() {
        let mut spells = std::collections::HashSet::new();
        for r in all() {
            assert!(spells.insert(r.spell), "{} listed twice", r.name);
            match r.enchant {
                Some(e) => {
                    assert_eq!((r.product, r.line), (0, Line::Enchanting), "{}", r.name);
                    assert!(!e.bonuses.is_empty(), "{}", r.name);
                    assert!(
                        !matches!(e.slot, Slot::Head | Slot::Legs),
                        "{}: no enchant on head or legs, as in 1.12",
                        r.name
                    );
                }
                None => assert!(items::template(r.product).is_some(), "{}", r.name),
            }
            for &(item, n) in r.reagents {
                assert!(items::template(item).is_some() && n >= 1, "{}", r.name);
            }
            assert!(r.trivial.0 <= r.trivial.1, "{}", r.name);
            assert!(r.req >= 1 && r.req <= 300, "{}", r.name);
            if let Learn::Drop(item) = r.learn {
                let def = items::template(item).expect("the recipe item is authored");
                assert_eq!(def.class, 9, "{}", r.name);
                assert_eq!(taught_by(item), Some(r));
            }
        }
        for line in [
            Line::Alchemy,
            Line::Cooking,
            Line::FirstAid,
            Line::Enchanting,
        ] {
            let n = book(line).len();
            assert!((10..=45).contains(&n), "{}: {n} recipes", line.name());
            assert!(
                book(line)
                    .iter()
                    .any(|r| r.req == 1 && r.learn == Learn::Skill),
                "{} starts with a recipe at 1",
                line.name()
            );
        }
    }

    /// A Miner sees only what they know; a cast takes the ore and makes the bar.
    #[test]
    fn survivors_recipes_smelt_copper_and_bronze() {
        let mut bags = Bags::default();
        bags.add(COPPER_ORE, 3);
        bags.add(TIN_ORE, 1);
        let skills = Skills::default();
        let book = trade_skill_state(&bags, &skills, Line::Mining, 0);
        assert_eq!(book.line, 186);
        assert_eq!(book.max_rank, 75);
        assert_eq!(book.recipes.len(), 1, "rank 1 knows Smelt Copper only");
        assert_eq!(book.recipes[0].num_available, 3);
        assert_eq!(book.recipes[0].tools, vec![("Forge".to_string(), true)]);
        assert_eq!(
            book.recipes[0].difficulty,
            benilla_ui::script::TradeSkillDifficulty::Optimal
        );
        let copper = recipe(2657).unwrap();
        craft_once(&mut bags, copper).unwrap();
        assert_eq!((bags.count(COPPER_ORE), bags.count(COPPER_BAR)), (2, 1));
        let bronze = recipe(2659).unwrap();
        assert_eq!(available(&bags, bronze), 0, "no Tin Bar yet");
        assert_eq!(craft_once(&mut bags, bronze), Err(BagError::NotFound));
        craft_once(&mut bags, recipe(3304).unwrap()).unwrap();
        assert_eq!(available(&bags, bronze), 1);
        craft_once(&mut bags, bronze).unwrap();
        assert_eq!(bags.count(BRONZE_BAR), 2, "two bars a cast");
        assert_eq!((bags.count(COPPER_BAR), bags.count(TIN_BAR)), (0, 0));
    }

    /// A skill recipe is known at its skill; a trainer's or a dropped one only once learned, and
    /// only learnable at its skill.
    #[test]
    fn survivors_recipes_learning() {
        use std::collections::BTreeMap;
        let fresh = Skills::default();
        let minor_healing = recipe(2330).unwrap();
        assert!(known(minor_healing, &fresh));
        let blackmouth = recipe(7836).unwrap();
        assert_eq!(blackmouth.learn, Learn::Trainer(400));
        assert!(!known(blackmouth, &fresh) && !learnable(blackmouth, &fresh));
        let mut alchemist = Skills::from_save(&BTreeMap::from([(
            "alchemy".to_string(),
            super::super::skills::Rank { rank: 90, max: 150 },
        )]));
        assert!(learnable(blackmouth, &alchemist));
        assert!(alchemist.learn(blackmouth.spell));
        assert!(known(blackmouth, &alchemist) && !learnable(blackmouth, &alchemist));
        let titans = recipe(17635).unwrap();
        assert_eq!(taught_by(RECIPE_FLASK_OF_THE_TITANS), Some(titans));
        assert!(!known(titans, &alchemist));
        // The window lists what is known, each against the bags.
        let mut bags = Bags::default();
        bags.add(PEACEBLOOM, 4);
        bags.add(SILVERLEAF, 4);
        bags.add(EMPTY_VIAL, 2);
        let window = trade_skill_state(&bags, &alchemist, Line::Alchemy, 0);
        let row = window.recipes.iter().find(|r| r.spell_id == 2330).unwrap();
        assert_eq!(row.num_available, 2, "two vials");
        assert_eq!(row.group.as_ref().map(|g| g.2.as_str()), Some("Consumable"));
        assert!(window.recipes.iter().any(|r| r.spell_id == 7836));
        assert!(!window.recipes.iter().any(|r| r.spell_id == 17635));
    }

    /// Enchanting's window: the known enchants, each describing what it does to its slot.
    #[test]
    fn survivors_recipes_enchants() {
        let crusader = enchant_by_id(1900).expect("Crusader");
        assert_eq!(crusader.slot, Slot::MainHand);
        assert_eq!(
            enchant_line(crusader),
            "Crusader (+5% damage, +1 health per second)"
        );
        let mut bags = Bags::default();
        bags.add(STRANGE_DUST, 3);
        let state = craft_state(&bags, &Skills::default());
        assert_eq!(state.name, "Enchanting");
        let minor = state
            .recipes
            .iter()
            .find(|r| r.spell_id == 7418)
            .expect("Minor Health at 1");
        assert_eq!(minor.num_available, 3);
        assert!(minor.description.as_deref().unwrap().contains("wrist"));
        assert!(
            !state.recipes.iter().any(|r| r.spell_id == 20034),
            "Crusader is a formula"
        );
        let mut bags = Bags::default();
        bags.add(STRANGE_DUST, 1);
        craft_once(&mut bags, recipe(7418).unwrap()).unwrap();
        assert_eq!(bags.count(STRANGE_DUST), 0, "an enchant makes no item");
    }

    /// The enchant budget (6.1): the best enchant in each slot together come to about +15-20%
    /// output (damage, crit and cooldowns), never more.
    #[test]
    fn survivors_recipes_the_enchant_budget() {
        let output = |e: &Enchant| -> f32 {
            e.bonuses
                .iter()
                .filter(|b| {
                    matches!(
                        b.stat,
                        StatKey::Damage | StatKey::CritChance | StatKey::Cooldown
                    )
                })
                .map(|b| b.value)
                .sum()
        };
        let total: f32 = Slot::ALL
            .iter()
            .map(|&slot| {
                book(Line::Enchanting)
                    .iter()
                    .filter_map(|r| r.enchant)
                    .filter(|e| e.slot == slot)
                    .map(output)
                    .fold(0.0, f32::max)
            })
            .sum();
        assert!((0.12..=0.20).contains(&total), "a full set: {total}");
    }

    /// The table is closed (2.6): every authored item is a reagent or product of a recipe, a node's
    /// or a kill's or a catch's yield, a disenchant's material, a rare drop, the goblin's goods, a
    /// consumable, or the test inventory's.
    #[test]
    fn survivors_recipes_every_item_earns_its_place() {
        use super::super::{disenchant, econ, effects, nodes};
        let mut used: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for r in all() {
            used.insert(r.product);
            used.extend(r.reagents.iter().map(|&(i, _)| i));
            if let Learn::Drop(item) = r.learn {
                used.insert(item);
            }
        }
        for n in nodes::NODES {
            used.insert(n.item);
            used.extend(n.extra.map(|(i, _)| i));
        }
        for level in 0..=63 {
            used.insert(nodes::skin(level).0);
            used.insert(nodes::meat(level));
            used.insert(nodes::cloth(level));
        }
        for band in [
            nodes::Band::B1,
            nodes::Band::B15,
            nodes::Band::B30,
            nodes::Band::B45,
            nodes::Band::B55,
            nodes::Band::Raid,
        ] {
            used.insert(nodes::camp_fish(band).0);
            used.extend(nodes::cooking_fish(band));
            used.extend(nodes::rare_drops(band).iter().map(|&(i, _)| i));
            let t = disenchant::tier(band);
            used.extend(t.dust.iter().chain(t.essence).chain(t.shard).copied());
            used.insert(t.best);
        }
        used.extend(econ::GOBLIN_WARES.iter().map(|&(i, _)| i));
        used.extend(effects::CONSUMABLES.iter().map(|c| c.item));
        used.extend(items::TEST_INVENTORY.iter().map(|&(i, _)| i));
        let idle: Vec<&str> = items::ITEMS
            .iter()
            .filter(|d| !used.contains(&d.entry))
            .map(|d| d.name)
            .collect();
        assert!(idle.is_empty(), "nothing uses {idle:?}");
    }

    /// The recipes are real: each spell is its line's, with these reagents, this product and count
    /// (or this enchant), these trivial bands, this focus and this cast; the openers are Mining's
    /// Smelting and Enchanting's Disenchant. Every mismatch is listed.
    #[test]
    fn every_recipe_resolves_in_the_client() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open the MPQ chain");
        let spells = benilla_formats::load_spell_catalog(&mut chain).expect("Spell.dbc");
        let skills = benilla_formats::load_skill_line_catalog(&mut chain).expect("SkillLine");
        let casts = benilla_formats::load_spell_cast_times(&mut chain).expect("cast times");
        let focus = benilla_formats::load_spell_focus_catalog(&mut chain).expect("focus");
        let enchants = benilla_formats::load_enchant_catalog(&mut chain).expect("enchants");
        let mut bad = Vec::new();
        let reagents = |s: &benilla_formats::SpellDisplay| -> Vec<(u32, u32)> {
            s.reagents.iter().copied().filter(|r| r.0 != 0).collect()
        };
        for r in all() {
            let Some(s) = spells.get(r.spell) else {
                bad.push(format!("{}: no spell {}", r.name, r.spell));
                continue;
            };
            let sla = skills.ability(r.spell);
            let cast = casts.get(s.casting_time_index).map(|c| c.base_ms);
            let made = match r.enchant {
                Some(e) => {
                    s.effects[0] == ENCHANT_ITEM
                        && s.effect_misc_value[0] == e.id as i32
                        && enchants.name(e.id) == Some(e.name)
                }
                None => {
                    s.effects[0] == CREATE_ITEM
                        && s.effect_item_type[0] == r.product
                        && s.effect_base_points[0] + s.effect_die_sides[0] == r.made as i32
                }
            };
            let ok = s.name == r.name
                && reagents(s) == r.reagents
                && made
                && sla.is_some_and(|a| {
                    a.skill_id == r.line.skill_line()
                        && (a.trivial_low, a.trivial_high)
                            == (u32::from(r.trivial.0), u32::from(r.trivial.1))
                })
                && s.requires_spell_focus == r.focus.map_or(0, |f| f.0)
                && cast == Some(r.cast_ms as i32);
            if !ok {
                bad.push(format!(
                    "{}: {:?} reagents {:?} effects {:?} makes {:?} misc {:?} sla {sla:?} focus \
                     {} cast {cast:?}",
                    r.name,
                    s.name,
                    reagents(s),
                    s.effects,
                    s.effect_item_type,
                    s.effect_misc_value,
                    s.requires_spell_focus
                ));
            }
        }
        for (id, name) in [FORGE, COOKING_FIRE, ALCHEMY_LAB] {
            if focus.name(id) != Some(name) {
                bad.push(format!("focus {id} is {:?}, not {name}", focus.name(id)));
            }
        }
        for (spell, name, line) in [
            (SMELTING_SPELL, "Smelting", Line::Mining),
            (DISENCHANT_SPELL, "Disenchant", Line::Enchanting),
        ] {
            let found = spells.get(spell).map(|s| s.name.as_str());
            if found != Some(name) || skills.spell_to_line(spell) != Some(line.skill_line()) {
                bad.push(format!("{name}: spell {spell} is {found:?}"));
            }
        }
        if !spells
            .get(ENCHANTING[0].spell)
            .and_then(|s| s.icon.as_deref())
            .is_some_and(|i| i.eq_ignore_ascii_case(ENCHANT_ICON))
        {
            bad.push(format!("the enchants' icon is not {ENCHANT_ICON}"));
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }
}
