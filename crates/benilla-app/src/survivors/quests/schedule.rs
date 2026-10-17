//! When a quest giver walks out, which quest it brings, and where it stands. Pure: the battleground
//! side ([`super::field`]) feeds it the clock, the roster and the dice.

use bevy::prelude::*;

use super::super::data::{self, EnemyDef};
use super::super::units::flat;
use super::catalog::{Goal, QuestDef};

/// The first giver walks out a minute in.
pub(in crate::survivors) const FIRST_GIVER_SECS: f32 = 60.0;
/// Then one about every three minutes (`WOW_SURVIVORS_QUEST_EVERY` sets it).
pub(in crate::survivors) const GIVER_EVERY_SECS: f32 = 180.0;
/// The shortest interval the env aid may set, so a typo cannot flood the field.
pub(in crate::survivors) const MIN_EVERY_SECS: f32 = 5.0;
/// No giver walks out within this many seconds of a boss, either side.
pub(in crate::survivors) const BOSS_CLEAR_SECS: f32 = 30.0;
/// Nor this near the bell, where no quest could be finished.
pub(in crate::survivors) const LAST_CALL_SECS: f32 = 120.0;
/// How far a named foe's place on the roster may sit below and above the moment's wave for its
/// quest to be offered: weaker foes are a chore, much stronger ones a wall.
const FOE_BELOW: usize = 3;
const FOE_ABOVE: usize = 2;
/// A Slay Elite is offered only if the next elite is due this long before its time runs out.
const ELITE_MARGIN_SECS: f32 = 40.0;
/// A giver stands this far from the party's middle, and a Defend's spot this far from its giver.
pub(in crate::survivors) const GIVER_GAP: f32 = 14.0;
pub(in crate::survivors) const SPOT_GAP: f32 = 18.0;
/// Both stay within this share of the leash, so they are always in reach.
const LEASH_SHARE: f32 = 0.85;

/// The run seconds `map`'s bosses walk out at.
pub(in crate::survivors) fn boss_times(map: usize) -> Vec<f32> {
    super::super::zones::boss_schedule(map)
        .iter()
        .map(|b| b.at)
        .collect()
}

/// `t`, or the first moment after it at least [`BOSS_CLEAR_SECS`] from every boss in `bosses`.
pub(in crate::survivors) fn clear_of_bosses(t: f32, bosses: &[f32]) -> f32 {
    let mut t = t;
    // Each step moves past one boss's window; bosses are far further apart than a window.
    for _ in 0..=bosses.len() {
        match bosses.iter().find(|&&b| (t - b).abs() < BOSS_CLEAR_SECS) {
            Some(&b) => t = b + BOSS_CLEAR_SECS,
            None => break,
        }
    }
    t
}

/// When a giver due at `due` may walk out: clear of the bosses, or never if that is past the last
/// call.
pub(in crate::survivors) fn next_giver(due: f32, bosses: &[f32]) -> Option<f32> {
    let t = clear_of_bosses(due, bosses);
    (t <= data::VICTORY_SECS - LAST_CALL_SECS).then_some(t)
}

/// A whole run's givers when no quest holds the next one up: the first at `first`, each next one
/// `every` seconds after the last walked out, each moved clear of the bosses, as
/// [`super::field::offer_quests`] sends them.
#[cfg(test)]
pub(in crate::survivors) fn giver_times(first: f32, every: f32, bosses: &[f32]) -> Vec<f32> {
    let every = every.max(MIN_EVERY_SECS);
    let mut out = Vec::new();
    let mut due = first;
    while let Some(t) = next_giver(due, bosses) {
        out.push(t);
        due = t + every;
    }
    out
}

