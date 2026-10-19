//! Professions and the WoW inventory (docs/PROFESSIONS.md).
//!
//! - **P0:** the stock 1.12 bags and item tooltip, loaded as a curated FrameXML
//!   ([`crate::ui_script`]'s `survivors_ui`), open over the survivors HUD on B, mirroring a local
//!   [`inventory::Bags`].
//! - **P1:** the bags and the four gathering skills persist in the profile (version 5,
//!   [`save`]); veins, herbs and fishing pools spawn in runs and are gathered while fighting, each
//!   hero's yield its own ([`gather`]); beasts are skinned; the camp's bobber lands fish; the
//!   potion is drunk from the bags ([`effects`]); K shows the skills.
//! - **P1, the economy:** each run banks a share of the party's gold into the profile's purse
//!   ([`econ`]); at camp the goblin's stock `MerchantFrame` buys and sells, the trainer's
//!   `ClassTrainerFrame` sells the gathering ranks, and Smelting's `TradeSkillFrame` turns ore into
//!   bars ([`stations`], [`recipes`]); Find Minerals and Find Herbs mark the minimap
//!   ([`tracking`]).
//!
//! - **Opt-in:** this plugin inserts [`crate::run_mode::SurvivorsUi`] beside `CaptureMode`, so the
//!   world entry loads the curated interface; the stock net drains stand down under it.
//! - **Kill switch:** `WOW_SURVIVORS_FRAMEXML=0` (also `off`, `false`, `no`) leaves the whole
//!   plugin out: no FrameXML, no bags, no gathering, as before P0.
//! - **Test aids:** `WOW_SURVIVORS_BAGS=test` fills the bags with [`items::TEST_INVENTORY`] in
//!   memory only, and saves neither the bags nor the skills. `WOW_SURVIVORS_OPEN=bags` opens every
//!   bag and hovers the first item once a run is playing; with `WOW_SURVIVORS_SHOTS=<dir>` it also
//!   saves `<dir>/bags.png`.

pub(crate) mod econ;
pub(crate) mod effects;
pub(crate) mod frames;
pub(crate) mod gather;
pub(crate) mod inventory;
pub(crate) mod items;
pub(crate) mod nodes;
pub(crate) mod recipes;
pub(crate) mod save;
pub(crate) mod skills;
pub(crate) mod stations;
pub(crate) mod tracking;

use benilla_world::schedule::WorldStage;
use bevy::prelude::*;

use crate::ui_script::{ingame_ui_up, UiInput, UiPaint};

/// Whether the FrameXML opt-in is on: anything but `0`, `off`, `false` or `no` in
/// `WOW_SURVIVORS_FRAMEXML`, or the variable unset.
pub(crate) fn framexml_enabled() -> bool {
    framexml_enabled_by(crate::webenv::var("WOW_SURVIVORS_FRAMEXML").as_deref())
}

/// [`framexml_enabled`] over the variable's value.
pub(crate) fn framexml_enabled_by(value: Option<&str>) -> bool {
    !value.is_some_and(|v| {
        let v = v.trim();
        ["0", "off", "false", "no"]
            .iter()
            .any(|off| v.eq_ignore_ascii_case(off))
    })
}

/// A run's world is up and not behind a loading screen.
fn world_shown(loading: Option<Res<crate::loading_screen::LoadingScreen>>) -> bool {
    !loading.is_some_and(|l| l.covering())
}

