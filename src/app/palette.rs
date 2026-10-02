//! The command palette's contents and filtering. Every entry comes from the menus,
//! so the palette can never offer something the menus don't (or miss something new).

use crate::model::{GridLayout, Rect};

use super::menu::{PRESETS, menu_bar};
use super::{AppState, Command, MenuAction, MenuItem};

/// Canvas a thumbnail is laid out on before scaling to a unit square (16:10, as drawn).
const PREVIEW_CANVAS: Rect = Rect { x: 0.0, y: 0.0, width: 160.0, height: 100.0 };

#[derive(Clone, Debug, PartialEq)]
pub struct PaletteEntry {
    /// "Menu: item", or "Menu › Submenu: item".
    pub label: String,
    /// The item's current hotkey, as people read it.
    pub chord: Option<String>,
    pub action: MenuAction,
    /// The layout it applies, as zone rects within a unit square, for a thumbnail.
    pub preview: Option<Vec<Rect>>,
}

/// An entry that matched the query, best first.
#[derive(Clone, Debug, PartialEq)]
pub struct PaletteMatch {
    /// Index into the entries.
    pub entry: usize,
    pub score: i32,
    /// Character positions in the label that matched, for highlighting.
    pub matched: Vec<usize>,
}

impl AppState {
    pub fn palette_entries(&self) -> Vec<PaletteEntry> {
        let mut entries = Vec::new();
        for menu in menu_bar(self) {
            flatten(menu.title, &menu.items, &mut entries);
        }
        for entry in &mut entries {
            entry.preview = match entry.action {
                MenuAction::ApplyPreset(i) => Some(preview(&(PRESETS[i].1)())),
                MenuAction::LoadWorkspace(slot) => self.workspace(slot).and_then(|ws| GridLayout::from_json(&ws.layout_json).ok()).map(|l| preview(&l)),
                _ => None,
            };
        }
        entries
    }
}

fn flatten(path: &str, items: &[MenuItem], out: &mut Vec<PaletteEntry>) {
    for item in items {
        match item {
            // The palette doesn't list itself.
            MenuItem::Item { action: MenuAction::OpenPalette | MenuAction::Run(Command::OpenPalette), .. } => {}
            MenuItem::Item { label, shortcut, enabled: true, action } => out.push(PaletteEntry {
                label: format!("{path}: {}", label.trim_end_matches("...").trim()),
                chord: shortcut.clone(),
                action: *action,
                preview: None,
            }),
            MenuItem::Submenu { label, items } => flatten(&format!("{path} \u{203A} {label}"), items, out),
            _ => {}
        }
    }
}

/// The entries matching `query`, best first; all of them, in menu order, for an empty query.
/// Every word of the query must match: as a substring if it can, else as letters in order.
pub fn filter(entries: &[PaletteEntry], query: &str) -> Vec<PaletteMatch> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut matches: Vec<PaletteMatch> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let label: Vec<char> = e.label.to_lowercase().chars().collect();
            let mut score = 0;
            let mut matched = Vec::new();
            for word in &words {
                let (s, m) = match_word(&label, word)?;
                score += s;
                matched.extend(m);
            }
            matched.sort_unstable();
            matched.dedup();
            Some(PaletteMatch { entry: i, score, matched })
        })
        .collect();
    // Stable: equal scores keep menu order.
    matches.sort_by(|a, b| b.score.cmp(&a.score));
    matches
}

/// Score and matched positions of one lowercase word in a lowercase label.
fn match_word(label: &[char], word: &str) -> Option<(i32, Vec<usize>)> {
    let word: Vec<char> = word.chars().collect();
    let at_word_start = |i: usize| i == 0 || !label[i - 1].is_alphanumeric();
    // Substrings first, preferring one that starts a word.
    let starts: Vec<usize> = (0..=label.len().saturating_sub(word.len())).filter(|&i| label[i..].starts_with(&word)).collect();
    if let Some(&start) = starts.iter().find(|&&i| at_word_start(i)).or(starts.first()) {
        let bonus = if at_word_start(start) { 20 } else { 8 };
        return Some((bonus + 3 * word.len() as i32, (start..start + word.len()).collect()));
    }
    // Otherwise the letters in order, each as early as possible.
    let mut matched = Vec::with_capacity(word.len());
    let mut from = 0;
    for c in &word {
        let i = from + label[from..].iter().position(|l| l == c)?;
        matched.push(i);
        from = i + 1;
    }
    let starts = matched.iter().filter(|&&i| at_word_start(i)).count() as i32;
    Some((word.len() as i32 + 2 * starts, matched))
}

/// A layout's zones scaled into a unit square, for drawing as a thumbnail.
pub fn preview(layout: &GridLayout) -> Vec<Rect> {
    let (w, h) = (PREVIEW_CANVAS.width, PREVIEW_CANVAS.height);
    layout.zone_rects(PREVIEW_CANVAS).into_iter().map(|z| Rect::new(z.bounds.x / w, z.bounds.y / h, z.bounds.width / w, z.bounds.height / h)).collect()
}
