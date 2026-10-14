//! The rewards' section of the profile, `Profile::rewards` in [`crate::survivors::profile`]: perks,
//! companions and reward-point spending. Only the rewards workstream edits this file.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// How many times an account may reset its perks.
pub(in crate::survivors) const PERK_RESETS: u32 = 2;

/// The account's saved rewards. Every field must read a save without it as its default.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(in crate::survivors) struct RewardsSave {
    /// Reward points spent on the ranks in `perks`. Points to spend are the earned achievements'
    /// points less this, so they are never stored themselves.
    pub(in crate::survivors) spent: u32,
    /// Perk resets left; a save from before resets reads the full [`PERK_RESETS`].
    pub(in crate::survivors) resets_left: u32,
    /// The companion's look, a [`super::companion::all_looks`] key; empty or unknown wears the
    /// first.
    pub(in crate::survivors) companion: String,
    /// Ranks bought, by perk id; a missing perk is rank 0, and an id this build lacks is ignored.
    pub(in crate::survivors) perks: BTreeMap<String, u32>,
    /// The displays of the [`super::companion::EARNED_LOOKS`] unlocked, which the quest chains
    /// award ([`crate::survivors::quests`]).
    pub(in crate::survivors) unlocked_looks: BTreeSet<u32>,
}

impl Default for RewardsSave {
    fn default() -> Self {
        Self {
            spent: 0,
            resets_left: PERK_RESETS,
            companion: String::new(),
            perks: BTreeMap::new(),
            unlocked_looks: BTreeSet::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_reads_old_saves() {
        let mut save = RewardsSave {
            spent: 45,
            resets_left: 1,
            companion: "frog".into(),
            ..Default::default()
        };
        save.perks.insert("lucky".into(), 2);
        save.perks.insert("companion".into(), 1);
        let text = toml::to_string_pretty(&save).unwrap();
        assert_eq!(toml::from_str::<RewardsSave>(&text).unwrap(), save);

        // A save from before rewards: nothing bought, every reset still to use.
        let old: RewardsSave = toml::from_str("").unwrap();
        assert_eq!(old, RewardsSave::default());
        assert_eq!(old.resets_left, PERK_RESETS);
        // A partial one keeps what it has and defaults the rest; unknown fields are ignored.
        let partial: RewardsSave =
            toml::from_str("spent = 10\nwide_hand = true\n[perks]\nhardy = 1\n").unwrap();
        assert_eq!(partial.spent, 10);
        assert_eq!(partial.resets_left, PERK_RESETS);
        assert_eq!(partial.perks.get("hardy"), Some(&1));
        assert!(partial.companion.is_empty());
    }
}