pub(super) fn plugin(app: &mut App) {
    if !framexml_enabled() {
        info!("survivors: WOW_SURVIVORS_FRAMEXML is off — no FrameXML, no bags");
        return;
    }
    let open_bags = crate::webenv::var("WOW_SURVIVORS_OPEN")
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("bags"));
    let shadow = crate::webenv::var("WOW_SURVIVORS_BAGS")
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("test"));
    // Present before `OnEnter(InWorld)`, where the UI lifecycle decides what to load.
    app.insert_resource(crate::run_mode::SurvivorsUi)
        .insert_resource(save::Shadow(shadow))
        .insert_resource(if shadow {
            inventory::Bags::test_inventory()
        } else {
            inventory::Bags::default()
        })
        .init_resource::<skills::Skills>()
        .init_resource::<gather::YieldInbox>()
        .init_resource::<gather::GatherOutbox>()
        .init_resource::<gather::RemoteYields>()
        .init_resource::<gather::NodeSpawner>()
        .init_resource::<gather::Gathering>()
        .init_resource::<gather::UiLines>()
        .init_resource::<effects::UseOutbox>()
        .init_resource::<econ::Purse>()
        .init_resource::<econ::GoldTally>()
        .init_resource::<econ::Shop>()
        .init_resource::<stations::CampDesk>()
        .init_resource::<stations::Smelter>()
        .init_resource::<tracking::Tracking>()
        .insert_resource(frames::OpenBagsAid {
            done: !open_bags,
            shot_dir: crate::webenv::var("WOW_SURVIVORS_SHOTS")
                .filter(|_| open_bags)
                .map(std::path::PathBuf::from),
            shot_t: None,
        })
        // The profile is in by then (`profile::load_profile`, `Startup`).
        .add_systems(PostStartup, save::load_saved)
        // The run purse watched, a recorded run banked, then whatever changed saved.
        .add_systems(
            Update,
            (econ::tally_gold, econ::bank_run_gold, save::save_changes).chain(),
        )
        // The keys, after the UI key feed (the keyboard capture) and the binding dispatch, and
        // before every survivors key system in `WorldStage::Input` and after it.
        .add_systems(
            Update,
            frames::bag_keys
                .in_set(UiInput)
                .after(crate::bindings::BindingSet)
                .run_if(ingame_ui_up),
        )
        // The gestures' intents, drained after the input pass that queued them (the camp windows'
        // first, so a right-click on a bag sells while the goblin's window is open, and a
        // right-click's use ahead of the stock script-call applier), and the push, before this
        // frame's paint.
        .add_systems(
            Update,
            (
                stations::run_stations,
                effects::use_items,
                frames::sync_bags,
                frames::open_bags_aid,
            )
                .chain()
                .after(UiInput)
                .before(UiPaint)
                .before(crate::script_calls::apply_script_calls)
                .run_if(ingame_ui_up),
        )
        // Gathering: the run's nodes where it is simulated, every machine's own gathers, the
        // yields into the bags, and what the stock UI is told of it all.
        .add_systems(
            Update,
            (
                (gather::spawn_nodes, gather::skin_kills)
                    .run_if(super::party::simulates)
                    .run_if(super::strip::world_runs)
                    .run_if(world_shown),
                gather::gather
                    .run_if(super::strip::world_runs)
                    .run_if(world_shown),
                gather::camp_catch,
                gather::take_yields,
                gather::show_lines.run_if(ingame_ui_up),
            )
                .chain()
                .after(WorldStage::Input)
                .after(super::enemies::drop_loot),
        )
        // A click on a bag is the bag's, never the HUD's under it.
        .add_systems(
            PreUpdate,
            frames::bags_take_hud_pointer.after(bevy::ui::UiSystems::Focus),
        )
        // A modal survivors screen closes the stock windows as it opens.
        .add_systems(OnExit(super::Phase::Playing), frames::close_windows);
}

/// The bags and the feed as a test drives them, without an app.
#[cfg(test)]
pub(crate) mod test_support {
    use bevy::prelude::*;

    use benilla_ui::script::UiScript;

    use super::frames::{handle_keys, sync, BagFeed};
    use super::inventory::Bags;
    use crate::ui_script::UiKeyboardCapture;

    /// The test inventory and its feed memory.
    pub(crate) struct Fixture {
        pub(crate) bags: Bags,
        feed: BagFeed,
    }

    impl Fixture {
        pub(crate) fn new() -> Self {
            Self {
                bags: Bags::test_inventory(),
                feed: BagFeed::default(),
            }
        }

        /// One frame's drain and push, with an empty purse.
        pub(crate) fn sync(&mut self, script: &mut UiScript) {
            sync(script, &mut self.bags, 0, &mut self.feed);
        }

