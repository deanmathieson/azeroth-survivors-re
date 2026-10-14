//! The Faire's section of the profile, `Profile::faire` in [`crate::survivors::profile`]: the
//! account's Darkmoon Prize Tickets and Faire Favor, the booth's pity counters and dice, and the
//! unlocks no other store keeps yet (titles, toys and Darkmoon cards). Companion looks, mounts,
//! mount hats and tabards are kept where the quests already keep them. Only the faire workstream
//! edits this file.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// One banner type's pity: pulls since its last Epic or better, and since its last Legendary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(in crate::survivors) struct Pity {
    pub since_epic: u32,
    pub since_legendary: u32,
}

/// The account's Faire. Every field must read a save without it as its default.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(in crate::survivors) struct FaireSave {
    /// Darkmoon Prize Tickets to spend at the Prize Booth.
    pub tickets: u32,
    /// Faire Favor, from duplicate prizes, to spend at the Prize Counter.
    pub favor: u32,
    /// Every ticket ever earned, for the record.
    pub tickets_earned: u64,
    /// Every prize ever pulled.
    pub pulls: u32,
    /// Pity by banner type ([`super::booth::BannerKind::key`]); a type never pulled is all zero.
    pub pity: BTreeMap<String, Pity>,
    /// The booth's dice: a SplitMix64 state, bit-cast to `i64` because TOML integers are signed.
    /// `None` until the first pull seeds it from the account's record ([`super::booth::seed`]),
    /// so no two accounts pull alike and nothing reads the clock.
    pub dice: Option<i64>,
    /// Unlock keys for the features still to come, each its prize's key
    /// ([`super::catalog::Prize::key`]).
    pub titles: BTreeSet<String>,
    pub toys: BTreeSet<String>,
    pub cards: BTreeSet<String>,
}

impl FaireSave {
    /// `kind`'s pity, zero if it was never pulled.
    pub(in crate::survivors) fn pity(&self, kind: &str) -> Pity {
        self.pity.get(kind).copied().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::profile::Profile;
    use super::*;

    #[test]
    fn round_trips_and_reads_old_saves() {
        let mut save = FaireSave {
            tickets: 140,
            favor: 33,
            tickets_earned: 9_001,
            pulls: 87,
            dice: Some(-7_046_029_254_386_353_131),
            ..Default::default()
        };
        save.pity.insert(
            "featured".into(),
            Pity {
                since_epic: 12,
                since_legendary: 70,
            },
        );
        save.titles.insert("the_insane".into());
        save.toys.insert("orb_of_deception".into());
        save.cards.insert("ace_of_beasts".into());
        let text = toml::to_string_pretty(&save).unwrap();
        assert_eq!(toml::from_str::<FaireSave>(&text).unwrap(), save);
        assert_eq!(save.pity("featured").since_legendary, 70);
        assert_eq!(save.pity("standard"), Pity::default());

        // A save from before the Faire, and one with fields this build does not know.
        assert_eq!(
            toml::from_str::<FaireSave>("").unwrap(),
            FaireSave::default()
        );
        let newer: FaireSave =
            toml::from_str("tickets = 5\nwheel = 3\n[pity.standard]\nsince_epic = 4\nspins = 1\n")
                .unwrap();
        assert_eq!(newer.tickets, 5);
        assert_eq!(newer.pity("standard").since_epic, 4);
        assert_eq!(newer.dice, None);
    }

    #[test]
    fn profiles_with_and_without_the_section_load() {
        // Written before the Faire: no `[faire]` at all.
        let old: Profile = toml::from_str("version = 2\n\n[account]\nruns = 3\n").unwrap();
        assert_eq!(old.account.runs, 3);
        assert_eq!(old.faire, FaireSave::default());

        let mut profile = Profile::default();
        profile.faire.tickets = 90;
        profile.faire.cards.insert("eight_of_portals".into());
        profile.faire.dice = Some(i64::MIN);
        let text = toml::to_string_pretty(&profile).unwrap();
        assert!(text.contains("[faire]"), "{text}");
        let back: Profile = toml::from_str(&text).unwrap();
        assert_eq!(back.faire, profile.faire);
    }
}
