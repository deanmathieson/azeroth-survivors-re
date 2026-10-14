//! Rewards bought with what runs earn: perks, companions and reward-point spending. The record
//! lives in [`RewardsSave`] on the profile; perks reach a run through `MetaParts::perks`
//! ([`super::meta`]), and a run's earnings arrive as [`super::profile::RunEnded`].

mod save;

use bevy::prelude::*;

pub(super) use save::RewardsSave;

/// This workstream's systems, registered by [`super::SurvivorsPlugin`].
pub(super) fn plugin(_app: &mut App) {}
