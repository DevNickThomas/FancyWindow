use fancy_window::app::*;
use fancy_window::model::*;

const BOUNDS: WindowBounds = WindowBounds { left: 10.0, top: 20.0, width: 1000.0, height: 700.0, maximized: true };

fn saved(effects: &[Effect]) -> Settings {
    effects
        .iter()
        .find_map(|e| match e {
            Effect::SaveSettings(s) => Some(s.clone()),
            _ => None,
        })
        .expect("a save effect")
}

#[test]
fn restores_last_layout_and_margin() {
    let layout = GridLayout::equal_rows(3);
    let settings = Settings { last_layout_json: Some(layout.to_json()), window_margin: 8, ..Settings::default() };
    let state = AppState::from_settings(settings, None);
    assert_eq!(state.layout, layout);
    assert_eq!(state.margin, 8.0);
}

#[test]
fn bad_saved_layout_falls_back_to_two_by_two() {
    let settings = Settings { last_layout_json: Some("garbage".into()), ..Settings::default() };
    assert_eq!(AppState::from_settings(settings, None).layout.leaves().len(), 4);
}

#[test]
fn closing_saves_live_layout_bounds_and_keeps_other_settings() {
    let settings = Settings { accent_color: "#A371F7".into(), ..Settings::default() };
    let mut state = AppState::from_settings(settings, None);
    state.layout = GridLayout::equal_columns(3);
    let s = saved(&update(&mut state, Msg::Closing { bounds: BOUNDS }));
    assert_eq!(GridLayout::from_json(s.last_layout_json.as_deref().unwrap()).unwrap(), state.layout);
    assert_eq!(s.bounds(), Some(BOUNDS));
    assert_eq!(s.accent_color, "#A371F7");
}

#[test]
fn save_comes_after_letting_hosted_windows_go() {
    let mut state = AppState::new();
    update(&mut state, Msg::FrameChanged(Frame::new(800.0, 600.0, Point::new(0.0, 0.0), 1.0)));
    update(&mut state, Msg::WindowDropped { window: WindowId(7), at: Point::new(10.0, 10.0), alt: true });
    let effects = update(&mut state, Msg::Closing { bounds: BOUNDS });
    assert_eq!(effects[0], Effect::Forget(WindowId(7)));
    assert!(matches!(effects.last(), Some(Effect::SaveSettings(_))));
}

#[test]
fn title_shows_profile() {
    assert_eq!(AppState::new().title(), "Fancy Window");
    assert_eq!(AppState::from_settings(Settings::default(), Some("work".into())).title(), "Fancy Window [work]");
}
