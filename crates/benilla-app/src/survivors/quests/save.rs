//! The quests' section of the profile, `Profile::quests` in [`crate::survivors::profile`]: each
//! zone's quest count and chain progress, and the unlocks recorded for features still to come.
//! Only the quests workstream edits this file.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// One zone's quest record.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(in crate::survivors) struct ZoneQuestRecord {
    /// In-run quests turned in on this zone, over every recorded run.
    pub completed: u32,
    /// The zone chain's steps done, 0 to 3 ([`super::chain::STEPS`]).
    pub steps: u8,
}

/// The account's saved quests. Every field must read a save without it as its default.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(in crate::survivors) struct QuestSave {
    /// Each zone's record, by the zone's slug; a zone this build lacks stays, unread.
    pub zones: BTreeMap<String, ZoneQuestRecord>,
    /// Unlock keys the chains awarded for features still to come, each its reward's slug
    /// ([`super::catalog::Reward::key`]): mounts, the hats a mount wears, and tabards.
    pub mounts: BTreeSet<String>,
    pub mount_hats: BTreeSet<String>,
    pub tabards: BTreeSet<String>,
}

impl QuestSave {
    /// `zone`'s record (a slug), empty if it has none.
    pub(in crate::survivors) fn zone(&self, zone: &str) -> ZoneQuestRecord {
        self.zones.get(zone).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::profile::Profile;
    use super::*;

    #[test]
    fn round_trips_and_reads_old_saves() {
        let mut save = QuestSave::default();
        save.zones.insert(
            "elwynn_forest".into(),
            ZoneQuestRecord {
                completed: 4,
                steps: 2,
            },
        );
        save.mounts.insert("brown_horse".into());
        save.mount_hats.insert("tiny_party_hat".into());
        save.tabards.insert("darnassus_tabard".into());
        let text = toml::to_string_pretty(&save).unwrap();
        assert_eq!(toml::from_str::<QuestSave>(&text).unwrap(), save);
        assert_eq!(save.zone("elwynn_forest").steps, 2);
        assert_eq!(save.zone("westfall"), ZoneQuestRecord::default());

        // A save from before quests, and one with fields this build does not know.
        assert_eq!(
            toml::from_str::<QuestSave>("").unwrap(),
            QuestSave::default()
        );
        let newer: QuestSave =
            toml::from_str("later = 1\n[zones.durotar]\ncompleted = 2\nbounty = 3\n").unwrap();
        assert_eq!(newer.zone("durotar").completed, 2);
        assert!(newer.mounts.is_empty());
    }

    #[test]
    fn profiles_with_and_without_the_section_load() {
        // Written before quests: no `[quests]` at all.
        let old: Profile = toml::from_str("version = 2\n\n[account]\nruns = 3\n").unwrap();
        assert_eq!(old.account.runs, 3);
        assert_eq!(old.quests, QuestSave::default());

        let mut profile = Profile::default();
        profile.quests.zones.insert(
            "mulgore".into(),
            ZoneQuestRecord {
                completed: 3,
                steps: 3,
            },
        );
        profile.quests.tabards.insert("nethergarde_tabard".into());
        let text = toml::to_string_pretty(&profile).unwrap();
        assert!(text.contains("[quests.zones.mulgore]"), "{text}");
        let back: Profile = toml::from_str(&text).unwrap();
        assert_eq!(back.quests, profile.quests);
    }
}
