#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use fancy_window::app::AppState;
use fancy_window::model::profile_from_args;
use fancy_window::platform::{self, crash, storage};

fn main() -> windows::core::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let profile = profile_from_args(&args);
    crash::install(profile.as_deref());
    let path = storage::settings_path(profile.as_deref());
    let settings = storage::load(&path);
    platform::run(AppState::from_settings(settings, profile), path)
}
