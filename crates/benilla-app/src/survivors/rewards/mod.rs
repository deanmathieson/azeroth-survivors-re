//! Rewards bought with what runs earn: [`perks`] and the [`companion`] one of them brings, paid for
//! in reward points. Points to spend are the earned achievements' points
//! ([`super::achievements`]) less what the perks cost, plus the `WOW_SURVIVORS_POINTS` test aid
//! ([`DevPoints`]). The record lives in [`RewardsSave`] on the profile; the stat perks reach a run
//! through `MetaParts::perks` ([`super::meta`]), and the reroll and banish perks are added to the
//! run's hand as it begins ([`stamp_hand`]). The Armory's Rewards tab draws them; its clicks land
//! here as [`RewardsAction`]s.
//!
//! The companion levels as a hunter's pet does ([`pet`]): each recorded run pays it experience for
//! what it fetched ([`award_pet_xp`]), and its training points buy skills. It wears one of the
//! [`looks`]. The Armory's Companion tab draws all of it; its clicks land here as
//! [`CompanionAction`]s.

mod companion;
pub(super) mod looks;
mod perks;
pub(super) mod pet;
mod save;

use bevy::prelude::*;

use super::achievements::{earned_points, AchievementSave};
use super::armory::{ArmoryState, ArmoryTab};
use super::hud::{NoteKind, ResultsNotes};
use super::meta::MetaParts;
use super::profile::{Profile, RunEnded};
use super::{HeroState, LocalHero, Phase};
use crate::glue::GlueClicks;

pub(super) use companion::{slot_look, PetRun};
pub(super) use looks::{all_looks, earned_look, look_for, offered};
pub(super) use perks::{companion_unlocked, Effect as PerkEffect, Perk, PerkSession, PERKS};
pub(super) use save::RewardsSave;

/// This workstream's systems, registered by [`super::SurvivorsPlugin`].
pub(super) fn plugin(app: &mut App) {
    app.insert_resource(DevPoints::from_env())
        .init_resource::<PerkSession>()
        .init_resource::<CompanionView>()
        .add_systems(
            OnTransition {
                exited: Phase::Menu,
                entered: Phase::Playing,
            },
            stamp_hand,
        )
        .add_systems(
            Update,
            (
                apply_perks.run_if(resource_exists_and_changed::<Profile>),
                (end_session, reward_clicks, companion_clicks).chain(),
                award_pet_xp,
            ),
        )
        .add_plugins(companion::plugin);
}

/// `WOW_SURVIVORS_POINTS=<n>`, a test aid for screenshots: n more reward points to spend, held in
/// memory and never saved. What they buy is saved as any purchase is.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct DevPoints(pub(super) u32);

impl DevPoints {
    fn from_env() -> Self {
        let Some(text) = crate::webenv::var("WOW_SURVIVORS_POINTS") else {
            return Self(0);
        };
        match text.trim().parse() {
            Ok(n) => {
                info!("survivors: {n} reward points from WOW_SURVIVORS_POINTS, not saved");
                Self(n)
            }
            Err(_) => {
                warn!("survivors: WOW_SURVIVORS_POINTS={text:?} is not a whole number");
                Self(0)
            }
        }
    }
}

/// Reward points to spend now.
pub(super) fn points(profile: &Profile, dev: DevPoints) -> u32 {
    perks::available(
        earned_points(&profile.achievements),
        dev.0,
        profile.rewards.spent,
    )
}

/// What a Rewards tab button does.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RewardsAction {
    /// Buy the next rank of [`PERKS`]`[i]`.
    Buy(usize),
    /// Refund its top rank, bought since the tab opened.
    Refund(usize),
    /// Refund every rank.
    Reset,
}

/// What a Companion tab button does.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CompanionAction {
    /// Train the next rank of [`pet::SKILLS`]`[i]`.
    Train(usize),
    /// Unlearn its top rank, the points back.
    Untrain(usize),
    /// The trainer's free reset: every rank unlearned.
    Reset,
    /// Dress the pet the looks grid is dressing in the `i`th of [`all_looks`], if it is offered.
    Look(usize),
    /// Dress the first pet (0) or, with the Second Companion, the second (1) from the grid.
    Dressing(u8),
    /// The next name on [`pet::PET_NAMES`].
    Rename,
}

