//! The curated recipe books (docs/PROFESSIONS.md, 5.1): Mining's Smelting, P2's Alchemy, Cooking,
//! First Aid and Enchanting, and P3's Blacksmithing, Leatherworking, Tailoring and Engineering,
//! done at the camp's fire. Each row is a real 1.12 tradeskill spell: its `Reagent[8]`, its
//! `SPELL_EFFECT_CREATE_ITEM` product and count (or, for an enchant, its
//! `SPELL_EFFECT_ENCHANT_ITEM_PERMANENT` and the `SpellItemEnchantment` row it names), its
//! `SkillLineAbility` line and trivial bands, its spell focus and its cast time are checked against
//! the client by [`tests::every_recipe_resolves_in_the_client`]. A recipe that drops is read off
//! a recipe item whose name is the client's own teaching spell's
//! ([`tests::recipe_items_teach_their_recipes_in_the_client`]).
//!
//! **Authored:** the skill each is learned at (the trainer's `reqskillvalue`, from memory: the
//! client does not carry it), how it is learned ([`Learn`]: at that skill, bought from the camp's
//! trainer, or read off a recipe that drops in runs), the trainer's prices, Engineering's
//! specialisations ([`Spec`]: which gadgets each keeps to itself follows the design doc, not 1.12),
//! and each enchant's survivors bonuses ([`Enchant`]).
//!
//! **The camp's fire** stands in for every focus a book names: Smelting's Forge and Black Forge,
//! Cooking's Cooking Fire, the flasks' Alchemy Lab, the smiths' Anvil and Black Anvil, and
//! Mooncloth's Moonwell.

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
/// Blacksmithing's and Engineering's focus.
pub(crate) const ANVIL: Focus = (1, "Anvil");
/// Molten Core's plate: Blackrock Depths' anvil.
pub(crate) const BLACK_ANVIL: Focus = (623, "Black Anvil");
/// Smelt Dark Iron's.
pub(crate) const BLACK_FORGE: Focus = (543, "Black Forge");
/// Mooncloth's.
pub(crate) const MOONWELL: Focus = (883, "Moonwell");
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

/// Engineering's 1.12 specialisation, learned from the trainer at Engineering 200: the two are
/// exclusive. The gadgets each keeps to itself are the design doc's (5.2), not 1.12's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Spec {
    Gnomish,
    Goblin,
}

impl Spec {
    pub(crate) const ALL: [Spec; 2] = [Spec::Gnomish, Spec::Goblin];

    /// The specialisation's spell (`Spell.dbc`, Engineering's line): "Gnomish Engineer" and
    /// "Goblin Engineer".
    pub(crate) const fn spell(self) -> u32 {
        match self {
            Spec::Gnomish => 20219,
            Spec::Goblin => 20222,
        }
    }

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Spec::Gnomish => "Gnomish Engineer",
            Spec::Goblin => "Goblin Engineer",
        }
    }

    /// The spells' icons (`SpellIcon.dbc`).
    pub(crate) const fn icon(self) -> &'static str {
        match self {
            Spec::Gnomish => "Interface\\Icons\\INV_Gizmo_02",
            Spec::Goblin => "Interface\\Icons\\Spell_Fire_SelfDestruct",
        }
    }

    /// The other one, which learning this rules out.
    pub(crate) const fn rival(self) -> Spec {
        match self {
            Spec::Gnomish => Spec::Goblin,
            Spec::Goblin => Spec::Gnomish,
        }
    }

    /// The specialisation `spell` is, if it is one.
    pub(crate) fn of_spell(spell: u32) -> Option<Spec> {
        Spec::ALL.into_iter().find(|s| s.spell() == spell)
    }
}

