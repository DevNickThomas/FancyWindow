use fancy_window::model::*;

#[test]
fn parses_modifiers_and_letter() {
    assert_eq!(Chord::parse("Ctrl+Win+W"), Some(Chord::new(MOD_CONTROL | MOD_WIN, 0x57)));
    assert_eq!(Chord::parse(" control + alt + shift + windows + a "), Some(Chord::new(MOD_CONTROL | MOD_ALT | MOD_SHIFT | MOD_WIN, 0x41)));
}

#[test]
fn parses_wpf_key_names() {
    assert_eq!(Chord::parse("Ctrl+Win+D7").unwrap().key, 0x37);
    assert_eq!(Chord::parse("Ctrl+Alt+F12").unwrap().key, 0x7B);
    assert_eq!(Chord::parse("Ctrl+NumPad3").unwrap().key, 0x63);
    assert_eq!(Chord::parse("Ctrl+Win+OemPlus").unwrap().key, 0xBB);
    assert_eq!(Chord::parse("Ctrl+Win+Oem6").unwrap().key, 0xDD);
    assert_eq!(Chord::parse("Ctrl+Win+OemOpenBrackets").unwrap().key, 0xDB);
    assert_eq!(Chord::parse("Alt+Space").unwrap().key, 0x20);
}

#[test]
fn rejects_bad_chords() {
    for bad in ["", "W", "Ctrl+", "Ctrl+Win", "Hyper+W", "Ctrl+F25", "Ctrl+Nope", "Ctrl+D10"] {
        assert_eq!(Chord::parse(bad), None, "{bad}");
    }
}

#[test]
fn formats_in_canonical_order_and_roundtrips() {
    let chord = Chord::new(MOD_WIN | MOD_SHIFT | MOD_CONTROL, 0xBD);
    assert_eq!(chord.format(), "Ctrl+Shift+Win+OemMinus");
    for text in ["Ctrl+Win+W", "Ctrl+Alt+F1", "Ctrl+Win+D0", "Win+NumPad9", "Ctrl+Win+OemCloseBrackets", "Shift+Win+Home"] {
        assert_eq!(Chord::parse(text).unwrap().format(), text);
    }
}

#[test]
fn display_puts_win_before_alt_but_files_keep_the_net_order() {
    let chord = Chord::parse("Alt+Win+OemCloseBrackets").unwrap();
    assert_eq!(chord.display(), "Win+Alt+]");
    assert_eq!(chord.format(), "Alt+Win+OemCloseBrackets");
    assert_eq!(Chord::parse("Ctrl+Win+OemMinus").unwrap().display(), "Ctrl+Win+-");
}
