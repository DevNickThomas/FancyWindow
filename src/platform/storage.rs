//! Settings files next to the executable, so the install folder is portable.

use std::fs;
use std::io::Write;
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

/// Profiles shown by File > Open window. Backups and corrupt files are not entries.
#[derive(Debug, PartialEq, Eq)]
pub struct SavedWindow {
    pub profile: Option<String>,
    pub name: String,
}

pub fn saved_windows(dir: &Path) -> std::io::Result<Vec<SavedWindow>> {
    let mut windows = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else { continue };
        let profile = if file_name == "settings.json" { None } else {
            let Some(profile) = file_name.strip_prefix("settings-").and_then(|n| n.strip_suffix(".json")) else { continue };
            if profile.is_empty() { continue; }
            Some(profile.to_string())
        };
        let Some(settings) = fs::read_to_string(entry.path()).ok().and_then(|json| Settings::from_json(&json)) else { continue };
        let name = settings.window_name.filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| profile.clone().unwrap_or_else(|| "Default window".into()));
        windows.push(SavedWindow { profile, name });
    }
    windows.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then_with(|| a.profile.cmp(&b.profile)));
    Ok(windows)
}

/// Every new window gets a fresh profile, even when two display names are identical.
pub fn create_window_profile(dir: &Path, settings: &Settings) -> std::io::Result<String> {
    let profile = format!("window-{}", uuid::Uuid::new_v4());
    let path = dir.join(settings_file_name(Some(&profile)));
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(settings.to_json().as_bytes())?;
    file.sync_all()?;
    Ok(profile)
}
