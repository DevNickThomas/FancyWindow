use fancy_window::app::*;
use fancy_window::model::*;

#[test]
fn resized_workspace_is_recognized_immediately_after_save() {
    let mut state = AppState::new();
    state.layout = GridLayout::equal_columns(2);
    let split = state.layout.splitters(Rect::new(0.0, 0.0, 1075.0, 770.0))[0];
    // Weights captured from an actual splitter drag in the native preview.
    state.layout = state
        .layout
        .set_weight(split.split_id, 0, 1.2064276885043261)
        .unwrap();
    state.layout = state
        .layout
        .set_weight(split.split_id, 1, 0.7935723114956739)
        .unwrap();
    update(
        &mut state,
        Msg::WorkspaceNamed {
            slot: 0,
            name: "Resized".into(),
            saved_at_utc: "2026-10-02T00:00:00Z".into(),
        },
    );
    assert_eq!(state.current_workspace(), Some(0));
    assert_eq!(state.status_bar().left[0].text, "Resized");
}

#[test]
fn saving_renaming_and_deleting_the_current_workspace_repaints_its_name() {
    let mut state = AppState::new();
    for name in ["First", "Renamed"] {
        let effects = update(
            &mut state,
            Msg::WorkspaceNamed {
                slot: 0,
                name: name.into(),
                saved_at_utc: "2026-10-02T00:00:00Z".into(),
            },
        );
        assert!(
            effects.contains(&Effect::Repaint),
            "saving {name} must refresh the title and status bars"
        );
        assert_eq!(state.status_bar().left[0].text, name);
    }
    let effects = update(&mut state, Msg::WorkspaceDeleteConfirmed { slot: 0 });
    assert!(effects.contains(&Effect::Repaint));
    assert_eq!(state.current_workspace(), None);
}

#[test]
fn occupied_header_splits_preserve_apps_and_leave_other_panes_in_place() {
    for layout in [GridLayout::equal_columns(2), GridLayout::uniform_grid(2, 2)] {
        for (mods, orientation) in [
            (
                Modifiers {
                    ctrl: true,
                    shift: false,
                },
                Orientation::Columns,
            ),
            (
                Modifiers {
                    ctrl: false,
                    shift: true,
                },
                Orientation::Rows,
            ),
        ] {
            for scale in [1.0, 1.5, 2.0] {
                let mut state = AppState::new();
                state.settings.show_zone_headers = true;
                state.layout = layout.clone();
                state.frame = Frame::new(1200.0, 900.0, Point::new(50.0, 90.0), scale);
                let zones = state.layout.leaves();
                let left = zones[0];
                let other = *zones.last().unwrap();
                let a = WindowId(1);
                let b = WindowId(2);
                state.attachments = vec![
                    Attachment {
                        zone: left,
                        window: a,
                    },
                    Attachment {
                        zone: other,
                        window: b,
                    },
                ];
                let before_a = state.host_screen_rect(left).unwrap();
                let before_b = state.host_screen_rect(other).unwrap();
                let header = state
                    .zone_headers()
                    .into_iter()
                    .find(|h| h.window == a)
                    .unwrap();
                let at = Point::new(
                    header.bounds.x + header.bounds.width / 2.0,
                    header.bounds.y + header.bounds.height / 2.0,
                );
                assert_eq!(state.header_at(at), Some(HeaderHit::Title(a)));
                assert!(state.frame.to_screen(header.bounds).bottom() <= before_a.y);
                let effects = update(
                    &mut state,
                    Msg::MouseDown {
                        at,
                        button: Button::Left,
                        mods,
                    },
                );
                assert_eq!(state.layout.leaves().len(), zones.len() + 1);
                assert_eq!(state.zone_of(a), Some(left));
                assert_eq!(state.zone_of(b), Some(other));
                assert_eq!(state.host_screen_rect(other).unwrap(), before_b);
                let after_a = state.host_screen_rect(left).unwrap();
                assert_eq!((after_a.x, after_a.y), (before_a.x, before_a.y));
                match orientation {
                    Orientation::Columns => assert!(after_a.width < before_a.width),
                    Orientation::Rows => assert!(after_a.height < before_a.height),
                }
                assert!(effects.contains(&Effect::Place {
                    window: a,
                    rect: after_a
                }));
                assert!(
                    !effects
                        .iter()
                        .any(|e| matches!(e, Effect::Release(_) | Effect::Forget(_)))
                );
            }
        }
    }
}

#[test]
fn occupied_header_context_menu_targets_the_console_pane_without_releasing_it() {
    let mut state = AppState::new();
    state.settings.show_zone_headers = true;
    state.frame = Frame::new(800.0, 600.0, Point::new(0.0, 0.0), 1.0);
    let zone = state.layout.leaves()[0];
    state.attachments.push(Attachment {
        zone,
        window: WindowId(1),
    });
    let at = Point::new(100.0, 14.0);
    let effects = update(
        &mut state,
        Msg::MouseDown {
            at,
            button: Button::Right,
            mods: Modifiers {
                ctrl: true,
                shift: false,
            },
        },
    );
    assert!(effects.contains(&Effect::ShowZoneMenu { zone, at }));
    assert_eq!(state.zone_of(WindowId(1)), Some(zone));
    assert!(
        !effects
            .iter()
            .any(|e| matches!(e, Effect::Release(_) | Effect::Forget(_)))
    );
}
