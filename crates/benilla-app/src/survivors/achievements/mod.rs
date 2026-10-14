//! Achievements, account-wide. The record lives in [`AchievementSave`] on the profile; a run's
//! achievements are judged from [`super::profile::RunEnded`] in this module's own systems, which
//! mutate the profile, mark it dirty and announce themselves in [`super::hud::ResultsNotes`].

mod save;

use bevy::prelude::*;

pub(super) use save::AchievementSave;

/// This workstream's systems, registered by [`super::SurvivorsPlugin`].
pub(super) fn plugin(_app: &mut App) {}
