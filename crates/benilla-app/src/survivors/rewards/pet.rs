//! The companion as a hunter's pet: it levels, and learns skills at the trainer. Every item it
//! fetches is worth [`XP_PER_ITEM`] experience and every run it comes along on [`RUN_XP`] more, up
//! to [`MAX_LEVEL`] on a gentle curve ([`xp_to_next`]). Each level brings a training point, with
//! one more at levels 5, 10, 15 and 20 ([`training_points`]); the [`SKILLS`] spend them, and the
//! trainer gives every point back for free ([`reset`]). The points spent are reckoned from the
//! ranks held, so only the ranks are saved ([`RewardsSave::pet_skills`]).
//!
//! What each skill does in a run is the pure functions here ([`fetch_reach`], [`fetch_every`],
//! [`run_speed`], [`items_per_trip`], [`truffle_every`], [`nip_damage`], [`loyal_heal`]), read by
//! [`super::companion`] through [`PetSkills`].

use super::perks;
use super::save::RewardsSave;

/// The pet's highest level.
pub(in crate::survivors) const MAX_LEVEL: u32 = 20;
/// Experience for each item fetched.
pub(in crate::survivors) const XP_PER_ITEM: u32 = 1;
/// Experience for each run the companion comes along on, however it ends.
pub(in crate::survivors) const RUN_XP: u32 = 30;
/// The levels that bring a training point more.
pub(in crate::survivors) const BONUS_TP_LEVELS: [u32; 4] = [5, 10, 15, 20];

/// The farthest a pickup is fetched from the hero, in yards, before Wide Patrol.
pub(in crate::survivors) const BASE_REACH: f32 = 25.0;
/// Seconds between fetches before Swiftness.
pub(in crate::survivors) const BASE_FETCH_EVERY: f32 = 2.0;
/// How near an enemy must be to the pet for Nip, in yards, and the seconds between bites.
pub(in crate::survivors) const NIP_RANGE: f32 = 6.0;
pub(in crate::survivors) const NIP_EVERY: f32 = 3.0;
/// A bite's damage at Nip rank 1: this, and this much more per run level.
const NIP_BASE: f32 = 3.0;
const NIP_PER_LEVEL: f32 = 1.0;
/// Coins in the pile Truffle Snout digs up.
pub(in crate::survivors) const TRUFFLE_COINS: u32 = 3;
/// Loyal's heal a fetched item and rank, in percent of the owner's max health.
const LOYAL_PERCENT: f32 = 0.5;

/// The names a companion can go by, picked in turn on the Companion tab: herbs from the
/// herbalist's satchel. The first is a new pet's.
pub(in crate::survivors) static PET_NAMES: &[&str] = &[
    "Peacebloom",
    "Silverleaf",
    "Earthroot",
    "Mageroyal",
    "Briarthorn",
    "Swiftthistle",
    "Bruiseweed",
    "Kingsblood",
    "Liferoot",
    "Fadeleaf",
    "Goldthorn",
    "Wintersbite",
    "Firebloom",
    "Sungrass",
    "Gromsblood",
    "Dreamfoil",
    "Icecap",
    "Blindweed",
];

/// The pet's name: the one picked, else the first.
pub(in crate::survivors) fn name(save: &RewardsSave) -> &str {
    if save.pet_name.is_empty() {
        PET_NAMES[0]
    } else {
        &save.pet_name
    }
}

/// The name after `current` in [`PET_NAMES`], round to the first; a name not on the list is
/// followed by the first.
pub(in crate::survivors) fn next_name(current: &str) -> &'static str {
    let at = PET_NAMES.iter().position(|n| *n == current);
    PET_NAMES[at.map_or(0, |i| (i + 1) % PET_NAMES.len())]
}

/// Experience from `level` to the next: 40, then 30 more each level. None past the top.
pub(in crate::survivors) fn xp_to_next(level: u32) -> Option<u32> {
    if (1..MAX_LEVEL).contains(&level) {
        Some(40 + 30 * (level - 1))
    } else {
        None
    }
}

