//! Smelting (docs/PROFESSIONS.md, 5.1): Mining's recipes, done at the camp's fire, which stands in
//! for a Forge. Each row is a real 1.12 `Smelt …` spell: its `Reagent[8]`, its
//! `SPELL_EFFECT_CREATE_ITEM` product, its `SkillLineAbility` trivial bands, its Forge focus and its
//! cast time are checked against the client by [`tests::smelting_recipes_resolve_in_the_client`].
//! The skill each needs is the trainer's (vmangos `npc_trainer.reqskillvalue`, from memory: the
//! client does not carry it). A recipe is known once Mining reaches it, as if its trainer had
//! taught it; only the ores the run's veins give are smelted (no Silver, Gold or Dark Iron yet).

use benilla_ui::script::{TradeSkillReagent, TradeSkillRecipe, TradeSkillState};

use super::inventory::{BagError, Bags};
use super::items::{self, *};
use super::skills::{Line, Skills};

/// Every Smelt spell's cast: `SpellCastTimes.dbc` row 14.
pub(crate) const SMELT_SECS: f32 = 3.0;
/// Every Smelt spell's focus, `SpellFocusObject.dbc`'s Forge (3), by its name: the tradeskill
/// window's "Requires: Forge", which the camp's fire meets.
pub(crate) const FORGE: &str = "Forge";

/// One Smelt spell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Recipe {
    pub(crate) spell: u32,
    pub(crate) name: &'static str,
    /// `(item, count)`, in `Reagent[8]`'s order.
    pub(crate) reagents: &'static [(u32, u32)],
    pub(crate) product: u32,
    /// Made per cast: `EffectBasePoints` plus its one die (Smelt Bronze's two, else one).
    pub(crate) made: u32,
    /// The Mining skill it is learned at.
    pub(crate) req: u16,
    /// `SkillLineAbility`'s `min_value` and `max_value`: yellow from the first, grey from the
    /// second.
    pub(crate) trivial: (u16, u16),
}

const fn smelt(
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
        reagents,
        product,
        made: 1,
        req,
        trivial,
    }
}

/// The curated Smelting book, in the order the trainer teaches it.
pub(crate) static SMELTING: &[Recipe] = &[
    smelt(
        2657,
        "Smelt Copper",
        &[(COPPER_ORE, 1)],
        COPPER_BAR,
        1,
        (25, 70),
    ),
    smelt(3304, "Smelt Tin", &[(TIN_ORE, 1)], TIN_BAR, 65, (50, 75)),
    Recipe {
        made: 2,
        ..smelt(
            2659,
            "Smelt Bronze",
            &[(COPPER_BAR, 1), (TIN_BAR, 1)],
            BRONZE_BAR,
            65,
            (65, 115),
        )
    },
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

/// The recipe a `DoTradeSkill` names.
pub(crate) fn recipe(spell: u32) -> Option<&'static Recipe> {
    SMELTING.iter().find(|r| r.spell == spell)
}

/// Whether a Miner at `rank` knows `r`.
pub(crate) fn known(r: &Recipe, rank: u16) -> bool {
    rank >= r.req
}

