//! The achievement list, the mode's own design: a win on every battleground and with every class,
//! feats of a single run, and lifetime totals. Each is a pure check over the account's [`Profile`]
//! and, as a run ends, that run's [`RunOutcome`]; nothing else is consulted.

use std::sync::LazyLock;

use super::super::data;
use super::super::profile::{slug, Profile, RunOutcome};
use super::AchievementSave;

/// Float dust allowance on the Run Options multiplier, which is summed from slider steps.
const MULT_SLACK: f32 = 1e-3;

/// An achievement's shelf in the Armory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::survivors) enum Category {
    Zones,
    Classes,
    Feats,
    Lifetime,
}

impl Category {
    pub(in crate::survivors) const ALL: [Self; 4] =
        [Self::Zones, Self::Classes, Self::Feats, Self::Lifetime];

    pub(in crate::survivors) fn label(self) -> &'static str {
        match self {
            Self::Zones => "Zones",
            Self::Classes => "Classes",
            Self::Feats => "Feats",
            Self::Lifetime => "Lifetime",
        }
    }
}

/// What an achievement asks for. The "in one run" goals read the ending run's [`RunOutcome`]; the
/// rest read the profile, which already counts that run by the time it is judged.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) enum Goal {
    /// Win a run on [`data::MAPS`]`[i]`.
    ZoneWin(usize),
    /// Win a run as [`data::CLASSES`]`[i]`.
    ClassWin(usize),
    /// Win a run under at least this Run Options rewards multiplier.
    RewardsWin(f32),
    /// Reach this level in one run.
    Level(u32),
    /// Last this many seconds in one run.
    Survive(u32),
    /// Face this many bosses in one run: the mode counts the bosses that come, not those slain.
    RunBosses(u32),
    /// Deal this much damage in one run.
    RunDamage(u64),
    /// Win a run holding at least this much gold.
    WinGold(u32),
    /// Defeat this many enemies in one run.
    RunKills(u32),
    /// Lifetime totals over every recorded run.
    Kills(u64),
    Bosses(u64),
    Gold(u64),
    Runs(u32),
    Wins(u32),
}

/// One achievement. `id` is the stable key a save stores; renaming a zone or a class re-keys its
/// win, as it re-keys that zone's or class's profile record.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::survivors) struct Achievement {
    pub(in crate::survivors) id: String,
    pub(in crate::survivors) name: String,
    pub(in crate::survivors) desc: String,
    /// Reward points, 5 to 25.
    pub(in crate::survivors) points: u32,
    pub(in crate::survivors) category: Category,
    pub(in crate::survivors) goal: Goal,
}

