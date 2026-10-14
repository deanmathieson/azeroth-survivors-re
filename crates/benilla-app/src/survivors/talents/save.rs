//! The talents' section of each class's profile record, `ClassRecord::talents` in
//! [`crate::survivors::profile`]. Only the talents workstream edits this file.

use serde::{Deserialize, Serialize};

/// One class's saved talents. Every field must read a save without it as its default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(in crate::survivors) struct TalentSave {}