        /// Press `key` through the survivors key handler, a run playing; whether it was spent.
        fn press(&mut self, script: &mut UiScript, key: KeyCode) -> bool {
            let mut keys = ButtonInput::<KeyCode>::default();
            keys.press(key);
            handle_keys(
                script,
                &mut keys,
                &UiKeyboardCapture::default(),
                true,
                &super::skills::Skills::default(),
            );
            !keys.just_pressed(key)
        }

        pub(crate) fn press_b(&mut self, script: &mut UiScript) {
            assert!(self.press(script, KeyCode::KeyB), "B is the bags'");
        }

        /// Esc: whether a stock window took it.
        pub(crate) fn escape(&mut self, script: &mut UiScript) -> bool {
            self.press(script, KeyCode::Escape)
        }
    }

    /// Esc through the survivors key handler with no bags of its own: whether a stock window took
    /// it.
    pub(crate) fn escape_closes(script: &mut UiScript) -> bool {
        Fixture::new().escape(script)
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::framexml_enabled_by;

    /// The whole client with survivors on top, its schedules built: a system-order cycle or a
    /// conflicting query in one system (Bevy's B0001) fails here, not at a player's launch.
    #[test]
    fn survivors_professions_the_survivors_schedules_build() {
        use crate::local_state::test_env::{EnvGuard, ENV_LOCK};
        let _lock = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _on = EnvGuard::unset("WOW_SURVIVORS_FRAMEXML");
        // `game_plugins`' headless client, with survivors added before it finishes.
        let mut app = App::new();
        app.add_plugins(
            benilla_world::boot::tuned_default_plugins(bevy::window::Window::default())
                .disable::<bevy::winit::WinitPlugin>()
                .disable::<bevy::log::LogPlugin>()
                .set(bevy::window::WindowPlugin {
                    primary_window: None,
                    exit_condition: bevy::window::ExitCondition::DontExit,
                    ..default()
                })
                .set(bevy::render::RenderPlugin {
                    render_creation: bevy::render::settings::WgpuSettings {
                        backends: None,
                        ..default()
                    }
                    .into(),
                    ..default()
                }),
        );
        app.add_plugins(benilla_world::world_plugins::WorldPlugins);
        app.add_plugins(crate::game_plugins::GamePlugins {
            connect: false,
            start: crate::char_select::ClientState::InWorld,
        });
        app.add_plugins(crate::survivors::SurvivorsPlugin);
        app.finish();
        app.cleanup();
        let labels: Vec<_> = app
            .world()
            .resource::<Schedules>()
            .iter()
            .map(|(_, schedule)| schedule.label())
            .collect();
        for label in labels {
            app.world_mut().schedule_scope(label, |world, schedule| {
                if let Err(e) = schedule.initialize(world) {
                    panic!("{label:?} does not build: {e}");
                }
            });
        }
    }

    #[test]
    fn survivors_framexml_kill_switch() {
        assert!(framexml_enabled_by(None), "on by default");
        assert!(framexml_enabled_by(Some("1")));
        for off in ["0", "off", "OFF", " false ", "no"] {
            assert!(!framexml_enabled_by(Some(off)), "{off:?} turns it off");
        }
    }

    /// The kill switch leaves the plugin out: no opt-in marker, so the world entry loads no UI,
    /// and no bags.
    #[test]
    fn survivors_framexml_kill_switch_leaves_the_opt_in_out() {
        use crate::local_state::test_env::{EnvGuard, ENV_LOCK};
        let _lock = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let build = || {
            let mut app = bevy::app::App::new();
            super::plugin(&mut app);
            app
        };
        {
            let _off = EnvGuard::set("WOW_SURVIVORS_FRAMEXML", "0");
            let app = build();
            assert!(!app
                .world()
                .contains_resource::<crate::run_mode::SurvivorsUi>());
            assert!(!app.world().contains_resource::<super::inventory::Bags>());
        }
        let _on = EnvGuard::unset("WOW_SURVIVORS_FRAMEXML");
        let app = build();
        assert!(app
            .world()
            .contains_resource::<crate::run_mode::SurvivorsUi>());
        assert!(app.world().contains_resource::<super::inventory::Bags>());
    }
}
