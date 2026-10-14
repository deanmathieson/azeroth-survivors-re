//! An accepted quest's progress, kept apart from the battleground: what counts, how far along the
//! party is, the clock, and the lines the tracker shows. Progress is the party's, so nothing here
//! asks whose kill or pickup it was.

use super::catalog::{Goal, QuestDef};

/// Where an accepted quest stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum Status {
    /// Under way, the clock running.
    Active,
    /// Done: back to the giver to turn it in, with no clock.
    Complete,
    /// The clock ran out first.
    Failed,
}

/// One accepted quest's progress.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::survivors) struct Tracker {
    pub goal: Goal,
    /// Kills, items or elites counted, never past what the goal asks.
    pub count: u32,
    /// Seconds the spot has been held, for a Defend.
    pub held: f32,
    /// Seconds left before it fails.
    pub left: f32,
    pub status: Status,
}

impl Tracker {
    /// `quest` as it is accepted, its whole time limit to go.
    pub(in crate::survivors) fn new(quest: &QuestDef) -> Self {
        Self {
            goal: quest.goal,
            count: 0,
            held: 0.0,
            left: quest.limit,
            status: Status::Active,
        }
    }

    /// A foe named `name` fell, an elite or not, whoever slew it.
    pub(in crate::survivors) fn kill(&mut self, name: &str, elite: bool) {
        if self.status != Status::Active {
            return;
        }
        match self.goal {
            Goal::Kill { foe, count } if foe.is_none_or(|f| f == name) => {
                self.count = (self.count + 1).min(count);
            }
            Goal::SlayElite if elite => self.count = 1,
            _ => {}
        }
        self.settle();
    }

    /// The chance a kill of `name` drops the quest item, while it is still wanted.
    pub(in crate::survivors) fn drop_chance(&self, name: &str) -> Option<f32> {
        match self.goal {
            Goal::Collect { foe, chance, .. } if self.status == Status::Active && foe == name => {
                Some(chance)
            }
            _ => None,
        }
    }

    /// A hero picked up the quest item.
    pub(in crate::survivors) fn pick_up(&mut self) {
        if self.status != Status::Active {
            return;
        }
        if let Goal::Collect { count, .. } = self.goal {
            self.count = (self.count + 1).min(count);
        }
        self.settle();
    }

    /// `dt` seconds pass, with a hero standing in the Defend spot or not: the spot is held, and
    /// the clock runs down. Once done or failed, nothing changes.
    pub(in crate::survivors) fn tick(&mut self, dt: f32, in_spot: bool) {
        if self.status != Status::Active {
            return;
        }
        if let Goal::Defend { secs } = self.goal {
            if in_spot {
                self.held = (self.held + dt).min(secs);
            }
        }
        self.settle();
        if self.status == Status::Active {
            self.left = (self.left - dt).max(0.0);
            if self.left <= 0.0 {
                self.status = Status::Failed;
            }
        }
    }

    fn settle(&mut self) {
        if self.status == Status::Active && self.done() {
            self.status = Status::Complete;
        }
    }

    /// Whether the goal is met.
    pub(in crate::survivors) fn done(&self) -> bool {
        match self.goal {
            Goal::Kill { count, .. } | Goal::Collect { count, .. } => self.count >= count,
            Goal::Defend { secs } => self.held >= secs,
            Goal::SlayElite => self.count >= 1,
        }
    }

    /// The objective line, as the quest log words one: "Kobold Tunneler slain: 3/10".
    pub(in crate::survivors) fn objective(&self) -> String {
        objective_line(self.goal, self.count, self.held)
    }
}

/// The objective line for `goal` with `count` counted and `held` seconds held.
pub(in crate::survivors) fn objective_line(goal: Goal, count: u32, held: f32) -> String {
    match goal {
        Goal::Kill {
            foe: Some(foe),
            count: of,
        } => format!("{foe} slain: {count}/{of}"),
        Goal::Kill {
            foe: None,
            count: of,
        } => format!("Enemies slain: {count}/{of}"),
        Goal::Collect {
            item, count: of, ..
        } => format!("{item}: {count}/{of}"),
        Goal::Defend { secs } => format!("Spot held: {}/{} s", held.floor() as u32, secs as u32),
        Goal::SlayElite => format!("Elite slain: {count}/1"),
    }
}