/// Experience over every run that reaches `level` (capped at [`MAX_LEVEL`]).
pub(in crate::survivors) fn xp_for_level(level: u32) -> u32 {
    (1..level.min(MAX_LEVEL)).filter_map(xp_to_next).sum()
}

/// The pet's level with `xp` experience.
pub(in crate::survivors) fn level(xp: u32) -> u32 {
    let mut level = 1;
    while level < MAX_LEVEL && xp >= xp_for_level(level + 1) {
        level += 1;
    }
    level
}

/// How far into its level `xp` is: `(into, needed)`, or `None` at the top level.
pub(in crate::survivors) fn progress(xp: u32) -> Option<(u32, u32)> {
    let level = level(xp);
    xp_to_next(level).map(|need| (xp - xp_for_level(level), need))
}

/// What a run brings: `items` fetched, and the run itself.
pub(in crate::survivors) fn run_xp(items: u32) -> u32 {
    items.saturating_mul(XP_PER_ITEM).saturating_add(RUN_XP)
}

/// Add `xp` to the pet, never past the top level: the experience it took, and the level before
/// and after.
pub(in crate::survivors) fn add_xp(save: &mut RewardsSave, xp: u32) -> (u32, u32, u32) {
    let before = level(save.pet_xp);
    let room = xp_for_level(MAX_LEVEL).saturating_sub(save.pet_xp);
    let took = xp.min(room);
    save.pet_xp += took;
    (took, before, level(save.pet_xp))
}

/// Training points `level` brings in all: one a level, and one more at each of
/// [`BONUS_TP_LEVELS`] reached.
pub(in crate::survivors) fn training_points(level: u32) -> u32 {
    let bonus = BONUS_TP_LEVELS.iter().filter(|&&l| level >= l).count() as u32;
    level.min(MAX_LEVEL) + bonus
}

/// What a skill does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum SkillKind {
    WidePatrol,
    Swiftness,
    Saddlebags,
    TruffleSnout,
    Nip,
    Loyal,
    SecondCompanion,
}

/// One skill the trainer teaches.
pub(in crate::survivors) struct Skill {
    /// The stable key a save keeps its ranks under.
    pub(in crate::survivors) id: &'static str,
    pub(in crate::survivors) name: &'static str,
    pub(in crate::survivors) desc: &'static str,
    /// Each rank's cost in training points: as many ranks as costs.
    pub(in crate::survivors) costs: &'static [u32],
    /// The pet level its first rank needs.
    pub(in crate::survivors) level: u32,
    pub(in crate::survivors) kind: SkillKind,
}

/// Every skill, in the Companion tab's order.
pub(in crate::survivors) static SKILLS: &[Skill] = &[
    Skill {
        id: "wide_patrol",
        name: "Wide Patrol",
        desc: "Fetches from farther afield.",
        costs: &[1, 1, 1],
        level: 1,
        kind: SkillKind::WidePatrol,
    },
    Skill {
        id: "swiftness",
        name: "Swiftness",
        desc: "Fetches more often, and runs faster.",
        costs: &[1, 2, 2],
        level: 1,
        kind: SkillKind::Swiftness,
    },
    Skill {
        id: "saddlebags",
        name: "Saddlebags",
        desc: "Carries more back on every trip.",
        costs: &[1, 2],
        level: 1,
        kind: SkillKind::Saddlebags,
    },
    Skill {
        id: "truffle_snout",
        name: "Truffle Snout",
        desc: "Now and then digs up a small pile of coins.",
        costs: &[1, 1],
        level: 1,
        kind: SkillKind::TruffleSnout,
    },
    Skill {
        id: "nip",
        name: "Nip",
        desc: "Bites the nearest enemy close by; the damage is yours.",
        costs: &[1, 1, 1],
        level: 1,
        kind: SkillKind::Nip,
    },
    Skill {
        id: "loyal",
        name: "Loyal",
        desc: "Every item it fetches heals you a little.",
        costs: &[1, 1],
        level: 1,
        kind: SkillKind::Loyal,
    },
    Skill {
        id: "second_companion",
        name: "Second Companion",
        desc: "A second pet joins in its own look, with these same skills.",
        costs: &[5],
        level: MAX_LEVEL,
        kind: SkillKind::SecondCompanion,
    },
];

