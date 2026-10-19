//! The professions' skills (docs/PROFESSIONS.md, 4.6, 5): the four gathering lines and P2's four
//! crafting ones, their 1.12 `SkillLine.dbc` ids, the player's ranks and learned recipes, and the
//! skill-up rule. A gather's bands are vmangos's `UpdateGatherSkill` (`SkillGainChance`): against
//! a node's requirement R, orange below R+25 always gains, yellow to R+50 three times in four,
//! green to R+100 one in four, grey never.
//!
//! Every hero knows every line from rank 1 at Apprentice (cap 75); the camp's trainer sells the
//! higher ranks ([`TrainerRank`]), each a real 1.12 rank spell whose `SPELL_EFFECT_SKILL` (118)
//! names the line and the step. A craft rises on its recipe's bands ([`craft_gain_chance`]). The
//! recipes a hero learned beyond its skill's (bought from the trainer, read off a dropped recipe)
//! are kept here too ([`Skills::learn`]).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// A profession's skill line: `SkillLine.dbc` 186 Mining, 182 Herbalism, 393 Skinning, 356
/// Fishing, 171 Alchemy, 185 Cooking, 129 First Aid, 333 Enchanting. A new line goes on the end:
/// a [`super::gather::YieldMsg`] carries one on the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) enum Line {
    Mining,
    Herbalism,
    Skinning,
    Fishing,
    Alchemy,
    Cooking,
    FirstAid,
    Enchanting,
}

impl Line {
    pub(crate) const ALL: [Line; 8] = [
        Line::Mining,
        Line::Herbalism,
        Line::Skinning,
        Line::Fishing,
        Line::Alchemy,
        Line::Cooking,
        Line::FirstAid,
        Line::Enchanting,
    ];

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Line::Mining => "Mining",
            Line::Herbalism => "Herbalism",
            Line::Skinning => "Skinning",
            Line::Fishing => "Fishing",
            Line::Alchemy => "Alchemy",
            Line::Cooking => "Cooking",
            Line::FirstAid => "First Aid",
            Line::Enchanting => "Enchanting",
        }
    }

    /// The save's key.
    pub(crate) const fn slug(self) -> &'static str {
        match self {
            Line::Mining => "mining",
            Line::Herbalism => "herbalism",
            Line::Skinning => "skinning",
            Line::Fishing => "fishing",
            Line::Alchemy => "alchemy",
            Line::Cooking => "cooking",
            Line::FirstAid => "first_aid",
            Line::Enchanting => "enchanting",
        }
    }

    /// The gather spell its casting bar and visuals come from: Mining 2575, Herb Gathering 2366,
    /// Skinning 8613, Fishing 7620. A crafting line's is its Apprentice rank spell.
    pub(crate) const fn spell(self) -> u32 {
        self.rank_spells()[0]
    }

    /// The spell's name, as the casting bar and the trainer show it.
    pub(crate) const fn spell_name(self) -> &'static str {
        match self {
            Line::Herbalism => "Herb Gathering",
            other => other.name(),
        }
    }

    /// Fishing, Cooking and First Aid are secondary skills; the others are professions.
    pub(crate) const fn secondary(self) -> bool {
        matches!(self, Line::Fishing | Line::Cooking | Line::FirstAid)
    }

    /// `SkillLine.dbc`'s id.
    pub(crate) const fn skill_line(self) -> u32 {
        match self {
            Line::Mining => 186,
            Line::Herbalism => 182,
            Line::Skinning => 393,
            Line::Fishing => 356,
            Line::Alchemy => 171,
            Line::Cooking => 185,
            Line::FirstAid => 129,
            Line::Enchanting => 333,
        }
    }

    /// The rank spells, Apprentice to Artisan: the trainer's services. A gathering line's
    /// Apprentice one is the gather spell itself.
    pub(crate) const fn rank_spells(self) -> [u32; 4] {
        match self {
            Line::Mining => [2575, 2576, 3564, 10248],
            Line::Herbalism => [2366, 2368, 3570, 11993],
            Line::Skinning => [8613, 8617, 8618, 10768],
            Line::Fishing => [7620, 7731, 7732, 18248],
            Line::Alchemy => [2259, 3101, 3464, 11611],
            Line::Cooking => [2550, 3102, 3413, 18260],
            Line::FirstAid => [3273, 3274, 7924, 10846],
            Line::Enchanting => [7411, 7412, 7413, 13920],
        }
    }

    /// The rank spells' icon (`SpellIcon.dbc`, shared by the four ranks).
    pub(crate) const fn icon(self) -> &'static str {
        match self {
            Line::Mining => "Interface\\Icons\\Trade_Mining",
            Line::Herbalism => "Interface\\Icons\\Spell_Nature_NatureTouchGrow",
            Line::Skinning => "Interface\\Icons\\INV_Misc_Pelt_Wolf_01",
            Line::Fishing => "Interface\\Icons\\Trade_Fishing",
            Line::Alchemy => "Interface\\Icons\\Trade_Alchemy",
            Line::Cooking => "Interface\\Icons\\INV_Misc_Food_15",
            Line::FirstAid => "Interface\\Icons\\Spell_Holy_SealOfSacrifice",
            Line::Enchanting => "Interface\\Icons\\Trade_Engraving",
        }
    }
}

