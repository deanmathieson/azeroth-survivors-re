//! The professions' half of the profile (docs/PROFESSIONS.md, 8): the bags (`[inventory]`), the
//! purse (`[purse]`), and the skills, the tracking, the learned recipes and the satchel bar
//! (`[professions]`). Read once after the profile loads; written back through
//! [`Profile::mark_dirty`] whenever any changes, so the profile's one writer saves them with
//! everything else. The bags, purse, skills and tracking are new in profile version 5; the recipes
//! and the satchel in version 6. Version 7 (P3) adds no field: it marks a save holding the four
//! crafting lines' skills, an Engineering specialisation (kept with the learned recipes), P3's
//! bag items and crafted gear, which a version 6 build would drop.

use std::collections::BTreeMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::super::profile::Profile;
use super::econ::{Purse, PurseSave};
use super::inventory::Bags;
use super::recipes;
use super::satchel::Satchel;
use super::skills::{Rank, Skills};
use super::tracking::Tracking;

/// `[professions]`: the skills by slug, the tracking in force (`minerals`, `herbs`, or empty), the
/// recipe spells learned beyond the skills' own (and, since P3, an Engineering specialisation's
/// spell), and the satchel bar's items (0 an empty slot). The other specialisations and the
/// cooldowns come with P4.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ProfessionsSave {
    pub(crate) skills: BTreeMap<String, Rank>,
    pub(crate) tracking: String,
    pub(crate) recipes: Vec<u32>,
    pub(crate) satchel: Vec<u32>,
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
    satchel: ResMut<'w, Satchel>,
}

/// `PostStartup`: the bags, purse, skills, recipes, tracking and satchel out of the profile. A
/// recipe this build does not know is dropped.
pub(super) fn load_saved(profile: Option<Res<Profile>>, shadow: Res<Shadow>, mut saved: Saved) {
    let Some(profile) = profile else {
        return;
    };
    if !shadow.0 {
        *saved.bags = Bags::from_save(&profile.inventory);
    }
    let known = profile
        .professions
        .recipes
        .iter()
        .copied()
        .filter(|&s| recipes::known_spell(s));
    *saved.skills = Skills::from_save(&profile.professions.skills).with_learned(known);
    *saved.purse = Purse::with(profile.purse.copper);
    *saved.tracking = Tracking::from_slug(&profile.professions.tracking);
    *saved.satchel = Satchel::from_save(&profile.professions.satchel);
}

/// What [`save_changes`] compares: the bags', skills', purse's and satchel's change counters, and
/// the tracking.
type Seen = (u64, u64, u64, Tracking, u64);

/// `Update`: a change to any of them goes into the profile and queues its save.
pub(super) fn save_changes(
    profile: Option<ResMut<Profile>>,
    shadow: Res<Shadow>,
    saved: Saved,
    mut seen: Local<Option<Seen>>,
) {
    let now = (
        saved.bags.generation(),
        saved.skills.generation(),
        saved.purse.generation(),
        *saved.tracking,
        saved.satchel.generation(),
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
    profile.professions.recipes = saved.skills.learned_recipes();
    profile.professions.tracking = saved.tracking.slug().to_string();
    profile.professions.satchel = saved.satchel.to_save();
    profile.purse = PurseSave {
        copper: saved.purse.copper(),
    };
    profile.mark_dirty();
}