impl Skill {
    pub(in crate::survivors) fn max_rank(&self) -> u32 {
        self.costs.len() as u32
    }

    /// The ranks `save` holds, never past the top.
    pub(in crate::survivors) fn rank(&self, save: &RewardsSave) -> u32 {
        save.pet_skills
            .get(self.id)
            .copied()
            .unwrap_or(0)
            .min(self.max_rank())
    }

    /// The next rank's cost; `None` at the top rank.
    pub(in crate::survivors) fn next_cost(&self, save: &RewardsSave) -> Option<u32> {
        self.costs.get(self.rank(save) as usize).copied()
    }

    /// What `rank` ranks do, as the Companion tab words it; `None` at rank 0.
    pub(in crate::survivors) fn effect_at(&self, rank: u32) -> Option<String> {
        if rank == 0 {
            return None;
        }
        Some(match self.kind {
            SkillKind::WidePatrol => format!("Fetches within {} yards", fetch_reach(rank)),
            SkillKind::Swiftness => format!(
                "A fetch every {:.1} s, {}% faster",
                fetch_every(rank),
                (run_speed(rank) * 100.0 - 100.0).round()
            ),
            SkillKind::Saddlebags => format!("{} items a trip", items_per_trip(rank)),
            SkillKind::TruffleSnout => {
                format!("Coins every {} s", truffle_every(rank).unwrap_or_default())
            }
            SkillKind::Nip => format!(
                "Bites for {} + {} per run level every {} s",
                NIP_BASE * rank as f32,
                NIP_PER_LEVEL * rank as f32,
                NIP_EVERY
            ),
            SkillKind::Loyal => {
                format!("Heals {}% max health a fetch", LOYAL_PERCENT * rank as f32)
            }
            SkillKind::SecondCompanion => "A second companion".into(),
        })
    }
}

/// Why a skill's next rank cannot be trained now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum Blocked {
    /// No companion: the perk is not bought.
    NoCompanion,
    TopRank,
    /// The pet is under the level the skill needs.
    Level(u32),
    /// Short of training points; the cost.
    Points(u32),
}

/// Training points spent on the ranks `save` holds.
pub(in crate::survivors) fn spent(save: &RewardsSave) -> u32 {
    SKILLS
        .iter()
        .map(|s| s.costs[..s.rank(save) as usize].iter().sum::<u32>())
        .sum()
}

/// Training points to spend now: those the pet's level brings, less those spent. Never below
/// zero, even if a save holds more than its level pays for.
pub(in crate::survivors) fn available(save: &RewardsSave) -> u32 {
    training_points(level(save.pet_xp)).saturating_sub(spent(save))
}

/// Whether `skill`'s next rank may be trained, and its cost if so.
pub(in crate::survivors) fn can_train(save: &RewardsSave, skill: &Skill) -> Result<u32, Blocked> {
    if !perks::companion_unlocked(save) {
        return Err(Blocked::NoCompanion);
    }
    let cost = skill.next_cost(save).ok_or(Blocked::TopRank)?;
    if level(save.pet_xp) < skill.level {
        return Err(Blocked::Level(skill.level));
    }
    if cost > available(save) {
        return Err(Blocked::Points(cost));
    }
    Ok(cost)
}

