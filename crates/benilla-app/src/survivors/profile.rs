//! The survivors account profile: a small, versioned TOML save shared by every run.

use std::collections::BTreeMap;
use std::io::ErrorKind;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::achievements::AchievementSave;
use super::gear::GearSave;
use super::rewards::RewardsSave;
use super::talents::TalentSave;
use super::{data, Run, Selection};

/// The newest profile shape this build knows how to write.
pub(super) const PROFILE_VERSION: u32 = 1;
/// The profile's single resident in [`crate::local_state`].
pub(super) const PROFILE_FILE: &str = "survivors-profile.toml";

/// Account-wide totals accumulated from finished runs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct AccountRecord {
    pub runs: u32,
    pub wins: u32,
    pub deaths: u32,
    pub kills: u64,
    pub bosses: u64,
    pub seconds_played: f64,
    pub damage_done: f64,
    pub best_level: u32,
    /// Gold picked up over every run; saves from before gold read as 0.
    pub gold_earned: u64,
}

/// One class's finished-run record.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct ClassRecord {
    pub runs: u32,
    pub wins: u32,
    pub kills: u64,
    pub best_seconds: f32,
    pub best_level: u32,
    /// The talents' section, defined and edited in [`super::talents`].
    pub talents: TalentSave,
    /// The gear's section, defined and edited in [`super::gear`].
    pub gear: GearSave,
}

/// One zone's finished-run record.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct ZoneRecord {
    pub runs: u32,
    pub wins: u32,
    pub best_seconds: f32,
}

/// The persistent survivors account. Renaming a class or zone orphans its slugged record.
#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Profile {
    #[serde(default = "default_version")]
    pub version: u32,
    pub account: AccountRecord,
    pub classes: BTreeMap<String, ClassRecord>,
    pub zones: BTreeMap<String, ZoneRecord>,
    /// The achievements' section, defined and edited in [`super::achievements`].
    pub achievements: AchievementSave,
    /// Perks, companions and reward points, defined and edited in [`super::rewards`].
    pub rewards: RewardsSave,
    #[serde(skip, default = "default_writable")]
    writable: bool,
    /// Unsaved changes: [`save_if_dirty`] writes them once and clears this.
    #[serde(skip)]
    dirty: bool,
}

fn default_version() -> u32 {
    PROFILE_VERSION
}

fn default_writable() -> bool {
    true
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            version: PROFILE_VERSION,
            account: AccountRecord::default(),
            classes: BTreeMap::new(),
            zones: BTreeMap::new(),
            achievements: AchievementSave::default(),
            rewards: RewardsSave::default(),
            writable: true,
            dirty: false,
        }
    }
}

/// A completed run, reduced to the values a profile keeps.
#[derive(Clone, Copy, Debug)]
pub(super) struct RunOutcome {
    pub class: usize,
    pub map: usize,
    pub won: bool,
    pub seconds: f32,
    pub kills: u32,
    pub bosses: u32,
    pub level: u32,
    pub damage: f64,
    pub gold: u32,
    /// The Run Options rewards multiplier the run was played under ([`Run::reward_mult`]).
    // Read by the reward workstreams' RunEnded systems, still to land.
    #[allow(dead_code)]
    pub reward_mult: f32,
}

/// A run, recorded: [`record_finished`] writes exactly one per run, on the GameOver frame after
/// [`Profile::record`], ahead of `Update`. A feature reads it from a system in its own plugin,
/// changes `ResMut<Profile>`, calls [`Profile::mark_dirty`] and may push
/// [`super::hud::ResultsNotes`]; [`save_if_dirty`] then writes the record and every feature's
/// changes in one save that frame. None is sent for a run that goes unrecorded.
#[derive(Message, Clone, Copy, Debug)]
pub(super) struct RunEnded {
    // Read by the feature workstreams' systems, still to land.
    #[allow(dead_code)]
    pub outcome: RunOutcome,
}

