//! Talent XP and the points it buys. Every recorded run pays its class talent XP ([`run_xp`]),
//! and a class's points come from its all-time XP on a rising curve ([`points_for_xp`]), up to
//! [`MAX_POINTS`]. Points are earned once and kept: a respec gives every one back.

use bevy::prelude::*;

use super::super::data;
use super::super::hud::{NoteKind, ResultsNotes};
use super::super::profile::{slug, Profile, RunEnded, RunOutcome};
use super::{TalentSave, MAX_POINTS};

const XP_PER_SECOND: f64 = 1.0;
const XP_PER_KILL: f64 = 0.05;
/// Per boss the run met (the mode counts bosses spawned, not killed).
const XP_PER_BOSS: f64 = 40.0;
const XP_PER_WIN: f64 = 300.0;

/// Point `n` (1-based) costs `POINT_BASE + POINT_STEP × n` XP more than point `n − 1`: 225 for
/// the first, 3 975 for the 51st, 107 100 for all. The brief's `60 + 30n` scaled 2.5×, which at 1×
/// paid four points for a first ten-minute death instead of the one or two it asked for.
pub(in crate::survivors) const POINT_BASE: u64 = 150;
pub(in crate::survivors) const POINT_STEP: u64 = 75;

/// The most XP a save keeps: TOML integers are `i64`, and a larger one would fail the whole save.
const MAX_XP: u64 = i64::MAX as u64;

/// The talent XP a run pays: `seconds + kills / 20 + 40 a boss + 300 for a win`, times the run's
/// rewards multiplier and its zone's [`zone_factor`], rounded.
pub(in crate::survivors) fn run_xp(run: &RunOutcome) -> u64 {
    let won = if run.won { XP_PER_WIN } else { 0.0 };
    let base = f64::from(run.seconds.max(0.0)) * XP_PER_SECOND
        + f64::from(run.kills) * XP_PER_KILL
        + f64::from(run.bosses) * XP_PER_BOSS
        + won;
    let xp = base * f64::from(run.reward_mult.max(0.0)) * f64::from(zone_factor(run.map));
    // `as` saturates, and reads NaN as 0.
    xp.round() as u64
}

/// How much more a zone pays: 1 where its horde averages level 10 or under (the starting zones),
/// rising by 1 over the next 50 levels to 2 at 60 and up (the raid doorsteps). Read off the
/// zone's own roster, so a new zone needs no entry anywhere.
pub(in crate::survivors) fn zone_factor(map: usize) -> f32 {
    let horde = &data::roster(map).enemies;
    if horde.is_empty() {
        return 1.0;
    }
    let mean = horde.iter().map(|e| e.level as f32).sum::<f32>() / horde.len() as f32;
    1.0 + ((mean - 10.0) / 50.0).clamp(0.0, 1.0)
}

/// The all-time XP that buys `points` points (0 for none).
pub(in crate::survivors) fn xp_for_points(points: u32) -> u64 {
    let n = u64::from(points);
    POINT_BASE * n + POINT_STEP * n * (n + 1) / 2
}

/// The points `xp` buys, at most [`MAX_POINTS`].
pub(in crate::survivors) fn points_for_xp(xp: u64) -> u32 {
    (1..=MAX_POINTS)
        .take_while(|&n| xp_for_points(n) <= xp)
        .last()
        .unwrap_or(0)
}

/// Add a run's `xp` to a class's save, returning the points it newly earned.
pub(in crate::survivors) fn add_xp(save: &mut TalentSave, xp: u64) -> u32 {
    let before = points_for_xp(save.xp);
    save.xp = save.xp.saturating_add(xp).min(MAX_XP);
    points_for_xp(save.xp) - before
}

/// `WOW_SURVIVORS_TALENT_XP=<n>|max`, a screenshot aid: `n` more talent XP for every class, or
/// exactly what all [`MAX_POINTS`] cost, so the Talents tab has points to spend. Never saved: it
/// is counted where points are, not written to the profile (the ranks it buys are, like any
/// others).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::survivors) struct TalentDevXp(pub u64);

impl TalentDevXp {
    pub(super) fn from_env() -> Option<Self> {
        let raw = crate::webenv::var("WOW_SURVIVORS_TALENT_XP")?;
        match Self::parse(&raw) {
            Some(dev) => {
                info!("survivors talents: {} dev talent XP for every class", dev.0);
                Some(dev)
            }
            None => {
                warn!("survivors talents: WOW_SURVIVORS_TALENT_XP={raw:?} is not a number or max");
                None
            }
        }
    }

    /// A whole number of XP, or `max` for every point.
    fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        if raw.eq_ignore_ascii_case("max") {
            return Some(Self(xp_for_points(MAX_POINTS)));
        }
        raw.parse().ok().map(Self)
    }
}