/// Every achievement that is not one per zone or class, in shelf order:
/// `(id, name, description, points, category, goal)`.
const FIXED: &[(&str, &str, &str, u32, Category, Goal)] = &[
    (
        "win_raised_stakes",
        "Raised Stakes",
        "Win a run with rewards at ×1.50 or more.",
        15,
        Category::Zones,
        Goal::RewardsWin(1.5),
    ),
    (
        "level_50",
        "Seasoned",
        "Reach level 50 in a single run.",
        10,
        Category::Classes,
        Goal::Level(50),
    ),
    (
        "level_100",
        "Centurion",
        "Reach level 100 in a single run.",
        25,
        Category::Classes,
        Goal::Level(100),
    ),
    (
        "survive_10_minutes",
        "Ten Minutes Standing",
        "Last 10 minutes in a single run.",
        10,
        Category::Feats,
        Goal::Survive(600),
    ),
    (
        "face_5_bosses",
        "Boss Rush",
        "Face 5 bosses in a single run.",
        15,
        Category::Feats,
        Goal::RunBosses(5),
    ),
    (
        "damage_1m",
        "Seven Figures",
        "Deal 1,000,000 damage in a single run.",
        20,
        Category::Feats,
        Goal::RunDamage(1_000_000),
    ),
    (
        "win_500_gold",
        "Deep Pockets",
        "Win a run holding 500 gold or more.",
        15,
        Category::Feats,
        Goal::WinGold(500),
    ),
    (
        "kills_2000_run",
        "Thinning the Herd",
        "Defeat 2,000 enemies in a single run.",
        10,
        Category::Feats,
        Goal::RunKills(2000),
    ),
    (
        "win_max_stakes",
        "Glutton for Punishment",
        "Win a run with rewards at ×2.00.",
        25,
        Category::Feats,
        Goal::RewardsWin(2.0),
    ),
    (
        "kills_1000",
        "Slayer",
        "Defeat 1,000 enemies across all runs.",
        5,
        Category::Lifetime,
        Goal::Kills(1_000),
    ),
    (
        "kills_10000",
        "Exterminator",
        "Defeat 10,000 enemies across all runs.",
        10,
        Category::Lifetime,
        Goal::Kills(10_000),
    ),
    (
        "kills_100000",
        "Calamity",
        "Defeat 100,000 enemies across all runs.",
        25,
        Category::Lifetime,
        Goal::Kills(100_000),
    ),
    (
        "bosses_10",
        "Boss Hunter",
        "Face 10 bosses across all runs.",
        5,
        Category::Lifetime,
        Goal::Bosses(10),
    ),
    (
        "bosses_100",
        "Nemesis",
        "Face 100 bosses across all runs.",
        15,
        Category::Lifetime,
        Goal::Bosses(100),
    ),
    (
        "bosses_500",
        "Bane of Bosses",
        "Face 500 bosses across all runs.",
        25,
        Category::Lifetime,
        Goal::Bosses(500),
    ),
    (
        "gold_1000",
        "Coin Collector",
        "Pick up 1,000 gold across all runs.",
        5,
        Category::Lifetime,
        Goal::Gold(1_000),
    ),
    (
        "gold_10000",
        "Treasure Hoard",
        "Pick up 10,000 gold across all runs.",
        15,
        Category::Lifetime,
        Goal::Gold(10_000),
    ),
    (
        "runs_10",
        "Regular",
        "Finish 10 runs.",
        5,
        Category::Lifetime,
        Goal::Runs(10),
    ),
    (
        "runs_50",
        "Veteran",
        "Finish 50 runs.",
        10,
        Category::Lifetime,
        Goal::Runs(50),
    ),
    (
        "runs_100",
        "Lifer",
        "Finish 100 runs.",
        20,
        Category::Lifetime,
        Goal::Runs(100),
    ),
    (
        "wins_5",
        "Proven",
        "Win 5 runs.",
        10,
        Category::Lifetime,
        Goal::Wins(5),
    ),
    (
        "wins_25",
        "Legend",
        "Win 25 runs.",
        25,
        Category::Lifetime,
        Goal::Wins(25),
    ),
];

/// Points for a win on one battleground, and with one class.
const ZONE_WIN_POINTS: u32 = 10;
const CLASS_WIN_POINTS: u32 = 10;

static CATALOG: LazyLock<Vec<Achievement>> = LazyLock::new(build);

/// Every achievement, shelf by shelf: each zone's and class's win ahead of its shelf's others.
pub(in crate::survivors) fn catalog() -> &'static [Achievement] {
    &CATALOG
}

fn build() -> Vec<Achievement> {
    let mut out = Vec::new();
    for category in Category::ALL {
        match category {
            Category::Zones => {
                for (i, map) in data::MAPS.iter().enumerate() {
                    out.push(Achievement {
                        id: format!("win_{}", slug(map.name)),
                        name: format!("{} Conquered", map.name),
                        desc: format!("Win a run in {}.", map.name),
                        points: ZONE_WIN_POINTS,
                        category,
                        goal: Goal::ZoneWin(i),
                    });
                }
            }
            Category::Classes => {
                for (i, class) in data::CLASSES.iter().enumerate() {
                    out.push(Achievement {
                        id: format!("class_{}", slug(class.name)),
                        name: format!("Victorious {}", class.name),
                        desc: format!("Win a run as a {}.", class.name),
                        points: CLASS_WIN_POINTS,
                        category,
                        goal: Goal::ClassWin(i),
                    });
                }
            }
            Category::Feats | Category::Lifetime => {}
        }
        for &(id, name, desc, points, cat, goal) in FIXED {
            if cat == category {
                out.push(Achievement {
                    id: id.into(),
                    name: name.into(),
                    desc: desc.into(),
                    points,
                    category,
                    goal,
                });
            }
        }
    }
    out
}

