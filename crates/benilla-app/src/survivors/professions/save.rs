//! The professions' half of the profile (docs/PROFESSIONS.md, 8): the bags (`[inventory]`), the
//! purse (`[purse]`), and the skills and the tracking (`[professions]`), new in profile version 5.
//! Read once after the profile loads; written back through [`Profile::mark_dirty`] whenever any
//! changes, so the profile's one writer saves them with everything else.

use std::collections::BTreeMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::super::profile::Profile;
use super::econ::{Purse, PurseSave};
use super::inventory::Bags;
use super::skills::{Rank, Skills};
use super::tracking::Tracking;

/// `[professions]`: the skills by slug, and the tracking in force (`minerals`, `herbs`, or empty).
/// Recipes, specialisations and cooldowns come with the phases that use them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ProfessionsSave {
    pub(crate) skills: BTreeMap<String, Rank>,
    pub(crate) tracking: String,
}

/// `WOW_SURVIVORS_BAGS=test`: the bags are the test inventory, and nothing is saved.
#[derive(Resource, Default)]
pub(crate) struct Shadow(pub(crate) bool);

/// Everything the professions save.
#[derive(SystemParam)]
pub(super) struct Saved<'w> {
    bags: ResMut<'w, Bags>,
    skills: ResMut<'w, Skills>,
    purse: ResMut<'w, Purse>,
    tracking: ResMut<'w, Tracking>,
}

/// `PostStartup`: the bags, purse, skills and tracking out of the profile.
pub(super) fn load_saved(profile: Option<Res<Profile>>, shadow: Res<Shadow>, mut saved: Saved) {
    let Some(profile) = profile else {
        return;
    };
    if !shadow.0 {
        *saved.bags = Bags::from_save(&profile.inventory);
    }
    *saved.skills = Skills::from_save(&profile.professions.skills);
    *saved.purse = Purse::with(profile.purse.copper);
    *saved.tracking = Tracking::from_slug(&profile.professions.tracking);
}

/// `Update`: a change to any of them goes into the profile and queues its save.
pub(super) fn save_changes(
    profile: Option<ResMut<Profile>>,
    shadow: Res<Shadow>,
    saved: Saved,
    mut seen: Local<Option<(u64, u64, u64, Tracking)>>,
) {
    let now = (
        saved.bags.generation(),
        saved.skills.generation(),
        saved.purse.generation(),
        *saved.tracking,
    );
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
    profile.inventory = saved.bags.to_save();
    profile.professions.skills = saved.skills.to_save();
    profile.professions.tracking = saved.tracking.slug().to_string();
    profile.purse = PurseSave {
        copper: saved.purse.copper(),
    };
    profile.mark_dirty();
}