/// Which pet the Companion tab's looks grid dresses: 0 the first, 1 the Second Companion.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct CompanionView {
    pub(super) dressing: u8,
}

/// The stat perks, as this workstream's share of the meta bonuses, whenever the profile changes.
fn apply_perks(profile: Res<Profile>, mut parts: ResMut<MetaParts>) {
    let want = perks::bonuses(&profile.rewards);
    if parts.perks != want {
        parts.perks = want;
    }
}

/// A run begins: the reroll and banish perks add to the local hero's hand tools (the perks are
/// its player's), as Run Options stamps its multiplier onto the run.
fn stamp_hand(profile: Res<Profile>, mut local: Query<&mut HeroState, With<LocalHero>>) {
    let Ok(mut me) = local.single_mut() else {
        return;
    };
    me.rerolls += perks::extra_rerolls(&profile.rewards);
    me.banishes += perks::extra_banishes(&profile.rewards);
}

/// Leaving the Rewards tab, or the Armory, commits the ranks bought there: they no longer refund.
fn end_session(armory: Res<ArmoryState>, mut session: ResMut<PerkSession>) {
    let editing = armory.open && armory.tab == ArmoryTab::Rewards;
    if !editing && !session.is_empty() {
        session.clear();
    }
}

/// A Rewards tab click: buy, refund or reset, then save and redraw the tab.
fn reward_clicks(
    clicks: Res<GlueClicks>,
    actions: Query<(Entity, &RewardsAction)>,
    mut profile: ResMut<Profile>,
    mut session: ResMut<PerkSession>,
    dev: Res<DevPoints>,
    mut armory: ResMut<ArmoryState>,
) {
    let Some(action) = actions
        .iter()
        .find(|(e, _)| clicks.hit(*e))
        .map(|(_, a)| *a)
    else {
        return;
    };
    let points = points(&profile, *dev);
    if act(action, &mut profile.rewards, &mut session, points) {
        profile.mark_dirty();
        armory.refresh();
    }
}

/// Carry out `action` with `points` to spend; whether anything changed.
fn act(
    action: RewardsAction,
    save: &mut RewardsSave,
    session: &mut PerkSession,
    points: u32,
) -> bool {
    match action {
        RewardsAction::Buy(i) => PERKS
            .get(i)
            .is_some_and(|perk| session.buy(save, perk, points).is_some()),
        RewardsAction::Refund(i) => PERKS
            .get(i)
            .is_some_and(|perk| session.refund(save, perk).is_some()),
        RewardsAction::Reset => session.reset(save).is_some(),
    }
}

/// A Companion tab click: train, untrain, reset, dress or rename, then save what changed and
/// redraw the tab.
fn companion_clicks(
    clicks: Res<GlueClicks>,
    actions: Query<(Entity, &CompanionAction)>,
    mut profile: ResMut<Profile>,
    mut view: ResMut<CompanionView>,
    mut armory: ResMut<ArmoryState>,
) {
    let Some(action) = actions
        .iter()
        .find(|(e, _)| clicks.hit(*e))
        .map(|(_, a)| *a)
    else {
        return;
    };
    let profile = &mut *profile;
    let before = profile.rewards.clone();
    if tend(
        action,
        &mut profile.rewards,
        &profile.achievements,
        &mut view,
    ) {
        if profile.rewards != before {
            profile.mark_dirty();
        }
        armory.refresh();
    }
}

