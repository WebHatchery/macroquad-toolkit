//! Shared text edits and visible-key geometry without a graphics context.

use macroquad::prelude::Rect;
use macroquad_toolkit::ui::text_entry::{
    apply_text_edit, keyboard_keys, KeyboardPage, TextEdit, TextEditError, TextEntryAction,
};
use std::collections::BTreeSet;

const PAGES: [KeyboardPage; 3] = [
    KeyboardPage::Lowercase,
    KeyboardPage::Uppercase,
    KeyboardPage::Numbers,
];

#[test]
fn inserts_and_limits_count_unicode_characters_without_changing_rejected_text() {
    let mut value = String::new();
    for character in ['A', 'é', '梅', '🌿'] {
        assert!(apply_text_edit(&mut value, TextEdit::Insert(character), 4).unwrap());
    }
    assert_eq!(value, "Aé梅🌿");
    assert_eq!(value.chars().count(), 4);
    for character in ['x', '界', ' '] {
        let before = value.clone();
        assert_eq!(
            apply_text_edit(&mut value, TextEdit::Insert(character), 4),
            Err(TextEditError::LimitReached { max_chars: 4 })
        );
        assert_eq!(value, before);
    }
    let mut empty = String::new();
    assert!(apply_text_edit(&mut empty, TextEdit::Insert('a'), 0).is_err());
    assert!(empty.is_empty());
}

#[test]
fn deletion_remains_available_and_single_line_edits_preserve_meaningful_spaces() {
    let mut value = "Aé梅🌿".to_owned();
    for expected in ["Aé梅", "Aé", "A", ""] {
        assert!(apply_text_edit(&mut value, TextEdit::Backspace, 0).unwrap());
        assert_eq!(value, expected);
    }
    assert!(!apply_text_edit(&mut value, TextEdit::Backspace, 0).unwrap());
    for character in " Name ".chars() {
        apply_text_edit(&mut value, TextEdit::Insert(character), 20).unwrap();
    }
    assert_eq!(value, " Name ");
    for invalid in ['\0', '\t', '\n', '\r', '\u{7f}', '\u{2028}', '\u{2029}'] {
        let before = value.clone();
        assert_eq!(
            apply_text_edit(&mut value, TextEdit::Insert(invalid), 20),
            Err(TextEditError::InvalidCharacter)
        );
        assert_eq!(value, before);
    }
    assert!(apply_text_edit(&mut value, TextEdit::Clear, 0).unwrap());
    assert!(!apply_text_edit(&mut value, TextEdit::Clear, 0).unwrap());
}

#[test]
fn every_printable_ascii_character_and_edit_action_is_available_by_touch() {
    let bounds = Rect::new(154.0, 280.0, 972.0, 290.0);
    let mut available = BTreeSet::new();
    for page in PAGES {
        let keys = keyboard_keys(bounds, page).unwrap();
        for next_page in PAGES {
            assert!(keys
                .iter()
                .any(|key| key.action == TextEntryAction::SetPage(next_page)));
        }
        for edit in [TextEdit::Backspace, TextEdit::Clear, TextEdit::Insert(' ')] {
            assert!(keys
                .iter()
                .any(|key| key.action == TextEntryAction::Edit(edit)));
        }
        for key in keys {
            if let TextEntryAction::Edit(TextEdit::Insert(character)) = key.action {
                available.insert(character);
            }
        }
    }
    assert_eq!(available, (' '..='~').collect());
    let mut value = String::new();
    for character in "Rosemarch 42's (West)!".chars() {
        let action = PAGES
            .into_iter()
            .flat_map(|page| keyboard_keys(bounds, page).unwrap())
            .find(|key| key.action == TextEntryAction::Edit(TextEdit::Insert(character)))
            .unwrap()
            .action;
        let TextEntryAction::Edit(edit) = action else {
            panic!("character key must edit");
        };
        apply_text_edit(&mut value, edit, 32).unwrap();
    }
    assert_eq!(value, "Rosemarch 42's (West)!");
    assert_eq!(KeyboardPage::default(), KeyboardPage::Lowercase);
}

#[test]
fn keyboard_targets_stay_large_inside_their_area_and_never_overlap() {
    for bounds in [
        Rect::new(154.0, 280.0, 972.0, 290.0),
        Rect::new(10.0, 20.0, 776.0, 216.0),
    ] {
        for page in PAGES {
            let keys = keyboard_keys(bounds, page).unwrap();
            for (index, key) in keys.iter().enumerate() {
                assert!(key.rect.w >= 48.0 && key.rect.h >= 48.0);
                assert!(key.rect.x >= bounds.x && key.rect.y >= bounds.y);
                assert!(key.rect.right() <= bounds.right() + 0.001);
                assert!(key.rect.bottom() <= bounds.bottom() + 0.001);
                for other in &keys[index + 1..] {
                    assert!(
                        !key.rect.overlaps(&other.rect),
                        "{} overlaps {}",
                        key.label,
                        other.label
                    );
                }
            }
        }
    }
}

#[test]
fn keyboard_refuses_areas_that_would_hide_or_shrink_touch_controls() {
    for bounds in [
        Rect::new(0.0, 0.0, 775.0, 216.0),
        Rect::new(0.0, 0.0, 776.0, 215.0),
        Rect::new(0.0, 0.0, -1.0, 300.0),
        Rect::new(f32::NAN, 0.0, 972.0, 290.0),
        Rect::new(0.0, f32::INFINITY, 972.0, 290.0),
        Rect::new(0.0, 0.0, f32::INFINITY, 290.0),
        Rect::new(0.0, 0.0, 972.0, f32::NAN),
    ] {
        for page in PAGES {
            assert!(keyboard_keys(bounds, page).is_err());
        }
    }
}
