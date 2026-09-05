use super::*;

fn width(text: &str) -> f32 {
    text.chars()
        .map(|ch| if ch.is_ascii() { 5.0 } else { 12.0 })
        .sum()
}

#[test]
fn paths_keep_spaces_unicode_and_every_original_byte() {
    let path = "C:\\Users\\Tester  Name\\存档\\saved camp.json";
    let lines = wrap_literal_text_with_measure(path, 37.0, width);
    assert_eq!(lines.concat(), path);
    assert!(lines.iter().all(|line| width(line) <= 37.0));
}

#[test]
fn exact_fit_empty_and_narrow_widths_do_not_drop_text() {
    assert_eq!(wrap_literal_text_with_measure("abc", 15.0, width), ["abc"]);
    assert_eq!(wrap_literal_text_with_measure("", 15.0, width), [""]);
    assert_eq!(
        wrap_literal_text_with_measure("存a", 1.0, width),
        ["存", "a"]
    );
}