/// One cast: the reagents out of the bags and the bars in, or nothing at all.
pub(crate) fn smelt_once(bags: &mut Bags, r: &Recipe) -> Result<(), BagError> {
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

/// The stock tradeskill window's snapshot: every recipe the Miner knows, its colour on
/// `ui_tradeskill::difficulty`'s bands, its reagents counted in the bags, and the camp's Forge.
pub(crate) fn trade_skill_state(bags: &Bags, skills: &Skills, repeat: u32) -> TradeSkillState {
    let r = skills.rank(Line::Mining);
    let recipes = SMELTING
        .iter()
        .filter(|rec| known(rec, r.rank))
        .map(|rec| {
            let product = items::template(rec.product);
            TradeSkillRecipe {
                // Bars are Trade Goods (`ItemClass` 7, subclass 0), the one header.
                group: Some((7, 0, "Trade Goods".to_string())),
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
                tools: vec![(FORGE.to_string(), true)],
            }
        })
        .collect();
    TradeSkillState {
        line: Line::Mining.skill_line(),
        line_name: Line::Mining.name().to_string(),
        rank: u32::from(r.rank),
        max_rank: u32::from(r.max),
        recipes,
        repeat_count: repeat,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Smelting` (2656), the tradeskill opener whose window lists these.
    const SMELTING_SPELL: u32 = 2656;
    /// `SpellFocusObject.dbc`'s Forge.
    const FORGE_FOCUS: u32 = 3;

    #[test]
    fn survivors_recipes_every_item_is_authored() {
        for r in SMELTING {
            assert!(items::template(r.product).is_some(), "{}", r.name);
            for &(item, n) in r.reagents {
                assert!(items::template(item).is_some() && n >= 1, "{}", r.name);
            }
            assert!(r.trivial.0 <= r.trivial.1, "{}", r.name);
        }
    }

    /// A Miner sees only what they know; a cast takes the ore and makes the bar.
    #[test]
    fn survivors_recipes_smelt_copper_and_bronze() {
        let mut bags = Bags::default();
        bags.add(COPPER_ORE, 3);
        bags.add(TIN_ORE, 1);
        let skills = Skills::default();
        let book = trade_skill_state(&bags, &skills, 0);
        assert_eq!(book.line, 186);
        assert_eq!(book.max_rank, 75);
        assert_eq!(book.recipes.len(), 1, "rank 1 knows Smelt Copper only");
        assert_eq!(book.recipes[0].num_available, 3);
        assert_eq!(
            book.recipes[0].difficulty,
            benilla_ui::script::TradeSkillDifficulty::Optimal
        );
        let copper = recipe(2657).unwrap();
        smelt_once(&mut bags, copper).unwrap();
        assert_eq!((bags.count(COPPER_ORE), bags.count(COPPER_BAR)), (2, 1));
        let bronze = recipe(2659).unwrap();
        assert_eq!(available(&bags, bronze), 0, "no Tin Bar yet");
        assert_eq!(smelt_once(&mut bags, bronze), Err(BagError::NotFound));
        smelt_once(&mut bags, recipe(3304).unwrap()).unwrap();
        assert_eq!(available(&bags, bronze), 1);
        smelt_once(&mut bags, bronze).unwrap();
        assert_eq!(bags.count(BRONZE_BAR), 2, "two bars a cast");
        assert_eq!((bags.count(COPPER_BAR), bags.count(TIN_BAR)), (0, 0));
    }

    /// The recipes are real: each spell is a Mining `Smelt …` with these reagents, this product,
    /// these trivial bands, the Forge's focus and the 3 s cast; the opener is Mining's Smelting;
    /// and the vendor's Empty Vial is the real Minor Healing Potion recipe's (2330), with its two
    /// herbs and its product. Every mismatch is listed.
    #[test]
    fn smelting_recipes_resolve_in_the_client() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open the MPQ chain");
        let spells = benilla_formats::load_spell_catalog(&mut chain).expect("Spell.dbc");
        let skills = benilla_formats::load_skill_line_catalog(&mut chain).expect("SkillLine");
        let casts = benilla_formats::load_spell_cast_times(&mut chain).expect("cast times");
        let focus = benilla_formats::load_spell_focus_catalog(&mut chain).expect("focus");
        let mut bad = Vec::new();
        let reagents = |s: &benilla_formats::SpellDisplay| -> Vec<(u32, u32)> {
            s.reagents.iter().copied().filter(|r| r.0 != 0).collect()
        };
        for r in SMELTING {
            let Some(s) = spells.get(r.spell) else {
                bad.push(format!("{}: no spell {}", r.name, r.spell));
                continue;
            };
            let sla = skills.ability(r.spell);
            let cast = casts.get(s.casting_time_index).map(|c| c.base_ms);
            let ok = s.name == r.name
                && reagents(s) == r.reagents
                && s.effects[0] == 24
                && s.effect_item_type[0] == r.product
                && s.effect_base_points[0] + s.effect_die_sides[0] == r.made as i32
                && sla.is_some_and(|a| {
                    a.skill_id == Line::Mining.skill_line()
                        && (a.trivial_low, a.trivial_high)
                            == (u32::from(r.trivial.0), u32::from(r.trivial.1))
                })
                && s.requires_spell_focus == FORGE_FOCUS
                && cast == Some((SMELT_SECS * 1000.0) as i32);
            if !ok {
                bad.push(format!(
                    "{}: {:?} reagents {:?} makes {:?} sla {sla:?} focus {} cast {cast:?}",
                    r.name,
                    s.name,
                    reagents(s),
                    s.effect_item_type,
                    s.requires_spell_focus
                ));
            }
        }
        if focus.name(FORGE_FOCUS) != Some(FORGE) {
            bad.push(format!(
                "focus {FORGE_FOCUS} is {:?}",
                focus.name(FORGE_FOCUS)
            ));
        }
        let opener = spells.get(SMELTING_SPELL);
        if !(opener.is_some_and(|s| s.name == "Smelting")
            && skills.spell_to_line(SMELTING_SPELL) == Some(Line::Mining.skill_line()))
        {
            bad.push(format!("Smelting: spell {SMELTING_SPELL}"));
        }
        // Minor Healing Potion: Empty Vial, Peacebloom, Silverleaf, making the potion.
        match spells.get(2330) {
            Some(s) => {
                let mut got = reagents(s);
                got.sort_unstable();
                let mut want = vec![(EMPTY_VIAL, 1), (PEACEBLOOM, 1), (SILVERLEAF, 1)];
                want.sort_unstable();
                if s.name != "Minor Healing Potion"
                    || got != want
                    || s.effect_item_type[0] != MINOR_HEALING_POTION
                {
                    bad.push(format!(
                        "2330: {:?} {:?} makes {:?}",
                        s.name, got, s.effect_item_type
                    ));
                }
            }
            None => bad.push("no spell 2330".into()),
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }
}
