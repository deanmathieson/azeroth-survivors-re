//! Rewards bought with what runs earn: [`perks`] and the [`companion`] one of them brings, paid for
//! in reward points. Points to spend are the earned achievements' points
//! ([`super::achievements`]) less what the perks cost, plus the `WOW_SURVIVORS_POINTS` test aid
//! ([`DevPoints`]). The record lives in [`RewardsSave`] on the profile; the stat perks reach a run
//! through `MetaParts::perks` ([`super::meta`]), and the reroll and banish perks are added to the
//! run's hand as it begins ([`stamp_hand`]). The Armory's Rewards tab draws them; its clicks land
//! here as [`RewardsAction`]s.

mod companion;
mod perks;
mod save;

use bevy::prelude::*;

use super::achievements::earned_points;
use super::armory::{ArmoryState, ArmoryTab};
use super::meta::MetaParts;
use super::profile::Profile;
use super::{HeroState, LocalHero, Phase};
use crate::glue::GlueClicks;

pub(super) use companion::{all_looks, earned_look, look_for, offered, FETCH_EVERY, FETCH_REACH};
pub(super) use perks::{companion_unlocked, Perk, PerkSession, PERKS};
pub(super) use save::RewardsSave;

/// This workstream's systems, registered by [`super::SurvivorsPlugin`].
pub(super) fn plugin(app: &mut App) {
    app.insert_resource(DevPoints::from_env())
        .init_resource::<PerkSession>()
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
                (end_session, reward_clicks).chain(),
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
    /// Dress the companion in the `i`th of [`all_looks`], if it is offered.
    Look(usize),
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

/// A Rewards tab click: buy, refund, reset or dress, then save and redraw the tab.
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
        RewardsAction::Look(i) => match all_looks().nth(i) {
            Some(look)
                if companion_unlocked(save)
                    && offered(look, save)
                    && look_for(&save.companion).key != look.key =>
            {
                save.companion = look.key.into();
                true
            }
            _ => false,
        },
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::super::bonus::StatKey;
    use super::super::hand::{BANISHES_PER_RUN, REROLLS_PER_RUN};
    use super::*;

    fn index(id: &str) -> usize {
        PERKS.iter().position(|p| p.id == id).unwrap()
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
    fn clicks_buy_refund_reset_and_dress() {
        let mut save = RewardsSave::default();
        let mut session = PerkSession::default();
        let companion = index("companion");
        assert!(!act(
            RewardsAction::Buy(companion),
            &mut save,
            &mut session,
            24
        ));
        assert!(
            !act(RewardsAction::Look(2), &mut save, &mut session, 0),
            "no companion to dress yet"
        );
        assert!(act(
            RewardsAction::Buy(companion),
            &mut save,
            &mut session,
            25
        ));
        assert!(act(RewardsAction::Look(2), &mut save, &mut session, 0));
        assert_eq!(save.companion, super::companion::LOOKS[2].key);
        assert!(
            !act(RewardsAction::Look(2), &mut save, &mut session, 0),
            "already worn"
        );
        assert!(!act(RewardsAction::Look(99), &mut save, &mut session, 0));
        // A quest chain's look is offered once its chain unlocked it.
        let snake = all_looks()
            .position(|l| l.key == "black_kingsnake")
            .unwrap();
        assert!(!act(RewardsAction::Look(snake), &mut save, &mut session, 0));
        save.unlocked_looks.insert(1206);
        assert!(act(RewardsAction::Look(snake), &mut save, &mut session, 0));
        assert_eq!(look_for(&save.companion).display, 1206);
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
