//! Darkmoon Prize Tickets, the account's Faire currency, earned only by playing: a share of each
//! run's gold, a win, a first win in a zone, each quest-chain step and each achievement the run
//! brings. The Faire itself travels: it pitches its tents in Elwynn Forest and Mulgore by turns,
//! [`RUNS_PER_STAY`] runs in each (counted on the account's runs, never a clock), and a run in
//! the Faire's zone pays double.

use super::super::data;
use super::super::profile::RunOutcome;

/// Gold a ticket costs from a run's purse, rounded down.
pub(in crate::survivors) const GOLD_PER_TICKET: u32 = 20;
/// Tickets for a win, and more for a zone's first win.
pub(in crate::survivors) const WIN_TICKETS: u32 = 10;
pub(in crate::survivors) const FIRST_WIN_TICKETS: u32 = 25;
/// Tickets for each quest-chain step a run finishes (the last one, the chain, included).
pub(in crate::survivors) const CHAIN_STEP_TICKETS: u32 = 15;
/// Tickets for each achievement a run earns.
pub(in crate::survivors) const ACHIEVEMENT_TICKETS: u32 = 5;
/// What a run in the Faire's zone multiplies its tickets by.
pub(in crate::survivors) const FAIRE_ZONE_MULT: u32 = 2;
/// Runs the Faire stays in a zone, and the featured banner on the booth.
pub(in crate::survivors) const RUNS_PER_STAY: u32 = 5;
/// The zones the Faire visits by turns.
pub(in crate::survivors) const FAIRE_ZONES: [&str; 2] = ["Elwynn Forest", "Mulgore"];

/// The zone the Faire is in for the run after `runs` recorded ones.
pub(in crate::survivors) fn faire_zone(runs: u32) -> &'static str {
    FAIRE_ZONES[(runs / RUNS_PER_STAY) as usize % FAIRE_ZONES.len()]
}

/// [`faire_zone`]'s index in [`data::MAPS`], if this build has it.
pub(in crate::survivors) fn faire_map(runs: u32) -> Option<usize> {
    let zone = faire_zone(runs);
    data::MAPS.iter().position(|m| m.name == zone)
}

/// Runs, counting the next, before the Faire moves on: 5 on the first run of a stay, 1 on its
/// last.
pub(in crate::survivors) fn runs_left_here(runs: u32) -> u32 {
    RUNS_PER_STAY - runs % RUNS_PER_STAY
}

/// What one run earned, by source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::survivors) struct Earned {
    pub gold: u32,
    pub win: u32,
    pub first_win: u32,
    pub chain_steps: u32,
    pub achievements: u32,
    /// The run was in the Faire's zone: everything above pays [`FAIRE_ZONE_MULT`] times.
    pub faire_zone: bool,
}

impl Earned {
    /// The tickets before the Faire's multiplier.
    pub(in crate::survivors) fn base(&self) -> u32 {
        self.gold
            .saturating_add(self.win)
            .saturating_add(self.first_win)
            .saturating_add(self.chain_steps)
            .saturating_add(self.achievements)
    }

    pub(in crate::survivors) fn total(&self) -> u32 {
        let mult = if self.faire_zone { FAIRE_ZONE_MULT } else { 1 };
        self.base().saturating_mul(mult)
    }

    /// The results screen's line: "+42 Darkmoon Prize Tickets (doubled: the Faire is in Mulgore)".
    pub(in crate::survivors) fn note(&self, zone: &str) -> String {
        let n = self.total();
        let s = if n == 1 { "" } else { "s" };
        if self.faire_zone {
            format!("+{n} Darkmoon Prize Ticket{s} (doubled: the Faire is in {zone}!)")
        } else {
            format!("+{n} Darkmoon Prize Ticket{s}")
        }
    }
}

