//! The zone unlock chain and each zone's standing on the account. The zones of [`data::MAPS`] fall
//! into four [`Tier`]s: the starting zones are always open, and a win in a tier opens the next.
//! [`Unlocks`] holds every zone's standing, rebuilt from the [`Profile`] whenever it changes; the
//! menu's zone list reads it for its locks, its difficulty pips and its zone card.
//!
//! Test aid: `WOW_SURVIVORS_UNLOCK_ALL=1` opens every zone.

use bevy::prelude::*;

use super::data;
use super::profile::{slug, Profile, RunEnded};
use super::Selection;

/// A step of the unlock chain, easiest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Tier {
    Starting,
    Frontier,
    Elite,
    Raid,
}

impl Tier {
    /// The tier's name on the zone card: "Starting zone".
    pub fn label(self) -> &'static str {
        match self {
            Tier::Starting => "Starting zone",
            Tier::Frontier => "Frontier zone",
            Tier::Elite => "Elite zone",
            Tier::Raid => "Raid zone",
        }
    }

    /// The tier whose win opens this one; none for the starting zones.
    pub fn before(self) -> Option<Tier> {
        match self {
            Tier::Starting => None,
            Tier::Frontier => Some(Tier::Starting),
            Tier::Elite => Some(Tier::Frontier),
            Tier::Raid => Some(Tier::Elite),
        }
    }

    /// What opens a locked zone of this tier: "Locked · win a starting zone".
    pub fn lock_line(self) -> String {
        let need = match self.before() {
            None | Some(Tier::Starting) => "a starting",
            Some(Tier::Frontier) => "a frontier",
            Some(Tier::Elite) => "an elite",
            Some(Tier::Raid) => "a raid",
        };
        format!("Locked · win {need} zone")
    }
}

/// Every zone's tier, by its [`data::MapDef::name`].
const TIERS: &[(&str, Tier)] = &[
    ("Elwynn Forest", Tier::Starting),
    ("Teldrassil", Tier::Starting),
    ("Mulgore", Tier::Starting),
    ("Durotar", Tier::Starting),
    ("Dun Morogh", Tier::Starting),
    ("Tirisfal Glades", Tier::Starting),
    ("The Barrens", Tier::Frontier),
    ("Westfall", Tier::Frontier),
    ("Redridge Mountains", Tier::Frontier),
    ("Duskwood", Tier::Frontier),
    ("Gurubashi Arena", Tier::Elite),
    ("The Dark Portal", Tier::Elite),
    ("Fire Plume Ridge", Tier::Elite),
    ("Mount Hyjal", Tier::Raid),
    ("Naxxramas", Tier::Raid),
    ("Gates of Ahn'Qiraj", Tier::Raid),
    ("Molten Core: Raid Start", Tier::Raid),
    ("Molten Core: Lucifron's Cavern", Tier::Raid),
    ("Molten Core: Magmadar's Lair", Tier::Raid),
    ("Molten Core: South-centre Cavern", Tier::Raid),
    ("Molten Core: West Cavern", Tier::Raid),
    ("Molten Core: North-west Cavern", Tier::Raid),
    ("Molten Core: Centre Cavern", Tier::Raid),
    ("Molten Core: Majordomo's Camp", Tier::Raid),
    ("Molten Core: Ragnaros's Dry Ledge", Tier::Raid),
    ("Molten Core: Ragnaros' Lair", Tier::Raid),
    ("Molten Core: Gehennas' Cavern", Tier::Raid),
    ("Molten Core: Garr's Cavern", Tier::Raid),
    ("Molten Core: Shazzrah's Cavern", Tier::Raid),
    ("Molten Core: Baron Geddon's Cavern", Tier::Raid),
    ("Molten Core: Sulfuron's Cavern", Tier::Raid),
    ("Molten Core: Golemagg's Ridge", Tier::Raid),
];

fn tier_of(name: &str) -> Option<Tier> {
    TIERS.iter().find(|(n, _)| *n == name).map(|&(_, t)| t)
}

/// A zone's tier. One missing from [`TIERS`] counts as starting, so it is never out of reach.
pub(super) fn map_tier(map: usize) -> Tier {
    data::MAPS
        .get(map)
        .and_then(|m| tier_of(m.name))
        .unwrap_or(Tier::Starting)
}

/// The difficulty pips on a zone row, after its won marker.
pub(super) const PIPS: u8 = 8;
/// The rewards multiplier one pip stands for: eight span ×1.00 to the ×2.00 Run Options cap.
const PIP_STEP: f32 = 0.125;

