//! Keyboard chords like "Ctrl+Win+W". Key names follow WPF's `Key` enum so chords
//! saved by the .NET app keep working.

/// RegisterHotKey modifier flags.
pub const MOD_ALT: u32 = 0x1;
pub const MOD_CONTROL: u32 = 0x2;
pub const MOD_SHIFT: u32 = 0x4;
pub const MOD_WIN: u32 = 0x8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Chord {
    /// `MOD_*` flags; never zero.
    pub modifiers: u32,
    /// Win32 virtual-key code.
    pub key: u16,
}

impl Chord {
    pub const fn new(modifiers: u32, key: u16) -> Self {
        Self { modifiers, key }
    }

    /// Parses "Mod+...+Key". Needs at least one modifier and exactly one key.
    pub fn parse(text: &str) -> Option<Self> {
        let tokens: Vec<&str> = text.split('+').map(str::trim).filter(|t| !t.is_empty()).collect();
        let (key_name, modifier_names) = tokens.split_last()?;
        let mut modifiers = 0;
        for name in modifier_names {
            modifiers |= match name.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => MOD_CONTROL,
                "alt" => MOD_ALT,
                "shift" => MOD_SHIFT,
                "win" | "windows" => MOD_WIN,
                _ => return None,
            };
        }
        if modifiers == 0 {
            return None;
        }
        Some(Self { modifiers, key: key_code(key_name)? })
    }

    /// Ctrl, Alt, Shift, Win, then the key, e.g. "Ctrl+Win+OemPlus". The .NET app's order.
    pub fn format(&self) -> String {
        self.join(key_name(self.key), FILE_ORDER)
    }

    /// For people rather than files: symbols instead of WPF names, and Win before Alt
    /// as Windows writes it, e.g. "Win+Alt+]".
    pub fn display(&self) -> String {
        let name = key_name(self.key);
        let symbol = DISPLAY_NAMES.iter().find(|(n, _)| *n == name).map(|(_, s)| s.to_string());
        let digit = name.strip_prefix('D').filter(|d| d.len() == 1).map(str::to_string);
        self.join(symbol.or(digit).unwrap_or(name), DISPLAY_ORDER)
    }

    fn join(&self, key: String, order: &[(u32, &str)]) -> String {
        let mut parts: Vec<String> =
            order.iter().filter(|(f, _)| self.modifiers & f != 0).map(|(_, n)| n.to_string()).collect();
        parts.push(key);
        parts.join("+")
    }
}

const FILE_ORDER: &[(u32, &str)] = &[(MOD_CONTROL, "Ctrl"), (MOD_ALT, "Alt"), (MOD_SHIFT, "Shift"), (MOD_WIN, "Win")];
const DISPLAY_ORDER: &[(u32, &str)] = &[(MOD_CONTROL, "Ctrl"), (MOD_WIN, "Win"), (MOD_ALT, "Alt"), (MOD_SHIFT, "Shift")];

/// Named keys outside the letter/digit/function ranges.
const NAMED_KEYS: &[(&str, u16)] = &[
    ("Back", 0x08), ("Tab", 0x09), ("Enter", 0x0D), ("Return", 0x0D), ("Pause", 0x13),
    ("Escape", 0x1B), ("Space", 0x20), ("PageUp", 0x21), ("Prior", 0x21), ("PageDown", 0x22),
    ("Next", 0x22), ("End", 0x23), ("Home", 0x24), ("Left", 0x25), ("Up", 0x26), ("Right", 0x27),
    ("Down", 0x28), ("PrintScreen", 0x2C), ("Insert", 0x2D), ("Delete", 0x2E),
    ("Multiply", 0x6A), ("Add", 0x6B), ("Subtract", 0x6D), ("Decimal", 0x6E), ("Divide", 0x6F),
    ("OemSemicolon", 0xBA), ("Oem1", 0xBA), ("OemPlus", 0xBB), ("OemComma", 0xBC), ("OemMinus", 0xBD),
    ("OemPeriod", 0xBE), ("OemQuestion", 0xBF), ("Oem2", 0xBF), ("OemTilde", 0xC0), ("Oem3", 0xC0),
    ("OemOpenBrackets", 0xDB), ("Oem4", 0xDB), ("OemPipe", 0xDC), ("Oem5", 0xDC),
    ("OemCloseBrackets", 0xDD), ("Oem6", 0xDD), ("OemQuotes", 0xDE), ("Oem7", 0xDE),
];

const DISPLAY_NAMES: &[(&str, &str)] = &[
    ("OemPlus", "="), ("OemMinus", "-"), ("OemOpenBrackets", "["), ("OemCloseBrackets", "]"), ("OemComma", ","),
    ("OemPeriod", "."), ("OemQuestion", "/"), ("OemSemicolon", ";"), ("OemQuotes", "'"), ("OemTilde", "`"), ("OemPipe", "\\"),
];

fn key_code(name: &str) -> Option<u16> {
    let upper = name.to_ascii_uppercase();
    let b = upper.as_bytes();
    match b {
        [c] if c.is_ascii_uppercase() => return Some(*c as u16),
        [b'D', d] if d.is_ascii_digit() => return Some(*d as u16),
        _ => {}
    }
    if let Some(n) = upper.strip_prefix("NUMPAD").and_then(|n| n.parse::<u16>().ok()).filter(|n| *n <= 9) {
        return Some(0x60 + n);
    }
    if let Some(n) = upper.strip_prefix('F').and_then(|n| n.parse::<u16>().ok()).filter(|n| (1..=24).contains(n)) {
        return Some(0x6F + n);
    }
    NAMED_KEYS.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, vk)| *vk)
}

fn key_name(vk: u16) -> String {
    match vk {
        0x41..=0x5A => (vk as u8 as char).to_string(),
        0x30..=0x39 => format!("D{}", vk - 0x30),
        0x60..=0x69 => format!("NumPad{}", vk - 0x60),
        0x70..=0x87 => format!("F{}", vk - 0x6F),
        _ => NAMED_KEYS.iter().find(|(_, k)| *k == vk).map(|(n, _)| n.to_string()).unwrap_or_else(|| format!("0x{vk:02X}")),
    }
}
