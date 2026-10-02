use fancy_window::model::*;

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn reads_settings_written_by_dotnet_app() {
    let json = include_str!("fixtures/dotnet-settings.json");
    let s = Settings::from_json(json).expect("parses");
    assert_eq!(s.theme_name, "Dark");
    assert_eq!(s.accent_color, "#A371F7");
    assert_eq!(s.workspaces.len(), WORKSPACE_SLOTS);
    assert_eq!(s.bounds(), Some(WindowBounds { left: 32.0, top: 32.0, width: 1511.0, height: 962.0, maximized: false }));
    let layout = GridLayout::from_json(s.last_layout_json.as_deref().unwrap()).expect("embedded layout parses");
    assert_eq!(layout.leaves().len(), 2);
}

#[test]
fn roundtrip_preserves_everything() {
    let mut s = Settings { window_margin: 6, last_layout_json: Some(GridLayout::equal_rows(3).to_json()), ..Settings::default() };
    s.workspaces[2] = Some(Workspace { name: Some("Code".into()), layout_json: "{}".into(), saved_at_utc: "2026-10-01T10:00:00Z".into(), hotkey_chord: Some("Ctrl+Win+1".into()) });
    s.set_bounds(WindowBounds { left: 10.0, top: 20.0, width: 900.0, height: 600.0, maximized: true });
    assert_eq!(Settings::from_json(&s.to_json()), Some(s));
}

#[test]
fn writes_dotnet_property_names() {
    let json = Settings::default().to_json();
    for key in ["windowMargin", "lastLayoutJson", "workspaces", "themeName", "accentColor", "windowWidth", "windowMaximized"] {
        assert!(json.contains(&format!("\"{key}\"")), "missing {key}");
    }
}

#[test]
fn missing_fields_fall_back_to_defaults_and_unknown_fields_are_ignored() {
    let s = Settings::from_json(r##"{ "accentColor": "#FF0000", "someFutureField": 42 }"##).unwrap();
    assert_eq!(s.accent_color, "#FF0000");
    assert_eq!(s.theme_name, "Dark");
    assert_eq!(s.workspaces.len(), WORKSPACE_SLOTS);
    assert_eq!(s.bounds(), None);
}

#[test]
fn normalizes_workspace_count_and_margin() {
    let s = Settings::from_json(r#"{ "workspaces": [null, null], "windowMargin": 500 }"#).unwrap();
    assert_eq!(s.workspaces.len(), WORKSPACE_SLOTS);
    assert_eq!(s.window_margin, MAX_MARGIN);
    let s = Settings::from_json(r#"{ "windowMargin": -3 }"#).unwrap();
    assert_eq!(s.window_margin, 0);
}

#[test]
fn rejects_empty_or_corrupt_json() {
    assert_eq!(Settings::from_json(""), None);
    assert_eq!(Settings::from_json("{ not json"), None);
    assert_eq!(Settings::from_json("[]"), None);
}

#[test]
fn profile_argument_forms() {
    assert_eq!(profile_from_args(&args(&["--profile", "work"])), Some("work".into()));
    assert_eq!(profile_from_args(&args(&["--PROFILE=home"])), Some("home".into()));
    assert_eq!(profile_from_args(&args(&["--profile", "  "])), None);
    assert_eq!(profile_from_args(&args(&["--profile"])), None);
    assert_eq!(profile_from_args(&args(&["other"])), None);
}

#[test]
fn profile_file_names() {
    assert_eq!(settings_file_name(None), "settings.json");
    assert_eq!(settings_file_name(Some("work")), "settings-work.json");
    assert_eq!(settings_file_name(Some("a/b:c")), "settings-a_b_c.json");
    assert_eq!(crash_log_name(Some("work")), "crash-work.log");
}
