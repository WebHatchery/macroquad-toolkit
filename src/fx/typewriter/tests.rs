use super::*;

#[test]
fn reveals_progressively_then_saturates() {
    let text = "hello world";
    assert_eq!(typed_char_count(text, 0.0, 10.0), 0);
    assert_eq!(typed_char_count(text, 0.5, 10.0), 5);
    assert_eq!(typed_char_count(text, 100.0, 10.0), text.len());
    assert!(is_fully_typed(text, 100.0, 10.0));
}

#[test]
fn non_positive_rate_reveals_all_immediately() {
    let text = "instant";
    assert_eq!(typed_char_count(text, 0.0, 0.0), 7);
    assert_eq!(typed_prefix(text, 0.0, -1.0), "instant");
    assert!(is_fully_typed(text, 0.0, 0.0));
}

#[test]
fn prefix_respects_char_boundaries() {
    // Each accented glyph is multi-byte; the prefix must stay valid UTF-8.
    let text = "café⚙ok";
    for e in 0..20 {
        let p = typed_prefix(text, e as f32 * 0.1, 5.0);
        assert!(text.starts_with(p), "prefix {p:?} not a prefix of {text:?}");
    }
    assert_eq!(typed_prefix(text, 0.6, 5.0), "caf");
}

#[test]
fn empty_text_is_always_done() {
    assert_eq!(typed_char_count("", 0.0, 10.0), 0);
    assert!(is_fully_typed("", 0.0, 10.0));
    assert_eq!(typed_prefix("", 5.0, 10.0), "");
}

#[test]
fn block_streams_budget_across_lines() {
    let lines = ["abc", "de", "fghi"]; // 3 + 2 + 4 = 9 chars
                                       // 5 chars in: first line full, 2 into the second.
    let r = reveal_block(&lines, 0.5, 10.0);
    assert_eq!(r.shown, vec![3, 2, 0]);
    assert_eq!(r.cursor_line, 2); // second line just completed; head is line 3's start
    assert!(!r.complete);

    // Mid-first-line: cursor sits on line 0, later lines untouched.
    let r = reveal_block(&lines, 0.2, 10.0);
    assert_eq!(r.shown, vec![2, 0, 0]);
    assert_eq!(r.cursor_line, 0);
}

#[test]
fn block_completes_and_parks_cursor_on_last_line() {
    let lines = ["one", "two"];
    let r = reveal_block(&lines, 100.0, 10.0);
    assert_eq!(r.shown, vec![3, 3]);
    assert!(r.complete);
    assert_eq!(r.cursor_line, 1);

    // Non-positive rate reveals everything at t=0.
    let instant = reveal_block(&lines, 0.0, 0.0);
    assert!(instant.complete);
    assert_eq!(instant.shown, vec![3, 3]);
}

#[test]
fn block_skips_empty_lines() {
    // A blank separator costs no budget and never holds the cursor.
    let lines = ["ab", "", "cd"];
    let r = reveal_block(&lines, 0.3, 10.0); // 3 chars in
    assert_eq!(r.shown, vec![2, 0, 1]);
    assert_eq!(r.cursor_line, 2);
    assert!(!r.complete);
}

#[test]
fn block_prefix_stays_on_char_boundaries() {
    let lines = ["café", "⚙ok"];
    let r = reveal_block(&lines, 0.6, 5.0); // 3 chars in
    assert_eq!(prefix_chars(lines[0], r.shown[0]), "caf");
    assert_eq!(prefix_chars(lines[1], r.shown[1]), "");
}

#[test]
fn empty_block_has_valid_cursor() {
    let r = reveal_block(&[], 1.0, 10.0);
    assert!(r.complete);
    assert_eq!(r.cursor_line, 0);
    assert!(r.shown.is_empty());
}