/// Carry out a Companion tab `action` on `save`, the achievements `earned` opening their looks;
/// whether anything changed, the save or only the view.
fn tend(
    action: CompanionAction,
    save: &mut RewardsSave,
    earned: &AchievementSave,
    view: &mut CompanionView,
) -> bool {
    let second = pet::PetSkills::of(save).second;
    let changed = match action {
        CompanionAction::Train(i) => pet::SKILLS
            .get(i)
            .is_some_and(|skill| pet::train(save, skill).is_ok()),
        CompanionAction::Untrain(i) => pet::SKILLS
            .get(i)
            .is_some_and(|skill| pet::untrain(save, skill).is_some()),
        CompanionAction::Reset => pet::reset(save).is_some(),
        CompanionAction::Look(i) => match all_looks().nth(i) {
            Some(look) if companion_unlocked(save) && offered(look, save, earned) => {
                let worn = if view.dressing == 1 && second {
                    &mut save.second_look
                } else {
                    &mut save.companion
                };
                let fresh = look_for(worn).key != look.key;
                if fresh {
                    *worn = look.key.into();
                }
                fresh
            }
            _ => false,
        },
        CompanionAction::Dressing(slot) => {
            let fresh = view.dressing != slot && (slot == 0 || (slot == 1 && second));
            if fresh {
                view.dressing = slot;
            }
            fresh
        }
        CompanionAction::Rename => {
            save.pet_name = pet::next_name(pet::name(save)).into();
            true
        }
    };
    // Without the Second Companion, the grid dresses the first pet.
    if view.dressing != 0 && !pet::PetSkills::of(save).second {
        view.dressing = 0;
        return true;
    }
    changed
}

/// Each recorded run pays the companion its experience, if it came along: what it fetched and the
/// run itself, announced on the results screen with any level it reached.
fn award_pet_xp(
    mut ended: MessageReader<RunEnded>,
    mut profile: ResMut<Profile>,
    mut haul: ResMut<PetRun>,
    mut notes: ResMut<ResultsNotes>,
) {
    for _ in ended.read() {
        let items = std::mem::take(&mut haul.fetched);
        if !companion_unlocked(&profile.rewards) {
            continue;
        }
        let lines = grant_pet_xp(&mut profile.rewards, items);
        if !lines.is_empty() {
            profile.mark_dirty();
        }
        for line in lines {
            notes.push(line, NoteKind::Good);
        }
    }
}