impl Achievement {
    /// `(current, target)`, current capped at target: earned once they meet. `run` is the run
    /// ending now; without one, the "in one run" goals read as unmet.
    pub(in crate::survivors) fn progress(
        &self,
        profile: &Profile,
        run: Option<&RunOutcome>,
    ) -> (u64, u64) {
        let account = &profile.account;
        let won = run.filter(|r| r.won);
        let once = |done: bool| (u64::from(done), 1);
        let reach = |have: u64, want: u64| (have.min(want), want);
        match self.goal {
            Goal::ZoneWin(i) => {
                let key = data::MAPS.get(i).map(|m| slug(m.name));
                let recorded = key
                    .and_then(|k| profile.zones.get(&k))
                    .is_some_and(|z| z.wins > 0);
                once(recorded || won.is_some_and(|r| r.map == i))
            }
            Goal::ClassWin(i) => {
                let key = data::CLASSES.get(i).map(|c| slug(c.name));
                let recorded = key
                    .and_then(|k| profile.classes.get(&k))
                    .is_some_and(|c| c.wins > 0);
                once(recorded || won.is_some_and(|r| r.class == i))
            }
            Goal::RewardsWin(mult) => once(won.is_some_and(|r| r.reward_mult + MULT_SLACK >= mult)),
            Goal::Level(level) => {
                let best = account.best_level.max(run.map_or(0, |r| r.level));
                reach(best.into(), level.into())
            }
            Goal::Survive(secs) => {
                let best = profile
                    .zones
                    .values()
                    .map(|z| z.best_seconds)
                    .chain(profile.classes.values().map(|c| c.best_seconds))
                    .chain(run.map(|r| r.seconds))
                    .fold(0.0f32, f32::max);
                reach(best as u64, secs.into())
            }
            Goal::RunBosses(n) => reach(run.map_or(0, |r| r.bosses).into(), n.into()),
            Goal::RunDamage(damage) => reach(run.map_or(0, |r| r.damage.max(0.0) as u64), damage),
            Goal::WinGold(gold) => reach(won.map_or(0, |r| r.gold).into(), gold.into()),
            Goal::RunKills(n) => reach(run.map_or(0, |r| r.kills).into(), n.into()),
            Goal::Kills(n) => reach(account.kills, n),
            Goal::Bosses(n) => reach(account.bosses, n),
            Goal::Gold(n) => reach(account.gold_earned, n),
            Goal::Runs(n) => reach(account.runs.into(), n.into()),
            Goal::Wins(n) => reach(account.wins.into(), n.into()),
        }
    }

    /// Whether the profile and the ending run (if any) meet the goal.
    pub(in crate::survivors) fn met(&self, profile: &Profile, run: Option<&RunOutcome>) -> bool {
        let (have, want) = self.progress(profile, run);
        have >= want
    }

    /// Whether the profile alone tracks the way toward it, so a bar can show how far along it is.
    /// The single-run feats are all or nothing: no run's best is kept for them.
    pub(in crate::survivors) fn measured(&self) -> bool {
        matches!(
            self.goal,
            Goal::Level(_)
                | Goal::Survive(_)
                | Goal::Kills(_)
                | Goal::Bosses(_)
                | Goal::Gold(_)
                | Goal::Runs(_)
                | Goal::Wins(_)
        )
    }

    /// How far along the profile is, 0 to 1.
    pub(in crate::survivors) fn fraction(&self, profile: &Profile) -> f32 {
        let (have, want) = self.progress(profile, None);
        (have as f64 / want.max(1) as f64) as f32
    }

    /// The profile's progress as a bar's label: "1,234 / 10,000", or "8:20 / 10:00" for time.
    pub(in crate::survivors) fn progress_label(&self, profile: &Profile) -> String {
        let (have, want) = self.progress(profile, None);
        let show = |v: u64| match self.goal {
            Goal::Survive(_) => format!("{}:{:02}", v / 60, v % 60),
            _ => grouped(v),
        };
        format!("{} / {}", show(have), show(want))
    }
}

/// The achievements the profile and the ending run complete that the save does not hold yet.
pub(in crate::survivors) fn newly_earned(
    profile: &Profile,
    run: Option<&RunOutcome>,
) -> Vec<&'static Achievement> {
    catalog()
        .iter()
        .filter(|a| !profile.achievements.has(&a.id) && a.met(profile, run))
        .collect()
}

/// The reward points the save's achievements are worth; ids the catalog no longer has count none.
pub(in crate::survivors) fn earned_points(save: &AchievementSave) -> u32 {
    catalog()
        .iter()
        .filter(|a| save.has(&a.id))
        .map(|a| a.points)
        .sum()
}