/// Train `skill`'s next rank: what it cost, or why not.
pub(in crate::survivors) fn train(save: &mut RewardsSave, skill: &Skill) -> Result<u32, Blocked> {
    let cost = can_train(save, skill)?;
    let rank = skill.rank(save) + 1;
    save.pet_skills.insert(skill.id.into(), rank);
    Ok(cost)
}

/// Unlearn `skill`'s top rank, its points back: what they were, or `None` at rank 0.
pub(in crate::survivors) fn untrain(save: &mut RewardsSave, skill: &Skill) -> Option<u32> {
    let rank = skill.rank(save).checked_sub(1)?;
    if rank == 0 {
        save.pet_skills.remove(skill.id);
    } else {
        save.pet_skills.insert(skill.id.into(), rank);
    }
    Some(skill.costs[rank as usize])
}

/// The trainer's free reset: every rank unlearned. The points it gave back, or `None` with
/// nothing trained.
pub(in crate::survivors) fn reset(save: &mut RewardsSave) -> Option<u32> {
    let back = spent(save);
    let any = SKILLS.iter().any(|s| s.rank(save) > 0);
    save.pet_skills.clear();
    any.then_some(back)
}

/// The ranks a companion works with, read from the save as it joins a run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::survivors) struct PetSkills {
    pub(in crate::survivors) wide_patrol: u32,
    pub(in crate::survivors) swiftness: u32,
    pub(in crate::survivors) saddlebags: u32,
    pub(in crate::survivors) truffle_snout: u32,
    pub(in crate::survivors) nip: u32,
    pub(in crate::survivors) loyal: u32,
    pub(in crate::survivors) second: bool,
}

impl PetSkills {
    pub(in crate::survivors) fn of(save: &RewardsSave) -> Self {
        let rank = |kind: SkillKind| {
            SKILLS
                .iter()
                .find(|s| s.kind == kind)
                .map_or(0, |s| s.rank(save))
        };
        Self {
            wide_patrol: rank(SkillKind::WidePatrol),
            swiftness: rank(SkillKind::Swiftness),
            saddlebags: rank(SkillKind::Saddlebags),
            truffle_snout: rank(SkillKind::TruffleSnout),
            nip: rank(SkillKind::Nip),
            loyal: rank(SkillKind::Loyal),
            second: rank(SkillKind::SecondCompanion) > 0,
        }
    }

    /// How many companions come along: two with the Second Companion.
    pub(in crate::survivors) fn pets(self) -> u8 {
        if self.second {
            2
        } else {
            1
        }
    }
}

/// Wide Patrol: the farthest from the hero a pickup is fetched, 8 yards more a rank.
pub(in crate::survivors) fn fetch_reach(wide_patrol: u32) -> f32 {
    BASE_REACH + 8.0 * wide_patrol as f32
}

/// Swiftness: seconds between fetches, 0.3 fewer a rank.
pub(in crate::survivors) fn fetch_every(swiftness: u32) -> f32 {
    BASE_FETCH_EVERY - 0.3 * swiftness as f32
}

/// Swiftness: the pet's speed against its own, 15% more a rank.
pub(in crate::survivors) fn run_speed(swiftness: u32) -> f32 {
    1.0 + 0.15 * swiftness as f32
}

/// Saddlebags: items brought back a trip, one more a rank.
pub(in crate::survivors) fn items_per_trip(saddlebags: u32) -> u32 {
    1 + saddlebags
}

/// Truffle Snout: seconds between coin piles, 60 at rank 1 and 40 at rank 2; `None` untrained.
pub(in crate::survivors) fn truffle_every(truffle_snout: u32) -> Option<f32> {
    match truffle_snout {
        0 => None,
        1 => Some(60.0),
        _ => Some(40.0),
    }
}

/// Nip: a bite's damage at run level `run_level`, growing with both; 0 untrained.
pub(in crate::survivors) fn nip_damage(nip: u32, run_level: u32) -> f32 {
    nip as f32 * (NIP_BASE + NIP_PER_LEVEL * run_level as f32)
}

