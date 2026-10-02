use fancy_window::view::{ACCENT_SWATCHES, Color, PRESETS, Theme};

#[test]
fn parses_hex_colours() {
    assert_eq!(Color::parse("#A371F7"), Some(Color(0xA3, 0x71, 0xF7)));
    assert_eq!(Color::parse(" #fff "), Some(Color(0xFF, 0xFF, 0xFF)));
    assert_eq!(Color::parse("A371F7"), None);
    assert_eq!(Color::parse("#12345"), None);
    assert_eq!(Color::parse("#GGGGGG"), None);
    assert_eq!(Color(0x00, 0x7A, 0xCC).hex(), "#007ACC");
}

#[test]
fn eight_presets_with_original_names() {
    let names: Vec<&str> = PRESETS.iter().map(|p| p.name).collect();
    assert_eq!(names, ["Dark", "Dark Dimmed", "High Contrast", "Light", "GitHub Light", "Solarized Light", "Solarized Dark", "Monokai"]);
    assert_eq!(ACCENT_SWATCHES.len(), 8);
}

#[test]
fn accent_drives_zone_tint_and_selection() {
    let theme = Theme::new("Dark", "#A371F7");
    assert_eq!(theme.accent, Color(0xA3, 0x71, 0xF7));
    // 20% accent over #1E1E1E.
    assert_eq!(theme.zone_fill, Color(0x39, 0x2F, 0x49));
    // 45% toward black.
    assert_eq!(theme.active_bg, Color(0x5A, 0x3E, 0x88));
}

#[test]
fn active_zone_is_brighter_than_the_rest() {
    let dark = Theme::new("Dark", "#007ACC");
    // 45% accent over #1E1E1E, against 20% for other zones.
    assert_eq!(dark.active_glow, Color(0x11, 0x47, 0x6C));
    // Accent lifted 35% toward white.
    assert_eq!(dark.active_bevel, Color(0x59, 0xA9, 0xDE));
    let light = Theme::new("Light", "#007ACC");
    assert_eq!(light.active_bevel, Color(0x00, 0x68, 0xAD));
}

#[test]
fn status_bar_takes_the_accent_with_readable_text() {
    let blue = Theme::new("Dark", "#007ACC");
    assert_eq!(blue.status_bg, Color(0x00, 0x7A, 0xCC));
    assert_eq!(blue.status_text, Color(0xFF, 0xFF, 0xFF));
    let green = Theme::new("Dark", "#7EE787");
    assert_eq!(green.status_text, Color(0x1F, 0x1F, 0x1F));
}

#[test]
fn unknown_names_and_bad_accents_fall_back() {
    assert_eq!(Theme::new("Nope", "nope"), Theme::dark());
}

#[test]
fn light_and_dark_presets_are_told_apart() {
    for (name, light) in [("Dark", false), ("Light", true), ("Solarized Light", true), ("Solarized Dark", false), ("High Contrast", false)] {
        assert_eq!(Theme::new(name, "#007ACC").is_light(), light, "{name}");
    }
}
