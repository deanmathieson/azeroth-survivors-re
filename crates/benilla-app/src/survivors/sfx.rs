//! The mode's sounds, on the client's own engine. Spells already ring through the spell-visual
//! kits the [`super::units::Fx`] messages play; this puts the ears on the local hero (the camera
//! hangs too far overhead for a kit's rolloff), budgets the kits ([`SpellSfxThrottle`]), weighs a
//! boss's, an elite's and a party extra's ([`SpellSfxGain`]), and sounds the run's moments with
//! the client's own `SoundEntries`: a level-up, coins, a boss arriving. A chest rings its own
//! model's open sound, or [`super::treasure`]'s cue where the model has none.
//!
//! The music is the engine's too: a run hears its battleground's zone music and ambience, picked
//! from the area under the hero (the hero drives the engine's player position), and the pick
//! screen asks for the login screen's theme ([`SoundConfig::menu_music`]).

use bevy::prelude::*;

use crate::sound::{
    ListenerAnchor, SoundConfig, SoundCue, SpellSfxGain, SpellSfxThrottle, GAIN_BOSS, GAIN_ELITE,
    GAIN_EXTRA,
};

use super::enemies::Enemy;
use super::hud::Replay;
use super::{party, HeroState, LocalHero, Phase, Run};

/// FrameXML's level-up sound (`LevelUp.wav`).
const LEVEL_UP: &str = "LEVELUPSOUND";
/// The loot window's coin (`LootCoinSmall.wav`), the client's sound for any purse change.
const COIN: &str = "LOOTWINDOWCOINSOUND";
/// The raid warning horn, for a boss walking out.
const BOSS: &str = "RaidWarning";
/// Coins chime at most this often, in seconds: a vacuumed field is one jingle, not fifty.
const COIN_GAP: f64 = 0.12;

/// This workstream's systems, registered by [`super::SurvivorsPlugin`].
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<SpellSfxThrottle>()
        .add_systems(Update, (tag_units, run_cues, menu_music));
}

/// The pick screen plays the title theme, a run its battleground's music. "Play again" and Enter
/// World pass through the menu without asking for it.
fn menu_music(
    phase: Res<State<Phase>>,
    next: Res<NextState<Phase>>,
    replay: Res<Replay>,
    mut config: ResMut<SoundConfig>,
) {
    let starting = replay.0 || matches!(*next, NextState::Pending(Phase::Playing));
    let menu = *phase.get() == Phase::Menu && !starting;
    if config.menu_music != menu {
        config.menu_music = menu;
    }
}

/// The ears on the local hero; a boss's and an elite's kits louder, a party extra's quieter.
fn tag_units(
    mut commands: Commands,
    locals: Query<Entity, Added<LocalHero>>,
    heroes: Query<(Entity, &HeroState), Added<HeroState>>,
    enemies: Query<(Entity, &Enemy), Added<Enemy>>,
) {
    for e in &locals {
        commands.entity(e).try_insert(ListenerAnchor);
    }
    for (e, hero) in &heroes {
        if hero.player != party::LOCAL_PLAYER {
            commands.entity(e).try_insert(SpellSfxGain(GAIN_EXTRA));
        }
    }
    for (e, enemy) in &enemies {
        if let Some(gain) = enemy_gain(enemy.boss, enemy.elite) {
            commands.entity(e).try_insert(SpellSfxGain(gain));
        }
    }
}

fn enemy_gain(boss: bool, elite: bool) -> Option<f32> {
    if boss {
        Some(GAIN_BOSS)
    } else if elite {
        Some(GAIN_ELITE)
    } else {
        None
    }
}

/// The run's counts the cues follow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Tally {
    level: u32,
    gold: u32,
    bosses: u32,
}

impl Tally {
    fn of(run: &Run) -> Self {
        Self {
            level: run.level,
            gold: run.gold,
            bosses: run.bosses,
        }
    }
}