/// The skill cap a fresh hero starts at: Apprentice.
pub(crate) const APPRENTICE_CAP: u16 = 75;
/// The highest cap, Artisan's.
pub(crate) const MAX_RANK: u16 = 300;

/// A rank the trainer sells, in order: its `SpellNameSubtext`, the cap it raises the line to, the
/// skill it needs (1.12's: 50 for Journeyman, 125 for Expert, 200 for Artisan) and its price in
/// copper. The prices are ours, in 1.12's coin, sized against a run's banked gold (about 60
/// silver): Journeyman after a run or two, Artisan after fifteen or so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TrainerRank {
    pub(crate) name: &'static str,
    pub(crate) cap: u16,
    pub(crate) needs: u16,
    pub(crate) cost: u32,
}

/// Apprentice to Artisan; Apprentice is known from the start and costs nothing.
pub(crate) const TRAINER_RANKS: [TrainerRank; 4] = [
    TrainerRank {
        name: "Apprentice",
        cap: APPRENTICE_CAP,
        needs: 0,
        cost: 0,
    },
    TrainerRank {
        name: "Journeyman",
        cap: 150,
        needs: 50,
        cost: 10_000,
    },
    TrainerRank {
        name: "Expert",
        cap: 225,
        needs: 125,
        cost: 40_000,
    },
    TrainerRank {
        name: "Artisan",
        cap: MAX_RANK,
        needs: 200,
        cost: 100_000,
    },
];

/// Why the trainer refused a rank.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TrainError {
    /// Known already, or not the next rank.
    NotNext,
    /// The line's skill is below the rank's requirement.
    LowSkill,
    /// The purse cannot pay for it.
    NoMoney,
}

/// A gather's colour against its requirement, as the 1.12 tooltip and tracking colour it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Difficulty {
    /// Above the player's skill: it cannot be gathered.
    Red,
    Orange,
    Yellow,
    Green,
    Grey,
}

/// The colour of a requirement `req` for a player at `rank`.
pub(crate) fn difficulty(rank: u16, req: u16) -> Difficulty {
    let req = req.max(1);
    if rank < req {
        Difficulty::Red
    } else if rank < req + 25 {
        Difficulty::Orange
    } else if rank < req + 50 {
        Difficulty::Yellow
    } else if rank < req + 100 {
        Difficulty::Green
    } else {
        Difficulty::Grey
    }
}

/// The chance, in percent, that a gather at `req` raises a skill at `rank`.
pub(crate) fn gain_chance(rank: u16, req: u16) -> u32 {
    match difficulty(rank, req) {
        Difficulty::Red | Difficulty::Grey => 0,
        Difficulty::Orange => 100,
        Difficulty::Yellow => 75,
        Difficulty::Green => 25,
    }
}

/// The chance, in percent, that a craft with `SkillLineAbility`'s trivial bands `low` and `high`
/// raises a skill at `rank`: vmangos's `SkillGainChance`, yellow from `low`, green from the
/// midpoint and grey from `high`, on the bands `ui_tradeskill::difficulty` colours the window by.
pub(crate) fn craft_gain_chance(rank: u16, low: u16, high: u16) -> u32 {
    use benilla_ui::script::TradeSkillDifficulty as D;
    match crate::ui_tradeskill::difficulty(u32::from(rank), u32::from(low), u32::from(high)) {
        D::Optimal => 100,
        D::Medium => 75,
        D::Easy => 25,
        D::Trivial => 0,
    }
}