/// Seconds as `m:ss`.
pub(in crate::survivors) fn clock(secs: f32) -> String {
    let s = secs.max(0.0).ceil() as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::survivors::quests::catalog::ItemLook;

    fn quest(goal: Goal, limit: f32) -> QuestDef {
        QuestDef {
            id: "test",
            name: "Test",
            text: "",
            goal,
            limit,
        }
    }

    #[test]
    fn a_kill_count_counts_its_foe_or_any() {
        let goal = Goal::Kill {
            foe: Some("Kobold Tunneler"),
            count: 3,
        };
        let mut t = Tracker::new(&quest(goal, 60.0));
        t.kill("Kobold Miner", false);
        assert_eq!(t.count, 0, "another foe does not count");
        t.kill("Kobold Tunneler", false);
        t.kill("Kobold Tunneler", true);
        assert_eq!((t.count, t.status), (2, Status::Active));
        assert_eq!(t.objective(), "Kobold Tunneler slain: 2/3");
        t.tick(10.0, false);
        assert_eq!(t.left, 50.0);
        t.kill("Kobold Tunneler", false);
        assert_eq!((t.count, t.status), (3, Status::Complete));
        // Done: the clock stops and more kills change nothing.
        t.kill("Kobold Tunneler", false);
        t.tick(100.0, false);
        assert_eq!((t.count, t.left, t.status), (3, 50.0, Status::Complete));

        let mut any = Tracker::new(&quest(
            Goal::Kill {
                foe: None,
                count: 2,
            },
            60.0,
        ));
        any.kill("Young Wolf", false);
        any.kill("Hogger", false);
        assert_eq!(any.status, Status::Complete);
        assert_eq!(any.objective(), "Enemies slain: 2/2");
    }

    #[test]
    fn a_kill_count_fails_when_the_clock_runs_out() {
        let goal = Goal::Kill {
            foe: Some("Murloc"),
            count: 5,
        };
        let mut t = Tracker::new(&quest(goal, 30.0));
        t.kill("Murloc", false);
        t.tick(29.0, false);
        assert_eq!(t.status, Status::Active);
        t.tick(2.0, false);
        assert_eq!((t.status, t.left), (Status::Failed, 0.0));
        // Failed for good: a late kill does not finish it.
        for _ in 0..10 {
            t.kill("Murloc", false);
        }
        assert_eq!((t.count, t.status), (1, Status::Failed));
    }

    #[test]
    fn a_collect_drops_from_its_foe_and_counts_pickups() {
        let goal = Goal::Collect {
            item: "Large Candle",
            look: ItemLook::Candle,
            foe: "Kobold Miner",
            chance: 0.4,
            count: 2,
        };
        let mut t = Tracker::new(&quest(goal, 90.0));
        assert_eq!(t.drop_chance("Kobold Miner"), Some(0.4));
        assert_eq!(t.drop_chance("Kobold Tunneler"), None);
        t.kill("Kobold Miner", false);
        assert_eq!(t.count, 0, "a kill alone is no candle");
        t.pick_up();
        assert_eq!(t.objective(), "Large Candle: 1/2");
        t.pick_up();
        assert_eq!(t.status, Status::Complete);
        assert_eq!(
            t.drop_chance("Kobold Miner"),
            None,
            "no more drops once done"
        );
        t.pick_up();
        assert_eq!(t.count, 2);

        let mut late = Tracker::new(&quest(goal, 10.0));
        late.pick_up();
        late.tick(10.0, false);
        assert_eq!(late.status, Status::Failed);
        assert_eq!(late.drop_chance("Kobold Miner"), None);
    }

    #[test]
    fn a_defend_holds_only_while_a_hero_stands_in_the_spot() {
        let mut t = Tracker::new(&quest(Goal::Defend { secs: 10.0 }, 30.0));
        t.tick(5.0, true);
        t.tick(5.0, false);
        assert_eq!((t.held, t.left), (5.0, 20.0));
        assert_eq!(t.objective(), "Spot held: 5/10 s");
        // Kills never count for a Defend.
        t.kill("Defias Thug", true);
        assert_eq!(t.status, Status::Active);
        t.tick(5.0, true);
        assert_eq!((t.held, t.status), (10.0, Status::Complete));
        assert_eq!(t.left, 20.0, "the clock stops the moment it is held");

        let mut away = Tracker::new(&quest(Goal::Defend { secs: 10.0 }, 30.0));
        for _ in 0..3 {
            away.tick(9.0, true);
            away.tick(9.0, false);
        }
        assert_eq!(away.status, Status::Complete, "held in parts adds up");
        let mut lost = Tracker::new(&quest(Goal::Defend { secs: 10.0 }, 30.0));
        lost.tick(9.0, true);
        lost.tick(21.0, false);
        assert_eq!(lost.status, Status::Failed);
    }

    #[test]
    fn slay_elite_wants_the_next_elite() {
        let mut t = Tracker::new(&quest(Goal::SlayElite, 120.0));
        t.kill("Hogger", false);
        assert_eq!(t.objective(), "Elite slain: 0/1");
        t.tick(60.0, false);
        t.kill("Rockhide Boar", true);
        assert_eq!(
            (t.status, t.objective().as_str()),
            (Status::Complete, "Elite slain: 1/1")
        );
        let mut slow = Tracker::new(&quest(Goal::SlayElite, 120.0));
        slow.tick(120.0, false);
        assert_eq!(slow.status, Status::Failed);
    }

    #[test]
    fn clocks_read_as_minutes_and_seconds() {
        assert_eq!(clock(0.0), "0:00");
        assert_eq!(clock(59.2), "1:00");
        assert_eq!(clock(150.0), "2:30");
        assert_eq!(clock(-3.0), "0:00");
    }
}