/// The profile changes a results screen can announce.
#[derive(Resource, Clone, Debug, Default)]
pub(super) struct LastRunNews {
    pub zone: String,
    pub class: String,
    pub zone_runs: u32,
    pub best_seconds: f32,
    pub new_best_survival: bool,
    pub first_zone_win: bool,
    pub first_class_win: bool,
}

type RunNews = LastRunNews;

impl Profile {
    /// Record a run without performing IO, returning the news for its results screen.
    pub(super) fn record(&mut self, outcome: &RunOutcome) -> RunNews {
        let class_name = data::CLASSES
            .get(outcome.class)
            .map(|class| class.name)
            .unwrap_or("Unknown");
        let zone_name = data::MAPS
            .get(outcome.map)
            .map(|map| map.name)
            .unwrap_or("Unknown");
        let class = self.classes.entry(slug(class_name)).or_default();
        let first_class_win = outcome.won && class.wins == 0;
        class.runs = class.runs.saturating_add(1);
        class.kills = class.kills.saturating_add(u64::from(outcome.kills));
        class.best_seconds = class.best_seconds.max(outcome.seconds);
        class.best_level = class.best_level.max(outcome.level);
        if outcome.won {
            class.wins = class.wins.saturating_add(1);
        }

        let zone = self.zones.entry(slug(zone_name)).or_default();
        let new_best_survival =
            !outcome.won && zone.runs > 0 && (outcome.seconds as u32 > zone.best_seconds as u32);
        let first_zone_win = outcome.won && zone.wins == 0;
        zone.runs = zone.runs.saturating_add(1);
        zone.best_seconds = zone.best_seconds.max(outcome.seconds);
        if outcome.won {
            zone.wins = zone.wins.saturating_add(1);
        }

        self.account.runs = self.account.runs.saturating_add(1);
        self.account.kills = self.account.kills.saturating_add(u64::from(outcome.kills));
        self.account.bosses = self
            .account
            .bosses
            .saturating_add(u64::from(outcome.bosses));
        self.account.seconds_played += f64::from(outcome.seconds);
        self.account.damage_done += outcome.damage;
        self.account.best_level = self.account.best_level.max(outcome.level);
        self.account.gold_earned = self
            .account
            .gold_earned
            .saturating_add(u64::from(outcome.gold));
        if outcome.won {
            self.account.wins = self.account.wins.saturating_add(1);
        } else {
            self.account.deaths = self.account.deaths.saturating_add(1);
        }

        RunNews {
            zone: zone_name.into(),
            class: class_name.into(),
            zone_runs: zone.runs,
            best_seconds: zone.best_seconds,
            new_best_survival,
            first_zone_win,
            first_class_win,
        }
    }

    /// Queue a save: [`save_if_dirty`] writes once this frame, after `Update`, however many
    /// systems changed the profile.
    pub(super) fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Hand the profile to `write` if it has unsaved changes and may be written, clearing the
    /// flag either way (a read-only profile's changes stay in memory); whether `write` ran.
    fn flush(&mut self, write: impl FnOnce(&Profile)) -> bool {
        if !std::mem::take(&mut self.dirty) || !self.writable {
            return false;
        }
        write(self);
        true
    }
}

/// A stable key for display names: lower-case ASCII words joined by one underscore.
pub(super) fn slug(name: &str) -> String {
    let mut slug = String::new();
    let mut separator = false;
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() {
            if separator && !slug.is_empty() {
                slug.push('_');
            }
            slug.push(byte.to_ascii_lowercase() as char);
            separator = false;
        } else {
            separator = true;
        }
    }
    slug
}

struct LoadedProfile {
    profile: Profile,
    corrupt_text: Option<String>,
}

#[derive(Deserialize)]
struct VersionProbe {
    #[serde(default)]
    version: u32,
}

/// Parse a profile without touching local state, retaining bad input for quarantine.
fn load_text(text: &str) -> LoadedProfile {
    match toml::from_str::<Profile>(text) {
        Ok(mut profile) => {
            profile.writable = profile.version <= PROFILE_VERSION;
            LoadedProfile {
                profile,
                corrupt_text: None,
            }
        }
        Err(_) => {
            let newer_version = toml::from_str::<VersionProbe>(text)
                .is_ok_and(|probe| probe.version > PROFILE_VERSION);
            let profile = Profile {
                writable: false,
                ..Profile::default()
            };
            LoadedProfile {
                profile,
                corrupt_text: (!newer_version).then(|| text.into()),
            }
        }
    }
}

