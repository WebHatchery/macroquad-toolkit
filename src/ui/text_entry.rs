//! Touch-accessible single-line text editing without ownership of game state.
//!
//! Games own the text, character limit, active page, focus, and confirmation.
//! Draw [`keyboard_keys`] using the game's controls and return each key's action
//! to the dispatcher. Only the dispatcher calls [`apply_text_edit`]. This keeps
//! drawing read-only and lets touch and optional physical typing share edits.
//!
//! The three pages provide every printable ASCII character through visible keys.
//! Physical typing also accepts Unicode. Limits and Backspace use Unicode scalar
//! values, matching Rust's `chars()`; they do not count bytes or grapheme clusters.

use macroquad::prelude::{get_char_pressed, is_key_pressed, KeyCode, Rect};
use std::fmt;

const GAP: f32 = 8.0;
const MIN_KEY: f32 = 48.0;
const PREFERRED_HEIGHT: f32 = 56.0;
const COLUMNS: usize = 14;
const ROWS: usize = 4;

/// One edit at the end of a game-owned single-line value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextEdit {
    Insert(char),
    Backspace,
    Clear,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum KeyboardPage {
    #[default]
    Lowercase,
    Uppercase,
    Numbers,
}

/// A key changes text or selects a page; it never saves or confirms a dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextEntryAction {
    Edit(TextEdit),
    SetPage(KeyboardPage),
}

/// Logical-pixel geometry and a suggested label for a game-styled control.
/// Games may localize utility labels by matching `action`.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyboardKey {
    pub rect: Rect,
    pub label: String,
    pub action: TextEntryAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextEditError {
    InvalidCharacter,
    LimitReached { max_chars: usize },
}

impl fmt::Display for TextEditError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCharacter => {
                formatter.write_str("Text must stay on one line without control characters.")
            }
            Self::LimitReached { max_chars } => {
                write!(formatter, "Text is limited to {max_chars} characters.")
            }
        }
    }
}

impl std::error::Error for TextEditError {}

/// Applies one edit, returning whether text changed. Rejection preserves text.
/// Backspace and Clear remain usable even if an existing value exceeds the limit.
pub fn apply_text_edit(
    value: &mut String,
    edit: TextEdit,
    max_chars: usize,
) -> Result<bool, TextEditError> {
    match edit {
        TextEdit::Insert(character) => {
            if !single_line_character(character) {
                return Err(TextEditError::InvalidCharacter);
            }
            if value.chars().count() >= max_chars {
                return Err(TextEditError::LimitReached { max_chars });
            }
            value.push(character);
            Ok(true)
        }
        TextEdit::Backspace => Ok(value.pop().is_some()),
        TextEdit::Clear => {
            let changed = !value.is_empty();
            value.clear();
            Ok(changed)
        }
    }
}

fn single_line_character(character: char) -> bool {
    !character.is_control() && !matches!(character, '\u{2028}' | '\u{2029}')
}

/// Drains Macroquad's character queue each frame, including when unfocused.
/// Call once from application input, then dispatch the returned edits. This
/// prevents typing outside an editor from appearing when it opens later.
/// Backspace supplements the visible key; confirmation remains a game action.
pub fn read_text_edits(focused: bool) -> Vec<TextEdit> {
    let mut edits = Vec::new();
    while let Some(character) = get_char_pressed() {
        if focused && single_line_character(character) {
            edits.push(TextEdit::Insert(character));
        }
    }
    if focused && is_key_pressed(KeyCode::Backspace) {
        edits.push(TextEdit::Backspace);
    }
    edits
}

/// Builds four rows with at least 48-pixel keys and 8-pixel gaps.
///
/// All pages fit the same minimum 776 x 216 logical area. Rows use up to 56
/// pixels of height, leaving extra vertical room untouched. Preserve logical
/// scale when drawing; shrinking the result also shrinks the touch targets.
/// The final row always offers abc, ABC, 123, Space, Backspace and Clear.
pub fn keyboard_keys(bounds: Rect, page: KeyboardPage) -> Result<Vec<KeyboardKey>, String> {
    let minimum_width = COLUMNS as f32 * MIN_KEY + (COLUMNS - 1) as f32 * GAP;
    let minimum_height = ROWS as f32 * MIN_KEY + (ROWS - 1) as f32 * GAP;
    if ![
        bounds.x,
        bounds.y,
        bounds.w,
        bounds.h,
        bounds.right(),
        bounds.bottom(),
    ]
    .iter()
    .all(|value| value.is_finite())
        || bounds.w < minimum_width
        || bounds.h < minimum_height
    {
        return Err("Text keyboard needs a finite area at least 776 x 216 logical pixels.".into());
    }
    let height = ((bounds.h - (ROWS - 1) as f32 * GAP) / ROWS as f32).min(PREFERRED_HEIGHT);
    let rows = character_rows(page);
    let columns = rows
        .iter()
        .map(|row| row.chars().count())
        .max()
        .unwrap_or(1);
    let width = (bounds.w - (columns - 1) as f32 * GAP) / columns as f32;
    let mut keys = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let count = row.chars().count();
        let row_width = count as f32 * width + (count - 1) as f32 * GAP;
        let left = bounds.x + (bounds.w - row_width) / 2.0;
        let top = bounds.y + index as f32 * (height + GAP);
        for (column, character) in row.chars().enumerate() {
            keys.push(KeyboardKey {
                rect: Rect::new(left + column as f32 * (width + GAP), top, width, height),
                label: character.to_string(),
                action: TextEntryAction::Edit(TextEdit::Insert(character)),
            });
        }
    }
    utility_row(&mut keys, bounds, height);
    Ok(keys)
}

fn character_rows(page: KeyboardPage) -> [&'static str; 3] {
    match page {
        KeyboardPage::Lowercase => ["qwertyuiop", "asdfghjkl", "zxcvbnm"],
        KeyboardPage::Uppercase => ["QWERTYUIOP", "ASDFGHJKL", "ZXCVBNM"],
        KeyboardPage::Numbers => ["1234567890-_=+", "!@#$%^&*()[]{}", ";:'\",.<>/?\\|`~"],
    }
}

fn utility_row(keys: &mut Vec<KeyboardKey>, bounds: Rect, height: f32) {
    use TextEntryAction::{Edit, SetPage};
    let controls = [
        ("abc", SetPage(KeyboardPage::Lowercase), 1.0),
        ("ABC", SetPage(KeyboardPage::Uppercase), 1.0),
        ("123", SetPage(KeyboardPage::Numbers), 1.0),
        ("Space", Edit(TextEdit::Insert(' ')), 3.0),
        ("Backspace", Edit(TextEdit::Backspace), 2.0),
        ("Clear", Edit(TextEdit::Clear), 1.5),
    ];
    let total_weight: f32 = controls.iter().map(|(_, _, weight)| weight).sum();
    let unit_width = (bounds.w - (controls.len() - 1) as f32 * GAP) / total_weight;
    let mut left = bounds.x;
    let top = bounds.y + 3.0 * (height + GAP);
    for (label, action, weight) in controls {
        let width = unit_width * weight;
        keys.push(KeyboardKey {
            rect: Rect::new(left, top, width, height),
            label: label.into(),
            action,
        });
        left += width + GAP;
    }
}
