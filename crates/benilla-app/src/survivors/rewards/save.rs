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
    /// The companion's look, a [`super::looks::all_looks`] key; empty or unknown wears the first.
    pub(in crate::survivors) companion: String,
    /// Ranks bought, by perk id; a missing perk is rank 0, and an id this build lacks is ignored.
    pub(in crate::survivors) perks: BTreeMap<String, u32>,
    /// The displays of the [`super::looks::EARNED_LOOKS`] unlocked, which the quest chains award
    /// ([`crate::survivors::quests`]).
    pub(in crate::survivors) unlocked_looks: BTreeSet<u32>,
    /// The companion's experience over every run, which sets its level ([`super::pet`]).
    pub(in crate::survivors) pet_xp: u32,
    /// The companion's name; empty is the first of [`super::pet::PET_NAMES`].
    pub(in crate::survivors) pet_name: String,
    /// Skill ranks trained, by skill id ([`super::pet::SKILLS`]); a missing skill is rank 0, and
    /// an id this build lacks is ignored. The training points they cost are never stored.
    pub(in crate::survivors) pet_skills: BTreeMap<String, u32>,
    /// The Second Companion's look, as `companion` is the first's.
    pub(in crate::survivors) second_look: String,
}

impl Default for RewardsSave {
    fn default() -> Self {
        Self {
            spent: 0,
            resets_left: PERK_RESETS,
            companion: String::new(),
            perks: BTreeMap::new(),
            unlocked_looks: BTreeSet::new(),
            pet_xp: 0,
            pet_name: String::new(),
            pet_skills: BTreeMap::new(),
            second_look: String::new(),
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
            pet_xp: 1234,
            pet_name: "Silverleaf".into(),
            second_look: "bombay".into(),
            ..Default::default()
        };
        save.perks.insert("lucky".into(), 2);
        save.perks.insert("companion".into(), 1);
        save.unlocked_looks.insert(1206);
        save.pet_skills.insert("swiftness".into(), 2);
        save.pet_skills.insert("second_companion".into(), 1);
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
        // From before the companion's levels: a fresh pet, its skills untrained.
        let before_pets: RewardsSave = toml::from_str(
            "spent = 25\ncompanion = \"cat\"\nunlocked_looks = [2177]\n[perks]\ncompanion = 1\n",
        )
        .unwrap();
        assert_eq!(before_pets.companion, "cat");
        assert!(before_pets.unlocked_looks.contains(&2177));
        assert_eq!(before_pets.pet_xp, 0);
        assert!(before_pets.pet_name.is_empty() && before_pets.second_look.is_empty());
        assert!(before_pets.pet_skills.is_empty());
    }
}
