//! The rewards' section of the profile, `Profile::rewards` in [`crate::survivors::profile`]: perks,
//! companions and reward-point spending. Only the rewards workstream edits this file.

use serde::{Deserialize, Serialize};

/// The account's saved rewards. Every field must read a save without it as its default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(in crate::survivors) struct RewardsSave {}
