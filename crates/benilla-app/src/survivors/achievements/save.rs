//! The achievements' section of the profile, `Profile::achievements` in
//! [`crate::survivors::profile`]. Only the achievements workstream edits this file.

use serde::{Deserialize, Serialize};

/// The account's saved achievements. Every field must read a save without it as its default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(in crate::survivors) struct AchievementSave {}
