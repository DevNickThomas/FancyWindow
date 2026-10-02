//! Settings files next to the executable, so the install folder is portable.

use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{Settings, settings_file_name};

pub fn settings_path(profile: Option<&str>) -> PathBuf {
    exe_dir().join(settings_file_name(profile))
}

pub fn exe_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current exe path");
    exe.parent().expect("exe has a folder").to_path_buf()
}

/// The main file, else the last known-good `.bak`, else defaults. Never fails.
pub fn load(path: &Path) -> Settings {
    [path.to_path_buf(), with_suffix(path, ".bak")]
        .iter()
        .find_map(|p| Settings::from_json(&fs::read_to_string(p).ok()?))
        .unwrap_or_default()
}

/// Writes `.tmp`, keeps the previous file as `.bak`, then swaps the new one in,
/// so a crash mid-save never leaves a half-written settings file.
pub fn save(path: &Path, settings: &Settings) -> std::io::Result<()> {
    let tmp = with_suffix(path, ".tmp");
    fs::write(&tmp, settings.to_json())?;
    if path.exists() {
        fs::copy(path, with_suffix(path, ".bak"))?;
    }
    fs::rename(&tmp, path)
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}