/// The cues the change from `was` to `now` sounds, coins only when `coin_ok`. A count that fell
/// is a run starting over, which sounds nothing.
fn cues_for(was: Tally, now: Tally, coin_ok: bool) -> Vec<&'static str> {
    let mut out = Vec::new();
    if now.level > was.level {
        out.push(LEVEL_UP);
    }
    if now.bosses > was.bosses {
        out.push(BOSS);
    }
    if now.gold > was.gold && coin_ok {
        out.push(COIN);
    }
    out
}

/// Sound the run's moments as its counts move.
fn run_cues(
    run: Res<Run>,
    time: Res<Time<Real>>,
    mut cues: MessageWriter<SoundCue>,
    mut was: Local<Option<Tally>>,
    mut last_coin: Local<f64>,
) {
    let now = Tally::of(&run);
    let Some(before) = was.replace(now) else {
        return;
    };
    if before == now {
        return;
    }
    let t = time.elapsed_secs_f64();
    let coin_ok = t - *last_coin >= COIN_GAP;
    for kit in cues_for(before, now, coin_ok) {
        if kit == COIN {
            *last_coin = t;
        }
        cues.write(SoundCue::flat(kit));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_moment_sounds_its_cue() {
        let was = Tally {
            level: 3,
            gold: 10,
            bosses: 0,
        };
        assert!(cues_for(was, was, true).is_empty());
        let now = Tally {
            level: 4,
            gold: 15,
            bosses: 1,
        };
        assert_eq!(cues_for(was, now, true), [LEVEL_UP, BOSS, COIN]);
        assert_eq!(
            cues_for(was, now, false),
            [LEVEL_UP, BOSS],
            "coins wait their gap"
        );
    }

    #[test]
    fn a_run_starting_over_is_silent() {
        let late = Tally {
            level: 20,
            gold: 900,
            bosses: 3,
        };
        assert!(cues_for(late, Tally::default(), true).is_empty());
    }

    /// Each battleground's centre stands in an area whose `AreaTable` chain has ambience, and zone
    /// music but where 1.12 gave the zone none: Mount Hyjal (area 616), closed in 1.12, has no
    /// `ZoneMusic` row anywhere up its chain, so its runs hear only the ambience.
    #[test]
    fn every_battleground_stands_in_a_zone_with_music() {
        const NO_MUSIC_IN_1_12: &[&str] = &["Mount Hyjal"];
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).unwrap();
        let areas = benilla_formats::load_area_sound_catalog(&mut chain).unwrap();
        let mut missing = Vec::new();
        for m in super::super::data::MAPS {
            let dir = match m.map {
                0 => "Azeroth",
                1 => "Kalimdor",
                other => panic!("{} is on map {other}", m.name),
            };
            let tiles =
                benilla_formats::load_tiles_around(&mut chain, dir, m.center[0], m.center[1], 0)
                    .unwrap();
            let area = tiles
                .iter()
                .find_map(|(_, t)| benilla_formats::area_id_at(&t.chunks, m.center));
            let audio = area.and_then(|a| areas.resolve(a));
            let music = audio
                .as_ref()
                .and_then(|a| a.music)
                .map(|z| z.set_name.clone());
            let ambience = audio.as_ref().is_some_and(|a| a.ambience.is_some());
            println!(
                "{}: area {area:?}, music {music:?}, ambience {ambience}",
                m.name
            );
            if music.is_none() != NO_MUSIC_IN_1_12.contains(&m.name) || !ambience {
                missing.push(m.name);
            }
        }
        assert!(
            missing.is_empty(),
            "music or ambience not as expected: {missing:?}"
        );
    }

    #[test]
    fn bosses_outweigh_elites_and_the_horde_has_no_gain() {
        assert_eq!(enemy_gain(true, false), Some(GAIN_BOSS));
        assert_eq!(enemy_gain(false, true), Some(GAIN_ELITE));
        assert_eq!(enemy_gain(false, false), None);
    }
}