/// What `run` earns: its gold's share, its win and first zone win, and the `chain_steps` and
/// `achievements` it finished, doubled when `faire_map` (the Faire's zone while it ran) is its
/// zone.
pub(in crate::survivors) fn earn(
    run: &RunOutcome,
    first_zone_win: bool,
    chain_steps: u32,
    achievements: u32,
    faire_map: Option<usize>,
) -> Earned {
    Earned {
        gold: run.gold / GOLD_PER_TICKET,
        win: if run.won { WIN_TICKETS } else { 0 },
        first_win: if run.won && first_zone_win {
            FIRST_WIN_TICKETS
        } else {
            0
        },
        chain_steps: chain_steps.saturating_mul(CHAIN_STEP_TICKETS),
        achievements: achievements.saturating_mul(ACHIEVEMENT_TICKETS),
        faire_zone: faire_map == Some(run.map),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index(name: &str) -> usize {
        data::MAPS.iter().position(|m| m.name == name).unwrap()
    }

    fn run(map: usize, gold: u32, won: bool) -> RunOutcome {
        RunOutcome {
            class: 0,
            map,
            won,
            seconds: 600.0,
            kills: 900,
            bosses: 3,
            level: 20,
            damage: 1.0,
            gold,
            reward_mult: 1.0,
            quests: 0,
            final_boss: false,
        }
    }

    #[test]
    fn a_run_earns_its_gold_share_and_its_wins() {
        let westfall = index("Westfall");
        // 259 gold is 12 tickets, rounded down; a loss pays nothing more.
        let loss = earn(&run(westfall, 259, false), false, 0, 0, None);
        assert_eq!((loss.gold, loss.win, loss.first_win), (12, 0, 0));
        assert_eq!(loss.total(), 12);
        assert_eq!(loss.note("Mulgore"), "+12 Darkmoon Prize Tickets");
        // A first win: +10 and +25 on 600 gold's 30.
        let first = earn(&run(westfall, 600, true), true, 0, 0, None);
        assert_eq!(first.total(), 30 + 10 + 25);
        // "First" only counts on a win.
        assert_eq!(earn(&run(westfall, 0, false), true, 0, 0, None).total(), 0);
        // Two chain steps and three achievements.
        let busy = earn(&run(westfall, 40, false), false, 2, 3, None);
        assert_eq!(busy.total(), 2 + 30 + 15);
        assert_eq!(
            earn(&run(westfall, 19, false), false, 0, 0, None).note("x"),
            "+0 Darkmoon Prize Tickets"
        );
        assert_eq!(
            earn(&run(westfall, 20, false), false, 0, 0, None).note("x"),
            "+1 Darkmoon Prize Ticket"
        );
    }

    #[test]
    fn the_faire_zone_pays_double() {
        let elwynn = index("Elwynn Forest");
        let here = Some(elwynn);
        let doubled = earn(&run(elwynn, 300, true), true, 1, 2, here);
        assert!(doubled.faire_zone);
        assert_eq!(doubled.base(), 15 + 10 + 25 + 15 + 10);
        assert_eq!(doubled.total(), 2 * 75);
        assert_eq!(
            doubled.note("Elwynn Forest"),
            "+150 Darkmoon Prize Tickets (doubled: the Faire is in Elwynn Forest!)"
        );
        // Elsewhere, single.
        let mulgore = index("Mulgore");
        assert_eq!(earn(&run(mulgore, 300, true), true, 1, 2, here).total(), 75);
    }

    #[test]
    fn the_faire_moves_every_five_runs() {
        for runs in 0..5 {
            assert_eq!(faire_zone(runs), "Elwynn Forest", "{runs}");
        }
        for runs in 5..10 {
            assert_eq!(faire_zone(runs), "Mulgore", "{runs}");
        }
        assert_eq!(faire_zone(10), "Elwynn Forest");
        assert_eq!(faire_zone(17), "Mulgore");
        assert_eq!(faire_map(0), Some(index("Elwynn Forest")));
        assert_eq!(faire_map(5), Some(index("Mulgore")));
        assert_eq!(runs_left_here(0), 5);
        assert_eq!(runs_left_here(4), 1);
        assert_eq!(runs_left_here(5), 5);
    }
}
