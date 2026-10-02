//! Colour palettes. A theme is a preset palette plus a user accent colour.

/// An RGB colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color(pub u8, pub u8, pub u8);

impl Color {
    /// "#RRGGBB" or "#RGB".
    pub fn parse(hex: &str) -> Option<Color> {
        let digits = hex.trim().strip_prefix('#')?;
        let channel = |s: &str| u8::from_str_radix(s, 16).ok();
        match digits.len() {
            6 => Some(Color(channel(&digits[0..2])?, channel(&digits[2..4])?, channel(&digits[4..6])?)),
            3 => {
                let one = |i: usize| channel(&digits[i..=i].repeat(2));
                Some(Color(one(0)?, one(1)?, one(2)?))
            }
            _ => None,
        }
    }

    pub fn hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }

    /// `self` laid over `background` at `alpha` (0..=1). GDI has no alpha, so blend up front.
    pub fn over(self, background: Color, alpha: f64) -> Color {
        let mix = |fg: u8, bg: u8| (fg as f64 * alpha + bg as f64 * (1.0 - alpha)).round() as u8;
        Color(mix(self.0, background.0), mix(self.1, background.1), mix(self.2, background.2))
    }

    /// Scales toward black by `amount` (0..=1).
    pub fn darken(self, amount: f64) -> Color {
        Color(0, 0, 0).over(self, amount)
    }

    pub fn is_light(self) -> bool {
        0.299 * self.0 as f64 + 0.587 * self.1 as f64 + 0.114 * self.2 as f64 > 128.0
    }
}

/// A named palette, matching the original app's presets.
pub struct Preset {
    pub name: &'static str,
    pub window_bg: Color,
    pub bar_bg: Color,
    pub popup_bg: Color,
    pub divider: Color,
    pub text: Color,
    pub muted: Color,
    pub splitter: Color,
    pub zone_border: Color,
    pub active_text: Color,
}

const fn rgb(v: u32) -> Color {
    Color((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

macro_rules! preset {
    ($name:expr, $window:expr, $bar:expr, $popup:expr, $divider:expr, $text:expr, $muted:expr, $splitter:expr, $border:expr, $active:expr) => {
        Preset {
            name: $name,
            window_bg: rgb($window),
            bar_bg: rgb($bar),
            popup_bg: rgb($popup),
            divider: rgb($divider),
            text: rgb($text),
            muted: rgb($muted),
            splitter: rgb($splitter),
            zone_border: rgb($border),
            active_text: rgb($active),
        }
    };
}

/// In the order the Preferences dialog lists them; the first is the default.
pub const PRESETS: &[Preset] = &[
    preset!("Dark", 0x1E1E1E, 0x2D2D30, 0x252526, 0x3F3F46, 0xCCCCCC, 0x9B9B9B, 0x3F3F46, 0x3F3F46, 0xFFFFFF),
    preset!("Dark Dimmed", 0x1C2128, 0x22272E, 0x2D333B, 0x444C56, 0xADBAC7, 0x768390, 0x444C56, 0x444C56, 0xFFFFFF),
    preset!("High Contrast", 0x000000, 0x0D0D0D, 0x0D0D0D, 0x808080, 0xFFFFFF, 0xBFBFBF, 0x7F7F7F, 0x7F7F7F, 0xFFFFFF),
    preset!("Light", 0xFFFFFF, 0xF3F3F3, 0xFFFFFF, 0xD4D4D4, 0x1F1F1F, 0x616161, 0xD4D4D4, 0xCECECE, 0xFFFFFF),
    preset!("GitHub Light", 0xFFFFFF, 0xF6F8FA, 0xFFFFFF, 0xD8DEE4, 0x24292F, 0x57606A, 0xD0D7DE, 0xD0D7DE, 0xFFFFFF),
    preset!("Solarized Light", 0xFDF6E3, 0xEEE8D5, 0xFDF6E3, 0x93A1A1, 0x586E75, 0x839496, 0x93A1A1, 0x93A1A1, 0xFDF6E3),
    preset!("Solarized Dark", 0x002B36, 0x073642, 0x073642, 0x586E75, 0x93A1A1, 0x839496, 0x586E75, 0x586E75, 0xFDF6E3),
    preset!("Monokai", 0x272822, 0x2D2E27, 0x272822, 0x3E3D32, 0xF8F8F2, 0x75715E, 0x3E3D32, 0x75715E, 0xFFFFFF),
];

pub const DEFAULT_ACCENT: Color = rgb(0x007ACC);
const WHITE: Color = rgb(0xFFFFFF);
/// Accent strength behind the focused window, against 0x33/255 (20%) for the other zones.
const ACTIVE_GLOW_ALPHA: f64 = 0.45;

/// Accent swatches offered in Preferences.
pub const ACCENT_SWATCHES: [Color; 8] =
    [rgb(0x007ACC), rgb(0xA371F7), rgb(0xF778BA), rgb(0xFF7B72), rgb(0xFFA657), rgb(0xD29922), rgb(0x7EE787), rgb(0x39C5CF)];

/// The palette the view paints with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub window_bg: Color,
    pub zone_fill: Color,
    pub zone_border: Color,
    pub splitter: Color,
    pub accent: Color,
    pub bar_bg: Color,
    pub popup_bg: Color,
    pub divider: Color,
    pub text: Color,
    pub muted: Color,
    /// Selected or highlighted item background, and its text.
    pub active_bg: Color,
    pub active_text: Color,
    /// The focused hosted window's zone: a stronger tint than `zone_fill`...
    pub active_glow: Color,
    /// ...and the bright ring right around the window.
    pub active_bevel: Color,
    /// The accent-coloured status bar, its text, its workspace segment and its warning segment.
    pub status_bg: Color,
    pub status_text: Color,
    pub status_strong: Color,
    pub status_warn: Color,
}

impl Theme {
    /// The named preset (default if unknown) with the accent (default if unparseable).
    pub fn new(preset_name: &str, accent_hex: &str) -> Self {
        let p = find_preset(preset_name);
        let accent = Color::parse(accent_hex).unwrap_or(DEFAULT_ACCENT);
        Self {
            window_bg: p.window_bg,
            zone_fill: accent.over(p.window_bg, 0x33 as f64 / 255.0),
            zone_border: p.zone_border,
            splitter: p.splitter,
            accent,
            bar_bg: p.bar_bg,
            popup_bg: p.popup_bg,
            divider: p.divider,
            text: p.text,
            muted: p.muted,
            active_bg: accent.darken(0.45),
            active_text: p.active_text,
            active_glow: accent.over(p.window_bg, ACTIVE_GLOW_ALPHA),
            // Lifted toward white on dark themes; on light ones a touch darker so it still shows.
            active_bevel: if p.window_bg.is_light() { accent.darken(0.15) } else { WHITE.over(accent, 0.35) },
            status_bg: accent,
            // Light accents (yellow, green) need dark text to stay readable.
            status_text: if accent.is_light() { rgb(0x1F1F1F) } else { WHITE },
            status_strong: accent.darken(0.22),
            status_warn: rgb(0xC27C0E),
        }
    }

    pub fn dark() -> Self {
        Self::new("Dark", &DEFAULT_ACCENT.hex())
    }

    pub fn is_light(&self) -> bool {
        self.window_bg.is_light()
    }
}

pub fn find_preset(name: &str) -> &'static Preset {
    PRESETS.iter().find(|p| p.name.eq_ignore_ascii_case(name)).unwrap_or(&PRESETS[0])
}