/// Whether a giver may offer `quest` `elapsed` seconds in, on a zone whose horde is `roster` and
/// whose next elite is due at `next_elite`: a named foe near the moment's wave, an elite due in
/// time to be hunted. A Defend and a kill of anything are always on offer.
pub(in crate::survivors) fn fits(
    quest: &QuestDef,
    roster: &[EnemyDef],
    elapsed: f32,
    next_elite: f32,
) -> bool {
    let foe = match quest.goal {
        Goal::Kill { foe: Some(f), .. } | Goal::Collect { foe: f, .. } => f,
        Goal::SlayElite => return next_elite - elapsed <= quest.limit - ELITE_MARGIN_SECS,
        Goal::Kill { foe: None, .. } | Goal::Defend { .. } => return true,
    };
    let Some(at) = roster.iter().position(|e| e.name == foe) else {
        return false;
    };
    let pool = data::wave_pool(elapsed, roster.len());
    at + FOE_BELOW >= pool[0] && at <= pool[2] + FOE_ABOVE
}

/// The quest a giver brings: one of `quests` that `fits`, preferring those not yet `offered` this
/// run; `roll` picks among them. `None` when nothing fits.
pub(in crate::survivors) fn pick<'a>(
    quests: &'a [QuestDef],
    offered: &[&str],
    fits: impl Fn(&QuestDef) -> bool,
    roll: u64,
) -> Option<&'a QuestDef> {
    let fitting: Vec<&QuestDef> = quests.iter().filter(|q| fits(q)).collect();
    let fresh: Vec<&QuestDef> = fitting
        .iter()
        .copied()
        .filter(|q| !offered.contains(&q.id))
        .collect();
    let from = if fresh.is_empty() { fitting } else { fresh };
    (!from.is_empty()).then(|| from[(roll % from.len() as u64) as usize])
}

/// A spot `gap` yards from `from` along `angle`, pulled back to within [`LEASH_SHARE`] of the
/// `leash` round the battleground's `arena` centre. Its height is `from`'s, for the caller to
/// ground.
pub(in crate::survivors) fn post_near(
    from: Vec3,
    arena: Vec3,
    leash: f32,
    angle: f32,
    gap: f32,
) -> Vec3 {
    let mut p = from + Vec3::new(angle.cos(), 0.0, angle.sin()) * gap;
    let off = flat(p - arena);
    let max = leash * LEASH_SHARE;
    if off.length() > max {
        p = arena + off.normalize_or_zero() * max;
    }
    p.y = from.y;
    p
}

#[cfg(test)]
mod tests {
    use super::super::catalog::{for_map, ZONES};
    use super::*;

    fn index(name: &str) -> usize {
        data::MAPS.iter().position(|m| m.name == name).unwrap()
    }

    #[test]
    fn givers_keep_clear_of_the_bosses_and_the_interval_holds() {
        for map in 0..data::MAPS.len() {
            let bosses = boss_times(map);
            for every in [GIVER_EVERY_SECS, 100.0, 45.0, 20.0, 1.0] {
                let times = giver_times(FIRST_GIVER_SECS, every, &bosses);
                assert!(!times.is_empty());
                assert!(times[0] >= FIRST_GIVER_SECS);
                for t in &times {
                    for b in &bosses {
                        assert!(
                            (t - b).abs() >= BOSS_CLEAR_SECS,
                            "a giver at {t} is within {BOSS_CLEAR_SECS} s of the boss at {b}"
                        );
                    }
                    assert!(*t <= data::VICTORY_SECS - LAST_CALL_SECS);
                }
                for pair in times.windows(2) {
                    assert!(
                        pair[1] - pair[0] >= every.max(MIN_EVERY_SECS) - 1e-3,
                        "{pair:?} closer than {every}"
                    );
                }
            }
        }
        // The default rhythm: a minute in, then every three, the third nudged past the boss at
        // 6:45, and none in the last two minutes.
        let times = giver_times(
            FIRST_GIVER_SECS,
            GIVER_EVERY_SECS,
            &boss_times(index("Elwynn Forest")),
        );
        assert_eq!(times, [60.0, 240.0, 435.0, 615.0]);
    }