/// Load the account once. A malformed resident is copied before this build starts fresh.
pub(super) fn load_profile(mut commands: Commands) {
    commands.insert_resource(read_profile());
}

fn read_profile() -> Profile {
    let Some(home) = crate::local_state::home() else {
        return Profile::default();
    };
    let path = home.join(PROFILE_FILE);
    let text = match crate::local_state::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Profile::default(),
        Err(error) => {
            warn!(
                "survivors profile: could not read {}: {error}",
                path.display()
            );
            return read_only_default();
        }
    };
    let loaded = load_text(&text);
    if let Some(corrupt) = loaded.corrupt_text {
        match quarantine(&home, &corrupt) {
            Ok(()) => {
                let profile = Profile::default();
                if !save_at(&path, &profile) {
                    return read_only_default();
                }
                warn!("survivors profile: malformed save kept in a corrupt copy; starting fresh");
                return profile;
            }
            Err(error) => {
                warn!("survivors profile: could not quarantine malformed save: {error}");
                return read_only_default();
            }
        }
    } else if !loaded.profile.writable {
        warn!(
            "survivors profile: version {} is newer; leaving it read-only",
            loaded.profile.version
        );
    }
    loaded.profile
}

fn read_only_default() -> Profile {
    Profile {
        writable: false,
        ..default()
    }
}

/// Resolve the profile state after an injected read and quarantine result.
/// Keeping this separate makes the failure policy testable without local state.
#[cfg(test)]
fn profile_from_read(
    read: Result<&str, ErrorKind>,
    quarantine_result: Result<(), ()>,
    fresh_save_result: Result<(), ()>,
) -> Profile {
    match read {
        Err(ErrorKind::NotFound) => Profile::default(),
        Err(_) => read_only_default(),
        Ok(text) => {
            let loaded = load_text(text);
            if loaded.corrupt_text.is_some() {
                if quarantine_result.is_ok() && fresh_save_result.is_ok() {
                    Profile::default()
                } else {
                    read_only_default()
                }
            } else {
                loaded.profile
            }
        }
    }
}

fn quarantine(home: &std::path::Path, text: &str) -> Result<(), String> {
    for n in 1u32.. {
        let path = home.join(format!("survivors-profile.corrupt-{n}.toml"));
        match crate::local_state::read_to_string(&path) {
            Ok(_) => continue,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                if let Err(error) = crate::local_state::write_atomic(&path, text) {
                    return Err(error.to_string());
                }
                return Ok(());
            }
            Err(error) => {
                return Err(format!("cannot safely choose corrupt copy {n}: {error}"));
            }
        }
    }
    unreachable!()
}

fn save(profile: &Profile) {
    if !profile.writable {
        return;
    }
    let Some(path) = crate::local_state::home().map(|home| home.join(PROFILE_FILE)) else {
        return;
    };
    let _ = save_at(&path, profile);
}

fn save_at(path: &std::path::Path, profile: &Profile) -> bool {
    let text = match toml::to_string_pretty(profile) {
        Ok(text) => text,
        Err(error) => {
            warn!("survivors profile: could not serialize save: {error}");
            return false;
        }
    };
    if let Err(error) = crate::local_state::write_atomic(path, &text) {
        warn!(
            "survivors profile: could not save {}: {error}",
            path.display()
        );
        false
    } else {
        true
    }
}

/// The profile's one writer: after `Update`, a profile marked dirty is saved once, so a run's
/// record and every feature's [`RunEnded`] changes land in a single write. Never reloads: what is
/// in memory is the newest, and a reload here would drop the changes systems made this frame.
pub(super) fn save_if_dirty(mut profile: ResMut<Profile>) {
    if profile.dirty {
        profile.bypass_change_detection().flush(save);
    }
}

