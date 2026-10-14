//! The talents' section of each class's profile record, `ClassRecord::talents` in
//! [`crate::survivors::profile`]. Only the talents workstream edits this file.

use serde::{Deserialize, Serialize};

use super::Allocation;

/// One class's saved talents. Every field must read a save without it as its default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(in crate::survivors) struct TalentSave {
    /// Talent XP from every recorded run of the class. Its points come from this
    /// ([`super::points_for_xp`]), so unlearning never loses one.
    pub xp: u64,
    /// The learned ranks as `[talent id, rank]` pairs in id order: a TOML array of arrays.
    pub ranks: Vec<(u32, u8)>,
}

impl TalentSave {
    /// The learned ranks, for the rules to check.
    pub fn allocation(&self) -> Allocation {
        self.ranks.iter().copied().collect()
    }

    /// Keep `allocation` as the learned ranks.
    pub fn set_allocation(&mut self, allocation: &Allocation) {
        self.ranks = allocation.iter().collect();
    }

    /// Points spent in all trees.
    pub fn spent(&self) -> u32 {
        self.allocation().spent()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_save_round_trips_through_toml() {
        let save = TalentSave {
            xp: 12_345,
            ranks: vec![(35, 2), (140, 5)],
        };
        let text = toml::to_string_pretty(&save).unwrap();
        assert_eq!(toml::from_str::<TalentSave>(&text).unwrap(), save);
        let alloc = save.allocation();
        assert_eq!((alloc.rank(35), alloc.rank(140), alloc.spent()), (2, 5, 7));
        let mut again = TalentSave::default();
        again.set_allocation(&alloc);
        assert_eq!(again.ranks, save.ranks);
        assert_eq!(again.spent(), 7);
    }

    #[test]
    fn saves_without_the_fields_read_as_defaults() {
        assert_eq!(
            toml::from_str::<TalentSave>("").unwrap(),
            TalentSave::default()
        );
        let old: TalentSave = toml::from_str("xp = 90\n").unwrap();
        assert_eq!((old.xp, old.ranks.len()), (90, 0));
        let unknown: TalentSave = toml::from_str("ranks = [[35, 1]]\nlater = true\n").unwrap();
        assert_eq!((unknown.xp, unknown.ranks), (0, vec![(35, 1)]));
    }

    #[test]
    fn zero_and_repeated_ranks_read_as_the_rules_would() {
        let save = TalentSave {
            xp: 0,
            ranks: vec![(35, 2), (140, 0), (35, 3)],
        };
        let alloc = save.allocation();
        assert_eq!((alloc.rank(35), alloc.rank(140), save.spent()), (3, 0, 3));
    }
}
