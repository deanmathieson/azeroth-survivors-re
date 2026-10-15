//! Quests. In a run, a quest giver walks out near the party about every three minutes, clear of
//! the bosses ([`schedule`]), offering one of its zone's quests ([`catalog`]): kill so many of a
//! foe (or of anything), collect an item a foe drops, hold a marked spot, or slay the next elite.
//! Walking into the giver accepts it, the tracker under the minimap follows it ([`tracker`]), and
//! walking back once it is done turns it in for a cache's worth and a bonus ([`field`]). Progress
//! is the party's; the hand a turned-in quest pays is dealt to every hero.
//!
//! Between runs, each zone has a three-step chain judged as each run is recorded ([`chain`]): its
//! quests, its last boss, then a win at ×1.25 rewards or more, for a talent XP, companion look,
//! mount, mount hat or tabard reward. The record lives in [`QuestSave`] on the profile; the
//! Armory's Quests tab draws it, its zone picks landing here in [`QuestsView`].
//!
//! Test aids: `WOW_SURVIVORS_QUEST_EVERY=<secs>` sets the giver interval (and brings the first one
//! that soon if it is under a minute), and `WOW_SURVIVORS_QUEST=<quest id>` sets the quest the
//! run's first giver brings.

mod catalog;
mod chain;
mod field;
mod objective;
mod save;
mod schedule;
mod tracker;

use bevy::prelude::*;

use super::armory::ArmoryState;
use super::{enemies, hero, hud, treasure, Phase};
use crate::glue::GlueClicks;

pub(super) use catalog::{for_map, QuestDef, Reward, ZONES};
pub(super) use chain::{Step, CHAIN_QUESTS, STEPS};
pub(super) use field::MapPin;
pub(super) use objective::{clock, objective_line};
pub(super) use save::QuestSave;

/// This workstream's systems, registered by [`super::SurvivorsPlugin`].
pub(super) fn plugin(app: &mut App) {
    let aids = field::QuestAids::from_env();
    app.insert_resource(field::QuestRun::new(&aids))
        .insert_resource(aids)
        .init_resource::<QuestsView>()
        .add_systems(OnEnter(Phase::Menu), field::reset)
        .add_systems(
            Update,
            (
                field::count_kills,
                field::collect_items,
                field::tick_quests,
                field::meet_givers,
                field::offer_quests,
                field::feed_foes,
                field::walk_off,
                field::mark_givers,
            )
                .chain()
                .after(enemies::collect_gems)
                .after(treasure::open_caches)
                .before(hero::hero_vitals)
                .run_if(in_state(Phase::Playing))
                .run_if(super::party::simulates)
                .run_if(|l: Res<crate::loading_screen::LoadingScreen>| !l.covering()),
        )
        .add_systems(
            Update,
            (tracker::spawn_tracker, tracker::refresh_tracker)
                .chain()
                .after(hud::refresh_hud)
                .before(hud::show_panels),
        )
        .add_systems(Update, (chain::judge_chains, pick_zone));
}

/// The zone the Armory's Quests tab shows; `None` is the zone picked at the menu.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct QuestsView(pub(super) Option<usize>);

/// A zone row of the Quests tab's list, by its index in [`super::data::MAPS`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ZonePick(pub(super) usize);

/// A click on the zone list shows that zone, rebuilding the open tab.
fn pick_zone(
    clicks: Res<GlueClicks>,
    picks: Query<(Entity, &ZonePick)>,
    mut view: ResMut<QuestsView>,
    mut armory: ResMut<ArmoryState>,
) {
    let Some(pick) = picks.iter().find(|(e, _)| clicks.hit(*e)).map(|(_, p)| p.0) else {
        return;
    };
    if view.0 != Some(pick) {
        view.0 = Some(pick);
        armory.refresh();
    }
}