/// Every achievement's points together.
pub(in crate::survivors) fn total_points() -> u32 {
    catalog().iter().map(|a| a.points).sum()
}

/// `(earned, all)` on one shelf, or over every shelf for `None`.
pub(in crate::survivors) fn counts(
    save: &AchievementSave,
    category: Option<Category>,
) -> (usize, usize) {
    let shelf = || {
        catalog()
            .iter()
            .filter(move |a| category.is_none_or(|c| a.category == c))
    };
    (shelf().filter(|a| save.has(&a.id)).count(), shelf().count())
}

/// The `n` achievements earned last, the latest first.
pub(in crate::survivors) fn recently_earned(
    save: &AchievementSave,
    n: usize,
) -> Vec<&'static Achievement> {
    let mut earned: Vec<(u32, &'static Achievement)> = catalog()
        .iter()
        .filter_map(|a| save.run_earned(&a.id).map(|run| (run, a)))
        .collect();
    earned.sort_by_key(|&(run, _)| std::cmp::Reverse(run));
    earned.into_iter().take(n).map(|(_, a)| a).collect()
}

/// The `n` unearned achievements the profile is furthest along, the furthest first: only those
/// it [measures](Achievement::measured), and only once begun.
pub(in crate::survivors) fn almost_there(profile: &Profile, n: usize) -> Vec<&'static Achievement> {
    let mut near: Vec<(f32, &'static Achievement)> = catalog()
        .iter()
        .filter(|a| a.measured() && !profile.achievements.has(&a.id))
        .map(|a| (a.fraction(profile), a))
        .filter(|&(f, _)| f > 0.0)
        .collect();
    near.sort_by(|a, b| b.0.total_cmp(&a.0));
    near.into_iter().take(n).map(|(_, a)| a).collect()
}

