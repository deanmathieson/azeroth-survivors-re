//! The account-wide quest chains, one a zone, three steps each: turn in [`CHAIN_QUESTS`] of the
//! zone's in-run quests over any runs, then kill its last boss, then win there at
//! [`CHAIN_WIN_MULT`] rewards or more. Steps go in order, judged from the profile and each
//! [`RunEnded`]'s outcome; one run may finish several. The last step pays the chain's
//! [`Reward`] into the section of the profile that owns it.

use bevy::prelude::*;

use super::super::data;
use super::super::hud::{NoteKind, ResultsNotes};
use super::super::profile::{slug, Profile, RunEnded, RunOutcome};
use super::super::talents;
use super::catalog::{for_map, Reward};

/// Quests turned in on a zone to finish its chain's first step.
pub(in crate::survivors) const CHAIN_QUESTS: u32 = 3;
/// The rewards multiplier the third step's win must be played at, or more.
pub(in crate::survivors) const CHAIN_WIN_MULT: f32 = 1.25;
/// Steps in every chain.
pub(in crate::survivors) const STEPS: u8 = 3;

/// One step of a chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum Step {
    Quests,
    FinalBoss,
    Win,
}

impl Step {
    pub(in crate::survivors) const ALL: [Self; 3] = [Self::Quests, Self::FinalBoss, Self::Win];

    /// What the step asks on `map`: "Defeat Gruff Swiftbite, Elwynn Forest's last boss".
    pub(in crate::survivors) fn line(self, map: usize) -> String {
        let zone = data::MAPS.get(map).map_or("the zone", |m| m.name);
        match self {
            Self::Quests => format!("Complete {CHAIN_QUESTS} quests in {zone}"),
            Self::FinalBoss => {
                let boss = data::roster(map)
                    .bosses
                    .last()
                    .map_or("the last boss", |b| b.name);
                format!("Defeat {boss}, {zone}'s last boss")
            }
            Self::Win => format!("Win in {zone} at ×{CHAIN_WIN_MULT:.2} rewards or more"),
        }
    }

    /// Whether this step, the one under way, is met: by the zone's quest count so far, or by
    /// `run`.
    fn met(self, completed: u32, run: &RunOutcome) -> bool {
        match self {
            Self::Quests => completed >= CHAIN_QUESTS,
            Self::FinalBoss => run.final_boss,
            // Rounding dust from the Run Options sliders must not cost a ×1.25 win.
            Self::Win => run.won && run.reward_mult >= CHAIN_WIN_MULT - 1e-4,
        }
    }
}

/// Fold one recorded run into its zone's quest record and chain: count its quests, advance every
/// step it meets in order, and pay the reward if the chain ends. Returns the results lines, empty
/// when nothing changed but the quest count.
pub(in crate::survivors) fn judge(profile: &mut Profile, run: &RunOutcome) -> Vec<String> {
    let (Some(map), Some(zone)) = (data::MAPS.get(run.map), for_map(run.map)) else {
        return Vec::new();
    };
    let key = slug(map.name);
    let mut record = profile.quests.zone(&key);
    let before = record.clone();
    record.completed = record.completed.saturating_add(run.quests);
    let mut lines = Vec::new();
    let mut finished = false;
    while let Some(&step) = Step::ALL.get(usize::from(record.steps)) {
        if !step.met(record.completed, run) {
            break;
        }
        record.steps += 1;
        if record.steps < STEPS {
            lines.push(format!(
                "{}: step {} of {STEPS} done",
                zone.chain.name, record.steps
            ));
        } else {
            finished = true;
        }
    }
    // A run that changes nothing leaves no empty record behind.
    if record != before {
        profile.quests.zones.insert(key, record);
    }
    if finished {
        let paid = grant(profile, zone.chain.reward, run.class);
        lines.push(format!(
            "Quest chain complete: {}! {}: {}",
            zone.chain.name,
            zone.chain.reward.kind(),
            zone.chain.reward.name()
        ));
        lines.extend(paid);
    }
    lines
}