/// Turn the one GameOver transition into exactly one persistent run record and one [`RunEnded`].
pub(super) fn record_finished(
    mut run: ResMut<Run>,
    selection: Res<Selection>,
    mut profile: ResMut<Profile>,
    mut news: ResMut<LastRunNews>,
    mut ended: MessageWriter<RunEnded>,
    auto: Option<Res<super::auto::Auto>>,
) {
    if run.recorded {
        return;
    }
    run.recorded = true;
    if auto.is_some() && std::env::var("WOW_SURVIVORS_RECORD").as_deref() != Ok("1") {
        return;
    }
    let class = selection.class.unwrap_or_else(|| {
        data::CLASSES
            .iter()
            .position(|class| class.name == run.class.name)
            .unwrap_or(0)
    });
    let outcome = RunOutcome {
        class,
        map: selection.map,
        won: run.won,
        seconds: run.elapsed,
        kills: run.kills,
        // The mode tracks bosses spawned, not bosses killed.
        bosses: run.bosses,
        level: run.level,
        damage: run.damage_done,
        gold: run.gold,
        reward_mult: run.reward_mult,
    };
    // The save on disk may be newer (another copy of the game): this run is applied to it. The
    // reload lives only here, where the change to reapply is known; a profile with unsaved changes
    // keeps them instead, since the file has not seen them.
    if profile.writable && !profile.dirty {
        match read_clean_profile() {
            Ok(Some(fresh)) => *profile = fresh,
            Ok(None) => {}
            Err(error) => {
                warn!("survivors profile: could not safely reload save: {error}");
                profile.writable = false;
                return;
            }
        }
    }
    *news = profile.record(&outcome);
    profile.mark_dirty();
    ended.write(RunEnded { outcome });
}