/// A run's experience into `save`, for `items` fetched: its results lines, "+N pet XP" and any
/// level reached; none at the top level.
fn grant_pet_xp(save: &mut RewardsSave, items: u32) -> Vec<String> {
    let (took, before, after) = pet::add_xp(save, pet::run_xp(items));
    let mut lines = Vec::new();
    if took > 0 {
        lines.push(format!("+{took} pet XP"));
    }
    if after > before {
        lines.push(format!("{} reached level {after}!", pet::name(save)));
    }
    lines
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::super::bonus::StatKey;
    use super::super::hand::{BANISHES_PER_RUN, REROLLS_PER_RUN};
    use super::super::profile::RunOutcome;
    use super::*;

    fn index(id: &str) -> usize {
        PERKS.iter().position(|p| p.id == id).unwrap()
    }

    fn look_index(key: &str) -> usize {
        all_looks().position(|l| l.key == key).unwrap()
    }

    fn skill_index(id: &str) -> usize {
        pet::SKILLS.iter().position(|s| s.id == id).unwrap()
    }

    #[test]
    fn points_spend_from_achievements_and_the_test_aid() {
        let mut profile = Profile::default();
        profile.achievements.earned.insert("runs_10".into(), 10);
        profile.achievements.earned.insert("wins_5".into(), 12);
        assert_eq!(points(&profile, DevPoints(0)), 15);
        assert_eq!(points(&profile, DevPoints(100)), 115);
        profile.rewards.spent = 10;
        assert_eq!(points(&profile, DevPoints(0)), 5);
    }

    #[test]
    fn clicks_buy_refund_and_reset() {
        let mut save = RewardsSave::default();
        let mut session = PerkSession::default();
        let companion = index("companion");
        assert!(!act(
            RewardsAction::Buy(companion),
            &mut save,
            &mut session,
            24
        ));
        assert!(act(
            RewardsAction::Buy(companion),
            &mut save,
            &mut session,
            25
        ));
        assert!(companion_unlocked(&save));
        assert!(act(
            RewardsAction::Refund(companion),
            &mut save,
            &mut session,
            0
        ));
        assert!(!companion_unlocked(&save));
        assert!(act(
            RewardsAction::Buy(index("hardy")),
            &mut save,
            &mut session,
            10
        ));
        assert!(act(RewardsAction::Reset, &mut save, &mut session, 0));
        assert_eq!(save.spent, 0);
        assert!(!act(RewardsAction::Buy(99), &mut save, &mut session, 1_000));
    }

    #[test]
    fn companion_clicks_dress_train_and_rename() {
        let mut save = RewardsSave::default();
        let mut earned = AchievementSave::default();
        let mut view = CompanionView::default();
        let frog = look_index("frog");
        assert!(
            !tend(CompanionAction::Look(frog), &mut save, &earned, &mut view),
            "no companion to dress yet"
        );
        assert!(
            !tend(CompanionAction::Train(0), &mut save, &earned, &mut view),
            "nor to train"
        );
        save.perks.insert("companion".into(), 1);
        assert!(tend(
            CompanionAction::Look(frog),
            &mut save,
            &earned,
            &mut view
        ));
        assert_eq!(save.companion, "frog");
        assert!(
            !tend(CompanionAction::Look(frog), &mut save, &earned, &mut view),
            "already worn"
        );
        assert!(!tend(
            CompanionAction::Look(99),
            &mut save,
            &earned,
            &mut view
        ));
        // Locked looks wait for their source: a quest chain's, a level's, an achievement's.
        let snake = look_index("black_kingsnake");
        assert!(!tend(
            CompanionAction::Look(snake),
            &mut save,
            &earned,
            &mut view
        ));
        save.unlocked_looks.insert(1206);
        assert!(tend(
            CompanionAction::Look(snake),
            &mut save,
            &earned,
            &mut view
        ));
        assert_eq!(look_for(&save.companion).display, 1206);
        let diablo = look_index("mini_diablo");
        assert!(!tend(
            CompanionAction::Look(diablo),
            &mut save,
            &earned,
            &mut view
        ));
        earned.earned.insert("bosses_100".into(), 3);
        assert!(tend(
            CompanionAction::Look(diablo),
            &mut save,
            &earned,
            &mut view
        ));

        // Training, untraining and the free reset.
        let nip = skill_index("nip");
        assert!(tend(
            CompanionAction::Train(nip),
            &mut save,
            &earned,
            &mut view
        ));
        assert!(
            !tend(CompanionAction::Train(nip), &mut save, &earned, &mut view),
            "one point at level 1"
        );
        assert!(tend(
            CompanionAction::Untrain(nip),
            &mut save,
            &earned,
            &mut view
        ));
        assert!(!tend(
            CompanionAction::Untrain(nip),
            &mut save,
            &earned,
            &mut view
        ));
        assert!(!tend(CompanionAction::Reset, &mut save, &earned, &mut view));
        assert!(!tend(
            CompanionAction::Train(99),
            &mut save,
            &earned,
            &mut view
        ));

        // The second pet is dressed from the grid once it is trained.
        assert!(!tend(
            CompanionAction::Dressing(1),
            &mut save,
            &earned,
            &mut view
        ));
        save.pet_xp = pet::xp_for_level(pet::MAX_LEVEL);
        let second = skill_index("second_companion");
        assert!(tend(
            CompanionAction::Train(second),
            &mut save,
            &earned,
            &mut view
        ));
        assert!(tend(
            CompanionAction::Dressing(1),
            &mut save,
            &earned,
            &mut view
        ));
        let cat = look_index("cat");
        assert!(tend(
            CompanionAction::Look(cat),
            &mut save,
            &earned,
            &mut view
        ));
        assert_eq!(
            (save.companion.as_str(), save.second_look.as_str()),
            ("mini_diablo", "cat")
        );
        // A reset sends the second pet off, and the grid back to the first.
        assert!(tend(CompanionAction::Reset, &mut save, &earned, &mut view));
        assert_eq!(view.dressing, 0);
        assert!(tend(
            CompanionAction::Look(frog),
            &mut save,
            &earned,
            &mut view
        ));
        assert_eq!(
            (save.companion.as_str(), save.second_look.as_str()),
            ("frog", "cat")
        );

        // Names go round the list.
        assert!(tend(CompanionAction::Rename, &mut save, &earned, &mut view));
        assert_eq!(pet::name(&save), pet::PET_NAMES[1]);
    }

    fn outcome() -> RunOutcome {
        RunOutcome {
            class: 0,
            map: 0,
            won: false,
            seconds: 300.0,
            kills: 100,
            bosses: 1,
            level: 12,
            damage: 1000.0,
            gold: 40,
            reward_mult: 1.0,
            quests: 0,
            final_boss: false,
        }
    }

    /// A recorded run with `fetched` items fetched, and the notes it left.
    fn record(profile: Profile, fetched: u32) -> (Profile, Vec<String>, u32) {
        let mut world = World::new();
        world.init_resource::<Messages<RunEnded>>();
        world.init_resource::<ResultsNotes>();
        world.insert_resource(PetRun { fetched });
        world.insert_resource(profile);
        world.write_message(RunEnded { outcome: outcome() });
        world.run_system_once(award_pet_xp).unwrap();
        let notes = world
            .resource::<ResultsNotes>()
            .0
            .iter()
            .map(|(text, kind)| {
                assert_eq!(*kind, NoteKind::Good);
                text.clone()
            })
            .collect();
        let left = world.resource::<PetRun>().fetched;
        (world.resource::<Profile>().clone(), notes, left)
    }

    #[test]
    fn a_recorded_run_pays_the_pet_and_says_so() {
        let mut profile = Profile::default();
        profile.rewards.perks.insert("companion".into(), 1);
        let (profile, notes, left) = record(profile, 60);
        assert_eq!(profile.rewards.pet_xp, 90);
        assert_eq!(notes, ["+90 pet XP", "Peacebloom reached level 2!"]);
        assert_eq!(left, 0, "the haul is spent");

        // A run that fetched nothing still pays, and the pet goes by its new name.
        let mut renamed = profile;
        renamed.rewards.pet_name = "Icecap".into();
        let (profile, notes, _) = record(renamed, 0);
        assert_eq!(profile.rewards.pet_xp, 120);
        assert_eq!(notes, ["+30 pet XP", "Icecap reached level 3!"]);
        // A run short of the next level says only what it paid.
        let (profile, notes, _) = record(profile, 10);
        assert_eq!(profile.rewards.pet_xp, 160);
        assert_eq!(notes, ["+40 pet XP"]);

        // At the top level there is nothing to say.
        let mut top = profile;
        top.rewards.pet_xp = pet::xp_for_level(pet::MAX_LEVEL);
        let (_, notes, _) = record(top, 500);
        assert!(notes.is_empty());

        // Without the companion, no pet came along.
        let (profile, notes, left) = record(Profile::default(), 60);
        assert_eq!(profile.rewards.pet_xp, 0);
        assert!(notes.is_empty());
        assert_eq!(left, 0);
    }

    #[test]
    fn perks_reach_the_meta_bonuses_and_the_runs_hand() {
        let mut world = World::new();
        let mut profile = Profile::default();
        profile.rewards.perks.insert("hardy".into(), 2);
        profile.rewards.perks.insert("extra_reroll".into(), 3);
        profile.rewards.perks.insert("extra_banish".into(), 1);
        world.insert_resource(profile);
        world.init_resource::<MetaParts>();
        let hero = world.spawn((HeroState::default(), LocalHero)).id();
        world.run_system_once(apply_perks).unwrap();
        assert_eq!(
            world.resource::<MetaParts>().perks.get(StatKey::MaxHealth),
            40.0
        );
        world.run_system_once(stamp_hand).unwrap();
        let me = world.get::<HeroState>(hero).unwrap();
        assert_eq!(me.rerolls, REROLLS_PER_RUN + 3);
        assert_eq!(me.banishes, BANISHES_PER_RUN + 1);
    }
}
