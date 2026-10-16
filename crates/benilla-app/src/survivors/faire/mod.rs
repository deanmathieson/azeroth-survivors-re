//! The Darkmoon Faire: a prize booth between runs, played for Darkmoon Prize Tickets that only
//! playing earns ([`tickets`]); there is no money in it anywhere. Each recorded run pays a share of
//! its gold, its win, a zone's first win, the quest-chain steps and the achievements it brought,
//! double in the zone the Faire is visiting ([`tickets::faire_zone`]), and says so on the results
//! screen ([`pay_tickets`]).
//!
//! The menu's Darkmoon Faire button opens a full-screen panel ([`screen`]): the Prize Booth pulls
//! prizes from the registry ([`catalog`]) at published odds with pity, on the week's featured
//! banner or the standing one ([`booth`]); duplicates turn into Faire Favor, which Silas Darkmoon's
//! Prize Counter takes for any prize; the Collection shows what the account has; and Odds & Pity
//! prints every number. A pull's prizes flip up as cards ([`reveal`]).
//!
//! The record lives in [`FaireSave`] on the profile; unlocks land in the stores that already keep
//! their kind ([`catalog::grant`]).
//!
//! Test aid: `WOW_SURVIVORS_TICKETS=<n>` adds n tickets in memory, never saved ([`DevTickets`]);
//! what they pull is saved as any pull is.

mod booth;
pub(in crate::survivors) mod catalog;
mod reveal;
mod save;
mod screen;
mod tickets;

use bevy::prelude::*;

use super::hud::{NoteKind, ResultsNotes, MAX_NOTES};
use super::profile::{self, LastRunNews, Profile, RunEnded};
use super::Phase;

#[cfg_attr(not(feature = "coop"), allow(unused_imports))]
pub(super) use catalog::Rarity as PrizeRarity;
pub(super) use catalog::{catalog, Kind, Prize};
pub(super) use save::FaireSave;
pub(super) use screen::{closed, FaireUi};

/// This workstream's systems, registered by [`super::SurvivorsPlugin`].
pub(super) fn plugin(app: &mut App) {
    app.insert_resource(DevTickets::from_env())
        .init_resource::<FaireLedger>()
        .add_message::<FairePulled>()
        .add_systems(
            OnEnter(Phase::GameOver),
            note_ledger.after(profile::record_finished),
        )
        .add_systems(PostUpdate, pay_tickets.before(profile::save_if_dirty))
        .add_plugins((screen::plugin, reveal::plugin));
}

/// A pull at the Prize Booth: each prize's name and rarity, for the co-op lobby's chat (the pull
/// itself stays this account's).
#[cfg_attr(not(feature = "coop"), allow(dead_code))]
#[derive(Message, Clone, Debug, PartialEq)]
pub(super) struct FairePulled(pub Vec<(String, catalog::Rarity)>);

/// `WOW_SURVIVORS_TICKETS=<n>`, a test aid for screenshots: n more tickets to pull with, held in
/// memory and spent before the saved ones, never saved.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct DevTickets(pub(super) u32);

impl DevTickets {
    fn from_env() -> Self {
        let Some(text) = crate::webenv::var("WOW_SURVIVORS_TICKETS") else {
            return Self(0);
        };
        match text.trim().parse() {
            Ok(n) => {
                info!(
                    "survivors: {n} Darkmoon Prize Tickets from WOW_SURVIVORS_TICKETS, not saved"
                );
                Self(n)
            }
            Err(_) => {
                warn!("survivors: WOW_SURVIVORS_TICKETS={text:?} is not a whole number");
                Self(0)
            }
        }
    }
}

/// The quest-chain steps and achievements the account had as the run was recorded, before the
/// features judged it: [`pay_tickets`] pays for what the run added since.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
struct FaireLedger {
    steps: u32,
    achievements: usize,
    /// Set by [`note_ledger`] for the run being recorded, spent by [`pay_tickets`].
    noted: bool,
}

impl FaireLedger {
    fn of(profile: &Profile) -> Self {
        Self {
            steps: chain_steps(profile),
            achievements: profile.achievements.earned.len(),
            noted: true,
        }
    }
}

/// Every zone chain's steps done, summed.
fn chain_steps(profile: &Profile) -> u32 {
    profile
        .quests
        .zones
        .values()
        .map(|z| u32::from(z.steps))
        .sum()
}

/// On the GameOver frame, right after the run is recorded and before any feature judges it.
fn note_ledger(profile: Res<Profile>, mut ledger: ResMut<FaireLedger>) {
    *ledger = FaireLedger::of(&profile);
}

