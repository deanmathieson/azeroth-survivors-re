//! The achievements' section of the profile, `Profile::achievements` in
//! [`crate::survivors::profile`]. Only the achievements workstream edits this file.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The account's saved achievements. Every field must read a save without it as its default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(in crate::survivors) struct AchievementSave {
    /// Each earned achievement's id, with the account's run count when it was earned: the run
    /// that earned it, its "date". An id the catalog no longer has stays, unread.
    pub(in crate::survivors) earned: BTreeMap<String, u32>,
}

impl AchievementSave {
    /// Whether `id` is earned.
    pub(in crate::survivors) fn has(&self, id: &str) -> bool {
        self.earned.contains_key(id)
    }

    /// The run that earned `id`, if it is earned.
    pub(in crate::survivors) fn run_earned(&self, id: &str) -> Option<u32> {
        self.earned.get(id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_reads_old_saves() {
        let mut save = AchievementSave::default();
        save.earned.insert("win_elwynn_forest".into(), 3);
        save.earned.insert("kills_1000".into(), 7);
        let text = toml::to_string_pretty(&save).unwrap();
        let back: AchievementSave = toml::from_str(&text).unwrap();
        assert_eq!(back, save);
        assert_eq!(back.run_earned("kills_1000"), Some(7));
        assert!(back.has("win_elwynn_forest") && !back.has("runs_10"));
        // A save from before achievements, and one with fields this build does not know.
        assert_eq!(
            toml::from_str::<AchievementSave>("").unwrap(),
            AchievementSave::default()
        );
        let newer: AchievementSave =
            toml::from_str("future = 1\n[earned]\nruns_10 = 12\n").unwrap();
        assert_eq!(newer.run_earned("runs_10"), Some(12));
    }
}
