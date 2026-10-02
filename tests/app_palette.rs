use fancy_window::app::*;

fn labels(state: &AppState) -> Vec<String> {
    state.palette_entries().into_iter().map(|e| e.label).collect()
}

fn top(state: &AppState, query: &str) -> Vec<String> {
    let entries = state.palette_entries();
    filter_palette(&entries, query).into_iter().take(3).map(|m| entries[m.entry].label.clone()).collect()
}

#[test]
fn entries_come_from_the_menus() {
    let all = labels(&AppState::new());
    for expected in [
        "Layout: Big left, stacked right",
        "Layout \u{203A} Add: Row above",
        "Layout: Hide zone headers",
        "File: Preferences",
        "Help: About",
        "Workspaces \u{203A} Save current as: Slot 1  (empty)",
    ] {
        assert!(all.iter().any(|l| l == expected), "missing {expected:?} in {all:#?}");
    }
}

#[test]
fn the_palette_does_not_list_itself_or_disabled_items() {
    let all = labels(&AppState::new());
    assert!(!all.iter().any(|l| l.contains("Command palette")));
    // With nothing saved, "Delete" slots are disabled and so left out.
    assert!(!all.iter().any(|l| l.starts_with("Workspaces \u{203A} Delete")));
}

#[test]
fn entries_carry_their_chords() {
    let entries = AppState::new().palette_entries();
    let reset = entries.iter().find(|e| e.label.starts_with("Layout: Reset")).expect("reset entry");
    assert_eq!(reset.chord.as_deref(), Some("Win+Alt+Home"));
    assert_eq!(reset.action, MenuAction::Run(Command::ResetLayout));
}

#[test]
fn empty_query_keeps_menu_order() {
    let state = AppState::new();
    let entries = state.palette_entries();
    let all = filter_palette(&entries, "  ");
    assert_eq!(all.len(), entries.len());
    assert!(all.windows(2).all(|w| w[0].entry < w[1].entry));
}

#[test]
fn words_match_anywhere_and_word_starts_rank_first() {
    let state = AppState::new();
    // The mockup's example: "lay big" finds the big layouts first.
    let found = top(&state, "lay big");
    assert_eq!(found[0], "Layout: Big left, stacked right");
    assert!(found.iter().all(|l| l.starts_with("Layout:") && l.to_lowercase().contains("big")));
}

#[test]
fn letters_in_order_still_match() {
    let state = AppState::new();
    // "bgl" isn't a substring of anything, but b..g..l is in order in "Big left".
    assert!(top(&state, "bgl").iter().any(|l| l == "Layout: Big left, stacked right"));
    assert!(top(&state, "zzqx").is_empty());
}

#[test]
fn matched_positions_point_at_the_typed_letters() {
    let state = AppState::new();
    let entries = state.palette_entries();
    let m = filter_palette(&entries, "about").into_iter().next().expect("match");
    let label: Vec<char> = entries[m.entry].label.chars().collect();
    let picked: String = m.matched.iter().map(|&i| label[i]).collect();
    assert_eq!(picked.to_lowercase(), "about");
}

#[test]
fn saved_workspaces_show_up_by_name() {
    let mut state = AppState::new();
    update(&mut state, Msg::WorkspaceNamed { slot: 1, name: "Coding".into(), saved_at_utc: "2026-10-02T00:00:00Z".into() });
    let found = top(&state, "coding");
    assert!(found.iter().any(|l| l == "Workspaces: 2  Coding"), "{found:?}");
}

#[test]
fn hotkey_and_centre_box_open_the_palette() {
    let mut state = AppState::new();
    assert_eq!(update(&mut state, Msg::Command(Command::OpenPalette)), vec![Effect::ShowPalette]);
    assert_eq!(update(&mut state, Msg::Menu(MenuAction::OpenPalette)), vec![Effect::ShowPalette]);
    assert_eq!(state.chord_label(Command::OpenPalette).as_deref(), Some("Win+Alt+Space"));
}

#[test]
fn layouts_carry_a_thumbnail_others_do_not() {
    let entries = AppState::new().palette_entries();
    let big_left = entries.iter().find(|e| e.label == "Layout: Big left, stacked right").expect("entry");
    let zones = big_left.preview.as_ref().expect("thumbnail");
    assert_eq!(zones.len(), 3);
    // The big zone is two thirds of the width; all zones sit in the unit square.
    assert!((zones[0].width - 2.0 / 3.0).abs() < 0.01, "{zones:?}");
    assert!(zones.iter().all(|z| z.x >= 0.0 && z.y >= 0.0 && z.right() <= 1.0 + 1e-9 && z.bottom() <= 1.0 + 1e-9));
    assert!(entries.iter().find(|e| e.label == "Help: About").unwrap().preview.is_none());
}

#[test]
fn saved_workspaces_preview_their_layout() {
    let mut state = AppState::new();
    state.layout = fancy_window::model::GridLayout::equal_columns(3);
    update(&mut state, Msg::WorkspaceNamed { slot: 0, name: "Three".into(), saved_at_utc: "2026-10-02T00:00:00Z".into() });
    let entries = state.palette_entries();
    let load = entries.iter().find(|e| e.action == MenuAction::LoadWorkspace(0)).expect("load entry");
    assert_eq!(load.preview.as_ref().map(Vec::len), Some(3));
}