/// Lit pips for a best winning multiplier: one per [`PIP_STEP`] above ×1.00, rounded, at most
/// [`PIPS`].
pub(super) fn pips(best_win_mult: f32) -> u8 {
    if !best_win_mult.is_finite() {
        return 0;
    }
    ((best_win_mult - 1.0) / PIP_STEP)
        .round()
        .clamp(0.0, f32::from(PIPS)) as u8
}

/// One zone's standing on the account.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Standing {
    pub open: bool,
    pub wins: u32,
    /// The highest rewards multiplier the zone was won under; 0 before a win.
    pub best_win_mult: f32,
}

impl Standing {
    /// The zone card's line for the best win, once there is one: "Best win: ×1.35 rewards". A
    /// win kept from before the multiplier was recorded reads as ×1.00.
    pub fn best_win(&self) -> Option<String> {
        (self.wins > 0).then(|| format!("Best win: ×{:.2} rewards", self.best_win_mult.max(1.0)))
    }
}

/// Every zone's [`Standing`], indexed like [`data::MAPS`].
#[derive(Resource, Clone, Debug, PartialEq)]
pub(super) struct Unlocks {
    /// `WOW_SURVIVORS_UNLOCK_ALL=1`: every zone open, whatever the record.
    all: bool,
    zones: Vec<Standing>,
}

impl Default for Unlocks {
    fn default() -> Self {
        Self::from_profile(&Profile::default(), unlock_all_env())
    }
}

fn unlock_all_env() -> bool {
    crate::webenv::var("WOW_SURVIVORS_UNLOCK_ALL").is_some_and(|v| v.trim() == "1")
}

impl Unlocks {
    /// The standings a profile's zone records give. A tier opens once a zone of the tier before
    /// it is won; a win in a later tier also opens the tiers below it, so wins kept from before
    /// the chain never leave a gap.
    pub fn from_profile(profile: &Profile, all: bool) -> Self {
        let records: Vec<_> = data::MAPS
            .iter()
            .map(|m| {
                profile
                    .zones
                    .get(&slug(m.name))
                    .cloned()
                    .unwrap_or_default()
            })
            .collect();
        let furthest = (0..records.len())
            .filter(|&i| records[i].wins > 0)
            .map(map_tier)
            .max();
        let zones = records
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let opener = map_tier(i).before();
                Standing {
                    open: all || opener.is_none_or(|t| furthest.is_some_and(|f| f >= t)),
                    wins: r.wins,
                    best_win_mult: r.best_win_mult,
                }
            })
            .collect();
        Self { all, zones }
    }

    pub fn standing(&self, map: usize) -> Standing {
        self.zones.get(map).copied().unwrap_or_default()
    }

    pub fn is_open(&self, map: usize) -> bool {
        self.standing(map).open
    }

    /// `map` if it is open, else the first open zone in the menu's order.
    pub fn fallback(&self, map: usize) -> usize {
        if self.is_open(map) {
            return map;
        }
        list_order()
            .into_iter()
            .find(|&i| self.is_open(i))
            .unwrap_or(map)
    }
}

/// Moves a pick off a zone not yet open, to [`Unlocks::fallback`]; the world goes with it, as it
/// does for the pick restored at boot.
pub(super) fn open_pick(selection: &mut Selection, unlocks: &Unlocks) {
    let open = unlocks.fallback(selection.map);
    if open != selection.map {
        selection.map = open;
        selection.stage = open;
        selection.version += 1;
    }
}

/// A zone's level band, `(lowest, highest)` of its horde.
pub(super) fn level_band(map: usize) -> (u32, u32) {
    let levels = data::roster(map).enemies.iter().map(|e| e.level);
    let lo = levels.clone().min().unwrap_or(1);
    let hi = levels.max().unwrap_or(lo);
    (lo, hi)
}

/// The zone list's order: lowest level first, then by name.
pub(super) fn list_order() -> Vec<usize> {
    let mut order: Vec<usize> = (0..data::MAPS.len()).collect();
    order.sort_by_key(|&i| (level_band(i).0, data::MAPS[i].name));
    order
}

/// One boss on a zone's schedule.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BossTime {
    /// Run seconds it walks out at.
    pub at: f32,
    pub name: &'static str,
    /// The last boss before the bell.
    pub last: bool,
}

