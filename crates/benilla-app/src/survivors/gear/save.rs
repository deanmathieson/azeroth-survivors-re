//! The gear's section of each class's profile record, `ClassRecord::gear` in
//! [`crate::survivors::profile`]. Only the gear workstream edits this file.

use serde::{Deserialize, Serialize};

/// One class's saved gear. Every field must read a save without it as its default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(in crate::survivors) struct GearSave {}
