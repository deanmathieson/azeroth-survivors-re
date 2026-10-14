//! The Codex's record: every piece of jewelry ever offered (seen) and taken, per account, in the
//! profile's `codex` section ([`CodexSave`]), and what the Armory's Codex tab has picked.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::super::armory::ArmoryState;
use super::Piece;
use crate::glue::GlueClicks;

/// The account's jewelry record, `Profile::codex`. Keyed by [`Piece::key`], so a piece this build
/// does not know is kept as it is. Every field must read a save without it as its default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(in crate::survivors) struct CodexSave {
    /// Every piece an offer has shown.
    pub seen: BTreeSet<String>,
    /// How many times each piece was taken.
    pub taken: BTreeMap<String, u32>,
}

impl CodexSave {
    pub fn saw(&self, key: &str) -> bool {
        self.seen.contains(key)
    }

    pub fn times_taken(&self, key: &str) -> u32 {
        self.taken.get(key).copied().unwrap_or(0)
    }

    /// Log `key` as offered; whether it is new.
    pub fn see(&mut self, key: &str) -> bool {
        self.seen.insert(key.to_string())
    }

    /// Log `key` as taken, and so seen.
    pub fn take(&mut self, key: &str) {
        self.see(key);
        let n = self.taken.entry(key.to_string()).or_default();
        *n = n.saturating_add(1);
    }

    /// How many of `pieces` were seen, and how many there are.
    pub fn seen_of<'a>(&self, pieces: impl Iterator<Item = &'a Piece>) -> (usize, usize) {
        pieces.fold((0, 0), |(seen, of), p| {
            (seen + usize::from(self.saw(p.key)), of + 1)
        })
    }
}

/// The piece the Codex tab shows under Details.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::survivors) struct CodexView {
    pub piece: Option<&'static str>,
}

/// A click in the Codex tab.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::survivors) enum CodexAction {
    /// Show this piece, by key, under Details.
    Pick(&'static str),
}

/// A Codex click: show the piece, and rebuild the tab.
pub(super) fn codex_clicks(
    clicks: Res<GlueClicks>,
    actions: Query<(Entity, &CodexAction)>,
    mut view: ResMut<CodexView>,
    mut armory: ResMut<ArmoryState>,
) {
    if !armory.open {
        return;
    }
    let Some(action) = actions
        .iter()
        .find(|(e, _)| clicks.hit(*e))
        .map(|(_, a)| *a)
    else {
        return;
    };
    match action {
        CodexAction::Pick(key) => view.piece = Some(key),
    }
    armory.refresh();
}

#[cfg(test)]
mod tests {
    use super::super::super::profile::Profile;
    use super::super::pieces;
    use super::*;

    #[test]
    fn the_record_counts_what_was_seen_and_taken() {
        let mut codex = CodexSave::default();
        assert!(codex.see("hand_of_justice"));
        assert!(!codex.see("hand_of_justice"), "seen once is enough");
        codex.take("band_of_accuria");
        codex.take("band_of_accuria");
        assert!(codex.saw("band_of_accuria"), "taken is seen");
        assert_eq!(codex.times_taken("band_of_accuria"), 2);
        assert_eq!(codex.times_taken("hand_of_justice"), 0);
        assert_eq!(codex.seen_of(pieces().iter()), (2, 45));
        // A key this build does not know counts toward nothing, and is kept.
        codex.see("ashbringer");
        assert_eq!(codex.seen_of(pieces().iter()), (2, 45));
        assert!(codex.saw("ashbringer"));
    }

    #[test]
    fn codex_save_round_trips() {
        let mut codex = CodexSave::default();
        codex.see("tidal_charm");
        codex.take("kiss_of_the_spider");
        codex.see("a_piece_from_a_newer_build");
        let text = toml::to_string_pretty(&codex).unwrap();
        assert_eq!(toml::from_str::<CodexSave>(&text).unwrap(), codex);
        // In the profile, as its own section.
        let mut profile = Profile::default();
        profile.codex = codex.clone();
        let text = toml::to_string_pretty(&profile).unwrap();
        assert!(text.contains("[codex.taken]"), "{text}");
        let back: Profile = toml::from_str(&text).unwrap();
        assert_eq!(back.codex, codex);
        assert_eq!(back, profile);
    }

    #[test]
    fn saves_from_before_the_codex_read_empty() {
        assert_eq!(
            toml::from_str::<CodexSave>("").unwrap(),
            CodexSave::default()
        );
        let part: CodexSave = toml::from_str("seen = [\"naglering\"]").unwrap();
        assert!(part.saw("naglering") && part.taken.is_empty());
        let old = "version = 2\n\n[account]\nruns = 4\n\n[classes.mage]\nruns = 4\n";
        let profile: Profile = toml::from_str(old).unwrap();
        assert_eq!(profile.codex, CodexSave::default());
        assert_eq!(profile.account.runs, 4);
        let with: Profile = toml::from_str(
            "version = 2\n\n[codex]\nseen = [\"naglering\", \"earthstrike\"]\n\n\
             [codex.taken]\nearthstrike = 3\n",
        )
        .unwrap();
        assert_eq!(with.codex.seen_of(pieces().iter()), (2, 45));
        assert_eq!(with.codex.times_taken("earthstrike"), 3);
    }
}
