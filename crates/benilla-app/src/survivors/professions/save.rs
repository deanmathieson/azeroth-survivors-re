//! The professions' half of the profile (docs/PROFESSIONS.md, 8): the bags (`[inventory]`) and
//! the skills (`[professions.skills]`), new in profile version 5. Read once after the profile
//! loads; written back through [`Profile::mark_dirty`] whenever either changes, so the profile's
//! one writer saves them with everything else.

use std::collections::BTreeMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::super::profile::Profile;
use super::inventory::Bags;
use super::skills::{Rank, Skills};

/// `[professions]`: the skills by slug. Purse, recipes, specialisations and cooldowns come with
/// the phases that use them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ProfessionsSave {
    pub(crate) skills: BTreeMap<String, Rank>,
}

/// `WOW_SURVIVORS_BAGS=test`: the bags are the test inventory, and nothing is saved.
#[derive(Resource, Default)]
pub(crate) struct Shadow(pub(crate) bool);

/// `PostStartup`: the bags and skills out of the profile.
pub(super) fn load_saved(
    profile: Option<Res<Profile>>,
    shadow: Res<Shadow>,
    mut bags: ResMut<Bags>,
    mut skills: ResMut<Skills>,
) {
    let Some(profile) = profile else {
        return;
    };
    if !shadow.0 {
        *bags = Bags::from_save(&profile.inventory);
    }
    *skills = Skills::from_save(&profile.professions.skills);
}

/// `Update`: a change to the bags or the skills goes into the profile and queues its save.
pub(super) fn save_changes(
    profile: Option<ResMut<Profile>>,
    shadow: Res<Shadow>,
    bags: Res<Bags>,
    skills: Res<Skills>,
    mut seen: Local<Option<(u64, u64)>>,
) {
    let now = (bags.generation(), skills.generation());
    let Some(was) = seen.replace(now) else {
        // The first frame: what was just read is what is saved.
        return;
    };
    let Some(mut profile) = profile else {
        return;
    };
    if was == now || shadow.0 {
        return;
    }
    profile.inventory = bags.to_save();
    profile.professions.skills = skills.to_save();
    profile.mark_dirty();
}