/// Pay `reward` into the profile section that owns it: talent XP to the class's talents, a look
/// to the companion's rewards, and mounts, hats and tabards into the quests' own unlocks for the
/// features that will read them. Returns any extra results line.
pub(in crate::survivors) fn grant(
    profile: &mut Profile,
    reward: Reward,
    class: usize,
) -> Option<String> {
    match reward {
        Reward::TalentXp(xp) => {
            let class = data::CLASSES.get(class)?;
            let save = &mut profile.classes.entry(slug(class.name)).or_default().talents;
            let points = talents::add_xp(save, xp);
            (points > 0).then(|| {
                let s = if points == 1 { "" } else { "s" };
                format!("+{points} talent point{s} for {}!", class.name)
            })
        }
        Reward::CompanionLook(display) => {
            profile.rewards.unlocked_looks.insert(display);
            None
        }
        Reward::Mount(_) | Reward::MountHat(_) | Reward::Tabard(_) => {
            let key = reward.key()?;
            let unlocks = &mut profile.quests;
            let set = match reward {
                Reward::Mount(_) => &mut unlocks.mounts,
                Reward::MountHat(_) => &mut unlocks.mount_hats,
                _ => &mut unlocks.tabards,
            };
            set.insert(key);
            None
        }
    }
}

/// Each recorded run is judged once against its zone's chain; what it finishes is saved and
/// announced on the results screen.
pub(super) fn judge_chains(
    mut ended: MessageReader<RunEnded>,
    mut profile: ResMut<Profile>,
    mut notes: ResMut<ResultsNotes>,
) {
    for RunEnded { outcome } in ended.read() {
        let before = profile.quests.clone();
        let lines = judge(&mut profile, outcome);
        if profile.quests != before || !lines.is_empty() {
            profile.mark_dirty();
        }
        for line in lines {
            notes.push(line, NoteKind::Good);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::save::ZoneQuestRecord;
    use super::*;

    fn index(name: &str) -> usize {
        data::MAPS.iter().position(|m| m.name == name).unwrap()
    }

    fn run(map: usize) -> RunOutcome {
        RunOutcome {
            class: 0,
            map,
            won: false,
            seconds: 300.0,
            kills: 100,
            bosses: 2,
            level: 10,
            damage: 1.0,
            gold: 0,
            reward_mult: 1.0,
            quests: 0,
            final_boss: false,
        }
    }

    fn record(profile: &Profile, zone: &str) -> ZoneQuestRecord {
        profile.quests.zone(&slug(zone))
    }

    #[test]
    fn steps_go_in_order_across_runs() {
        let elwynn = index("Elwynn Forest");
        let mut p = Profile::default();
        // Two quests and the last boss: the boss is no use before the quests are in.
        let lines = judge(
            &mut p,
            &RunOutcome {
                quests: 2,
                final_boss: true,
                ..run(elwynn)
            },
        );
        assert!(lines.is_empty(), "{lines:?}");
        assert_eq!(
            record(&p, "Elwynn Forest"),
            ZoneQuestRecord {
                completed: 2,
                steps: 0
            }
        );
        // The third quest finishes the first step.
        let lines = judge(
            &mut p,
            &RunOutcome {
                quests: 1,
                ..run(elwynn)
            },
        );
        assert_eq!(lines, ["Hogger's Bounty: step 1 of 3 done"]);
        // A win at ×1.25 before the boss counts for nothing yet.
        let lines = judge(
            &mut p,
            &RunOutcome {
                won: true,
                reward_mult: 1.5,
                ..run(elwynn)
            },
        );
        assert!(lines.is_empty());
        assert_eq!(record(&p, "Elwynn Forest").steps, 1);
        let lines = judge(
            &mut p,
            &RunOutcome {
                final_boss: true,
                ..run(elwynn)
            },
        );
        assert_eq!(lines, ["Hogger's Bounty: step 2 of 3 done"]);
        // Won, but under the multiplier: not yet.
        let under = RunOutcome {
            won: true,
            reward_mult: 1.2,
            ..run(elwynn)
        };
        assert!(judge(&mut p, &under).is_empty());
        let lines = judge(
            &mut p,
            &RunOutcome {
                reward_mult: 1.25,
                ..under
            },
        );
        assert_eq!(
            lines,
            ["Quest chain complete: Hogger's Bounty! Mount: Brown Horse"]
        );
        assert!(p.quests.mounts.contains("brown_horse"));
        assert_eq!(record(&p, "Elwynn Forest").steps, STEPS);
        // Done is done: another perfect run pays nothing more, but its quests still count.
        let perfect = RunOutcome {
            quests: 4,
            final_boss: true,
            won: true,
            reward_mult: 2.0,
            ..run(elwynn)
        };
        assert!(judge(&mut p, &perfect).is_empty());
        assert_eq!(record(&p, "Elwynn Forest").completed, 7);
        assert_eq!(p.quests.mounts.len(), 1);
        // Another zone's chain is its own.
        assert_eq!(record(&p, "Westfall"), ZoneQuestRecord::default());
    }

    #[test]
    fn one_run_can_finish_a_whole_chain() {
        let mut p = Profile::default();
        let lines = judge(
            &mut p,
            &RunOutcome {
                quests: 3,
                final_boss: true,
                won: true,
                reward_mult: 1.25,
                ..run(index("Teldrassil"))
            },
        );
        assert_eq!(
            lines,
            [
                "Shadows in the Canopy: step 1 of 3 done",
                "Shadows in the Canopy: step 2 of 3 done",
                "Quest chain complete: Shadows in the Canopy! Tabard: Darnassus Tabard",
            ]
        );
        assert!(p.quests.tabards.contains("darnassus_tabard"));
    }

    #[test]
    fn rewards_land_in_their_owners_sections() {
        let done = |map: usize| RunOutcome {
            class: 2,
            quests: 3,
            final_boss: true,
            won: true,
            reward_mult: 1.5,
            ..run(map)
        };
        let mut p = Profile::default();
        // Talent XP, to the finishing class's talents.
        let class = slug(data::CLASSES[2].name);
        let lines = judge(&mut p, &done(index("Mulgore")));
        assert_eq!(p.classes[&class].talents.xp, 1000);
        assert!(
            lines.iter().any(|l| l.contains("+1000 talent XP")),
            "{lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.starts_with("+")),
            "the points line: {lines:?}"
        );
        // A companion look, to the rewards section the companion reads.
        judge(&mut p, &done(index("Durotar")));
        assert!(p.rewards.unlocked_looks.contains(&1206));
        // A hat and a mount and a tabard, recorded for the features to come.
        judge(&mut p, &done(index("Dun Morogh")));
        judge(&mut p, &done(index("Gates of Ahn'Qiraj")));
        judge(&mut p, &done(index("Westfall")));
        assert!(p.quests.mount_hats.contains("tiny_party_hat"));
        assert!(p.quests.mounts.contains("horn_of_the_black_war_wolf"));
        assert!(p.quests.tabards.contains("people_s_militia_tabard"));
        // Nothing leaked elsewhere: the other classes and the perks are untouched.
        assert_eq!(p.classes.len(), 1);
        assert_eq!(p.rewards.spent, 0);
        assert_eq!(p.rewards.unlocked_looks.len(), 1);
    }

    #[test]
    fn every_zone_words_its_steps() {
        for map in 0..data::MAPS.len() {
            for step in Step::ALL {
                assert!(step.line(map).contains(data::MAPS[map].name));
            }
        }
        assert_eq!(
            Step::FinalBoss.line(index("Elwynn Forest")),
            "Defeat Gruff Swiftbite, Elwynn Forest's last boss"
        );
        assert_eq!(
            Step::Win.line(index("Durotar")),
            "Win in Durotar at ×1.25 rewards or more"
        );
    }

    #[test]
    fn the_system_saves_and_announces_each_run_once() {
        let mut world = World::new();
        world.init_resource::<Messages<RunEnded>>();
        world.init_resource::<ResultsNotes>();
        world.insert_resource(Profile::default());
        world.resource_mut::<Messages<RunEnded>>().write(RunEnded {
            outcome: RunOutcome {
                quests: 3,
                ..run(index("Duskwood"))
            },
        });
        let judge = world.register_system(judge_chains);
        world.run_system(judge).unwrap();
        world.run_system(judge).unwrap();
        let notes = &world.resource::<ResultsNotes>().0;
        assert_eq!(
            notes,
            &[(
                "Lanterns of Darkshire: step 1 of 3 done".to_string(),
                NoteKind::Good
            )]
        );
        assert_eq!(
            world
                .resource::<Profile>()
                .quests
                .zone("duskwood")
                .completed,
            3
        );
        // A run with no quests on a zone with none counted changes nothing, and saves nothing.
        world.resource_mut::<Messages<RunEnded>>().write(RunEnded {
            outcome: run(index("Mulgore")),
        });
        let before = world.resource::<Profile>().quests.clone();
        world.run_system(judge).unwrap();
        assert_eq!(world.resource::<ResultsNotes>().0.len(), 1);
        assert_eq!(world.resource::<Profile>().quests, before);
        assert!(!world
            .resource::<Profile>()
            .quests
            .zones
            .contains_key("mulgore"));
    }
}