/// A zone's bosses in the order they come: the `i`th at `(i + 1)` × [`data::BOSS_EVERY_SECS`], as
/// the spawner sends them, and only those due before [`data::VICTORY_SECS`].
pub(super) fn boss_schedule(map: usize) -> Vec<BossTime> {
    let mut out: Vec<BossTime> = data::roster(map)
        .bosses
        .iter()
        .enumerate()
        .map(|(i, b)| BossTime {
            at: (i + 1) as f32 * data::BOSS_EVERY_SECS,
            name: b.name,
            last: false,
        })
        .take_while(|b| b.at < data::VICTORY_SECS)
        .collect();
    if let Some(last) = out.last_mut() {
        last.last = true;
    }
    out
}

/// Run seconds as `m:ss`.
pub(super) fn clock(secs: f32) -> String {
    let s = secs.max(0.0).round() as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

/// A zone's horde by name, weakest first, each once.
pub(super) fn foe_names(map: usize) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();
    for e in &data::roster(map).enemies {
        if !names.contains(&e.name) {
            names.push(e.name);
        }
    }
    names
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Unlocks>().add_systems(
        Update,
        (
            record_best_win,
            refresh_unlocks.run_if(resource_exists_and_changed::<Profile>),
        )
            .chain()
            .before(super::hud::keys),
    );
}

/// A won run raises its zone's best winning multiplier; a loss changes nothing.
fn record_best_win(mut ended: MessageReader<RunEnded>, mut profile: ResMut<Profile>) {
    for RunEnded { outcome } in ended.read() {
        let mult = outcome.reward_mult;
        if !outcome.won || !mult.is_finite() {
            continue;
        }
        let Some(map) = data::MAPS.get(outcome.map) else {
            continue;
        };
        let zone = profile.zones.entry(slug(map.name)).or_default();
        if mult > zone.best_win_mult {
            zone.best_win_mult = mult;
            profile.mark_dirty();
        }
    }
}

fn refresh_unlocks(profile: Res<Profile>, mut unlocks: ResMut<Unlocks>) {
    let all = unlocks.all;
    unlocks.set_if_neq(Unlocks::from_profile(&profile, all));
}

#[cfg(test)]
mod tests {
    use super::super::profile::{RunOutcome, ZoneRecord};
    use super::*;

    fn index(name: &str) -> usize {
        data::MAPS.iter().position(|m| m.name == name).unwrap()
    }

    fn won(names: &[&str]) -> Profile {
        let mut profile = Profile::default();
        for name in names {
            profile.zones.insert(
                slug(name),
                ZoneRecord {
                    runs: 1,
                    wins: 1,
                    ..Default::default()
                },
            );
        }
        profile
    }

    /// The tiers open under `profile`; a tier is open or locked as a whole.
    fn open_tiers(profile: &Profile) -> Vec<Tier> {
        let unlocks = Unlocks::from_profile(profile, false);
        let mut open: Vec<Tier> = Vec::new();
        for t in [Tier::Starting, Tier::Frontier, Tier::Elite, Tier::Raid] {
            let zones: Vec<usize> = (0..data::MAPS.len())
                .filter(|&i| map_tier(i) == t)
                .collect();
            let n = zones.iter().filter(|&&i| unlocks.is_open(i)).count();
            assert!(n == 0 || n == zones.len(), "{t:?} is half open");
            if n > 0 {
                open.push(t);
            }
        }
        open
    }

    fn outcome(map: usize, won: bool, reward_mult: f32) -> RunOutcome {
        RunOutcome {
            class: 0,
            map,
            won,
            seconds: 900.0,
            kills: 1,
            bosses: 6,
            level: 20,
            damage: 1.0,
            gold: 0,
            reward_mult,
            quests: 0,
            final_boss: false,
        }
    }

    #[test]
    fn every_zone_has_a_tier() {
        for map in data::MAPS {
            assert!(tier_of(map.name).is_some(), "{} has no tier", map.name);
        }
        for (name, _) in TIERS {
            assert!(
                data::MAPS.iter().any(|m| m.name == *name),
                "{name} is no zone"
            );
        }
        assert_eq!(TIERS.len(), data::MAPS.len());
        let count = |t| (0..data::MAPS.len()).filter(|&i| map_tier(i) == t).count();
        assert_eq!(
            [Tier::Starting, Tier::Frontier, Tier::Elite, Tier::Raid].map(count),
            [6, 4, 3, 19]
        );
    }