/// One line's rank and cap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Rank {
    pub(crate) rank: u16,
    pub(crate) max: u16,
}

impl Default for Rank {
    fn default() -> Self {
        Self {
            rank: 1,
            max: APPRENTICE_CAP,
        }
    }
}

impl Rank {
    /// The trainer rank this cap is: 0 Apprentice to 3 Artisan.
    pub(crate) fn step(self) -> usize {
        TRAINER_RANKS
            .iter()
            .rposition(|r| r.cap <= self.max)
            .unwrap_or(0)
    }
}

/// The player's profession skills, and the recipes learned beyond them.
#[derive(bevy::prelude::Resource, Clone, Debug, Default, PartialEq)]
pub(crate) struct Skills {
    ranks: BTreeMap<Line, Rank>,
    /// Recipe spells learned from the trainer or a recipe item: `[professions] recipes`.
    learned: BTreeSet<u32>,
    /// Bumped on every change, the save's gate.
    generation: u64,
}

impl Skills {
    /// Whether recipe `spell` was learned from the trainer or a recipe item.
    pub(crate) fn learned(&self, spell: u32) -> bool {
        self.learned.contains(&spell)
    }

    /// Learn recipe `spell`; false when it was known already.
    pub(crate) fn learn(&mut self, spell: u32) -> bool {
        let new = self.learned.insert(spell);
        if new {
            self.generation = self.generation.wrapping_add(1);
        }
        new
    }

    /// The learned recipes, for the save.
    pub(crate) fn learned_recipes(&self) -> Vec<u32> {
        self.learned.iter().copied().collect()
    }

    /// The saved recipes back; the caller drops the ones this build does not know.
    pub(crate) fn with_learned(mut self, spells: impl IntoIterator<Item = u32>) -> Self {
        self.learned.extend(spells);
        self
    }

