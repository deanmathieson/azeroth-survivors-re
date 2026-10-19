//! The four gathering skills (docs/PROFESSIONS.md, 4.6): their 1.12 `SkillLine.dbc` ids, the
//! player's ranks, and the skill-up rule, whose bands are vmangos's `UpdateGatherSkill`
//! (`SkillGainChance`): against a node's requirement R, orange below R+25 always gains, yellow to
//! R+50 three times in four, green to R+100 one in four, grey never.
//!
//! P1 has no trainers yet (they are P1's remaining work), so every hero knows all four lines from
//! rank 1 and the cap is Artisan's 300.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A gathering skill line: `SkillLine.dbc` 186 Mining, 182 Herbalism, 393 Skinning, 356 Fishing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) enum Line {
    Mining,
    Herbalism,
    Skinning,
    Fishing,
}

impl Line {
    pub(crate) const ALL: [Line; 4] =
        [Line::Mining, Line::Herbalism, Line::Skinning, Line::Fishing];

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Line::Mining => "Mining",
            Line::Herbalism => "Herbalism",
            Line::Skinning => "Skinning",
            Line::Fishing => "Fishing",
        }
    }

    /// The save's key.
    pub(crate) const fn slug(self) -> &'static str {
        match self {
            Line::Mining => "mining",
            Line::Herbalism => "herbalism",
            Line::Skinning => "skinning",
            Line::Fishing => "fishing",
        }
    }

    /// The gather spell its casting bar and visuals come from: Mining 2575, Herb Gathering 2366,
    /// Skinning 8613, Fishing 7620.
    pub(crate) const fn spell(self) -> u32 {
        match self {
            Line::Mining => 2575,
            Line::Herbalism => 2366,
            Line::Skinning => 8613,
            Line::Fishing => 7620,
        }
    }

    /// The spell's name, as the casting bar shows it.
    pub(crate) const fn spell_name(self) -> &'static str {
        match self {
            Line::Mining => "Mining",
            Line::Herbalism => "Herb Gathering",
            Line::Skinning => "Skinning",
            Line::Fishing => "Fishing",
        }
    }

    /// Fishing is a secondary skill; the other three are professions.
    pub(crate) const fn secondary(self) -> bool {
        matches!(self, Line::Fishing)
    }
}

/// The skill cap with no trainer: Artisan.
pub(crate) const MAX_RANK: u16 = 300;

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
            max: MAX_RANK,
        }
    }
}

/// The player's gathering skills.
#[derive(bevy::prelude::Resource, Clone, Debug, Default, PartialEq)]
pub(crate) struct Skills {
    ranks: BTreeMap<Line, Rank>,
    /// Bumped on every change, the save's gate.
    generation: u64,
}

impl Skills {
    pub(crate) fn rank(&self, line: Line) -> Rank {
        self.ranks.get(&line).copied().unwrap_or_default()
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    /// A gather at `req` finished: the skill-up roll, `roll` in 0..100. The new rank when it rose.
    pub(crate) fn gathered(&mut self, line: Line, req: u16, roll: u32) -> Option<u16> {
        let mut r = self.rank(line);
        if r.rank >= r.max || roll >= gain_chance(r.rank, req) {
            return None;
        }
        r.rank += 1;
        self.ranks.insert(line, r);
        self.generation = self.generation.wrapping_add(1);
        Some(r.rank)
    }

    /// The save's section: slug → rank.
    pub(crate) fn to_save(&self) -> BTreeMap<String, Rank> {
        Line::ALL
            .iter()
            .map(|&l| (l.slug().to_string(), self.rank(l)))
            .collect()
    }

    /// From the save; an unknown slug is skipped, a rank is clamped to its cap.
    pub(crate) fn from_save(save: &BTreeMap<String, Rank>) -> Self {
        let mut skills = Self::default();
        for (slug, r) in save {
            if let Some(line) = Line::ALL.iter().find(|l| l.slug() == slug) {
                let max = r.max.clamp(1, MAX_RANK);
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
        save.insert("cooking".into(), Rank { rank: 9, max: 75 });
        save.insert(
            "skinning".into(),
            Rank {
                rank: 900,
                max: 900,
            },
        );
        let back = Skills::from_save(&save);
        assert_eq!(back.rank(Line::Fishing).rank, 2);
        assert_eq!(back.rank(Line::Skinning).rank, MAX_RANK, "clamped");
    }
}