    #[test]
    fn a_win_opens_the_next_tier() {
        assert_eq!(open_tiers(&Profile::default()), [Tier::Starting]);
        // A loss opens nothing.
        let mut lost = Profile::default();
        lost.zones.insert(
            slug("Durotar"),
            ZoneRecord {
                runs: 3,
                ..Default::default()
            },
        );
        assert_eq!(open_tiers(&lost), [Tier::Starting]);
        assert_eq!(
            open_tiers(&won(&["Durotar"])),
            [Tier::Starting, Tier::Frontier]
        );
        assert_eq!(
            open_tiers(&won(&["Durotar", "Westfall"])),
            [Tier::Starting, Tier::Frontier, Tier::Elite]
        );
        assert_eq!(
            open_tiers(&won(&["Durotar", "Westfall", "The Dark Portal"])),
            [Tier::Starting, Tier::Frontier, Tier::Elite, Tier::Raid]
        );
        // A frontier win kept from before the chain still opens its own tier and the next.
        assert_eq!(
            open_tiers(&won(&["The Barrens"])),
            [Tier::Starting, Tier::Frontier, Tier::Elite]
        );
        let all = Unlocks::from_profile(&Profile::default(), true);
        assert!((0..data::MAPS.len()).all(|i| all.is_open(i)));
        assert!(!all.is_open(data::MAPS.len()));
    }

    #[test]
    fn lock_lines_name_the_tier_before() {
        assert_eq!(Tier::Frontier.lock_line(), "Locked · win a starting zone");
        assert_eq!(Tier::Elite.lock_line(), "Locked · win a frontier zone");
        assert_eq!(Tier::Raid.lock_line(), "Locked · win an elite zone");
    }

    #[test]
    fn a_locked_pick_falls_back_to_the_first_open_zone() {
        let fresh = Unlocks::from_profile(&Profile::default(), false);
        let first = list_order()[0];
        assert_eq!(map_tier(first), Tier::Starting);
        assert_eq!(fresh.fallback(index("Naxxramas")), first);
        assert_eq!(fresh.fallback(index("Mulgore")), index("Mulgore"));

        // The Barrens, the default pick, is no starting zone.
        let mut selection = Selection {
            map: index("The Barrens"),
            stage: index("The Barrens"),
            ..Default::default()
        };
        open_pick(&mut selection, &fresh);
        assert_eq!((selection.map, selection.stage), (first, first));
        assert_eq!(selection.version, 1);
        // An open pick stays put.
        let opened = Unlocks::from_profile(&won(&["Teldrassil"]), false);
        selection.map = index("Duskwood");
        open_pick(&mut selection, &opened);
        assert_eq!((selection.map, selection.version), (index("Duskwood"), 1));
    }

    #[test]
    fn pips_count_eighths_of_the_multiplier_above_one() {
        assert_eq!(pips(0.0), 0);
        assert_eq!(pips(1.0), 0);
        assert_eq!(pips(1.125), 1);
        assert_eq!(pips(1.35), 3);
        assert_eq!(pips(1.5), 4);
        assert_eq!(pips(2.0), PIPS);
        assert_eq!(pips(3.0), PIPS);
        assert_eq!(pips(f32::NAN), 0);
        let never = Standing::default();
        assert_eq!(never.best_win(), None);
        let won = Standing {
            open: true,
            wins: 1,
            best_win_mult: 1.35,
        };
        assert_eq!(won.best_win().as_deref(), Some("Best win: ×1.35 rewards"));
        // A win from before the multiplier was kept.
        let old = Standing {
            best_win_mult: 0.0,
            ..won
        };
        assert_eq!(old.best_win().as_deref(), Some("Best win: ×1.00 rewards"));
    }

    #[test]
    fn best_win_mult_rises_only_on_wins_and_each_run_counts_once() {
        let mut world = World::new();
        world.init_resource::<Messages<RunEnded>>();
        world.insert_resource(Profile::default());
        let mut schedule = Schedule::default();
        schedule.add_systems(record_best_win);
        let map = index("Elwynn Forest");
        let best = |world: &World| {
            world
                .resource::<Profile>()
                .zones
                .get(&slug("Elwynn Forest"))
                .map_or(0.0, |z| z.best_win_mult)
        };
        let mut play = |world: &mut World, won: bool, mult: f32| {
            world.resource_mut::<Messages<RunEnded>>().write(RunEnded {
                outcome: outcome(map, won, mult),
            });
            schedule.run(world);
            best(world)
        };
        assert_eq!(play(&mut world, false, 1.9), 0.0);
        assert_eq!(play(&mut world, true, 1.35), 1.35);
        assert_eq!(play(&mut world, true, 1.2), 1.35);
        assert_eq!(play(&mut world, false, 2.0), 1.35);
        assert_eq!(play(&mut world, true, 1.5), 1.5);
        // Nothing new: a run already read is not read again.
        world
            .resource_mut::<Profile>()
            .zones
            .get_mut(&slug("Elwynn Forest"))
            .unwrap()
            .best_win_mult = 0.0;
        schedule.run(&mut world);
        assert_eq!(best(&world), 0.0);
        // The other records are the profile's own to keep.
        assert_eq!(
            world.resource::<Profile>().zones[&slug("Elwynn Forest")].wins,
            0
        );
    }

