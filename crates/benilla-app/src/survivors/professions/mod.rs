//! Professions and the WoW inventory (docs/PROFESSIONS.md). P0, the spike: the stock 1.12 bags
//! and item tooltip, loaded as a curated FrameXML ([`crate::ui_script`]'s `survivors_ui`), open
//! over the survivors HUD on B, mirroring a local [`inventory::Bags`].
//!
//! - **Opt-in:** this plugin inserts [`crate::run_mode::SurvivorsUi`] beside `CaptureMode`, so the
//!   world entry loads the curated interface; the stock net drains stand down under it.
//! - **Kill switch:** `WOW_SURVIVORS_FRAMEXML=0` (also `off`, `false`, `no`) leaves the whole
//!   plugin out: no FrameXML, no bags, as before P0.
//! - **Test aid:** `WOW_SURVIVORS_OPEN=bags` opens every bag and hovers the first item once a run
//!   is playing; with `WOW_SURVIVORS_SHOTS=<dir>` it also saves `<dir>/bags.png`.
//!
//! Until P1's profile carries an inventory, the bags hold the P0 test inventory
//! ([`items::TEST_INVENTORY`]) in memory only; nothing is saved.

pub(crate) mod frames;
pub(crate) mod inventory;
pub(crate) mod items;

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

pub(super) fn plugin(app: &mut App) {
    if !framexml_enabled() {
        info!("survivors: WOW_SURVIVORS_FRAMEXML is off — no FrameXML, no bags");
        return;
    }
    let open_bags = crate::webenv::var("WOW_SURVIVORS_OPEN")
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("bags"));
    // Present before `OnEnter(InWorld)`, where the UI lifecycle decides what to load.
    app.insert_resource(crate::run_mode::SurvivorsUi)
        .insert_resource(inventory::Bags::test_inventory())
        .insert_resource(frames::OpenBagsAid {
            done: !open_bags,
            shot_dir: crate::webenv::var("WOW_SURVIVORS_SHOTS")
                .filter(|_| open_bags)
                .map(std::path::PathBuf::from),
            shot_t: None,
        })
        // The keys, after the UI key feed (the keyboard capture) and the binding dispatch, and
        // before every survivors key system in `WorldStage::Input` and after it.
        .add_systems(
            Update,
            frames::bag_keys
                .in_set(UiInput)
                .after(crate::bindings::BindingSet)
                .run_if(ingame_ui_up),
        )
        // The gestures' intents, drained after the input pass that queued them, and the push,
        // before this frame's paint.
        .add_systems(
            Update,
            (frames::sync_bags, frames::open_bags_aid)
                .chain()
                .after(UiInput)
                .before(UiPaint)
                .run_if(ingame_ui_up),
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

        /// One frame's drain and push.
        pub(crate) fn sync(&mut self, script: &mut UiScript) {
            sync(script, &mut self.bags, &mut self.feed);
        }

        /// Press `key` through the survivors key handler, a run playing; whether it was spent.
        fn press(&mut self, script: &mut UiScript, key: KeyCode) -> bool {
            let mut keys = ButtonInput::<KeyCode>::default();
            keys.press(key);
            handle_keys(script, &mut keys, &UiKeyboardCapture::default(), true);
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
}

#[cfg(test)]
mod tests {
    use super::framexml_enabled_by;

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
