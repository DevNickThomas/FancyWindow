use fancy_window::model::*;
use fancy_window::platform::storage;

#[test]
fn same_named_windows_get_independent_profiles_and_can_be_discovered_again() {
    let dir = std::env::temp_dir().join(format!("fancywindow-profiles-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&dir).unwrap();
    let a = Settings { window_name: Some("Agents".into()), last_layout_json: Some(GridLayout::equal_columns(2).to_json()), ..Settings::default() };
    let b = Settings { window_name: Some("Agents".into()), last_layout_json: Some(GridLayout::equal_rows(3).to_json()), ..Settings::default() };
    let first = storage::create_window_profile(&dir, &a).unwrap();
    let second = storage::create_window_profile(&dir, &b).unwrap();
    assert_ne!(first, second);
    assert_eq!(storage::load(&dir.join(settings_file_name(Some(&first)))), a);
    assert_eq!(storage::load(&dir.join(settings_file_name(Some(&second)))), b);
    std::fs::write(dir.join("settings-corrupt.json"), "not json").unwrap();
    std::fs::write(dir.join("settings-backup.json.bak"), a.to_json()).unwrap();
    let listed = storage::saved_windows(&dir).unwrap();
    assert_eq!(listed.len(), 2);
    assert!(listed.iter().all(|entry| entry.name == "Agents"));
    assert!(listed.iter().any(|entry| entry.profile.as_ref() == Some(&first)));
    std::fs::remove_dir_all(dir).unwrap();
}
