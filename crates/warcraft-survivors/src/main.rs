//! The `warcraft-survivors` launcher: the build stamp, handed to the survivors mode.

use benilla_app::BuildId;
#[cfg(target_os = "windows")]
use std::path::Path;
#[cfg(any(target_os = "windows", test))]
use std::path::PathBuf;

fn main() -> benilla_app::AppExit {
    #[cfg(target_os = "windows")]
    if launch_kit_if_needed() {
        return benilla_app::AppExit::Success;
    }
    benilla_app::run_survivors(BuildId {
        version: env!("CARGO_PKG_VERSION"),
        describe: env!("BENILLA_GIT_DESCRIBE"),
        sha: env!("BENILLA_GIT_SHA"),
        short: env!("BENILLA_GIT_SHORT"),
        date: env!("BENILLA_GIT_DATE"),
        profile: env!("BENILLA_PROFILE"),
        project_dir: env!("BENILLA_PROJECT_DIR"),
        ..Default::default()
    })
}

/// The kit accepts either the Data folder itself or its WoW install parent, with Explorer's
/// optional surrounding quotes and trailing separator removed.
#[cfg(any(target_os = "windows", test))]
fn data_candidates(saved: &str) -> Vec<PathBuf> {
    let path = saved.trim().trim_matches('"').trim_end_matches(['\\', '/']);
    if path.is_empty() {
        return Vec::new();
    }
    let path = PathBuf::from(path);
    vec![path.clone(), path.join("Data")]
}

#[cfg(target_os = "windows")]
fn has_mpq(data: &Path) -> bool {
    data.is_dir()
        && std::fs::read_dir(data).is_ok_and(|entries| {
            entries.flatten().any(|entry| {
                entry.path().is_file()
                    && entry
                        .path()
                        .extension()
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("mpq"))
            })
        })
}

#[cfg(any(target_os = "windows", test))]
fn needs_kit_launcher(has_data: bool, has_play_bat: bool) -> bool {
    !has_data && has_play_bat
}

/// Make double-clicking the player exe behave like Play.bat. This is before Bevy can create a
/// window or any library can start a thread, as `set_var` requires.
#[cfg(target_os = "windows")]
fn launch_kit_if_needed() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let Some(dir) = exe.parent() else {
        return false;
    };

    if std::env::var_os("WOW_DATA").is_none() {
        if let Ok(saved) = std::fs::read_to_string(dir.join("wow-data-path.txt")) {
            if let Some(data) = data_candidates(&saved)
                .into_iter()
                .find(|path| has_mpq(path))
            {
                // SAFETY: this runs at the very start of main, before this process creates threads.
                unsafe { std::env::set_var("WOW_DATA", data) };
            }
        }
    }
    if std::env::var_os("BENILLA_HOME").is_none() {
        let home = dir.join("benilla-config");
        if home.is_dir() {
            // SAFETY: this runs at the very start of main, before this process creates threads.
            unsafe { std::env::set_var("BENILLA_HOME", home) };
        }
    }

    let has_data = std::env::var_os("WOW_DATA").is_some_and(|path| has_mpq(Path::new(&path)));
    let play = dir.join("Play.bat");
    if !needs_kit_launcher(has_data, play.is_file()) {
        return false;
    }
    let _ = std::process::Command::new("cmd")
        .arg("/c")
        .arg("start")
        .arg("")
        .arg(&play)
        .spawn();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_data_path_accepts_data_or_the_install() {
        /*
            assert_eq!(
                data_candidates(" \"C:\\WoW\\Data\\\" \"),
                [PathBuf::from("C:\\WoW\\Data"), PathBuf::from("C:\\WoW\\Data\\Data")]
            );
            assert_eq!(
                data_candidates("C:\\WoW"),
                [PathBuf::from("C:\\WoW"), PathBuf::from("C:\\WoW\\Data")]
            );
        }

            */
        assert_eq!(
            data_candidates(" \"/wow-data/\" "),
            [PathBuf::from("/wow-data"), PathBuf::from("/wow-data/Data")]
        );
        assert_eq!(
            data_candidates("/wow"),
            [PathBuf::from("/wow"), PathBuf::from("/wow/Data")]
        );
    }

    #[test]
    fn only_a_missing_data_folder_with_play_bat_relaunches() {
        assert!(needs_kit_launcher(false, true));
        assert!(!needs_kit_launcher(true, true));
        assert!(!needs_kit_launcher(false, false));
    }
}