    #[test]
    fn unlocks_follow_the_profile() {
        let mut app = App::new();
        app.init_resource::<Messages<RunEnded>>()
            .insert_resource(Profile::default())
            .insert_resource(Unlocks::from_profile(&Profile::default(), false))
            .add_systems(Update, (record_best_win, refresh_unlocks).chain());
        app.update();
        let westfall = index("Westfall");
        assert!(!app.world().resource::<Unlocks>().is_open(westfall));
        // A win recorded at the results opens the next tier and lights the zone's pips.
        let elwynn = index("Elwynn Forest");
        app.world_mut()
            .resource_mut::<Profile>()
            .record(&outcome(elwynn, true, 1.5));
        app.world_mut()
            .resource_mut::<Messages<RunEnded>>()
            .write(RunEnded {
                outcome: outcome(elwynn, true, 1.5),
            });
        app.update();
        let unlocks = app.world().resource::<Unlocks>();
        assert!(unlocks.is_open(westfall));
        assert!(!unlocks.is_open(index("Gurubashi Arena")));
        assert_eq!(pips(unlocks.standing(elwynn).best_win_mult), 4);
        assert_eq!(unlocks.standing(elwynn).wins, 1);
    }

    #[test]
    fn old_zone_records_load_without_a_best_win() {
        let profile: Profile =
            toml::from_str("[zones.elwynn_forest]\nruns = 2\nwins = 1\nbest_seconds = 900.0\n")
                .unwrap();
        let zone = &profile.zones["elwynn_forest"];
        assert_eq!((zone.wins, zone.best_win_mult), (1, 0.0));
        let unlocks = Unlocks::from_profile(&profile, false);
        assert!(unlocks.is_open(index("The Barrens")));
    }

    #[test]
    fn bosses_come_every_two_and_a_quarter_minutes() {
        for map in 0..data::MAPS.len() {
            let schedule = boss_schedule(map);
            let roster = &data::roster(map).bosses;
            assert_eq!(schedule.len(), roster.len(), "{}", data::MAPS[map].name);
            for (i, b) in schedule.iter().enumerate() {
                assert_eq!(b.name, roster[i].name);
                assert_eq!(b.last, i + 1 == roster.len());
            }
            let times: Vec<String> = schedule.iter().map(|b| clock(b.at)).collect();
            assert_eq!(times, ["2:15", "4:30", "6:45", "9:00", "11:15", "13:30"]);
        }
        let elwynn = boss_schedule(index("Elwynn Forest"));
        assert_eq!(elwynn.last().unwrap().name, "Gruff Swiftbite");
        assert_eq!(clock(0.0), "0:00");
        assert_eq!(clock(59.6), "1:00");
        assert_eq!(clock(605.0), "10:05");
    }

    #[test]
    fn foes_are_named_once_weakest_first() {
        let durotar = foe_names(index("Durotar"));
        assert_eq!(durotar[0], "Vile Familiar");
        assert_eq!(durotar.len(), 16);
        for map in 0..data::MAPS.len() {
            let names = foe_names(map);
            assert!(!names.is_empty());
            for (i, n) in names.iter().enumerate() {
                assert!(!names[i + 1..].contains(n), "{n} twice");
            }
        }
    }

    #[test]
    fn the_list_runs_lowest_level_first() {
        let order = list_order();
        assert_eq!(order.len(), data::MAPS.len());
        for pair in order.windows(2) {
            assert!(level_band(pair[0]).0 <= level_band(pair[1]).0);
        }
        // The tiers climb with the list.
        for pair in order.windows(2) {
            assert!(map_tier(pair[0]) <= map_tier(pair[1]));
        }
    }
}