    pub(crate) fn rank(&self, line: Line) -> Rank {
        self.ranks.get(&line).copied().unwrap_or_default()
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    /// A gather at `req` finished: the skill-up roll, `roll` in 0..100. The new rank when it rose.
    pub(crate) fn gathered(&mut self, line: Line, req: u16, roll: u32) -> Option<u16> {
        let chance = gain_chance(self.rank(line).rank, req);
        self.roll_up(line, chance, roll)
    }

    /// A craft on `line` with trivial bands `low` and `high` finished: the skill-up roll.
    pub(crate) fn crafted(&mut self, line: Line, low: u16, high: u16, roll: u32) -> Option<u16> {
        let chance = craft_gain_chance(self.rank(line).rank, low, high);
        self.roll_up(line, chance, roll)
    }

    /// One point on `line` when `roll` (0..100) falls under `chance` and the cap allows it.
    fn roll_up(&mut self, line: Line, chance: u32, roll: u32) -> Option<u16> {
        let mut r = self.rank(line);
        if r.rank >= r.max || roll >= chance {
            return None;
        }
        r.rank += 1;
        self.ranks.insert(line, r);
        self.generation = self.generation.wrapping_add(1);
        Some(r.rank)
    }

    /// Whether `line` may train trainer rank `step` (1 Journeyman to 3 Artisan) now.
    pub(crate) fn can_train(&self, line: Line, step: usize) -> Result<(), TrainError> {
        let r = self.rank(line);
        let Some(next) = TRAINER_RANKS.get(step).filter(|_| step == r.step() + 1) else {
            return Err(TrainError::NotNext);
        };
        if r.rank < next.needs {
            return Err(TrainError::LowSkill);
        }
        Ok(())
    }

    /// Train rank `step` on `line`: its cap rises to the rank's. The price is the caller's.
    pub(crate) fn train(&mut self, line: Line, step: usize) -> Result<(), TrainError> {
        self.can_train(line, step)?;
        let mut r = self.rank(line);
        r.max = TRAINER_RANKS[step].cap;
        self.ranks.insert(line, r);
        self.generation = self.generation.wrapping_add(1);
        Ok(())
    }

    /// The save's section: slug → rank.
    pub(crate) fn to_save(&self) -> BTreeMap<String, Rank> {
        Line::ALL
            .iter()
            .map(|&l| (l.slug().to_string(), self.rank(l)))
            .collect()
    }

    /// From the save; an unknown slug is skipped, a cap is snapped up to a trainer rank's, and a
    /// rank is clamped to its cap.
    pub(crate) fn from_save(save: &BTreeMap<String, Rank>) -> Self {
        let mut skills = Self::default();
        for (slug, r) in save {
            if let Some(line) = Line::ALL.iter().find(|l| l.slug() == slug) {
                let max = TRAINER_RANKS
                    .iter()
                    .map(|t| t.cap)
                    .find(|&cap| cap >= r.max)
                    .unwrap_or(MAX_RANK);
                skills.ranks.insert(
                    *line,
                    Rank {
                        rank: r.rank.clamp(1, max),
                        max,
                    },
                );
            }
        }
        skills
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// vmangos's `UpdateGatherSkill` against a node at R: gray at R+100, green at R+50, yellow at
    /// R+25, with `SkillGainChance`'s 0 / 25 / 75 / 100.
    #[test]
    fn survivors_skills_bands_follow_vmangos() {
        let r = 65; // Tin
        assert_eq!(difficulty(64, r), Difficulty::Red);
        assert_eq!(gain_chance(64, r), 0, "too low: nothing starts");
        assert_eq!(gain_chance(65, r), 100);
        assert_eq!(gain_chance(89, r), 100);
        assert_eq!(difficulty(90, r), Difficulty::Yellow);
        assert_eq!(gain_chance(90, r), 75);
        assert_eq!(gain_chance(114, r), 75);
        assert_eq!(difficulty(115, r), Difficulty::Green);
        assert_eq!(gain_chance(115, r), 25);
        assert_eq!(gain_chance(164, r), 25);
        assert_eq!(difficulty(165, r), Difficulty::Grey);
        assert_eq!(gain_chance(165, r), 0);
        // A requirement of 0 (Copper, Peacebloom) reads as 1.
        assert_eq!(gain_chance(1, 0), 100);
    }

    #[test]
    fn survivors_skills_rise_by_the_roll_and_stop_at_the_cap() {
        let mut s = Skills::default();
        assert_eq!(s.rank(Line::Mining).rank, 1);
        assert_eq!(
            s.rank(Line::Mining).max,
            APPRENTICE_CAP,
            "a fresh hero is an Apprentice"
        );
        assert_eq!(s.gathered(Line::Mining, 1, 99), Some(2), "orange always");
        // Yellow: a roll under 75 gains, 75 and up does not.
        let mut y = Skills::from_save(&BTreeMap::from([(
            "herbalism".to_string(),
            Rank { rank: 30, max: 300 },
        )]));
        assert_eq!(y.gathered(Line::Herbalism, 1, 75), None);
        assert_eq!(y.gathered(Line::Herbalism, 1, 74), Some(31));
        let mut capped = Skills::from_save(&BTreeMap::from([(
            "mining".to_string(),
            Rank { rank: 75, max: 75 },
        )]));
        assert_eq!(capped.gathered(Line::Mining, 75, 0), None, "at the cap");
    }

    #[test]
    fn survivors_skills_save_round_trips_and_skips_unknown_lines() {
        let mut s = Skills::default();
        s.gathered(Line::Fishing, 1, 0);
        let mut save = s.to_save();
        assert_eq!(save["fishing"].rank, 2);
        save.insert("tailoring".into(), Rank { rank: 9, max: 75 });
        save.insert("cooking".into(), Rank { rank: 9, max: 75 });
        save.insert(
            "skinning".into(),
            Rank {
                rank: 900,
                max: 900,
            },
        );
        save.insert("mining".into(), Rank { rank: 80, max: 100 });
        let back = Skills::from_save(&save);
        assert_eq!(back.rank(Line::Fishing).rank, 2);
        assert_eq!(
            back.rank(Line::Cooking).rank,
            9,
            "a crafting line reads back"
        );
        assert!(!back.to_save().contains_key("tailoring"), "not a line yet");
        assert_eq!(back.rank(Line::Skinning).rank, MAX_RANK, "clamped");
        assert_eq!(
            back.rank(Line::Mining),
            Rank { rank: 80, max: 150 },
            "an odd cap snaps up to Journeyman's"
        );
    }

    /// The trainer sells the next rank only, once the skill reaches its requirement.
    #[test]
    fn survivors_skills_train_rank_by_rank() {
        let mut s = Skills::default();
        assert_eq!(s.rank(Line::Mining).step(), 0);
        assert_eq!(s.train(Line::Mining, 1), Err(TrainError::LowSkill));
        assert_eq!(s.train(Line::Mining, 2), Err(TrainError::NotNext));
        assert_eq!(s.train(Line::Mining, 0), Err(TrainError::NotNext), "known");
        let mut s = Skills::from_save(&BTreeMap::from([(
            "mining".to_string(),
            Rank { rank: 75, max: 75 },
        )]));
        assert_eq!(s.gathered(Line::Mining, 1, 0), None, "capped at 75");
        let g = s.generation();
        assert_eq!(s.train(Line::Mining, 1), Ok(()));
        assert!(s.generation() > g);
        assert_eq!(s.rank(Line::Mining).max, 150);
        assert_eq!(s.rank(Line::Mining).step(), 1);
        assert_eq!(s.gathered(Line::Mining, 60, 0), Some(76), "the cap rose");
        assert_eq!(
            s.train(Line::Mining, 2),
            Err(TrainError::LowSkill),
            "Expert needs 125"
        );
        assert_eq!(s.rank(Line::Herbalism).max, APPRENTICE_CAP, "per line");
    }

    /// Crafting rises on `SkillLineAbility`'s bands: Smelt Copper's 25 / 70 (midpoint 47).
    #[test]
    fn survivors_skills_crafting_bands() {
        assert_eq!(craft_gain_chance(1, 25, 70), 100);
        assert_eq!(craft_gain_chance(25, 25, 70), 75);
        assert_eq!(craft_gain_chance(47, 25, 70), 25);
        assert_eq!(craft_gain_chance(70, 25, 70), 0);
        let mut s = Skills::default();
        assert_eq!(s.crafted(Line::Mining, 25, 70, 99), Some(2));
    }

    /// The rank spells are real: each is its line's, its `SpellNameSubtext` the rank's name, its
    /// icon the line's, and its `SPELL_EFFECT_SKILL` (118) names the line and the step
    /// (`EffectBasePoints` + 1). The tracking spells are Mining's and Herbalism's, with their
    /// icons. Every mismatch is listed.
    #[test]
    fn survivors_skills_rank_spells_resolve_in_the_client() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open the MPQ chain");
        let spells = benilla_formats::load_spell_catalog(&mut chain).expect("Spell.dbc");
        let skills = benilla_formats::load_skill_line_catalog(&mut chain).expect("SkillLine");
        let mut bad = Vec::new();
        for line in Line::ALL {
            if skills.line(line.skill_line()).map(|l| l.name.as_str()) != Some(line.name()) {
                bad.push(format!("{}: SkillLine {}", line.name(), line.skill_line()));
            }
            for (step, &id) in line.rank_spells().iter().enumerate() {
                let Some(s) = spells.get(id) else {
                    bad.push(format!("{}: no spell {id}", line.name()));
                    continue;
                };
                let skill = s.effects.iter().position(|&e| e == 118);
                // Cooking's Apprentice subtext carries a trailing space in the client.
                let ok = s.name == line.spell_name()
                    && s.rank.as_deref().map(str::trim_end) == Some(TRAINER_RANKS[step].name)
                    && s.icon
                        .as_deref()
                        .is_some_and(|i| i.eq_ignore_ascii_case(line.icon()))
                    && skills.spell_to_line(id) == Some(line.skill_line())
                    && skill.is_some_and(|i| {
                        s.effect_misc_value[i] == line.skill_line() as i32
                            && s.effect_base_points[i] == step as i32
                    });
                if !ok {
                    bad.push(format!(
                        "{} {}: spell {id} is {:?} {:?} {:?}",
                        line.name(),
                        TRAINER_RANKS[step].name,
                        s.name,
                        s.rank,
                        s.icon
                    ));
                }
            }
        }
        for t in super::super::tracking::Tracking::ALL {
            let Some((id, name, icon, line)) = t.spell() else {
                continue;
            };
            let ok = spells.get(id).is_some_and(|s| {
                s.name == name
                    && s.icon
                        .as_deref()
                        .is_some_and(|i| i.eq_ignore_ascii_case(icon))
            }) && skills.spell_to_line(id) == Some(line.skill_line());
            if !ok {
                bad.push(format!("{name}: spell {id}"));
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }
}