/// `n` with thousands separated by commas.
pub(in crate::survivors) fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::super::super::profile::{ClassRecord, ZoneRecord};
    use super::*;

    fn by_id(id: &str) -> &'static Achievement {
        catalog()
            .iter()
            .find(|a| a.id == id)
            .unwrap_or_else(|| panic!("no achievement {id}"))
    }

    fn outcome() -> RunOutcome {
        RunOutcome {
            class: 0,
            map: 0,
            won: false,
            seconds: 120.0,
            kills: 10,
            bosses: 0,
            level: 5,
            damage: 1_000.0,
            gold: 20,
            reward_mult: 1.0,
        }
    }

    #[test]
    fn the_catalog_is_well_formed() {
        let all = catalog();
        let ids: HashSet<&str> = all.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids.len(), all.len(), "every id is its own");
        for a in all {
            assert!(
                (5..=25).contains(&a.points),
                "{} is worth {}",
                a.id,
                a.points
            );
            assert!(
                a.id.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
                "{} is a slug",
                a.id
            );
            assert!(!a.name.is_empty() && a.desc.ends_with('.'), "{}", a.id);
        }
        let count = |c: Category| all.iter().filter(|a| a.category == c).count();
        assert_eq!(count(Category::Zones), data::MAPS.len() + 1);
        assert_eq!(count(Category::Classes), data::CLASSES.len() + 2);
        assert_eq!(count(Category::Feats), 6);
        assert_eq!(count(Category::Lifetime), 13);
        // Shelf by shelf, in the order the Armory lists them.
        let order: Vec<Category> = all.iter().map(|a| a.category).collect();
        let mut sorted = order.clone();
        sorted.sort_by_key(|c| Category::ALL.iter().position(|x| x == c));
        assert_eq!(order, sorted);
        assert_eq!(total_points(), all.iter().map(|a| a.points).sum::<u32>());
    }

    #[test]
    fn boss_rush_can_be_met_on_every_battleground() {
        // Bosses come one every BOSS_EVERY_SECS until the roster runs out.
        let due = (data::VICTORY_SECS / data::BOSS_EVERY_SECS) as usize;
        for (i, map) in data::MAPS.iter().enumerate() {
            let bosses = data::roster(i).bosses.len().min(due);
            assert!(bosses >= 5, "{} brings {bosses} bosses", map.name);
        }
    }

    #[test]
    fn zone_and_class_wins_read_the_records_and_the_run() {
        let zone = by_id(&format!("win_{}", slug(data::MAPS[2].name)));
        let class = by_id(&format!("class_{}", slug(data::CLASSES[3].name)));
        let mut profile = Profile::default();
        assert!(!zone.met(&profile, None) && !class.met(&profile, None));
        // A loss there counts for nothing.
        profile.zones.insert(
            slug(data::MAPS[2].name),
            ZoneRecord {
                runs: 4,
                ..Default::default()
            },
        );
        assert!(!zone.met(&profile, None));
        profile
            .zones
            .get_mut(&slug(data::MAPS[2].name))
            .unwrap()
            .wins = 1;
        assert!(zone.met(&profile, None));
        profile.classes.insert(
            slug(data::CLASSES[3].name),
            ClassRecord {
                wins: 2,
                ..Default::default()
            },
        );
        assert!(class.met(&profile, None));
        // The ending run alone, before any record.
        let fresh = Profile::default();
        let win = RunOutcome {
            won: true,
            map: 2,
            class: 3,
            ..outcome()
        };
        assert!(zone.met(&fresh, Some(&win)) && class.met(&fresh, Some(&win)));
        let elsewhere = RunOutcome {
            map: 1,
            class: 4,
            ..win
        };
        assert!(!zone.met(&fresh, Some(&elsewhere)) && !class.met(&fresh, Some(&elsewhere)));
        let lost = RunOutcome { won: false, ..win };
        assert!(!zone.met(&fresh, Some(&lost)));
    }

    #[test]
    fn stakes_need_a_win_at_the_multiplier() {
        let raised = by_id("win_raised_stakes");
        let max = by_id("win_max_stakes");
        let p = Profile::default();
        let run = |won, reward_mult| RunOutcome {
            won,
            reward_mult,
            ..outcome()
        };
        assert!(!raised.met(&p, Some(&run(true, 1.45))));
        assert!(raised.met(&p, Some(&run(true, 1.5))));
        // Summed slider steps may land a hair under.
        assert!(raised.met(&p, Some(&run(true, 1.499_999_9))));
        assert!(!raised.met(&p, Some(&run(false, 2.0))));
        assert!(!max.met(&p, Some(&run(true, 1.85))));
        assert!(max.met(&p, Some(&run(true, 2.0))));
        assert!(!raised.met(&p, None), "no run, no stakes");
    }

    #[test]
    fn single_run_feats_read_the_run() {
        let p = Profile::default();
        let check = |id: &str, run: RunOutcome| by_id(id).met(&p, Some(&run));
        assert!(check(
            "face_5_bosses",
            RunOutcome {
                bosses: 5,
                ..outcome()
            }
        ));
        assert!(!check(
            "face_5_bosses",
            RunOutcome {
                bosses: 4,
                ..outcome()
            }
        ));
        assert!(check(
            "damage_1m",
            RunOutcome {
                damage: 1_000_000.0,
                ..outcome()
            }
        ));
        assert!(!check(
            "damage_1m",
            RunOutcome {
                damage: 999_999.0,
                ..outcome()
            }
        ));
        assert!(check(
            "kills_2000_run",
            RunOutcome {
                kills: 2000,
                ..outcome()
            }
        ));
        // Gold counts only on a win.
        let rich = RunOutcome {
            gold: 600,
            ..outcome()
        };
        assert!(!check("win_500_gold", rich));
        assert!(check("win_500_gold", RunOutcome { won: true, ..rich }));
        assert!(!check(
            "win_500_gold",
            RunOutcome {
                won: true,
                gold: 499,
                ..outcome()
            }
        ));
        // Without the run they read as unmet: the profile keeps no single run's best.
        assert!(!by_id("damage_1m").met(&p, None));
        assert!(!by_id("damage_1m").measured() && by_id("kills_1000").measured());
    }

    #[test]
    fn levels_and_survival_read_the_bests() {
        let mut p = Profile::default();
        let fifty = by_id("level_50");
        let ten = by_id("survive_10_minutes");
        p.account.best_level = 37;
        assert_eq!(fifty.progress(&p, None), (37, 50));
        assert!(fifty.met(
            &p,
            Some(&RunOutcome {
                level: 50,
                ..outcome()
            })
        ));
        p.account.best_level = 61;
        assert_eq!(fifty.progress(&p, None), (50, 50), "capped at the target");
        assert!(!by_id("level_100").met(&p, None));

        p.zones.insert(
            "a".into(),
            ZoneRecord {
                best_seconds: 500.0,
                ..Default::default()
            },
        );
        assert_eq!(ten.progress(&p, None), (500, 600));
        assert_eq!(ten.progress_label(&p), "8:20 / 10:00");
        p.classes.insert(
            "b".into(),
            ClassRecord {
                best_seconds: 610.0,
                ..Default::default()
            },
        );
        assert!(ten.met(&p, None));
        assert!(ten.met(
            &Profile::default(),
            Some(&RunOutcome {
                seconds: 600.0,
                ..outcome()
            })
        ));
    }

    #[test]
    fn lifetime_totals_cross_their_thresholds() {
        let mut p = Profile::default();
        p.account.kills = 999;
        p.account.bosses = 10;
        p.account.gold_earned = 10_000;
        p.account.runs = 49;
        p.account.wins = 5;
        let met = |p: &Profile, id: &str| by_id(id).met(p, None);
        assert!(!met(&p, "kills_1000"));
        assert!(met(&p, "bosses_10") && !met(&p, "bosses_100"));
        assert!(met(&p, "gold_1000") && met(&p, "gold_10000"));
        assert!(met(&p, "runs_10") && !met(&p, "runs_50"));
        assert!(met(&p, "wins_5") && !met(&p, "wins_25"));
        assert_eq!(by_id("kills_1000").progress_label(&p), "999 / 1,000");
        assert!((by_id("runs_50").fraction(&p) - 0.98).abs() < 1e-6);
        p.account.kills = 100_000;
        assert!(met(&p, "kills_1000") && met(&p, "kills_10000") && met(&p, "kills_100000"));
        // A lifetime goal never adds the ending run on top: the profile already counts it.
        assert!(!by_id("runs_50").met(&p, Some(&outcome())));
    }

    #[test]
    fn newly_earned_skips_what_the_save_holds_and_points_add_up() {
        let mut p = Profile::default();
        p.account.runs = 10;
        p.account.kills = 1_000;
        let new: Vec<&str> = newly_earned(&p, None)
            .iter()
            .map(|a| a.id.as_str())
            .collect();
        assert_eq!(new, ["kills_1000", "runs_10"]);
        p.achievements.earned.insert("runs_10".into(), 10);
        let new: Vec<&str> = newly_earned(&p, None)
            .iter()
            .map(|a| a.id.as_str())
            .collect();
        assert_eq!(new, ["kills_1000"]);
        p.achievements.earned.insert("kills_1000".into(), 10);
        // An id from another build is kept but worth nothing here.
        p.achievements.earned.insert("retired_feat".into(), 2);
        assert_eq!(earned_points(&p.achievements), 10);
        assert_eq!(earned_points(&AchievementSave::default()), 0);
    }

    #[test]
    fn the_summary_lists_recent_and_nearly_done() {
        let mut p = Profile::default();
        assert!(recently_earned(&p.achievements, 5).is_empty());
        assert!(almost_there(&p, 5).is_empty(), "nothing begun yet");
        assert_eq!(counts(&p.achievements, None), (0, catalog().len()));
        for (id, run) in [("runs_10", 10), ("kills_1000", 4), ("gold_1000", 12)] {
            p.achievements.earned.insert(id.into(), run);
        }
        let recent: Vec<&str> = recently_earned(&p.achievements, 2)
            .iter()
            .map(|a| a.id.as_str())
            .collect();
        assert_eq!(recent, ["gold_1000", "runs_10"]);
        assert_eq!(counts(&p.achievements, Some(Category::Lifetime)), (3, 13));
        assert_eq!(counts(&p.achievements, Some(Category::Zones)).0, 0);
        assert_eq!(counts(&p.achievements, None).0, 3);

        p.account.runs = 45;
        p.account.wins = 1;
        p.account.kills = 9_000;
        let near: Vec<&str> = almost_there(&p, 3).iter().map(|a| a.id.as_str()).collect();
        // 45 of 50 runs, 9,000 of 10,000 kills, then 45 of 100 runs; earned ones never show.
        assert_eq!(near, ["kills_10000", "runs_50", "runs_100"]);
    }

    #[test]
    fn numbers_group_by_thousands() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(1_000), "1,000");
        assert_eq!(grouped(123_456_789), "123,456,789");
    }
}