    #[test]
    fn a_boss_window_pushes_a_giver_past_it() {
        let bosses = [135.0, 270.0];
        assert_eq!(clear_of_bosses(60.0, &bosses), 60.0);
        assert_eq!(
            clear_of_bosses(105.0, &bosses),
            105.0,
            "30 s before is clear"
        );
        assert_eq!(clear_of_bosses(110.0, &bosses), 165.0);
        assert_eq!(clear_of_bosses(150.0, &bosses), 165.0);
        assert_eq!(clear_of_bosses(250.0, &bosses), 300.0);
        assert_eq!(next_giver(800.0, &[]), None, "too near the bell");
        assert_eq!(next_giver(10.0, &[]), Some(10.0));
    }

    #[test]
    fn quests_fit_the_moment() {
        let elwynn = index("Elwynn Forest");
        let roster = &data::roster(elwynn).enemies;
        let zone = for_map(elwynn).unwrap();
        let by = |id: &str| zone.quests.iter().find(|q| q.id == id).unwrap();
        let tunnelers = by("elwynn_fargodeep_mine");
        let candles = by("elwynn_kobold_candles");
        let hold = by("elwynn_pumpkin_patch");
        let princess = by("elwynn_princess_must_die");
        // A minute in, the kobolds are still to come; five minutes in, the tunnelers are here.
        assert!(!fits(tunnelers, roster, 60.0, 120.0));
        assert!(fits(tunnelers, roster, 300.0, 400.0));
        assert!(fits(candles, roster, 480.0, 600.0));
        assert!(
            !fits(tunnelers, roster, 800.0, 900.0),
            "long gone by the end"
        );
        assert!(fits(hold, roster, 60.0, 0.0) && fits(hold, roster, 870.0, 0.0));
        // An elite due a minute out can be hunted; one due in two and a half cannot.
        assert!(fits(princess, roster, 60.0, 120.0));
        assert!(!fits(princess, roster, 0.0, 150.0));
        // Every giver of the default rhythm has something to offer in every zone.
        for map in 0..data::MAPS.len() {
            let Some(zone) = for_map(map) else {
                continue; // MC stages have no quest shell until the raid milestone.
            };
            let roster = &data::roster(map).enemies;
            for t in giver_times(FIRST_GIVER_SECS, GIVER_EVERY_SECS, &boss_times(map)) {
                assert!(
                    zone.quests.iter().any(|q| fits(q, roster, t, t + 60.0)),
                    "{} has nothing at {t}",
                    zone.zone
                );
            }
        }
        assert_eq!(
            ZONES.len(),
            data::MAPS.iter().filter(|m| m.map != 409).count()
        );
    }

    #[test]
    fn a_giver_prefers_a_quest_not_yet_offered() {
        let zone = for_map(index("Durotar")).unwrap();
        let all = |_: &QuestDef| true;
        let first = pick(zone.quests, &[], all, 0).unwrap();
        assert_eq!(first.id, zone.quests[0].id);
        let ids: Vec<&str> = zone.quests.iter().map(|q| q.id).collect();
        // Everything but the last offered: the last it is, whatever the roll.
        for roll in 0..8 {
            let q = pick(zone.quests, &ids[..ids.len() - 1], all, roll).unwrap();
            assert_eq!(q.id, *ids.last().unwrap());
        }
        // All offered: one comes round again.
        assert!(pick(zone.quests, &ids, all, 5).is_some());
        assert!(pick(zone.quests, &[], |_| false, 0).is_none());
    }

    #[test]
    fn posts_stay_inside_the_leash() {
        let arena = Vec3::new(100.0, 5.0, -40.0);
        for leash in [30.0, 75.0, 150.0] {
            for i in 0..16 {
                let a = i as f32 / 16.0 * std::f32::consts::TAU;
                // The party at the leash's very edge, the giver stepping further out.
                let party = arena + Vec3::new(a.cos(), 0.0, a.sin()) * leash;
                let p = post_near(party, arena, leash, a, GIVER_GAP);
                assert!(flat(p - arena).length() <= leash * LEASH_SHARE + 1e-3);
                assert_eq!(p.y, party.y);
            }
        }
        // In the open, it stands its gap away.
        let p = post_near(arena, arena, 150.0, 0.0, GIVER_GAP);
        assert!((flat(p - arena).length() - GIVER_GAP).abs() < 1e-3);
    }
}