/// Loyal: the share of the owner's max health each fetched item heals, 0.5% a rank.
pub(in crate::survivors) fn loyal_heal(loyal: u32) -> f32 {
    LOYAL_PERCENT * loyal as f32 / 100.0
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn skill(id: &str) -> &'static Skill {
        SKILLS.iter().find(|s| s.id == id).unwrap()
    }

    /// A save with the companion bought, at `level`.
    fn pet_at(level: u32) -> RewardsSave {
        let mut save = RewardsSave::default();
        save.perks.insert("companion".into(), 1);
        save.pet_xp = xp_for_level(level);
        save
    }

    #[test]
    fn the_curve_is_gentle_and_tops_out_at_twenty() {
        assert_eq!(level(0), 1);
        assert_eq!(xp_to_next(1), Some(40));
        assert_eq!(xp_to_next(2), Some(70));
        assert_eq!(xp_to_next(19), Some(580));
        assert_eq!(xp_to_next(MAX_LEVEL), None);
        assert_eq!(xp_for_level(1), 0);
        assert_eq!(xp_for_level(2), 40);
        assert_eq!(xp_for_level(MAX_LEVEL), 5890);
        for l in 1..=MAX_LEVEL {
            assert_eq!(level(xp_for_level(l)), l);
            if l > 1 {
                assert_eq!(level(xp_for_level(l) - 1), l - 1);
            }
        }
        assert_eq!(level(u32::MAX), MAX_LEVEL);
        assert_eq!(progress(55), Some((15, 70)));
        assert_eq!(progress(xp_for_level(MAX_LEVEL)), None);

        // An early run, five minutes and some sixty items: a level or two.
        let first = run_xp(60);
        assert_eq!(first, 90);
        assert!((2..=3).contains(&level(first)), "{}", level(first));
        // Runs that fetch ten more each time, up to 250 a run, reach the top in 25 to 35.
        let mut xp = 0;
        let mut runs = 0;
        while level(xp) < MAX_LEVEL {
            xp += run_xp((60 + 10 * runs).min(250));
            runs += 1;
        }
        assert!((25..=35).contains(&runs), "{runs} runs to level 20");
    }

    #[test]
    fn experience_is_capped_at_the_top() {
        let mut save = RewardsSave::default();
        assert_eq!(add_xp(&mut save, 30), (30, 1, 1));
        assert_eq!(add_xp(&mut save, 60), (60, 1, 2));
        let cap = xp_for_level(MAX_LEVEL);
        let (took, _, after) = add_xp(&mut save, 1_000_000);
        assert_eq!((took, after, save.pet_xp), (cap - 90, MAX_LEVEL, cap));
        assert_eq!(add_xp(&mut save, 50), (0, MAX_LEVEL, MAX_LEVEL));
    }

    #[test]
    fn training_points_are_a_level_each_and_four_bonuses() {
        assert_eq!(training_points(1), 1);
        assert_eq!(training_points(4), 4);
        assert_eq!(training_points(5), 6);
        assert_eq!(training_points(10), 12);
        assert_eq!(training_points(15), 18);
        assert_eq!(training_points(MAX_LEVEL), 24);
        // Every skill at its top rank costs what level 20 brings, near enough.
        let all: u32 = SKILLS.iter().flat_map(|s| s.costs).sum();
        assert_eq!(all, 23);
        assert!(all <= training_points(MAX_LEVEL));
    }

    #[test]
    fn the_skill_table_is_as_designed() {
        let ids: HashSet<&str> = SKILLS.iter().map(|s| s.id).collect();
        assert_eq!(ids.len(), SKILLS.len());
        let table: Vec<(&str, &[u32], u32)> =
            SKILLS.iter().map(|s| (s.id, s.costs, s.level)).collect();
        assert_eq!(
            table,
            [
                ("wide_patrol", &[1, 1, 1][..], 1),
                ("swiftness", &[1, 2, 2][..], 1),
                ("saddlebags", &[1, 2][..], 1),
                ("truffle_snout", &[1, 1][..], 1),
                ("nip", &[1, 1, 1][..], 1),
                ("loyal", &[1, 1][..], 1),
                ("second_companion", &[5][..], 20),
            ]
        );
    }

    #[test]
    fn training_spends_points_up_to_the_top_rank() {
        let mut save = pet_at(5);
        assert_eq!(available(&save), 6);
        let swift = skill("swiftness");
        assert_eq!(train(&mut save, swift), Ok(1));
        assert_eq!(train(&mut save, swift), Ok(2), "rank 2 costs two");
        assert_eq!(train(&mut save, swift), Ok(2));
        assert_eq!(train(&mut save, swift), Err(Blocked::TopRank));
        assert_eq!(
            (swift.rank(&save), spent(&save), available(&save)),
            (3, 5, 1)
        );
        let bags = skill("saddlebags");
        assert_eq!(train(&mut save, bags), Ok(1));
        assert_eq!(train(&mut save, bags), Err(Blocked::Points(2)));
        assert_eq!(available(&save), 0);
        // The pet levels: the points come with it.
        save.pet_xp = xp_for_level(6);
        assert_eq!(available(&save), 1);
        assert_eq!(train(&mut save, skill("nip")), Ok(1));
        // Ranks past the top, and a skill this build lacks, are ignored.
        save.pet_skills.insert("wide_patrol".into(), 9);
        save.pet_skills.insert("fetch_sticks".into(), 4);
        assert_eq!(skill("wide_patrol").rank(&save), 3);
        assert_eq!(spent(&save), 5 + 1 + 1 + 3);
        assert_eq!(available(&save), 0, "never below zero");
    }

    #[test]
    fn training_needs_the_companion_and_the_second_needs_level_twenty() {
        let mut none = RewardsSave {
            pet_xp: xp_for_level(MAX_LEVEL),
            ..Default::default()
        };
        assert_eq!(train(&mut none, skill("nip")), Err(Blocked::NoCompanion));
        let second = skill("second_companion");
        let mut save = pet_at(19);
        assert_eq!(available(&save), 22);
        assert_eq!(can_train(&save, second), Err(Blocked::Level(20)));
        // At 20, with every other skill at its top rank, the five points are still there.
        save.pet_xp = xp_for_level(MAX_LEVEL);
        for s in SKILLS
            .iter()
            .filter(|s| s.kind != SkillKind::SecondCompanion)
        {
            while train(&mut save, s).is_ok() {}
            assert_eq!(s.rank(&save), s.max_rank(), "{}", s.id);
        }
        assert_eq!(available(&save), 6);
        assert_eq!(train(&mut save, second), Ok(5));
        assert_eq!(available(&save), 1);
        assert!(PetSkills::of(&save).second);
        assert_eq!(PetSkills::of(&save).pets(), 2);
        assert_eq!(train(&mut save, second), Err(Blocked::TopRank));
        // Unlearned, the second pet goes; trained again, it comes back.
        assert_eq!(untrain(&mut save, second), Some(5));
        assert_eq!(PetSkills::of(&save).pets(), 1);
        assert_eq!(train(&mut save, second), Ok(5));
    }

    #[test]
    fn untraining_and_the_free_reset_give_the_points_back() {
        let mut save = pet_at(10);
        let swift = skill("swiftness");
        train(&mut save, swift).unwrap();
        train(&mut save, swift).unwrap();
        train(&mut save, skill("loyal")).unwrap();
        assert_eq!(spent(&save), 4);
        assert_eq!(untrain(&mut save, swift), Some(2), "the top rank's cost");
        assert_eq!(swift.rank(&save), 1);
        assert_eq!(untrain(&mut save, swift), Some(1));
        assert_eq!(untrain(&mut save, swift), None, "nothing left");
        assert!(!save.pet_skills.contains_key("swiftness"));
        assert_eq!(reset(&mut save), Some(1));
        assert!(save.pet_skills.is_empty());
        assert_eq!(available(&save), training_points(10));
        assert_eq!(reset(&mut save), None, "nothing to reset");
        // As often as you like: it is free.
        for _ in 0..5 {
            train(&mut save, skill("nip")).unwrap();
            assert_eq!(reset(&mut save), Some(1));
        }
    }

    #[test]
    fn each_skill_does_what_its_ranks_say() {
        // Wide Patrol.
        assert_eq!(fetch_reach(0), 25.0);
        assert_eq!(fetch_reach(3), 49.0);
        // Swiftness: interval and speed.
        assert_eq!(fetch_every(0), 2.0);
        assert!((fetch_every(3) - 1.1).abs() < 1e-5);
        assert!((run_speed(2) - 1.3).abs() < 1e-5);
        // Saddlebags.
        assert_eq!(items_per_trip(0), 1);
        assert_eq!(items_per_trip(2), 3);
        // Truffle Snout.
        assert_eq!(truffle_every(0), None);
        assert_eq!(truffle_every(1), Some(60.0));
        assert_eq!(truffle_every(2), Some(40.0));
        // Nip: small, and growing with the run's level and the rank.
        assert_eq!(nip_damage(0, 30), 0.0);
        assert_eq!(nip_damage(1, 1), 4.0);
        assert_eq!(nip_damage(1, 20), 23.0);
        assert_eq!(nip_damage(3, 20), 69.0);
        // Loyal.
        assert_eq!(loyal_heal(0), 0.0);
        assert!((loyal_heal(1) - 0.005).abs() < 1e-6);
        assert!((loyal_heal(2) - 0.01).abs() < 1e-6);
        // As the tab words them.
        assert_eq!(skill("wide_patrol").effect_at(0), None);
        assert_eq!(
            skill("wide_patrol").effect_at(2).as_deref(),
            Some("Fetches within 41 yards")
        );
        assert_eq!(
            skill("swiftness").effect_at(1).as_deref(),
            Some("A fetch every 1.7 s, 15% faster")
        );
        assert_eq!(
            skill("saddlebags").effect_at(1).as_deref(),
            Some("2 items a trip")
        );
        assert_eq!(
            skill("truffle_snout").effect_at(2).as_deref(),
            Some("Coins every 40 s")
        );
        assert_eq!(
            skill("nip").effect_at(2).as_deref(),
            Some("Bites for 6 + 2 per run level every 3 s")
        );
        assert_eq!(
            skill("loyal").effect_at(1).as_deref(),
            Some("Heals 0.5% max health a fetch")
        );
    }

    #[test]
    fn skills_are_read_from_the_save() {
        let mut save = pet_at(12);
        assert_eq!(PetSkills::of(&save), PetSkills::default());
        assert_eq!(PetSkills::of(&save).pets(), 1);
        train(&mut save, skill("wide_patrol")).unwrap();
        train(&mut save, skill("nip")).unwrap();
        train(&mut save, skill("nip")).unwrap();
        let s = PetSkills::of(&save);
        assert_eq!((s.wide_patrol, s.nip, s.loyal, s.second), (1, 2, 0, false));
    }

    #[test]
    fn names_go_round_the_list() {
        let mut save = RewardsSave::default();
        assert_eq!(name(&save), "Peacebloom");
        assert_eq!(next_name(name(&save)), "Silverleaf");
        assert_eq!(next_name(PET_NAMES[PET_NAMES.len() - 1]), PET_NAMES[0]);
        assert_eq!(next_name("Mr. Bigglesworth"), PET_NAMES[0]);
        save.pet_name = "Icecap".into();
        assert_eq!(name(&save), "Icecap");
        let unique: HashSet<&str> = PET_NAMES.iter().copied().collect();
        assert_eq!(unique.len(), PET_NAMES.len());
    }
}
