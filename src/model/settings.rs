//! What is saved between runs. The JSON matches the .NET app's settings.json
//! (camelCase, layout stored as a JSON string, nine workspace slots).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const WORKSPACE_SLOTS: usize = 9;
pub const MAX_MARGIN: i32 = 64;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub window_margin: i32,
    pub last_layout_json: Option<String>,
    pub workspaces: Vec<Option<Workspace>>,
    pub theme_name: String,
    pub accent_color: String,
    pub window_width: Option<f64>,
    pub window_height: Option<f64>,
    pub window_left: Option<f64>,
    pub window_top: Option<f64>,
    pub window_maximized: bool,
    /// Reassigned global hotkeys: command name to chord ("" = none). Missing names use
    /// the defaults. New in the Rust version; the .NET app ignores it.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub hotkeys: BTreeMap<String, String>,
    /// A header over each zone with the hosted window's icon, title and a release button.
    /// New in the Rust version.
    pub show_zone_headers: bool,
}

/// A saved layout with an optional global hotkey.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Workspace {
    pub name: Option<String>,
    pub layout_json: String,
    /// ISO-8601 UTC, kept as text.
    pub saved_at_utc: String,
    /// e.g. "Ctrl+Win+W".
    pub hotkey_chord: Option<String>,
}

/// Main-window position and size in DIPs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowBounds {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
    pub maximized: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            window_margin: 0,
            last_layout_json: None,
            workspaces: vec![None; WORKSPACE_SLOTS],
            theme_name: "Dark Modern".into(),
            accent_color: "#007ACC".into(),
            window_width: None,
            window_height: None,
            window_left: None,
            window_top: None,
            window_maximized: false,
            hotkeys: BTreeMap::new(),
            show_zone_headers: false,
        }
    }
}

impl Settings {
    /// Parses settings, or `None` if the text is empty or not valid settings JSON.
    pub fn from_json(json: &str) -> Option<Self> {
        let value: serde_json::Value = serde_json::from_str(json).ok()?;
        if !value.is_object() {
            return None;
        }
        let mut settings: Settings = serde_json::from_value(value).ok()?;
        settings.workspaces.resize(WORKSPACE_SLOTS, None);
        settings.window_margin = settings.window_margin.clamp(0, MAX_MARGIN);
        Some(settings)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("settings always serialize")
    }

    pub fn bounds(&self) -> Option<WindowBounds> {
        Some(WindowBounds {
            left: self.window_left?,
            top: self.window_top?,
            width: self.window_width?,
            height: self.window_height?,
            maximized: self.window_maximized,
        })
    }

    pub fn set_bounds(&mut self, b: WindowBounds) {
        self.window_left = Some(b.left);
        self.window_top = Some(b.top);
        self.window_width = Some(b.width);
        self.window_height = Some(b.height);
        self.window_maximized = b.maximized;
    }
}

/// The `--profile name` / `--profile=name` launch argument, if given and non-blank.
pub fn profile_from_args(args: &[String]) -> Option<String> {
    let value = args.iter().enumerate().find_map(|(i, arg)| {
        if arg.eq_ignore_ascii_case("--profile") {
            args.get(i + 1).cloned()
        } else {
            arg.get(..10).filter(|p| p.eq_ignore_ascii_case("--profile=")).map(|_| arg[10..].to_string())
        }
    })?;
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// `settings.json` for the default profile, `settings-<name>.json` otherwise.
pub fn settings_file_name(profile: Option<&str>) -> String {
    match profile {
        None => "settings.json".into(),
        Some(p) => format!("settings-{}.json", sanitize_file_name(p)),
    }
}

/// `crash.log` for the default profile, `crash-<name>.log` otherwise.
pub fn crash_log_name(profile: Option<&str>) -> String {
    match profile {
        None => "crash.log".into(),
        Some(p) => format!("crash-{}.log", sanitize_file_name(p)),
    }
}

fn sanitize_file_name(name: &str) -> String {
    name.chars().map(|c| if c.is_control() || r#"<>:"/\|?*"#.contains(c) { '_' } else { c }).collect()
}
