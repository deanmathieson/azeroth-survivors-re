//! Disenchant (docs/PROFESSIONS.md, 5.2; 1.12's spell 13262): an Uncommon or better piece of
//! gear broken into Enchanting's dusts, essences and shards.
//!
//! - **In the bags:** with the Enchanting window open at camp, a right-click on an Uncommon or
//!   better armour or weapon disenchants it ([`from_bags`]), as the cast's item pick would.
//! - **In a run:** the jewelry choice's Disenchant takes the offer's rarest piece apart
//!   ([`yield_for`]).
//!
//! **Yields** follow 1.12's table with our odds: Uncommon gives dust (75%), an essence (20%) or a
//! shard (5%); Rare a shard; Epic one or two of the band's best shard. The tier follows the
//! battleground's band ([`TIERS`]). Disenchanting rolls Enchanting on the spell's own bands.

use super::super::Rng;
use super::gather::{CastLine, YieldMsg, NO_SKILL_UP};
use super::inventory::Bags;
use super::items::{self, *};
use super::nodes::Band;
use super::skills::{Line, Skills};

/// Disenchant's `SkillLineAbility` bands: it skills Enchanting up to 60 as a craft would.
pub(crate) const DISENCHANT_TRIVIAL: (u16, u16) = (20, 60);

/// One band's materials: the dusts, essences and shards an Uncommon piece gives (one of each list
/// at random), and the shard a Rare or Epic gives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Tier {
    pub(crate) dust: &'static [u32],
    pub(crate) essence: &'static [u32],
    pub(crate) shard: &'static [u32],
    /// What a Rare gives, and an Epic one or two of.
    pub(crate) best: u32,
}

/// The tiers by band, Elwynn's Strange Dust to Naxxramas's Large Brilliant Shards. Every material
/// the curated enchants need comes from one.
pub(crate) static TIERS: [(Band, Tier); 6] = [
    (
        Band::B1,
        Tier {
            dust: &[STRANGE_DUST],
            essence: &[LESSER_MAGIC_ESSENCE],
            shard: &[SMALL_GLIMMERING_SHARD],
            best: SMALL_GLIMMERING_SHARD,
        },
    ),
    (
        Band::B15,
        Tier {
            dust: &[STRANGE_DUST, SOUL_DUST],
            essence: &[GREATER_MAGIC_ESSENCE, LESSER_ASTRAL_ESSENCE],
            shard: &[SMALL_GLIMMERING_SHARD],
            best: LARGE_GLIMMERING_SHARD,
        },
    ),
    (
        Band::B30,
        Tier {
            dust: &[SOUL_DUST, VISION_DUST],
            essence: &[
                GREATER_ASTRAL_ESSENCE,
                LESSER_MYSTIC_ESSENCE,
                GREATER_MYSTIC_ESSENCE,
            ],
            shard: &[LARGE_GLIMMERING_SHARD],
            best: LARGE_GLOWING_SHARD,
        },
    ),
    (
        Band::B45,
        Tier {
            dust: &[VISION_DUST, DREAM_DUST],
            essence: &[LESSER_NETHER_ESSENCE, GREATER_NETHER_ESSENCE],
            shard: &[LARGE_GLOWING_SHARD, SMALL_RADIANT_SHARD],
            best: LARGE_RADIANT_SHARD,
        },
    ),
    (
        Band::B55,
        Tier {
            dust: &[DREAM_DUST, ILLUSION_DUST],
            essence: &[LESSER_ETERNAL_ESSENCE, GREATER_ETERNAL_ESSENCE],
            shard: &[SMALL_RADIANT_SHARD, SMALL_BRILLIANT_SHARD],
            best: LARGE_BRILLIANT_SHARD,
        },
    ),
    (
        Band::Raid,
        Tier {
            dust: &[ILLUSION_DUST],
            essence: &[GREATER_ETERNAL_ESSENCE],
            shard: &[SMALL_BRILLIANT_SHARD],
            best: LARGE_BRILLIANT_SHARD,
        },
    ),
];

/// `band`'s tier.
pub(crate) fn tier(band: Band) -> &'static Tier {
    TIERS
        .iter()
        .find(|(b, _)| *b == band)
        .map_or(&TIERS[0].1, |(_, t)| t)
}

/// What disenchanting a piece of `quality` (2 Uncommon, 3 Rare, 4 Epic and up) in `band` gives,
/// `(item, count)`; nothing below Uncommon.
pub(crate) fn roll(quality: u32, band: Band, rng: &mut Rng) -> Vec<(u32, u32)> {
    let t = tier(band);
    let pick = |list: &[u32], rng: &mut Rng| list[rng.index(list.len())];
    match quality {
        0 | 1 => Vec::new(),
        2 => {
            let r = rng.index(100);
            if r < 75 {
                vec![(pick(t.dust, rng), 1 + rng.index(3) as u32)]
            } else if r < 95 {
                vec![(pick(t.essence, rng), 1 + rng.index(2) as u32)]
            } else {
                vec![(pick(t.shard, rng), 1)]
            }
        }
        3 => vec![(t.best, 1)],
        _ => vec![(t.best, 1 + rng.index(2) as u32)],
    }
}