/// The Engineering a specialisation needs, and its price: ours, about five runs' gold.
pub(crate) const SPEC_SKILL: u16 = 200;
pub(crate) const SPEC_COST: u32 = 50_000;

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
    /// The Engineering specialisation it needs, if any.
    pub(crate) spec: Option<Spec>,
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

    const fn needs(self, spec: Spec) -> Self {
        Self {
            spec: Some(spec),
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
        spec: None,
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
    base(
        Line::Mining,
        2658,
        "Smelt Silver",
        &[(SILVER_ORE, 1)],
        SILVER_BAR,
        75,
        (100, 125),
    )
    .at(FORGE),
    base(
        Line::Mining,
        3308,
        "Smelt Gold",
        &[(GOLD_ORE, 1)],
        GOLD_BAR,
        155,
        (170, 185),
    )
    .at(FORGE),
    base(
        Line::Mining,
        14891,
        "Smelt Dark Iron",
        &[(DARK_IRON_ORE, 8)],
        DARK_IRON_BAR,
        230,
        (230, 230),
    )
    .at(BLACK_FORGE),
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
    base(
        Line::Alchemy,
        11479,
        "Transmute: Iron to Gold",
        &[(IRON_BAR, 1)],
        GOLD_BAR,
        225,
        (240, 280),
    )
    .casts(25000)
    .bought(6000),
    base(
        Line::Alchemy,
        17187,
        "Transmute: Arcanite",
        &[(THORIUM_BAR, 1), (ARCANE_CRYSTAL, 1)],
        ARCANITE_BAR,
        275,
        (275, 290),
    )
    .casts(25000)
    .dropped(RECIPE_TRANSMUTE_ARCANITE),
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
    base(
        Line::Enchanting,
        17180,
        "Enchanted Thorium",
        &[(THORIUM_BAR, 1), (DREAM_DUST, 3)],
        ENCHANTED_THORIUM_BAR,
        250,
        (250, 260),
    )
    .casts(12500)
    .bought(8000),
    base(
        Line::Enchanting,
        17181,
        "Enchanted Leather",
        &[(RUGGED_LEATHER, 1), (LESSER_ETERNAL_ESSENCE, 1)],
        ENCHANTED_LEATHER,
        250,
        (250, 260),
    )
    .casts(12500)
    .bought(8000),
];

/// Blacksmithing (P3): the stones, the copper-to-thorium ladder, the Imperial Plate, and the
/// recipes that drop: the Lionheart Helm, the Arcanite Reaper and Molten Core's plans.
static BLACKSMITHING: &[Recipe] = &[
    base(
        Line::Blacksmithing,
        2660,
        "Rough Sharpening Stone",
        &[(ROUGH_STONE, 1)],
        ROUGH_SHARPENING_STONE,
        1,
        (15, 55),
    )
    .casts(5125),
    base(
        Line::Blacksmithing,
        3115,
        "Rough Weightstone",
        &[(ROUGH_STONE, 1), (LINEN_CLOTH, 1)],
        ROUGH_WEIGHTSTONE,
        1,
        (15, 55),
    )
    .casts(5125),
    base(
        Line::Blacksmithing,
        2663,
        "Copper Bracers",
        &[(COPPER_BAR, 2)],
        COPPER_BRACERS,
        1,
        (20, 60),
    )
    .casts(5125)
    .at(ANVIL),
    base(
        Line::Blacksmithing,
        3320,
        "Rough Grinding Stone",
        &[(ROUGH_STONE, 2)],
        ROUGH_GRINDING_STONE,
        25,
        (45, 85),
    )
    .casts(5125),
    base(
        Line::Blacksmithing,
        2662,
        "Copper Chain Pants",
        &[(COPPER_BAR, 4)],
        COPPER_CHAIN_PANTS,
        40,
        (50, 90),
    )
    .casts(5125)
    .at(ANVIL),
    base(
        Line::Blacksmithing,
        2737,
        "Copper Mace",
        &[(COPPER_BAR, 6), (WEAK_FLUX, 1), (LINEN_CLOTH, 2)],
        COPPER_MACE,
        30,
        (55, 95),
    )
    .casts(5125)
    .at(ANVIL),
    base(
        Line::Blacksmithing,
        2661,
        "Copper Chain Belt",
        &[(COPPER_BAR, 6)],
        COPPER_CHAIN_BELT,
        55,
        (75, 115),
    )
    .casts(8000)
    .at(ANVIL),
    base(
        Line::Blacksmithing,
        2665,
        "Coarse Sharpening Stone",
        &[(COARSE_STONE, 1)],
        COARSE_SHARPENING_STONE,
        65,
        (65, 80),
    )
    .casts(5125),
    base(
        Line::Blacksmithing,
        3116,
        "Coarse Weightstone",
        &[(COARSE_STONE, 1), (WOOL_CLOTH, 1)],
        COARSE_WEIGHTSTONE,
        65,
        (65, 80),
    )
    .casts(5125),
    base(
        Line::Blacksmithing,
        3326,
        "Coarse Grinding Stone",
        &[(COARSE_STONE, 2)],
        COARSE_GRINDING_STONE,
        75,
        (75, 100),
    )
    .casts(5125),
    base(
        Line::Blacksmithing,
        2668,
        "Rough Bronze Leggings",
        &[(BRONZE_BAR, 6)],
        ROUGH_BRONZE_LEGGINGS,
        95,
        (145, 175),
    )
    .casts(12500)
    .at(ANVIL),
    base(
        Line::Blacksmithing,
        2741,
        "Bronze Axe",
        &[(BRONZE_BAR, 7), (WEAK_FLUX, 4), (MEDIUM_LEATHER, 1)],
        BRONZE_AXE,
        85,
        (145, 175),
    )
    .casts(12500)
    .at(ANVIL)
    .bought(600),
    base(
        Line::Blacksmithing,
        2674,
        "Heavy Sharpening Stone",
        &[(HEAVY_STONE, 1)],
        HEAVY_SHARPENING_STONE,
        125,
        (125, 140),
    )
    .casts(5125),
    base(
        Line::Blacksmithing,
        3117,
        "Heavy Weightstone",
        &[(HEAVY_STONE, 1), (WOOL_CLOTH, 1)],
        HEAVY_WEIGHTSTONE,
        125,
        (125, 140),
    )
    .casts(5125),
    base(
        Line::Blacksmithing,
        3337,
        "Heavy Grinding Stone",
        &[(HEAVY_STONE, 3)],
        HEAVY_GRINDING_STONE,
        125,
        (125, 150),
    )
    .casts(8000),
    base(
        Line::Blacksmithing,
        8768,
        "Iron Buckle",
        &[(IRON_BAR, 1)],
        IRON_BUCKLE,
        150,
        (150, 155),
    )
    .makes(2)
    .casts(5125)
    .at(ANVIL),
    base(
        Line::Blacksmithing,
        3501,
        "Green Iron Bracers",
        &[(IRON_BAR, 6), (GREEN_DYE, 1)],
        GREEN_IRON_BRACERS,
        165,
        (190, 215),
    )
    .casts(25000)
    .at(ANVIL)
    .bought(2000),
    base(
        Line::Blacksmithing,
        3502,
        "Green Iron Helm",
        &[(IRON_BAR, 12), (CITRINE, 1), (GREEN_DYE, 1)],
        GREEN_IRON_HELM,
        170,
        (195, 220),
    )
    .casts(25000)
    .at(ANVIL)
    .bought(2500),
    base(
        Line::Blacksmithing,
        9813,
        "Barbaric Iron Breastplate",
        &[(IRON_BAR, 20), (HEAVY_GRINDING_STONE, 4)],
        BARBARIC_IRON_BREASTPLATE,
        160,
        (185, 210),
    )
    .casts(25000)
    .at(ANVIL)
    .bought(2500),
    base(
        Line::Blacksmithing,
        3492,
        "Hardened Iron Shortsword",
        &[
            (IRON_BAR, 6),
            (STRONG_FLUX, 2),
            (HEAVY_GRINDING_STONE, 1),
            (LESSER_MOONSTONE, 2),
            (HEAVY_LEATHER, 3),
        ],
        HARDENED_IRON_SHORTSWORD,
        160,
        (185, 210),
    )
    .casts(45000)
    .at(ANVIL)
    .bought(3000),
    base(
        Line::Blacksmithing,
        9918,
        "Solid Sharpening Stone",
        &[(SOLID_STONE, 1)],
        SOLID_SHARPENING_STONE,
        200,
        (200, 210),
    )
    .casts(5125),
    base(
        Line::Blacksmithing,
        9921,
        "Solid Weightstone",
        &[(SOLID_STONE, 1), (SILK_CLOTH, 1)],
        SOLID_WEIGHTSTONE,
        200,
        (200, 210),
    )
    .casts(5125),
    base(
        Line::Blacksmithing,
        9920,
        "Solid Grinding Stone",
        &[(SOLID_STONE, 4)],
        SOLID_GRINDING_STONE,
        200,
        (200, 210),
    )
    .casts(8000),
    base(
        Line::Blacksmithing,
        11454,
        "Inlaid Mithril Cylinder",
        &[(MITHRIL_BAR, 5), (GOLD_BAR, 1), (TRUESILVER_BAR, 1)],
        INLAID_MITHRIL_CYLINDER,
        200,
        (225, 250),
    )
    .at(ANVIL),
    base(
        Line::Blacksmithing,
        9959,
        "Heavy Mithril Breastplate",
        &[(MITHRIL_BAR, 16)],
        HEAVY_MITHRIL_BREASTPLATE,
        230,
        (250, 270),
    )
    .casts(60000)
    .at(ANVIL)
    .bought(6000),
    base(
        Line::Blacksmithing,
        9968,
        "Heavy Mithril Boots",
        &[(MITHRIL_BAR, 14), (THICK_LEATHER, 4)],
        HEAVY_MITHRIL_BOOTS,
        235,
        (255, 275),
    )
    .casts(60000)
    .at(ANVIL)
    .bought(6000),
    base(
        Line::Blacksmithing,
        9997,
        "Wicked Mithril Blade",
        &[
            (MITHRIL_BAR, 14),
            (TRUESILVER_BAR, 4),
            (SOLID_GRINDING_STONE, 1),
            (THICK_LEATHER, 2),
        ],
        WICKED_MITHRIL_BLADE,
        230,
        (250, 275),
    )
    .casts(60000)
    .at(ANVIL)
    .bought(7000),
    base(
        Line::Blacksmithing,
        16641,
        "Dense Sharpening Stone",
        &[(DENSE_STONE, 1)],
        DENSE_SHARPENING_STONE,
        250,
        (255, 260),
    )
    .casts(8000),
    base(
        Line::Blacksmithing,
        16640,
        "Dense Weightstone",
        &[(DENSE_STONE, 1), (RUNECLOTH, 1)],
        DENSE_WEIGHTSTONE,
        250,
        (255, 260),
    )
    .casts(8000),
    base(
        Line::Blacksmithing,
        16639,
        "Dense Grinding Stone",
        &[(DENSE_STONE, 4)],
        DENSE_GRINDING_STONE,
        250,
        (255, 260),
    )
    .casts(8000),
    base(
        Line::Blacksmithing,
        16647,
        "Imperial Plate Belt",
        &[(THORIUM_BAR, 22), (RUGGED_LEATHER, 6), (AQUAMARINE, 1)],
        IMPERIAL_PLATE_BELT,
        265,
        (285, 305),
    )
    .casts(60000)
    .at(ANVIL)
    .dropped(PLANS_IMPERIAL_PLATE_BELT),
    base(
        Line::Blacksmithing,
        16649,
        "Imperial Plate Bracers",
        &[(THORIUM_BAR, 20), (STAR_RUBY, 1)],
        IMPERIAL_PLATE_BRACERS,
        270,
        (290, 310),
    )
    .casts(60000)
    .at(ANVIL)
    .dropped(PLANS_IMPERIAL_PLATE_BRACERS),
    base(
        Line::Blacksmithing,
        16646,
        "Imperial Plate Shoulders",
        &[(THORIUM_BAR, 24), (RUGGED_LEATHER, 6), (CITRINE, 2)],
        IMPERIAL_PLATE_SHOULDERS,
        265,
        (285, 305),
    )
    .casts(60000)
    .at(ANVIL)
    .dropped(PLANS_IMPERIAL_PLATE_SHOULDERS),
    base(
        Line::Blacksmithing,
        16657,
        "Imperial Plate Boots",
        &[(THORIUM_BAR, 34), (STAR_RUBY, 1), (AQUAMARINE, 1)],
        IMPERIAL_PLATE_BOOTS,
        295,
        (315, 335),
    )
    .casts(60000)
    .at(ANVIL)
    .dropped(PLANS_IMPERIAL_PLATE_BOOTS),
    base(
        Line::Blacksmithing,
        16658,
        "Imperial Plate Helm",
        &[(THORIUM_BAR, 34), (STAR_RUBY, 2)],
        IMPERIAL_PLATE_HELM,
        295,
        (315, 335),
    )
    .casts(60000)
    .at(ANVIL)
    .dropped(PLANS_IMPERIAL_PLATE_HELM),
    base(
        Line::Blacksmithing,
        16663,
        "Imperial Plate Chest",
        &[(THORIUM_BAR, 40), (STAR_RUBY, 2)],
        IMPERIAL_PLATE_CHEST,
        300,
        (320, 340),
    )
    .casts(60000)
    .at(ANVIL)
    .dropped(PLANS_IMPERIAL_PLATE_CHEST),
    base(
        Line::Blacksmithing,
        16730,
        "Imperial Plate Leggings",
        &[(THORIUM_BAR, 44), (STAR_RUBY, 2)],
        IMPERIAL_PLATE_LEGGINGS,
        300,
        (320, 340),
    )
    .casts(60000)
    .at(ANVIL)
    .dropped(PLANS_IMPERIAL_PLATE_LEGGINGS),
    base(
        Line::Blacksmithing,
        16729,
        "Lionheart Helm",
        &[
            (THORIUM_BAR, 80),
            (ARCANITE_BAR, 12),
            (WICKED_CLAW, 40),
            (BLUE_SAPPHIRE, 10),
            (AZEROTHIAN_DIAMOND, 4),
        ],
        LIONHEART_HELM,
        300,
        (320, 340),
    )
    .casts(60000)
    .at(ANVIL)
    .dropped(PLANS_LIONHEART_HELM),
    base(
        Line::Blacksmithing,
        16994,
        "Arcanite Reaper",
        &[
            (ARCANITE_BAR, 20),
            (ENCHANTED_LEATHER, 6),
            (DENSE_GRINDING_STONE, 2),
        ],
        ARCANITE_REAPER,
        300,
        (320, 340),
    )
    .casts(60000)
    .at(ANVIL)
    .dropped(PLANS_ARCANITE_REAPER),
    base(
        Line::Blacksmithing,
        22757,
        "Elemental Sharpening Stone",
        &[(ELEMENTAL_EARTH, 2), (DENSE_STONE, 3)],
        ELEMENTAL_SHARPENING_STONE,
        300,
        (300, 320),
    )
    .casts(8000)
    .dropped(PLANS_ELEMENTAL_SHARPENING_STONE),
    base(
        Line::Blacksmithing,
        15296,
        "Dark Iron Plate",
        &[(DARK_IRON_BAR, 20), (HEART_OF_FIRE, 8)],
        DARK_IRON_PLATE,
        285,
        (305, 325),
    )
    .casts(45000)
    .at(BLACK_ANVIL)
    .dropped(PLANS_DARK_IRON_PLATE),
    base(
        Line::Blacksmithing,
        20872,
        "Fiery Chain Girdle",
        &[(DARK_IRON_BAR, 6), (FIERY_CORE, 3), (LAVA_CORE, 3)],
        FIERY_CHAIN_GIRDLE,
        295,
        (315, 335),
    )
    .casts(60000)
    .at(BLACK_ANVIL)
    .dropped(PLANS_FIERY_CHAIN_GIRDLE),
    base(
        Line::Blacksmithing,
        21161,
        "Sulfuron Hammer",
        &[
            (SULFURON_INGOT, 8),
            (DARK_IRON_BAR, 20),
            (ARCANITE_BAR, 50),
            (ESSENCE_OF_FIRE, 25),
            (BLOOD_OF_THE_MOUNTAIN, 10),
            (LAVA_CORE, 10),
            (FIERY_CORE, 10),
        ],
        SULFURON_HAMMER,
        300,
        (325, 350),
    )
    .casts(60000)
    .at(BLACK_ANVIL)
    .dropped(PLANS_SULFURON_HAMMER),
];

/// Leatherworking (P3): the leathers and armor kits, the leather ladder, and the patterns that
/// drop: the Devilsaur pair, the Volcanic pieces and Molten Core's.
static LEATHERWORKING: &[Recipe] = &[
    base(
        Line::Leatherworking,
        2881,
        "Light Leather",
        &[(RUINED_LEATHER_SCRAPS, 3)],
        LIGHT_LEATHER,
        1,
        (20, 40),
    )
    .casts(5125),
    base(
        Line::Leatherworking,
        2152,
        "Light Armor Kit",
        &[(LIGHT_LEATHER, 1)],
        LIGHT_ARMOR_KIT,
        1,
        (30, 60),
    )
    .casts(5125),
    base(
        Line::Leatherworking,
        9059,
        "Handstitched Leather Bracers",
        &[(LIGHT_LEATHER, 2), (COARSE_THREAD, 3)],
        HANDSTITCHED_LEATHER_BRACERS,
        1,
        (40, 70),
    )
    .casts(5125),
    base(
        Line::Leatherworking,
        2149,
        "Handstitched Leather Boots",
        &[(LIGHT_LEATHER, 2), (COARSE_THREAD, 1)],
        HANDSTITCHED_LEATHER_BOOTS,
        1,
        (40, 70),
    )
    .casts(5125),
    base(
        Line::Leatherworking,
        2153,
        "Handstitched Leather Pants",
        &[(LIGHT_LEATHER, 4), (COARSE_THREAD, 1)],
        HANDSTITCHED_LEATHER_PANTS,
        15,
        (45, 75),
    )
    .casts(5125),
    base(
        Line::Leatherworking,
        7126,
        "Handstitched Leather Vest",
        &[(LIGHT_LEATHER, 3), (COARSE_THREAD, 1)],
        HANDSTITCHED_LEATHER_VEST,
        1,
        (40, 70),
    )
    .casts(5125),
    base(
        Line::Leatherworking,
        3753,
        "Handstitched Leather Belt",
        &[(LIGHT_LEATHER, 6), (COARSE_THREAD, 1)],
        HANDSTITCHED_LEATHER_BELT,
        25,
        (55, 85),
    )
    .casts(5125),
    base(
        Line::Leatherworking,
        2160,
        "Embossed Leather Vest",
        &[(LIGHT_LEATHER, 8), (COARSE_THREAD, 4)],
        EMBOSSED_LEATHER_VEST,
        55,
        (70, 100),
    )
    .casts(12500),
    base(
        Line::Leatherworking,
        3756,
        "Embossed Leather Gloves",
        &[(LIGHT_LEATHER, 3), (COARSE_THREAD, 2)],
        EMBOSSED_LEATHER_GLOVES,
        55,
        (85, 115),
    )
    .casts(8000),
    base(
        Line::Leatherworking,
        20648,
        "Medium Leather",
        &[(LIGHT_LEATHER, 4)],
        MEDIUM_LEATHER,
        100,
        (100, 110),
    )
    .casts(8000),
    base(
        Line::Leatherworking,
        2165,
        "Medium Armor Kit",
        &[(MEDIUM_LEATHER, 4), (COARSE_THREAD, 1)],
        MEDIUM_ARMOR_KIT,
        100,
        (115, 130),
    )
    .casts(5125),
    base(
        Line::Leatherworking,
        2167,
        "Dark Leather Boots",
        &[(MEDIUM_LEATHER, 4), (FINE_THREAD, 2), (GRAY_DYE, 1)],
        DARK_LEATHER_BOOTS,
        100,
        (125, 150),
    )
    .casts(8000)
    .bought(600),
    base(
        Line::Leatherworking,
        2169,
        "Dark Leather Tunic",
        &[(MEDIUM_LEATHER, 6), (FINE_THREAD, 1), (GRAY_DYE, 1)],
        DARK_LEATHER_TUNIC,
        100,
        (125, 150),
    )
    .casts(12500)
    .bought(600),
    base(
        Line::Leatherworking,
        7135,
        "Dark Leather Pants",
        &[(MEDIUM_LEATHER, 12), (GRAY_DYE, 1), (FINE_THREAD, 1)],
        DARK_LEATHER_PANTS,
        120,
        (140, 165),
    )
    .casts(25000)
    .bought(900),
    base(
        Line::Leatherworking,
        20649,
        "Heavy Leather",
        &[(MEDIUM_LEATHER, 5)],
        HEAVY_LEATHER,
        150,
        (150, 160),
    )
    .casts(8000),
    base(
        Line::Leatherworking,
        3780,
        "Heavy Armor Kit",
        &[(HEAVY_LEATHER, 5), (FINE_THREAD, 1)],
        HEAVY_ARMOR_KIT,
        150,
        (170, 190),
    )
    .casts(5125),
    base(
        Line::Leatherworking,
        6661,
        "Barbaric Harness",
        &[(HEAVY_LEATHER, 14), (FINE_THREAD, 2), (IRON_BUCKLE, 1)],
        BARBARIC_HARNESS,
        190,
        (210, 230),
    )
    .casts(25000)
    .bought(2000),
    base(
        Line::Leatherworking,
        7149,
        "Barbaric Leggings",
        &[(HEAVY_LEATHER, 10), (FINE_THREAD, 2), (MOSS_AGATE, 1)],
        BARBARIC_LEGGINGS,
        170,
        (190, 210),
    )
    .casts(45000)
    .bought(2000),
    base(
        Line::Leatherworking,
        20650,
        "Thick Leather",
        &[(HEAVY_LEATHER, 6)],
        THICK_LEATHER,
        200,
        (200, 205),
    )
    .casts(8000),
    base(
        Line::Leatherworking,
        10487,
        "Thick Armor Kit",
        &[(THICK_LEATHER, 5), (SILKEN_THREAD, 1)],
        THICK_ARMOR_KIT,
        200,
        (220, 240),
    )
    .casts(5125),
    base(
        Line::Leatherworking,
        10499,
        "Nightscape Tunic",
        &[(THICK_LEATHER, 7), (SILKEN_THREAD, 2)],
        NIGHTSCAPE_TUNIC,
        205,
        (225, 245),
    )
    .casts(60000)
    .bought(4000),
    base(
        Line::Leatherworking,
        10507,
        "Nightscape Headband",
        &[(THICK_LEATHER, 5), (SILKEN_THREAD, 2)],
        NIGHTSCAPE_HEADBAND,
        205,
        (225, 245),
    )
    .casts(60000)
    .bought(4000),
    base(
        Line::Leatherworking,
        10548,
        "Nightscape Pants",
        &[(THICK_LEATHER, 14), (SILKEN_THREAD, 4)],
        NIGHTSCAPE_PANTS,
        230,
        (250, 270),
    )
    .casts(60000)
    .bought(5000),
    base(
        Line::Leatherworking,
        10558,
        "Nightscape Boots",
        &[(THICK_LEATHER, 16), (HEAVY_SILKEN_THREAD, 2)],
        NIGHTSCAPE_BOOTS,
        235,
        (255, 275),
    )
    .casts(60000)
    .bought(5000),
    base(
        Line::Leatherworking,
        22331,
        "Rugged Leather",
        &[(THICK_LEATHER, 6)],
        RUGGED_LEATHER,
        250,
        (250, 250),
    )
    .casts(8000),
    base(
        Line::Leatherworking,
        19058,
        "Rugged Armor Kit",
        &[(RUGGED_LEATHER, 5)],
        RUGGED_ARMOR_KIT,
        250,
        (250, 270),
    )
    .casts(12500),
    base(
        Line::Leatherworking,
        19047,
        "Cured Rugged Hide",
        &[(RUGGED_HIDE, 1), (REFINED_DEEPROCK_SALT, 1)],
        CURED_RUGGED_HIDE,
        250,
        (250, 260),
    )
    .casts(12500),
    base(
        Line::Leatherworking,
        19084,
        "Devilsaur Gauntlets",
        &[
            (RUGGED_LEATHER, 30),
            (DEVILSAUR_LEATHER, 8),
            (RUNE_THREAD, 1),
        ],
        DEVILSAUR_GAUNTLETS,
        290,
        (310, 330),
    )
    .casts(60000)
    .dropped(PATTERN_DEVILSAUR_GAUNTLETS),
    base(
        Line::Leatherworking,
        19097,
        "Devilsaur Leggings",
        &[
            (RUGGED_LEATHER, 30),
            (DEVILSAUR_LEATHER, 14),
            (CURED_RUGGED_HIDE, 1),
            (RUNE_THREAD, 1),
        ],
        DEVILSAUR_LEGGINGS,
        300,
        (320, 340),
    )
    .casts(60000)
    .dropped(PATTERN_DEVILSAUR_LEGGINGS),
    base(
        Line::Leatherworking,
        19076,
        "Volcanic Breastplate",
        &[
            (RUGGED_LEATHER, 8),
            (ESSENCE_OF_FIRE, 1),
            (ESSENCE_OF_EARTH, 1),
            (RUNE_THREAD, 1),
        ],
        VOLCANIC_BREASTPLATE,
        285,
        (305, 325),
    )
    .casts(60000)
    .dropped(PATTERN_VOLCANIC_BREASTPLATE),
    base(
        Line::Leatherworking,
        19059,
        "Volcanic Leggings",
        &[
            (RUGGED_LEATHER, 6),
            (ESSENCE_OF_FIRE, 1),
            (CORE_OF_EARTH, 1),
            (RUNE_THREAD, 1),
        ],
        VOLCANIC_LEGGINGS,
        270,
        (290, 310),
    )
    .casts(60000)
    .dropped(PATTERN_VOLCANIC_LEGGINGS),
    base(
        Line::Leatherworking,
        19101,
        "Volcanic Shoulders",
        &[
            (RUGGED_LEATHER, 10),
            (ESSENCE_OF_FIRE, 1),
            (ESSENCE_OF_EARTH, 1),
            (RUNE_THREAD, 2),
        ],
        VOLCANIC_SHOULDERS,
        300,
        (320, 340),
    )
    .casts(60000)
    .dropped(PATTERN_VOLCANIC_SHOULDERS),
    base(
        Line::Leatherworking,
        22727,
        "Core Armor Kit",
        &[(CORE_LEATHER, 3), (RUNE_THREAD, 2)],
        CORE_ARMOR_KIT,
        300,
        (320, 340),
    )
    .casts(25000)
    .dropped(PATTERN_CORE_ARMOR_KIT),
    base(
        Line::Leatherworking,
        20853,
        "Corehound Boots",
        &[
            (CORE_LEATHER, 20),
            (FIERY_CORE, 6),
            (LAVA_CORE, 2),
            (RUNE_THREAD, 2),
        ],
        COREHOUND_BOOTS,
        295,
        (315, 335),
    )
    .casts(60000)
    .dropped(PATTERN_COREHOUND_BOOTS),
    base(
        Line::Leatherworking,
        20854,
        "Molten Helm",
        &[
            (CORE_LEATHER, 15),
            (FIERY_CORE, 3),
            (LAVA_CORE, 6),
            (RUNE_THREAD, 2),
        ],
        MOLTEN_HELM,
        300,
        (320, 340),
    )
    .casts(60000)
    .dropped(PATTERN_MOLTEN_HELM),
];

/// Tailoring (P3): the bolts and bags, the cloth ladder, Mooncloth, and the patterns that drop:
/// the Mooncloth and Bottomless bags, the epic robes and Molten Core's.
static TAILORING: &[Recipe] = &[
    base(
        Line::Tailoring,
        2963,
        "Bolt of Linen Cloth",
        &[(LINEN_CLOTH, 2)],
        BOLT_OF_LINEN_CLOTH,
        1,
        (25, 50),
    ),
    base(
        Line::Tailoring,
        12044,
        "Simple Linen Pants",
        &[(BOLT_OF_LINEN_CLOTH, 1), (COARSE_THREAD, 1)],
        SIMPLE_LINEN_PANTS,
        1,
        (35, 60),
    )
    .casts(5125),
    base(
        Line::Tailoring,
        2385,
        "Brown Linen Vest",
        &[(BOLT_OF_LINEN_CLOTH, 1), (COARSE_THREAD, 1)],
        BROWN_LINEN_VEST,
        10,
        (45, 70),
    )
    .casts(5125),
    base(
        Line::Tailoring,
        8776,
        "Linen Belt",
        &[(BOLT_OF_LINEN_CLOTH, 1), (COARSE_THREAD, 1)],
        LINEN_BELT,
        25,
        (50, 85),
    )
    .casts(5125),
    base(
        Line::Tailoring,
        3840,
        "Heavy Linen Gloves",
        &[(BOLT_OF_LINEN_CLOTH, 2), (COARSE_THREAD, 1)],
        HEAVY_LINEN_GLOVES,
        35,
        (60, 95),
    )
    .casts(5125),
    base(
        Line::Tailoring,
        3755,
        "Linen Bag",
        &[(BOLT_OF_LINEN_CLOTH, 3), (COARSE_THREAD, 3)],
        LINEN_BAG,
        45,
        (70, 105),
    )
    .casts(8000),
    base(
        Line::Tailoring,
        2964,
        "Bolt of Woolen Cloth",
        &[(WOOL_CLOTH, 3)],
        BOLT_OF_WOOLEN_CLOTH,
        75,
        (90, 105),
    )
    .casts(5125),
    base(
        Line::Tailoring,
        3757,
        "Woolen Bag",
        &[(BOLT_OF_WOOLEN_CLOTH, 3), (FINE_THREAD, 1)],
        WOOLEN_BAG,
        80,
        (105, 140),
    )
    .casts(12500)
    .bought(300),
    base(
        Line::Tailoring,
        2401,
        "Woolen Boots",
        &[
            (BOLT_OF_WOOLEN_CLOTH, 4),
            (FINE_THREAD, 2),
            (LIGHT_LEATHER, 2),
        ],
        WOOLEN_BOOTS,
        95,
        (120, 155),
    )
    .casts(12500)
    .bought(400),
    base(
        Line::Tailoring,
        3843,
        "Heavy Woolen Gloves",
        &[(BOLT_OF_WOOLEN_CLOTH, 3), (FINE_THREAD, 1)],
        HEAVY_WOOLEN_GLOVES,
        85,
        (110, 145),
    )
    .casts(12500)
    .bought(400),
    base(
        Line::Tailoring,
        3850,
        "Heavy Woolen Pants",
        &[(BOLT_OF_WOOLEN_CLOTH, 5), (FINE_THREAD, 4)],
        HEAVY_WOOLEN_PANTS,
        110,
        (135, 170),
    )
    .casts(25000)
    .bought(500),
    base(
        Line::Tailoring,
        3848,
        "Double-stitched Woolen Shoulders",
        &[(BOLT_OF_WOOLEN_CLOTH, 3), (FINE_THREAD, 2)],
        DOUBLE_STITCHED_WOOLEN_SHOULDERS,
        110,
        (135, 170),
    )
    .casts(12500)
    .bought(500),
    base(
        Line::Tailoring,
        3839,
        "Bolt of Silk Cloth",
        &[(SILK_CLOTH, 4)],
        BOLT_OF_SILK_CLOTH,
        125,
        (135, 145),
    )
    .casts(5125),
    base(
        Line::Tailoring,
        3813,
        "Small Silk Pack",
        &[
            (BOLT_OF_SILK_CLOTH, 3),
            (HEAVY_LEATHER, 2),
            (FINE_THREAD, 3),
        ],
        SMALL_SILK_PACK,
        150,
        (170, 200),
    )
    .casts(25000)
    .bought(1500),
    base(
        Line::Tailoring,
        8762,
        "Silk Headband",
        &[(BOLT_OF_SILK_CLOTH, 3), (FINE_THREAD, 2)],
        SILK_HEADBAND,
        145,
        (170, 180),
    )
    .casts(25000),
    base(
        Line::Tailoring,
        8784,
        "Green Silk Armor",
        &[(BOLT_OF_SILK_CLOTH, 5), (GREEN_DYE, 2), (SILKEN_THREAD, 1)],
        GREEN_SILK_ARMOR,
        165,
        (185, 215),
    )
    .casts(45000)
    .bought(2000),
    base(
        Line::Tailoring,
        8774,
        "Green Silken Shoulders",
        &[(BOLT_OF_SILK_CLOTH, 5), (SILKEN_THREAD, 2)],
        GREEN_SILKEN_SHOULDERS,
        180,
        (200, 230),
    )
    .casts(45000)
    .bought(2000),
    base(
        Line::Tailoring,
        3865,
        "Bolt of Mageweave",
        &[(MAGEWEAVE_CLOTH, 5)],
        BOLT_OF_MAGEWEAVE,
        175,
        (180, 185),
    )
    .casts(8000),
    base(
        Line::Tailoring,
        12065,
        "Mageweave Bag",
        &[(BOLT_OF_MAGEWEAVE, 4), (SILKEN_THREAD, 2)],
        MAGEWEAVE_BAG,
        225,
        (240, 270),
    )
    .casts(60000)
    .bought(4000),
    base(
        Line::Tailoring,
        12048,
        "Black Mageweave Vest",
        &[(BOLT_OF_MAGEWEAVE, 2), (SILKEN_THREAD, 3)],
        BLACK_MAGEWEAVE_VEST,
        205,
        (220, 250),
    )
    .casts(60000),
    base(
        Line::Tailoring,
        12049,
        "Black Mageweave Leggings",
        &[(BOLT_OF_MAGEWEAVE, 2), (SILKEN_THREAD, 3)],
        BLACK_MAGEWEAVE_LEGGINGS,
        205,
        (220, 250),
    )
    .casts(60000),
    base(
        Line::Tailoring,
        12053,
        "Black Mageweave Gloves",
        &[(BOLT_OF_MAGEWEAVE, 2), (HEAVY_SILKEN_THREAD, 2)],
        BLACK_MAGEWEAVE_GLOVES,
        215,
        (230, 260),
    )
    .casts(60000)
    .bought(3500),
    base(
        Line::Tailoring,
        12073,
        "Black Mageweave Boots",
        &[
            (BOLT_OF_MAGEWEAVE, 3),
            (HEAVY_SILKEN_THREAD, 2),
            (THICK_LEATHER, 2),
        ],
        BLACK_MAGEWEAVE_BOOTS,
        230,
        (245, 275),
    )
    .casts(60000)
    .bought(3500),
    base(
        Line::Tailoring,
        12069,
        "Cindercloth Robe",
        &[
            (BOLT_OF_MAGEWEAVE, 5),
            (HEART_OF_FIRE, 2),
            (HEAVY_SILKEN_THREAD, 2),
        ],
        CINDERCLOTH_ROBE,
        225,
        (240, 270),
    )
    .casts(60000)
    .bought(4000),
    base(
        Line::Tailoring,
        18401,
        "Bolt of Runecloth",
        &[(RUNECLOTH, 5)],
        BOLT_OF_RUNECLOTH,
        250,
        (255, 260),
    )
    .casts(12500),
    base(
        Line::Tailoring,
        18405,
        "Runecloth Bag",
        &[
            (BOLT_OF_RUNECLOTH, 5),
            (RUGGED_LEATHER, 2),
            (RUNE_THREAD, 1),
        ],
        RUNECLOTH_BAG,
        260,
        (275, 305),
    )
    .casts(60000)
    .bought(8000),
    base(
        Line::Tailoring,
        18402,
        "Runecloth Belt",
        &[(BOLT_OF_RUNECLOTH, 3), (RUNE_THREAD, 1)],
        RUNECLOTH_BELT,
        255,
        (270, 300),
    )
    .casts(60000),
    base(
        Line::Tailoring,
        18417,
        "Runecloth Gloves",
        &[
            (BOLT_OF_RUNECLOTH, 4),
            (RUGGED_LEATHER, 4),
            (RUNE_THREAD, 1),
        ],
        RUNECLOTH_GLOVES,
        275,
        (290, 320),
    )
    .casts(60000)
    .bought(8000),
    base(
        Line::Tailoring,
        18408,
        "Cindercloth Vest",
        &[(BOLT_OF_RUNECLOTH, 5), (HEART_OF_FIRE, 3), (RUNE_THREAD, 1)],
        CINDERCLOTH_VEST,
        260,
        (275, 305),
    )
    .casts(60000)
    .bought(8000),
    base(
        Line::Tailoring,
        18560,
        "Mooncloth",
        &[(FELCLOTH, 2)],
        MOONCLOTH,
        250,
        (290, 320),
    )
    .casts(60000)
    .at(MOONWELL),
    base(
        Line::Tailoring,
        18445,
        "Mooncloth Bag",
        &[(BOLT_OF_RUNECLOTH, 4), (MOONCLOTH, 1), (RUNE_THREAD, 1)],
        MOONCLOTH_BAG,
        285,
        (315, 345),
    )
    .casts(60000)
    .dropped(PATTERN_MOONCLOTH_BAG),
    base(
        Line::Tailoring,
        18440,
        "Mooncloth Leggings",
        &[(BOLT_OF_RUNECLOTH, 6), (MOONCLOTH, 4), (RUNE_THREAD, 1)],
        MOONCLOTH_LEGGINGS,
        290,
        (305, 335),
    )
    .casts(60000)
    .dropped(PATTERN_MOONCLOTH_LEGGINGS),
    base(
        Line::Tailoring,
        18447,
        "Mooncloth Vest",
        &[(BOLT_OF_RUNECLOTH, 6), (MOONCLOTH, 4), (RUNE_THREAD, 1)],
        MOONCLOTH_VEST,
        300,
        (315, 345),
    )
    .casts(60000)
    .dropped(PATTERN_MOONCLOTH_VEST),
    base(
        Line::Tailoring,
        18455,
        "Bottomless Bag",
        &[
            (BOLT_OF_RUNECLOTH, 8),
            (MOONCLOTH, 12),
            (LARGE_BRILLIANT_SHARD, 2),
            (CORE_LEATHER, 2),
            (RUNE_THREAD, 2),
        ],
        BOTTOMLESS_BAG,
        300,
        (315, 345),
    )
    .casts(60000)
    .dropped(PATTERN_BOTTOMLESS_BAG),
    base(
        Line::Tailoring,
        18457,
        "Robe of the Archmage",
        &[
            (BOLT_OF_RUNECLOTH, 12),
            (ESSENCE_OF_FIRE, 10),
            (ESSENCE_OF_AIR, 10),
            (ESSENCE_OF_EARTH, 10),
            (ESSENCE_OF_WATER, 10),
            (RUNE_THREAD, 2),
        ],
        ROBE_OF_THE_ARCHMAGE,
        300,
        (315, 345),
    )
    .casts(60000)
    .dropped(PATTERN_ROBE_OF_THE_ARCHMAGE),
    base(
        Line::Tailoring,
        18456,
        "Truefaith Vestments",
        &[
            (BOLT_OF_RUNECLOTH, 12),
            (MOONCLOTH, 10),
            (RIGHTEOUS_ORB, 4),
            (GOLDEN_PEARL, 4),
            (GHOST_DYE, 10),
            (RUNE_THREAD, 2),
        ],
        TRUEFAITH_VESTMENTS,
        300,
        (315, 345),
    )
    .casts(60000)
    .dropped(PATTERN_TRUEFAITH_VESTMENTS),
    base(
        Line::Tailoring,
        20848,
        "Flarecore Mantle",
        &[
            (BOLT_OF_RUNECLOTH, 12),
            (FIERY_CORE, 4),
            (LAVA_CORE, 4),
            (ENCHANTED_LEATHER, 6),
            (RUNE_THREAD, 2),
        ],
        FLARECORE_MANTLE,
        300,
        (315, 345),
    )
    .casts(60000)
    .dropped(PATTERN_FLARECORE_MANTLE),
    base(
        Line::Tailoring,
        20849,
        "Flarecore Gloves",
        &[
            (BOLT_OF_RUNECLOTH, 8),
            (FIERY_CORE, 6),
            (ESSENCE_OF_FIRE, 4),
            (ENCHANTED_LEATHER, 2),
            (RUNE_THREAD, 2),
        ],
        FLARECORE_GLOVES,
        300,
        (315, 345),
    )
    .casts(60000)
    .dropped(PATTERN_FLARECORE_GLOVES),
    base(
        Line::Tailoring,
        26087,
        "Core Felcloth Bag",
        &[
            (FELCLOTH, 20),
            (CORE_LEATHER, 16),
            (BLOODVINE, 8),
            (ESSENCE_OF_FIRE, 4),
            (IRONWEB_SPIDER_SILK, 4),
        ],
        CORE_FELCLOTH_BAG,
        300,
        (315, 345),
    )
    .casts(60000)
    .dropped(PATTERN_CORE_FELCLOTH_BAG),
];

/// Engineering (P3): the parts, the bombs and gadgets, and the schematics that drop; the
/// specialisations' gadgets need theirs ([`Spec`]).
static ENGINEERING: &[Recipe] = &[
    base(
        Line::Engineering,
        3918,
        "Rough Blasting Powder",
        &[(ROUGH_STONE, 1)],
        ROUGH_BLASTING_POWDER,
        1,
        (20, 40),
    )
    .casts(5125),
    base(
        Line::Engineering,
        3922,
        "Handful of Copper Bolts",
        &[(COPPER_BAR, 1)],
        HANDFUL_OF_COPPER_BOLTS,
        30,
        (45, 60),
    )
    .casts(5125)
    .at(ANVIL),
    base(
        Line::Engineering,
        3923,
        "Rough Copper Bomb",
        &[
            (COPPER_BAR, 1),
            (HANDFUL_OF_COPPER_BOLTS, 1),
            (ROUGH_BLASTING_POWDER, 2),
            (LINEN_CLOTH, 1),
        ],
        ROUGH_COPPER_BOMB,
        30,
        (60, 90),
    )
    .makes(2)
    .casts(5125)
    .at(ANVIL),
    base(
        Line::Engineering,
        3926,
        "Copper Modulator",
        &[
            (HANDFUL_OF_COPPER_BOLTS, 2),
            (COPPER_BAR, 1),
            (LINEN_CLOTH, 2),
        ],
        COPPER_MODULATOR,
        65,
        (95, 125),
    )
    .casts(5125)
    .at(ANVIL),
    base(
        Line::Engineering,
        3929,
        "Coarse Blasting Powder",
        &[(COARSE_STONE, 1)],
        COARSE_BLASTING_POWDER,
        75,
        (85, 95),
    )
    .casts(5125),
    base(
        Line::Engineering,
        3932,
        "Target Dummy",
        &[
            (COPPER_MODULATOR, 1),
            (HANDFUL_OF_COPPER_BOLTS, 2),
            (BRONZE_BAR, 1),
            (WOOL_CLOTH, 1),
        ],
        TARGET_DUMMY,
        85,
        (115, 145),
    )
    .casts(8000)
    .at(ANVIL)
    .bought(500),
    base(
        Line::Engineering,
        3942,
        "Whirring Bronze Gizmo",
        &[(BRONZE_BAR, 2), (WOOL_CLOTH, 1)],
        WHIRRING_BRONZE_GIZMO,
        100,
        (125, 175),
    )
    .casts(5125)
    .at(ANVIL),
    base(
        Line::Engineering,
        3945,
        "Heavy Blasting Powder",
        &[(HEAVY_STONE, 1)],
        HEAVY_BLASTING_POWDER,
        125,
        (125, 145),
    )
    .casts(5125),
    base(
        Line::Engineering,
        3953,
        "Bronze Framework",
        &[(BRONZE_BAR, 2), (MEDIUM_LEATHER, 1), (WOOL_CLOTH, 1)],
        BRONZE_FRAMEWORK,
        125,
        (145, 195),
    )
    .casts(5125),
    base(
        Line::Engineering,
        3955,
        "Explosive Sheep",
        &[
            (BRONZE_FRAMEWORK, 1),
            (WHIRRING_BRONZE_GIZMO, 1),
            (HEAVY_BLASTING_POWDER, 2),
            (WOOL_CLOTH, 2),
        ],
        EXPLOSIVE_SHEEP,
        150,
        (175, 200),
    )
    .casts(8000)
    .at(ANVIL)
    .bought(2000),
    base(
        Line::Engineering,
        3958,
        "Iron Strut",
        &[(IRON_BAR, 2)],
        IRON_STRUT,
        160,
        (160, 180),
    )
    .casts(8000)
    .at(ANVIL),
    base(
        Line::Engineering,
        3962,
        "Iron Grenade",
        &[(IRON_BAR, 1), (HEAVY_BLASTING_POWDER, 1), (SILK_CLOTH, 1)],
        IRON_GRENADE,
        175,
        (175, 215),
    )
    .makes(4)
    .casts(8000)
    .at(ANVIL)
    .bought(2000),
    base(
        Line::Engineering,
        3973,
        "Silver Contact",
        &[(SILVER_BAR, 1)],
        SILVER_CONTACT,
        90,
        (110, 140),
    )
    .makes(5)
    .casts(5125),
    base(
        Line::Engineering,
        3967,
        "Big Iron Bomb",
        &[
            (IRON_BAR, 3),
            (HEAVY_BLASTING_POWDER, 3),
            (SILVER_CONTACT, 1),
        ],
        BIG_IRON_BOMB,
        190,
        (190, 230),
    )
    .makes(2)
    .casts(12500)
    .at(ANVIL)
    .bought(3000),
    base(
        Line::Engineering,
        9273,
        "Goblin Jumper Cables",
        &[
            (IRON_BAR, 6),
            (WHIRRING_BRONZE_GIZMO, 2),
            (FLASK_OF_OIL, 2),
            (SILK_CLOTH, 2),
            (SHADOWGEM, 2),
            (FUSED_WIRING, 1),
        ],
        GOBLIN_JUMPER_CABLES,
        165,
        (160, 200),
    )
    .casts(12500)
    .bought(3000),
    base(
        Line::Engineering,
        12584,
        "Gold Power Core",
        &[(GOLD_BAR, 1)],
        GOLD_POWER_CORE,
        150,
        (150, 190),
    )
    .makes(3)
    .casts(25000)
    .at(ANVIL),
    base(
        Line::Engineering,
        3961,
        "Gyrochronatom",
        &[(IRON_BAR, 1), (GOLD_POWER_CORE, 1)],
        GYROCHRONATOM,
        170,
        (170, 210),
    )
    .casts(8000)
    .at(ANVIL)
    .bought(2000),
    base(
        Line::Engineering,
        3969,
        "Mechanical Dragonling",
        &[
            (BRONZE_FRAMEWORK, 1),
            (IRON_STRUT, 4),
            (GYROCHRONATOM, 4),
            (CITRINE, 2),
            (FUSED_WIRING, 1),
        ],
        MECHANICAL_DRAGONLING,
        200,
        (220, 240),
    )
    .casts(45000)
    .at(ANVIL)
    .bought(6000),
    base(
        Line::Engineering,
        12585,
        "Solid Blasting Powder",
        &[(SOLID_STONE, 2)],
        SOLID_BLASTING_POWDER,
        175,
        (175, 195),
    )
    .casts(8000),
    base(
        Line::Engineering,
        12589,
        "Mithril Tube",
        &[(MITHRIL_BAR, 3)],
        MITHRIL_TUBE,
        195,
        (195, 235),
    )
    .casts(8000)
    .at(ANVIL),
    base(
        Line::Engineering,
        12591,
        "Unstable Trigger",
        &[
            (MITHRIL_BAR, 1),
            (MAGEWEAVE_CLOTH, 1),
            (SOLID_BLASTING_POWDER, 1),
        ],
        UNSTABLE_TRIGGER,
        200,
        (200, 240),
    )
    .casts(8000)
    .at(ANVIL),
    base(
        Line::Engineering,
        12599,
        "Mithril Casing",
        &[(MITHRIL_BAR, 3)],
        MITHRIL_CASING,
        215,
        (215, 255),
    )
    .casts(8000)
    .at(ANVIL),
    base(
        Line::Engineering,
        12760,
        "Goblin Sapper Charge",
        &[
            (MAGEWEAVE_CLOTH, 1),
            (SOLID_BLASTING_POWDER, 3),
            (UNSTABLE_TRIGGER, 1),
        ],
        GOBLIN_SAPPER_CHARGE,
        205,
        (205, 245),
    )
    .casts(25000)
    .bought(6000)
    .needs(Spec::Goblin),
    base(
        Line::Engineering,
        8895,
        "Goblin Rocket Boots",
        &[
            (BLACK_MAGEWEAVE_BOOTS, 1),
            (MITHRIL_TUBE, 2),
            (HEAVY_LEATHER, 4),
            (GOBLIN_ROCKET_FUEL, 2),
            (UNSTABLE_TRIGGER, 1),
        ],
        GOBLIN_ROCKET_BOOTS,
        225,
        (245, 265),
    )
    .casts(12500)
    .at(ANVIL)
    .bought(8000)
    .needs(Spec::Goblin),
    base(
        Line::Engineering,
        12716,
        "Goblin Mortar",
        &[
            (MITHRIL_TUBE, 2),
            (MITHRIL_BAR, 4),
            (SOLID_BLASTING_POWDER, 5),
            (GOLD_POWER_CORE, 1),
            (ELEMENTAL_FIRE, 1),
        ],
        GOBLIN_MORTAR,
        205,
        (225, 245),
    )
    .casts(25000)
    .at(ANVIL)
    .bought(8000)
    .needs(Spec::Goblin),
    base(
        Line::Engineering,
        12908,
        "Goblin Dragon Gun",
        &[
            (MITHRIL_TUBE, 2),
            (GOBLIN_ROCKET_FUEL, 4),
            (MITHRIL_BAR, 6),
            (TRUESILVER_BAR, 6),
            (UNSTABLE_TRIGGER, 1),
        ],
        GOBLIN_DRAGON_GUN,
        240,
        (260, 280),
    )
    .casts(25000)
    .at(ANVIL)
    .bought(10000)
    .needs(Spec::Goblin),
    base(
        Line::Engineering,
        12906,
        "Gnomish Battle Chicken",
        &[
            (MITHRIL_CASING, 1),
            (TRUESILVER_BAR, 6),
            (MITHRIL_BAR, 6),
            (INLAID_MITHRIL_CYLINDER, 2),
            (GOLD_POWER_CORE, 1),
            (JADE, 2),
        ],
        GNOMISH_BATTLE_CHICKEN,
        230,
        (250, 270),
    )
    .casts(25000)
    .at(ANVIL)
    .bought(8000)
    .needs(Spec::Gnomish),
    base(
        Line::Engineering,
        12902,
        "Gnomish Net-o-Matic Projector",
        &[
            (MITHRIL_TUBE, 1),
            (SHADOW_SILK, 2),
            (THICK_SPIDERS_SILK, 4),
            (SOLID_BLASTING_POWDER, 2),
            (MITHRIL_BAR, 4),
        ],
        GNOMISH_NET_O_MATIC_PROJECTOR,
        210,
        (230, 250),
    )
    .casts(25000)
    .at(ANVIL)
    .bought(8000)
    .needs(Spec::Gnomish),
    base(
        Line::Engineering,
        12759,
        "Gnomish Death Ray",
        &[
            (MITHRIL_TUBE, 2),
            (UNSTABLE_TRIGGER, 1),
            (ESSENCE_OF_UNDEATH, 1),
            (ICHOR_OF_UNDEATH, 4),
            (INLAID_MITHRIL_CYLINDER, 1),
        ],
        GNOMISH_DEATH_RAY,
        240,
        (260, 280),
    )
    .casts(12500)
    .at(ANVIL)
    .bought(10000)
    .needs(Spec::Gnomish),
    base(
        Line::Engineering,
        12624,
        "Mithril Mechanical Dragonling",
        &[
            (MITHRIL_BAR, 14),
            (HEART_OF_FIRE, 4),
            (TRUESILVER_BAR, 4),
            (INLAID_MITHRIL_CYLINDER, 2),
            (GOBLIN_ROCKET_FUEL, 2),
            (STAR_RUBY, 2),
        ],
        MITHRIL_MECHANICAL_DRAGONLING,
        250,
        (270, 290),
    )
    .casts(45000)
    .at(ANVIL)
    .dropped(SCHEMATIC_MITHRIL_MECHANICAL_DRAGONLING),
    base(
        Line::Engineering,
        19788,
        "Dense Blasting Powder",
        &[(DENSE_STONE, 2)],
        DENSE_BLASTING_POWDER,
        250,
        (250, 260),
    )
    .casts(8000),
    base(
        Line::Engineering,
        19791,
        "Thorium Widget",
        &[(THORIUM_BAR, 3), (RUNECLOTH, 1)],
        THORIUM_WIDGET,
        260,
        (280, 300),
    )
    .casts(8000)
    .at(ANVIL),
    base(
        Line::Engineering,
        23070,
        "Dense Dynamite",
        &[(DENSE_BLASTING_POWDER, 2), (RUNECLOTH, 3)],
        DENSE_DYNAMITE,
        250,
        (250, 270),
    )
    .makes(2)
    .casts(8000),
    base(
        Line::Engineering,
        19795,
        "Thorium Tube",
        &[(THORIUM_BAR, 6)],
        THORIUM_TUBE,
        275,
        (295, 315),
    )
    .casts(8000)
    .at(ANVIL),
    base(
        Line::Engineering,
        19790,
        "Thorium Grenade",
        &[
            (THORIUM_WIDGET, 1),
            (THORIUM_BAR, 3),
            (DENSE_BLASTING_POWDER, 3),
            (RUNECLOTH, 3),
        ],
        THORIUM_GRENADE,
        260,
        (280, 300),
    )
    .makes(3)
    .casts(8000)
    .at(ANVIL)
    .dropped(SCHEMATIC_THORIUM_GRENADE),
    base(
        Line::Engineering,
        19814,
        "Masterwork Target Dummy",
        &[
            (MITHRIL_CASING, 1),
            (THORIUM_TUBE, 1),
            (THORIUM_WIDGET, 2),
            (TRUESILVER_BAR, 1),
            (RUGGED_LEATHER, 2),
            (RUNECLOTH, 4),
        ],
        MASTERWORK_TARGET_DUMMY,
        275,
        (295, 315),
    )
    .casts(12500)
    .at(ANVIL)
    .dropped(SCHEMATIC_MASTERWORK_TARGET_DUMMY),
    base(
        Line::Engineering,
        19815,
        "Delicate Arcanite Converter",
        &[(ARCANITE_BAR, 1), (IRONWEB_SPIDER_SILK, 1)],
        DELICATE_ARCANITE_CONVERTER,
        285,
        (305, 325),
    )
    .casts(8000)
    .at(ANVIL),
    base(
        Line::Engineering,
        19830,
        "Arcanite Dragonling",
        &[
            (MITHRIL_MECHANICAL_DRAGONLING, 1),
            (DELICATE_ARCANITE_CONVERTER, 8),
            (ENCHANTED_THORIUM_BAR, 10),
            (THORIUM_WIDGET, 6),
            (GOLD_POWER_CORE, 4),
            (ENCHANTED_LEATHER, 6),
        ],
        ARCANITE_DRAGONLING,
        300,
        (320, 340),
    )
    .casts(60000)
    .at(ANVIL)
    .dropped(SCHEMATIC_ARCANITE_DRAGONLING),
];

/// `line`'s book, in the order its trainer teaches it; empty for a gathering line without one.
pub(crate) fn book(line: Line) -> &'static [Recipe] {
    match line {
        Line::Mining => SMELTING,
        Line::Alchemy => ALCHEMY,
        Line::Cooking => COOKING,
        Line::FirstAid => FIRST_AID,
        Line::Enchanting => ENCHANTING,
        Line::Blacksmithing => BLACKSMITHING,
        Line::Leatherworking => LEATHERWORKING,
        Line::Tailoring => TAILORING,
        Line::Engineering => ENGINEERING,
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
/// once learned; a specialisation's only with that specialisation.
pub(crate) fn known(r: &Recipe, skills: &Skills) -> bool {
    let learned = match r.learn {
        Learn::Skill => skills.rank(r.line).rank >= r.req,
        Learn::Trainer(_) | Learn::Drop(_) => skills.learned(r.spell),
    };
    learned && r.spec.is_none_or(|s| skills.learned(s.spell()))
}

/// Whether the player may learn `r` now: not learned yet, the skill there, and its
/// specialisation held.
pub(crate) fn learnable(r: &Recipe, skills: &Skills) -> bool {
    !known(r, skills)
        && !skills.learned(r.spell)
        && skills.rank(r.line).rank >= r.req
        && r.spec.is_none_or(|s| skills.learned(s.spell()))
}

/// The Engineering specialisation the player holds, if any.
pub(crate) fn spec_of(skills: &Skills) -> Option<Spec> {
    Spec::ALL.into_iter().find(|s| skills.learned(s.spell()))
}

/// Whether the player may take a specialisation now: Engineering at [`SPEC_SKILL`], and neither
/// held.
pub(crate) fn spec_open(skills: &Skills) -> bool {
    spec_of(skills).is_none() && skills.rank(Line::Engineering).rank >= SPEC_SKILL
}

/// Whether a saved learned spell is one this build knows: a recipe or a specialisation.
pub(crate) fn known_spell(spell: u32) -> bool {
    recipe(spell).is_some() || Spec::of_spell(spell).is_some()
}

/// The tradeskill window's header for a product: its item class.
fn group_name(class: u32) -> &'static str {
    match class {
        0 => "Consumable",
        1 => "Container",
        2 => "Weapon",
        4 => "Armor",
        _ => "Trade Goods",
    }
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
                // The product's class is the header: Consumable, Armor, Trade Goods…
                group: product.map(|p| (p.class, 0, group_name(p.class).to_string())),
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
/// its reagents, and what it does to the piece it goes on; and the items Enchanting makes
/// (Enchanted Thorium and Leather), as the 1.12 window lists them beside the enchants.
pub(crate) fn craft_state(bags: &Bags, skills: &Skills) -> CraftState {
    let r = skills.rank(Line::Enchanting);
    let recipes = ENCHANTING
        .iter()
        .filter(|rec| known(rec, skills))
        .map(|rec| {
            let product = items::template(rec.product);
            let (icon, description) = match rec.enchant {
                Some(e) => (
                    ENCHANT_ICON.to_string(),
                    format!(
                        "Permanently enchant the {} piece you wear: {}. It takes hold from your \
                         next run.",
                        e.slot.label().to_lowercase(),
                        enchant_line(e)
                    ),
                ),
                None => (
                    product.map_or_else(|| ENCHANT_ICON.to_string(), items::icon_path),
                    format!("Creates {}.", product.map_or(rec.name, |p| p.name)),
                ),
            };
            CraftRecipe {
                spell_id: rec.spell,
                name: rec.name.to_string(),
                sub_name: String::new(),
                difficulty: crate::ui_tradeskill::difficulty(
                    u32::from(r.rank),
                    u32::from(rec.trivial.0),
                    u32::from(rec.trivial.1),
                ),
                num_available: available(bags, rec),
                icon: Some(icon),
                description: Some(description),
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
            }
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
            if r.spec.is_some() {
                assert_eq!(r.line, Line::Engineering, "{}", r.name);
                assert!(r.req >= SPEC_SKILL, "{}", r.name);
            }
        }
        for line in [
            Line::Alchemy,
            Line::Cooking,
            Line::FirstAid,
            Line::Enchanting,
            Line::Blacksmithing,
            Line::Leatherworking,
            Line::Tailoring,
            Line::Engineering,
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
            used.extend(n.rares.iter().map(|&(i, _)| i));
        }
        for level in 0..=63 {
            used.insert(nodes::skin(level).0);
            used.insert(nodes::meat(level));
            used.insert(nodes::cloth(level));
            for name in ["Devilsaur", "Core Hound", "Widow", "Raptor"] {
                used.extend(nodes::beast_extra(name, level).map(|(i, _)| i));
            }
        }
        for boss in nodes::MC_DROPS {
            used.extend(boss.1.iter().map(|&(i, _)| i));
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
                    // `BasePoints` plus the die's roll, at least one: vmangos's `DoCreateItem`
                    // makes one where the transmutes' sideless die adds nothing.
                    let count = (s.effect_base_points[0] + s.effect_die_sides[0]).max(1);
                    s.effects[0] == CREATE_ITEM
                        && s.effect_item_type[0] == r.product
                        && count == r.made as i32
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
        for (id, name) in [
            FORGE,
            COOKING_FIRE,
            ALCHEMY_LAB,
            ANVIL,
            BLACK_ANVIL,
            BLACK_FORGE,
            MOONWELL,
        ] {
            if focus.name(id) != Some(name) {
                bad.push(format!("focus {id} is {:?}, not {name}", focus.name(id)));
            }
        }
        for (spell, name, line) in [
            (SMELTING_SPELL, "Smelting", Line::Mining),
            (DISENCHANT_SPELL, "Disenchant", Line::Enchanting),
            (
                Spec::Gnomish.spell(),
                Spec::Gnomish.name(),
                Line::Engineering,
            ),
            (Spec::Goblin.spell(), Spec::Goblin.name(), Line::Engineering),
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
        for spec in Spec::ALL {
            let icon = spells.get(spec.spell()).and_then(|s| s.icon.as_deref());
            if !icon.is_some_and(|i| i.eq_ignore_ascii_case(spec.icon())) {
                bad.push(format!("{}: icon {icon:?}", spec.name()));
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    /// `SPELL_EFFECT_LEARN_SPELL`, a recipe item's use.
    const LEARN_SPELL: u32 = 36;

    /// The recipe items are real: each one's name is a `Spell.dbc` spell (the item's use: "Plans:
    /// Lionheart Helm") whose `SPELL_EFFECT_LEARN_SPELL` teaches the recipe it is read for. Its
    /// entry is from memory. Every mismatch is listed.
    #[test]
    fn recipe_items_teach_their_recipes_in_the_client() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open the MPQ chain");
        let spells = benilla_formats::load_spell_catalog(&mut chain).expect("Spell.dbc");
        let mut teaches: std::collections::HashMap<&str, Vec<u32>> =
            std::collections::HashMap::new();
        for id in 1..40_000 {
            if let Some(s) = spells.get(id).filter(|s| s.effects[0] == LEARN_SPELL) {
                teaches
                    .entry(s.name.as_str())
                    .or_default()
                    .push(s.effect_trigger_spell[0]);
            }
        }
        let mut bad = Vec::new();
        let mut checked = 0;
        for r in all() {
            let Learn::Drop(item) = r.learn else {
                continue;
            };
            let Some(def) = items::template(item) else {
                bad.push(format!("{}: no recipe item {item}", r.name));
                continue;
            };
            checked += 1;
            if !teaches
                .get(def.name)
                .is_some_and(|learnt| learnt.contains(&r.spell))
            {
                bad.push(format!(
                    "{}: no spell named it teaches {} ({:?})",
                    def.name,
                    r.spell,
                    teaches.get(def.name)
                ));
            }
        }
        assert!(checked >= 40, "{checked} recipe items");
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    /// The specialisations: neither until Engineering 200, then either, never both; a
    /// specialisation's recipes stay unknown without it.
    #[test]
    fn survivors_recipes_engineering_specialisations() {
        use std::collections::BTreeMap;
        let novice = Skills::from_save(&BTreeMap::from([(
            "engineering".to_string(),
            super::super::skills::Rank {
                rank: 150,
                max: 225,
            },
        )]));
        assert!(!spec_open(&novice), "not before 200");
        let mut tinker = Skills::from_save(&BTreeMap::from([(
            "engineering".to_string(),
            super::super::skills::Rank {
                rank: 250,
                max: 300,
            },
        )]));
        assert!(spec_open(&tinker));
        let chicken = recipe(12906).expect("Gnomish Battle Chicken");
        assert_eq!(chicken.spec, Some(Spec::Gnomish));
        assert!(!learnable(chicken, &tinker), "Gnomish only");
        tinker.learn(Spec::Gnomish.spell());
        assert_eq!(spec_of(&tinker), Some(Spec::Gnomish));
        assert!(!spec_open(&tinker), "one at a time");
        assert!(learnable(chicken, &tinker));
        tinker.learn(chicken.spell);
        assert!(known(chicken, &tinker));
        let sapper = recipe(12760).expect("Goblin Sapper Charge");
        assert_eq!(sapper.spec, Some(Spec::Goblin));
        assert!(!learnable(sapper, &tinker), "the rival's");
        assert!(known_spell(Spec::Goblin.spell()) && known_spell(12760));
        assert!(!known_spell(1));
        // The window lists the chicken, and not the sapper.
        let state = trade_skill_state(&Bags::default(), &tinker, Line::Engineering, 0);
        assert!(state.recipes.iter().any(|r| r.spell_id == 12906));
        assert!(!state.recipes.iter().any(|r| r.spell_id == 12760));
    }
}
