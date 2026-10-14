//! Achievements, account-wide: the mode's own list ([`catalog`]) of wins, single-run feats and
//! lifetime totals, each worth the reward points [`super::rewards`] spends. The record lives in
//! [`AchievementSave`] on the profile; a run's achievements are judged from
//! [`super::profile::RunEnded`] in [`judge_run`], which records them, marks the profile dirty and
//! announces each in [`super::hud::ResultsNotes`]. The Armory's Achievements tab draws them; its
//! category picks land here, in [`AchievementsView`].

mod catalog;
mod save;

use bevy::prelude::*;

use super::armory::ArmoryState;
use super::hud::{NoteKind, ResultsNotes};
use super::profile::{Profile, RunEnded};
use crate::glue::GlueClicks;

pub(super) use catalog::{
    almost_there, catalog, counts, earned_points, newly_earned, recently_earned, total_points,
    Achievement, Category,
};
pub(super) use save::AchievementSave;

/// This workstream's systems, registered by [`super::SurvivorsPlugin`].
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<AchievementsView>()
        .add_systems(Update, (judge_run, pick_category));
}

/// What the Achievements tab shows beside its category list: a category, or `None` for the
/// summary.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct AchievementsView(pub(super) Option<Category>);

/// A row of the Achievements tab's category list; `None` is the summary.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CategoryPick(pub(super) Option<Category>);

/// Each recorded run is judged once: whatever it completes is stamped with the account's run
/// count (the run that earned it) and announced on the results screen.
fn judge_run(
    mut ended: MessageReader<RunEnded>,
    mut profile: ResMut<Profile>,
    mut notes: ResMut<ResultsNotes>,
) {
    for run in ended.read() {
        let new = newly_earned(&profile, Some(&run.outcome));
        if new.is_empty() {
            continue;
        }
        let at = profile.account.runs;
        for a in new {
            profile.achievements.earned.insert(a.id.clone(), at);
            notes.push(
                format!("Achievement: {} (+{})", a.name, a.points),
                NoteKind::Achievement,
            );
        }
        profile.mark_dirty();
    }
}

/// A click on the category list shows that category, rebuilding the open tab.
fn pick_category(
    clicks: Res<GlueClicks>,
    picks: Query<(Entity, &CategoryPick)>,
    mut view: ResMut<AchievementsView>,
    mut armory: ResMut<ArmoryState>,
) {
    let Some(pick) = picks.iter().find(|(e, _)| clicks.hit(*e)).map(|(_, p)| p.0) else {
        return;
    };
    if view.0 != pick {
        view.0 = pick;
        armory.refresh();
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::super::data;
    use super::super::profile::{slug, RunOutcome};
    use super::*;

    #[test]
    fn a_run_earns_records_and_announces_each_once() {
        let mut world = World::new();
        world.init_resource::<Messages<RunEnded>>();
        world.init_resource::<ResultsNotes>();
        let outcome = RunOutcome {
            class: 1,
            map: 0,
            won: true,
            seconds: 900.0,
            kills: 300,
            bosses: 6,
            level: 31,
            damage: 2_000_000.0,
            gold: 200,
            reward_mult: 1.0,
            quests: 0,
            final_boss: false,
        };
        let mut profile = Profile::default();
        profile.account.runs = 6;
        // The profile counts the run before the judges read it.
        profile.record(&outcome);
        world.insert_resource(profile);
        world
            .resource_mut::<Messages<RunEnded>>()
            .write(RunEnded { outcome });
        world.run_system_once(judge_run).unwrap();

        let want = [
            format!("win_{}", slug(data::MAPS[0].name)),
            format!("class_{}", slug(data::CLASSES[1].name)),
            "survive_10_minutes".to_string(),
            "face_5_bosses".to_string(),
            "damage_1m".to_string(),
        ];
        let profile = world.resource::<Profile>();
        let earned: Vec<&String> = profile.achievements.earned.keys().collect();
        assert_eq!(earned.len(), want.len(), "{earned:?}");
        for id in &want {
            assert_eq!(profile.achievements.run_earned(id), Some(7), "{id}");
        }
        let notes = &world.resource::<ResultsNotes>().0;
        assert_eq!(notes.len(), want.len());
        assert!(notes.iter().all(|(_, kind)| *kind == NoteKind::Achievement));
        assert!(notes.contains(&(
            "Achievement: Seven Figures (+20)".into(),
            NoteKind::Achievement
        )));

        // Read again (a fresh reader sees the same message): nothing is earned twice.
        world.run_system_once(judge_run).unwrap();
        assert_eq!(world.resource::<ResultsNotes>().0.len(), want.len());
        assert_eq!(
            world.resource::<Profile>().achievements.earned.len(),
            want.len()
        );
    }
}