/// Whether a bag item can be disenchanted: an Uncommon or better armour or weapon.
pub(crate) fn disenchantable(def: &items::ItemDef) -> bool {
    def.quality >= 2 && matches!(def.class, 2 | 4)
}

/// One right-click with the Enchanting window open: the piece at `(bag, slot)` broken into
/// materials, which go into the bags, each announced; Enchanting rolls on Disenchant's bands. A
/// piece that cannot be disenchanted, or materials with no room, change nothing.
pub(crate) fn from_bags(
    bags: &mut Bags,
    skills: &mut Skills,
    at: (i64, u32),
    band: Band,
    rng: &mut Rng,
    lines: &mut Vec<CastLine>,
) {
    let Some(def) = bags.get(at.0, at.1).and_then(|s| items::template(s.item)) else {
        return;
    };
    if !disenchantable(def) {
        lines.push(CastLine::Refused(
            "SPELL_FAILED_CANT_BE_DISENCHANTED",
            "Item cannot be disenchanted",
        ));
        return;
    }
    let mats = roll(def.quality, band, rng);
    let mut trial = bags.clone();
    if trial.destroy(at.0, at.1, 1).is_err() || trial.exchange(&[], &mats).is_err() {
        lines.push(CastLine::Full);
        return;
    }
    *bags = trial;
    for (item, n) in mats {
        lines.push(CastLine::Loot(item, n as u16));
    }
    let (low, high) = DISENCHANT_TRIVIAL;
    if let Some(rank) = skills.crafted(Line::Enchanting, low, high, rng.index(100) as u32) {
        lines.push(CastLine::SkillUp(Line::Enchanting, rank));
    }
}

/// The jewelry choice's Disenchant: a piece of `quality` taken apart in `band`, as a yield for this
/// player's bags.
pub(crate) fn yield_for(quality: u32, band: Band, rng: &mut Rng) -> YieldMsg {
    YieldMsg {
        line: Line::Enchanting,
        req: NO_SKILL_UP,
        items: roll(quality, band, rng)
            .into_iter()
            .map(|(item, n)| (item, n as u16))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The yields follow the table: dust most of the time from an Uncommon, a shard from a Rare,
    /// one or two of the best from an Epic, nothing from a Common.
    #[test]
    fn survivors_disenchant_tables() {
        let mut rng = Rng(11);
        let mut dust = 0;
        for _ in 0..1000 {
            let got = roll(2, Band::B1, &mut rng);
            assert_eq!(got.len(), 1);
            if got[0].0 == STRANGE_DUST {
                dust += 1;
            } else {
                assert!([LESSER_MAGIC_ESSENCE, SMALL_GLIMMERING_SHARD].contains(&got[0].0));
            }
        }
        assert!((680..=820).contains(&dust), "about 75%: {dust}");
        assert_eq!(
            roll(3, Band::B55, &mut rng),
            vec![(LARGE_BRILLIANT_SHARD, 1)]
        );
        let epic = roll(4, Band::B55, &mut rng);
        assert_eq!(epic[0].0, LARGE_BRILLIANT_SHARD);
        assert!((1..=2).contains(&epic[0].1));
        assert!(roll(1, Band::B1, &mut rng).is_empty());
        // Every material an enchant needs comes from some tier (Enchanted Thorium and Leather,
        // which make items, take a bar and a leather as well).
        for r in super::super::recipes::book(Line::Enchanting)
            .iter()
            .filter(|r| r.enchant.is_some())
        {
            for &(item, _) in r.reagents {
                let rare = [ESSENCE_OF_FIRE, RIGHTEOUS_ORB].contains(&item);
                let found = TIERS.iter().any(|(_, t)| {
                    t.dust.contains(&item)
                        || t.essence.contains(&item)
                        || t.shard.contains(&item)
                        || t.best == item
                });
                assert!(rare || found, "{}: {item} drops nowhere", r.name);
            }
        }
    }

    /// The helm goes, its materials come; ore cannot be disenchanted.
    #[test]
    fn survivors_disenchant_from_the_bags() {
        let mut bags = Bags::default();
        bags.add(HELM_OF_VALOR, 1);
        bags.add(COPPER_ORE, 3);
        let mut skills = Skills::default();
        let mut lines = Vec::new();
        from_bags(
            &mut bags,
            &mut skills,
            (0, 1),
            Band::B30,
            &mut Rng(3),
            &mut lines,
        );
        assert_eq!(bags.count(HELM_OF_VALOR), 0);
        assert_eq!(bags.count(LARGE_GLOWING_SHARD), 1, "a Rare's shard");
        assert!(lines.contains(&CastLine::Loot(LARGE_GLOWING_SHARD, 1)));
        assert_eq!(skills.rank(Line::Enchanting).rank, 2, "orange: always");
        lines.clear();
        from_bags(
            &mut bags,
            &mut skills,
            (0, 2),
            Band::B30,
            &mut Rng(3),
            &mut lines,
        );
        assert_eq!(bags.count(COPPER_ORE), 3);
        assert!(matches!(lines[0], CastLine::Refused(..)));
    }
}
