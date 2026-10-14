//! Which view a run is played in: WoW's own third-person camera and controls ([`super::wowcam`],
//! the default), or the classic eased top-down view with world-relative WASD and the hero facing
//! the cursor. `V` swaps them, and the choice is kept across launches in
//! `benilla-config/survivors-view.toml`. The menu keeps its slow orbit either way.

use std::path::PathBuf;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::Phase;

/// The swap key. `C` stays free for the character pane.
pub(super) const KEY: KeyCode = KeyCode::KeyV;

/// The two views.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum ViewMode {
    /// WoW's camera behind the hero, mouselook, and 1.12's movement keys.
    #[default]
    Wow,
    /// The eased top-down view.
    Classic,
}

impl ViewMode {
    pub(super) fn toggled(self) -> Self {
        match self {
            Self::Wow => Self::Classic,
            Self::Classic => Self::Wow,
        }
    }
}

/// The view picked, loaded at startup.
#[derive(Resource, Default)]
pub(super) struct View {
    pub(super) mode: ViewMode,
}

/// Whether WoW's camera and controls drive this frame: in a run (every screen of it but the
/// menu's), in the WoW view, and not under the auto runner, which steers world-relative.
pub(super) fn wow_in_play(phase: Phase, mode: ViewMode, auto: bool) -> bool {
    phase != Phase::Menu && mode == ViewMode::Wow && !auto
}

/// The settings file's shape.
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct ViewFile {
    view: ViewMode,
}

fn to_toml(mode: ViewMode) -> String {
    format!(
        "# Warcraft Survivors view (V swaps it): \"wow\" is WoW's camera and controls, \"classic\" \
         the top-down view.\n{}",
        toml::to_string(&ViewFile { view: mode }).unwrap_or_default()
    )
}

fn from_toml(text: &str) -> Result<ViewMode, toml::de::Error> {
    toml::from_str::<ViewFile>(text).map(|f| f.view)
}

/// `benilla-config/survivors-view.toml`.
fn view_path() -> Option<PathBuf> {
    crate::local_state::home().map(|h| h.join("survivors-view.toml"))
}

/// The saved view; a missing file is the default, and a broken one too, with a warning.
fn load() -> ViewMode {
    let Some(path) = view_path() else {
        return ViewMode::default();
    };
    match crate::local_state::read_to_string(&path) {
        Ok(text) => from_toml(&text).unwrap_or_else(|e| {
            warn!(
                "survivors: {} is malformed ({e}); the WoW view",
                path.display()
            );
            ViewMode::default()
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => ViewMode::default(),
        Err(e) => {
            warn!(
                "survivors: cannot read {}: {e}; the WoW view",
                path.display()
            );
            ViewMode::default()
        }
    }
}

fn save(mode: ViewMode) {
    let Some(path) = view_path() else { return };
    if let Err(e) = crate::local_state::write_atomic(&path, &to_toml(mode)) {
        warn!("survivors: cannot save {}: {e}", path.display());
    }
}

pub(super) fn load_view(mut view: ResMut<View>) {
    view.mode = load();
}

/// `V` swaps the view and remembers it.
pub(super) fn toggle_view(keys: Res<ButtonInput<KeyCode>>, mut view: ResMut<View>) {
    if keys.just_pressed(KEY) {
        view.mode = view.mode.toggled();
        save(view.mode);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_view_survives_a_save_and_load() {
        for mode in [ViewMode::Wow, ViewMode::Classic] {
            let text = to_toml(mode);
            assert_eq!(from_toml(&text).unwrap(), mode, "{text}");
        }
        assert!(to_toml(ViewMode::Classic).contains("view = \"classic\""));
    }

    #[test]
    fn a_missing_entry_is_the_wow_view_and_a_bad_one_is_refused() {
        assert_eq!(from_toml("").unwrap(), ViewMode::Wow);
        assert_eq!(from_toml("# nothing set\n").unwrap(), ViewMode::Wow);
        assert!(from_toml("view = \"sideways\"").is_err());
        assert!(from_toml("view = 3").is_err());
    }

    #[test]
    fn v_swaps_the_two_views() {
        assert_eq!(ViewMode::default(), ViewMode::Wow);
        assert_eq!(ViewMode::Wow.toggled(), ViewMode::Classic);
        assert_eq!(ViewMode::Wow.toggled().toggled(), ViewMode::Wow);
        assert_eq!(KEY, KeyCode::KeyV);
    }

    #[test]
    fn wow_drives_every_run_screen_but_never_the_menu_or_the_auto_runner() {
        for phase in [
            Phase::Playing,
            Phase::LevelUp,
            Phase::Paused,
            Phase::GameOver,
        ] {
            assert!(wow_in_play(phase, ViewMode::Wow, false), "{phase:?}");
            assert!(!wow_in_play(phase, ViewMode::Classic, false), "{phase:?}");
            assert!(!wow_in_play(phase, ViewMode::Wow, true), "{phase:?}");
        }
        assert!(!wow_in_play(Phase::Menu, ViewMode::Wow, false));
    }
}