/// Each recorded run pays its tickets once, after every feature has judged it (so the chain steps
/// and achievements it finished are in), and before the frame's one save. The line goes where the
/// results screen shows it, however many notes the run had. A run [`note_ledger`] did not note
/// (or one already paid) pays nothing.
fn pay_tickets(
    mut ended: MessageReader<RunEnded>,
    news: Res<LastRunNews>,
    mut ledger: ResMut<FaireLedger>,
    mut profile: ResMut<Profile>,
    mut notes: ResMut<ResultsNotes>,
) {
    for RunEnded { outcome } in ended.read() {
        // Noted as the run was recorded, and paid only once.
        if !ledger.noted {
            continue;
        }
        let now = FaireLedger::of(&profile);
        let steps = now.steps.saturating_sub(ledger.steps);
        let achievements = now.achievements.saturating_sub(ledger.achievements);
        *ledger = FaireLedger {
            noted: false,
            ..now
        };
        // The run is counted already: the Faire stood where it did for the runs before it.
        let runs_before = profile.account.runs.saturating_sub(1);
        let earned = tickets::earn(
            outcome,
            news.first_zone_win,
            steps,
            u32::try_from(achievements).unwrap_or(u32::MAX),
            tickets::faire_map(runs_before),
        );
        let n = earned.total();
        if n == 0 {
            continue;
        }
        let faire = &mut profile.faire;
        faire.tickets = faire.tickets.saturating_add(n);
        faire.tickets_earned = faire.tickets_earned.saturating_add(u64::from(n));
        profile.mark_dirty();
        let at = notes.0.len().min(MAX_NOTES - 1);
        notes.0.insert(
            at,
            (
                earned.note(tickets::faire_zone(runs_before)),
                NoteKind::Good,
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::{RunSystemOnce, SystemId};

    use super::super::data;
    use super::super::profile::RunOutcome;
    use super::*;

    fn index(name: &str) -> usize {
        data::MAPS.iter().position(|m| m.name == name).unwrap()
    }

    fn outcome(map: usize, gold: u32, won: bool) -> RunOutcome {
        RunOutcome {
            class: 0,
            map,
            won,
            seconds: 900.0,
            kills: 1_000,
            bosses: 3,
            level: 25,
            damage: 1.0,
            gold,
            reward_mult: 1.0,
            quests: 0,
            final_boss: false,
        }
    }

    /// One recorded run through the ledger and the payout, with `between` standing in for the
    /// features that judge the run in between.
    fn play(
        world: &mut World,
        pay: SystemId,
        run: RunOutcome,
        first_zone_win: bool,
        between: impl FnOnce(&mut Profile),
    ) {
        world.resource_mut::<Profile>().record(&run);
        world.run_system_once(note_ledger).unwrap();
        between(&mut world.resource_mut::<Profile>());
        world.resource_mut::<LastRunNews>().first_zone_win = first_zone_win;
        world
            .resource_mut::<Messages<RunEnded>>()
            .write(RunEnded { outcome: run });
        world.run_system(pay).unwrap();
    }

    /// A world with what the payout reads, and the payout registered so its reader keeps its
    /// place between runs, as the app's does.
    fn world() -> (World, SystemId) {
        let mut world = World::new();
        world.init_resource::<Messages<RunEnded>>();
        world.init_resource::<ResultsNotes>();
        world.init_resource::<LastRunNews>();
        world.init_resource::<FaireLedger>();
        world.insert_resource(Profile::default());
        let pay = world.register_system(pay_tickets);
        (world, pay)
    }

    #[test]
    fn a_run_pays_its_tickets_once_and_says_so() {
        let (mut world, pay) = world();
        // The first run is in Elwynn Forest, where the Faire starts: doubled. A first win on 330
        // gold, a chain step and two achievements: (16 + 10 + 25 + 15 + 10) × 2.
        play(
            &mut world,
            pay,
            outcome(index("Elwynn Forest"), 330, true),
            true,
            |p| {
                p.quests
                    .zones
                    .entry("elwynn_forest".into())
                    .or_default()
                    .steps = 1;
                p.achievements.earned.insert("a".into(), 1);
                p.achievements.earned.insert("b".into(), 1);
            },
        );
        let faire = &world.resource::<Profile>().faire;
        assert_eq!((faire.tickets, faire.tickets_earned), (152, 152));
        assert_eq!(
            world.resource::<ResultsNotes>().0,
            [(
                "+152 Darkmoon Prize Tickets (doubled: the Faire is in Elwynn Forest!)".into(),
                NoteKind::Good
            )]
        );
        // Read again (a fresh reader sees the same message): nothing is paid twice.
        world.run_system_once(pay_tickets).unwrap();
        assert_eq!(world.resource::<Profile>().faire.tickets, 152);
        assert_eq!(world.resource::<ResultsNotes>().0.len(), 1);
    }

    #[test]
    fn the_line_shows_among_many_notes_and_elsewhere_pays_single() {
        let (mut world, pay) = world();
        for i in 0..6 {
            world
                .resource_mut::<ResultsNotes>()
                .push(format!("note {i}"), NoteKind::Achievement);
        }
        play(
            &mut world,
            pay,
            outcome(index("Mulgore"), 100, false),
            false,
            |_| {},
        );
        assert_eq!(world.resource::<Profile>().faire.tickets, 5);
        let notes = &world.resource::<ResultsNotes>().0;
        assert_eq!(notes[MAX_NOTES - 1].0, "+5 Darkmoon Prize Tickets");
        assert_eq!(notes.len(), 7);
        // A run worth nothing says nothing.
        play(
            &mut world,
            pay,
            outcome(index("Mulgore"), 7, false),
            false,
            |_| {},
        );
        assert_eq!(world.resource::<Profile>().faire.tickets, 5);
        assert_eq!(world.resource::<ResultsNotes>().0.len(), 7);
    }

    #[test]
    fn the_faire_moves_to_mulgore_after_five_runs() {
        let (mut world, pay) = world();
        let mulgore = index("Mulgore");
        // Runs 1 to 5: the Faire is in Elwynn Forest, Mulgore pays single.
        for _ in 0..5 {
            play(&mut world, pay, outcome(mulgore, 200, false), false, |_| {});
        }
        assert_eq!(world.resource::<Profile>().faire.tickets, 5 * 10);
        // Run 6: the Faire is in Mulgore.
        play(&mut world, pay, outcome(mulgore, 200, false), false, |_| {});
        assert_eq!(world.resource::<Profile>().faire.tickets, 50 + 20);
    }
}