/// The points a class has earned: its saved XP's, with `dev` ([`TalentDevXp`]) more XP.
pub(in crate::survivors) fn earned_points(save: &TalentSave, dev: u64) -> u32 {
    points_for_xp(save.xp.saturating_add(dev))
}

/// The points a class has earned and not spent.
pub(in crate::survivors) fn unspent(save: &TalentSave, dev: u64) -> u32 {
    earned_points(save, dev).saturating_sub(save.spent())
}

/// Pays each recorded run's class its talent XP, and says so on the results screen.
pub(super) fn award_xp(
    mut ended: MessageReader<RunEnded>,
    mut profile: ResMut<Profile>,
    mut notes: ResMut<ResultsNotes>,
) {
    for ended in ended.read() {
        let run = &ended.outcome;
        let Some(class) = data::CLASSES.get(run.class) else {
            continue;
        };
        let xp = run_xp(run);
        if xp == 0 {
            continue;
        }
        let save = &mut profile.classes.entry(slug(class.name)).or_default().talents;
        let points = add_xp(save, xp);
        profile.mark_dirty();
        notes.push(format!("+{xp} talent XP"), NoteKind::Good);
        if points > 0 {
            let s = if points == 1 { "" } else { "s" };
            notes.push(
                format!("+{points} talent point{s} for {}!", class.name),
                NoteKind::Good,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(name: &str) -> usize {
        data::MAPS.iter().position(|m| m.name == name).unwrap()
    }

    fn run(map: usize, seconds: f32, kills: u32, bosses: u32, won: bool) -> RunOutcome {
        RunOutcome {
            class: 0,
            map,
            won,
            seconds,
            kills,
            bosses,
            level: 1,
            damage: 0.0,
            gold: 0,
            reward_mult: 1.0,
            quests: 0,
            final_boss: false,
        }
    }

    #[test]
    fn the_zone_factor_rises_from_the_starting_zones_to_the_raids() {
        for name in [
            "Elwynn Forest",
            "Durotar",
            "Teldrassil",
            "Dun Morogh",
            "Mulgore",
            "Tirisfal Glades",
        ] {
            assert_eq!(zone_factor(map(name)), 1.0, "{name}");
        }
        let westfall = zone_factor(map("Westfall"));
        let duskwood = zone_factor(map("Duskwood"));
        assert!(1.0 < westfall && westfall < duskwood && duskwood < 1.5);
        for name in ["Naxxramas", "Gates of Ahn'Qiraj", "Mount Hyjal"] {
            let f = zone_factor(map(name));
            assert!((1.85..=2.0).contains(&f), "{name} pays {f}");
        }
        for (i, m) in data::MAPS.iter().enumerate() {
            assert!((1.0..=2.0).contains(&zone_factor(i)), "{}", m.name);
        }
    }

    #[test]
    fn run_xp_follows_the_formula() {
        let elwynn = map("Elwynn Forest");
        // 600 s + 2000 kills / 20 + 4 bosses × 40.
        assert_eq!(run_xp(&run(elwynn, 600.0, 2000, 4, false)), 860);
        assert_eq!(run_xp(&run(elwynn, 600.0, 2000, 4, true)), 1160);
        let harder = RunOutcome {
            reward_mult: 1.5,
            ..run(elwynn, 600.0, 2000, 4, false)
        };
        assert_eq!(run_xp(&harder), 1290);
        let naxx = map("Naxxramas");
        let f = f64::from(zone_factor(naxx));
        assert_eq!(
            run_xp(&run(naxx, 600.0, 2000, 4, false)),
            (860.0 * f).round() as u64
        );
        assert_eq!(run_xp(&run(elwynn, 0.0, 0, 0, false)), 0);
        assert_eq!(run_xp(&run(elwynn, -5.0, 0, 0, false)), 0);
    }

    #[test]
    fn each_point_costs_more_and_they_stop_at_fifty_one() {
        assert_eq!(xp_for_points(0), 0);
        assert_eq!(points_for_xp(0), 0);
        for n in 1..=MAX_POINTS {
            let step = xp_for_points(n) - xp_for_points(n - 1);
            assert_eq!(step, POINT_BASE + POINT_STEP * u64::from(n), "point {n}");
            assert_eq!(points_for_xp(xp_for_points(n)), n);
            assert_eq!(points_for_xp(xp_for_points(n) - 1), n - 1);
        }
        let mut last = 0;
        for xp in (0..xp_for_points(MAX_POINTS) + 10_000).step_by(37) {
            let points = points_for_xp(xp);
            assert!(points >= last, "{xp} XP");
            last = points;
        }
        assert_eq!(last, MAX_POINTS);
        assert_eq!(points_for_xp(u64::MAX), MAX_POINTS);
        assert_eq!(xp_for_points(MAX_POINTS), 107_100);
    }

    #[test]
    fn the_curve_meets_its_tuning_targets() {
        let elwynn = map("Elwynn Forest");
        // A first ten-minute death in a starting zone, about 2000 kills and 4 bosses in.
        let death = run_xp(&run(elwynn, 600.0, 2000, 4, false));
        assert!((1..=2).contains(&points_for_xp(death)), "{death} XP");
        // A climb: ten such deaths, then full 15-minute wins, fifteen in a starting zone, fifteen
        // in Duskwood, then raid doorsteps until every point is earned.
        let win = |m| run_xp(&run(m, 900.0, 3500, 6, true));
        let climb = std::iter::repeat_n(death, 10)
            .chain(std::iter::repeat_n(win(elwynn), 15))
            .chain(std::iter::repeat_n(win(map("Duskwood")), 15))
            .chain(std::iter::repeat(win(map("Naxxramas"))));
        let mut save = TalentSave::default();
        let runs = climb
            .take_while(|&xp| {
                add_xp(&mut save, xp);
                points_for_xp(save.xp) < MAX_POINTS
            })
            .count()
            + 1;
        assert!((40..=60).contains(&runs), "all 51 points in {runs} runs");
        // The first runs pay a point or two each.
        let mut save = TalentSave::default();
        assert_eq!(add_xp(&mut save, death), 2);
        assert!(add_xp(&mut save, death) >= 1);
    }

    #[test]
    fn xp_saturates_below_what_toml_can_hold() {
        let mut save = TalentSave {
            xp: MAX_XP - 1,
            ..default()
        };
        assert_eq!(add_xp(&mut save, u64::MAX), 0);
        assert_eq!(save.xp, MAX_XP);
        assert!(toml::to_string(&save).is_ok());
    }

    #[test]
    fn a_recorded_run_pays_its_class_and_says_so() {
        let mage = data::CLASSES.iter().position(|c| c.name == "Mage").unwrap();
        let mut world = World::new();
        world.init_resource::<Messages<RunEnded>>();
        world.insert_resource(Profile::default());
        world.init_resource::<ResultsNotes>();
        // One system throughout, as in the app: its reader reads each run once.
        let award = world.register_system(award_xp);
        world.resource_mut::<Messages<RunEnded>>().write(RunEnded {
            outcome: RunOutcome {
                class: mage,
                ..run(map("Elwynn Forest"), 600.0, 2000, 4, false)
            },
        });
        world.run_system(award).unwrap();
        let profile = world.resource::<Profile>();
        assert_eq!(profile.classes["mage"].talents.xp, 860);
        let notes: Vec<&str> = world
            .resource::<ResultsNotes>()
            .0
            .iter()
            .map(|(s, _)| s.as_str())
            .collect();
        assert_eq!(notes, ["+860 talent XP", "+2 talent points for Mage!"]);

        // The next run pays less than the next point costs: XP, and no points line.
        world.resource_mut::<ResultsNotes>().0.clear();
        world.resource_mut::<Messages<RunEnded>>().write(RunEnded {
            outcome: RunOutcome {
                class: mage,
                ..run(map("Elwynn Forest"), 30.0, 0, 0, false)
            },
        });
        world.run_system(award).unwrap();
        assert_eq!(world.resource::<Profile>().classes["mage"].talents.xp, 890);
        assert_eq!(world.resource::<ResultsNotes>().0.len(), 1);
    }

    #[test]
    fn the_dev_xp_takes_a_number_or_max() {
        assert_eq!(TalentDevXp::parse(" 1200 "), Some(TalentDevXp(1200)));
        let max = TalentDevXp::parse("max").unwrap();
        assert_eq!(TalentDevXp::parse(" MAX "), Some(max));
        assert_eq!(max.0, xp_for_points(MAX_POINTS));
        assert_eq!(
            points_for_xp(max.0),
            MAX_POINTS,
            "all 51, where 100000 buys fewer"
        );
        assert_eq!(points_for_xp(max.0 - 1), MAX_POINTS - 1);
        assert_eq!(earned_points(&TalentSave::default(), max.0), MAX_POINTS);
        assert!(points_for_xp(100_000) < MAX_POINTS);
        assert_eq!(TalentDevXp::parse("lots"), None);
        assert_eq!(TalentDevXp::parse("-5"), None);
    }

    #[test]
    fn unspent_points_count_the_dev_xp_and_never_go_negative() {
        let save = TalentSave {
            xp: xp_for_points(3),
            ranks: vec![(35, 2)],
        };
        assert_eq!(earned_points(&save, 0), 3);
        assert_eq!(unspent(&save, 0), 1);
        assert_eq!(unspent(&save, xp_for_points(5) - xp_for_points(3)), 3);
        let over = TalentSave {
            xp: 0,
            ranks: vec![(35, 2)],
        };
        assert_eq!(unspent(&over, 0), 0);
    }
}