fn read_clean_profile() -> Result<Option<Profile>, String> {
    let Some(home) = crate::local_state::home() else {
        return Ok(None);
    };
    let path = home.join(PROFILE_FILE);
    match crate::local_state::read_to_string(&path) {
        Ok(text) => clean_profile_from_text(&text)
            .map(Some)
            .map_err(|()| "save is malformed or from a newer version".into()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(Some(Profile::default())),
        Err(error) => Err(error.to_string()),
    }
}

fn clean_profile_from_text(text: &str) -> Result<Profile, ()> {
    let loaded = load_text(text);
    (loaded.corrupt_text.is_none() && loaded.profile.writable)
        .then_some(loaded.profile)
        .ok_or(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mut profile = Profile::default();
        profile.account.runs = 4;
        profile.classes.insert(
            "mage".into(),
            ClassRecord {
                wins: 1,
                ..default()
            },
        );
        assert_eq!(
            toml::from_str::<Profile>(&toml::to_string_pretty(&profile).unwrap()).unwrap(),
            profile
        );
    }

    #[test]
    fn missing_fields_and_unknown_fields_are_safe() {
        let profile: Profile =
            toml::from_str("extra = 9\n[account]\nruns = 2\nunknown = true\n").unwrap();
        assert_eq!(profile.version, PROFILE_VERSION);
        assert_eq!(profile.account.runs, 2);
        assert_eq!(profile.account.kills, 0);
        assert_eq!(profile.account.gold_earned, 0);
        assert!(profile.classes.is_empty());
    }

    #[test]
    fn saves_without_feature_sections_load_and_round_trip() {
        let text = "version = 1\n\n[account]\nruns = 3\nwins = 1\n\n\
                    [classes.mage]\nruns = 3\nwins = 1\nbest_level = 9\n\n\
                    [zones.elwynn]\nruns = 3\nbest_seconds = 250.0\n";
        let loaded = load_text(text);
        assert!(loaded.corrupt_text.is_none());
        let profile = loaded.profile;
        assert!(profile.writable);
        assert_eq!(profile.account.runs, 3);
        let mage = &profile.classes["mage"];
        assert_eq!((mage.runs, mage.wins, mage.best_level), (3, 1, 9));
        assert_eq!(mage.talents, TalentSave::default());
        assert_eq!(mage.gear, GearSave::default());
        assert_eq!(profile.zones["elwynn"].best_seconds, 250.0);
        assert_eq!(profile.achievements, AchievementSave::default());
        assert_eq!(profile.rewards, RewardsSave::default());
        let again = load_text(&toml::to_string_pretty(&profile).unwrap());
        assert!(again.corrupt_text.is_none());
        assert_eq!(again.profile, profile);
    }

    #[test]
    fn a_dirty_writable_profile_is_written_once() {
        let mut written = Vec::new();
        let mut profile = Profile::default();
        assert!(!profile.flush(|p| written.push(p.clone())));
        profile.account.runs = 5;
        profile.mark_dirty();
        profile.mark_dirty();
        assert!(profile.flush(|p| written.push(p.clone())));
        assert!(!profile.dirty);
        assert!(!profile.flush(|p| written.push(p.clone())));
        assert_eq!(written.len(), 1);
        assert_eq!(written[0].account.runs, 5);
        // Read-only: never written, and the flag still clears.
        let mut read_only = read_only_default();
        read_only.mark_dirty();
        assert!(!read_only.flush(|p| written.push(p.clone())));
        assert!(!read_only.dirty);
        assert_eq!(written.len(), 1);
        // The flag is not part of the save.
        profile.mark_dirty();
        let text = toml::to_string_pretty(&profile).unwrap();
        assert!(!toml::from_str::<Profile>(&text).unwrap().dirty);
    }

    #[test]
    fn run_ended_is_sent_once_per_recorded_run() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.init_resource::<Messages<RunEnded>>();
        world.insert_resource(Run {
            kills: 7,
            reward_mult: 1.5,
            ..default()
        });
        world.init_resource::<Selection>();
        // Read-only, so the system neither reloads nor writes a save.
        world.insert_resource(read_only_default());
        world.init_resource::<LastRunNews>();
        // A repeated GameOver transition for the same run.
        for _ in 0..2 {
            world.run_system_once(record_finished).unwrap();
        }
        let sent: Vec<RunEnded> = world
            .resource::<Messages<RunEnded>>()
            .iter_current_update_messages()
            .copied()
            .collect();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].outcome.kills, 7);
        assert_eq!(sent[0].outcome.reward_mult, 1.5);
        let profile = world.resource::<Profile>();
        assert_eq!(profile.account.runs, 1);
        assert!(profile.dirty, "the record waits for save_if_dirty");
        // The next run is recorded and announced in its turn.
        world.resource_mut::<Run>().recorded = false;
        world.run_system_once(record_finished).unwrap();
        assert_eq!(world.resource::<Messages<RunEnded>>().len(), 2);
        assert_eq!(world.resource::<Profile>().account.runs, 2);
    }

    #[test]
    fn record_accumulates_records_and_news() {
        let mut profile = Profile::default();
        let death = RunOutcome {
            class: 0,
            map: 0,
            won: false,
            seconds: 20.0,
            kills: 3,
            bosses: 1,
            level: 2,
            damage: 5.0,
            gold: 40,
            reward_mult: 1.0,
        };
        let first = profile.record(&death);
        assert!(!first.new_best_survival);
        assert_eq!(first.zone_runs, 1);
        let win = RunOutcome {
            won: true,
            seconds: 30.0,
            ..death
        };
        let second = profile.record(&win);
        assert!(second.first_zone_win && second.first_class_win && !second.new_best_survival);
        let longer_death = RunOutcome {
            won: false,
            seconds: 35.0,
            ..death
        };
        let third = profile.record(&longer_death);
        let zone = profile.zones.get(&slug(data::MAPS[0].name)).unwrap();
        assert_eq!(profile.account.runs, 3);
        assert_eq!(profile.account.wins, 1);
        assert_eq!(profile.account.deaths, 2);
        assert_eq!(profile.account.gold_earned, 120);
        assert_eq!(zone.runs, 3);
        assert_eq!(zone.wins, 1);
        assert_eq!(zone.best_seconds, 35.0);
        assert!(third.new_best_survival);
    }

    #[test]
    fn data_names_have_distinct_slugs() {
        let classes: std::collections::BTreeSet<_> =
            data::CLASSES.iter().map(|class| slug(class.name)).collect();
        let maps: std::collections::BTreeSet<_> =
            data::MAPS.iter().map(|map| slug(map.name)).collect();
        assert_eq!(classes.len(), data::CLASSES.len());
        assert_eq!(maps.len(), data::MAPS.len());
        assert!(classes.iter().all(|slug| !slug.is_empty()));
        assert!(maps.iter().all(|slug| !slug.is_empty()));
    }

    #[test]
    fn corrupt_text_is_retained_for_quarantine() {
        let text = "this is not = toml";
        let loaded = load_text(text);
        assert!(!loaded.profile.writable);
        assert_eq!(loaded.corrupt_text.as_deref(), Some(text));
    }

    #[test]
    fn newer_profile_is_read_only() {
        let loaded = load_text("version = 2");
        assert!(!loaded.profile.writable);
    }

    #[test]
    fn non_not_found_read_failure_is_read_only() {
        assert!(!profile_from_read(Err(ErrorKind::PermissionDenied), Ok(()), Ok(())).writable);
    }

    #[test]
    fn failed_quarantine_is_read_only() {
        assert!(!profile_from_read(Ok("this is not = toml"), Err(()), Ok(())).writable);
    }

    #[test]
    fn newer_unparseable_version_is_read_only_without_quarantine() {
        let text = "version = 2\nclasses = 1";
        let loaded = load_text(text);
        assert!(!loaded.profile.writable);
        assert!(loaded.corrupt_text.is_none());
        assert!(!profile_from_read(Ok(text), Err(()), Ok(())).writable);
    }

    #[test]
    fn failed_fresh_save_after_quarantine_is_read_only() {
        assert!(!profile_from_read(Ok("this is not = toml"), Ok(()), Err(())).writable);
    }

    #[test]
    fn unclean_reloads_are_rejected() {
        assert!(clean_profile_from_text("this is not = toml").is_err());
        assert!(clean_profile_from_text("version = 2").is_err());
    }

    #[test]
    fn counters_saturate() {
        let mut profile = Profile {
            account: AccountRecord {
                runs: u32::MAX,
                wins: u32::MAX,
                deaths: u32::MAX,
                kills: u64::MAX,
                bosses: u64::MAX,
                gold_earned: u64::MAX,
                ..default()
            },
            ..Profile::default()
        };
        profile.classes.insert(
            slug(data::CLASSES[0].name),
            ClassRecord {
                runs: u32::MAX,
                wins: u32::MAX,
                kills: u64::MAX,
                ..default()
            },
        );
        profile.zones.insert(
            slug(data::MAPS[0].name),
            ZoneRecord {
                runs: u32::MAX,
                wins: u32::MAX,
                ..default()
            },
        );
        profile.record(&RunOutcome {
            class: 0,
            map: 0,
            won: true,
            seconds: 1.0,
            kills: u32::MAX,
            bosses: u32::MAX,
            level: 1,
            damage: 1.0,
            gold: u32::MAX,
            reward_mult: 1.0,
        });
        assert_eq!(profile.account.runs, u32::MAX);
        assert_eq!(profile.account.wins, u32::MAX);
        assert_eq!(profile.account.deaths, u32::MAX);
        assert_eq!(profile.account.kills, u64::MAX);
        assert_eq!(profile.account.bosses, u64::MAX);
        assert_eq!(profile.account.gold_earned, u64::MAX);
        let class = profile.classes.get(&slug(data::CLASSES[0].name)).unwrap();
        assert_eq!(class.runs, u32::MAX);
        assert_eq!(class.wins, u32::MAX);
        assert_eq!(class.kills, u64::MAX);
        let zone = profile.zones.get(&slug(data::MAPS[0].name)).unwrap();
        assert_eq!(zone.runs, u32::MAX);
        assert_eq!(zone.wins, u32::MAX);
    }
}
