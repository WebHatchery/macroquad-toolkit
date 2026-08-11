use super::*;

fn plain() -> Pseudo {
    Pseudo {
        expansion: 0.4,
        brackets: true,
        accents: true,
    }
}

#[test]
fn nothing_happens_until_it_is_enabled() {
    disable();
    assert_eq!(apply("Settings"), None);
    enable(plain());
    assert!(apply("Settings").is_some());
    disable();
}

#[test]
fn the_text_grows_by_about_the_expansion() {
    // The whole point: a panel laid out to the exact width of its English
    // copy has to fail here rather than in the German build.
    let out = transform("Session Limits", plain());
    let letters = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).count();
    assert!(out.chars().count() > "Session Limits".chars().count());
    // 13 letters, so 5 dots of padding plus two brackets.
    assert_eq!(letters(&out), letters("Session Limits"));
    assert_eq!(out.matches('\u{00B7}').count(), 5);
}

#[test]
fn the_words_stay_readable() {
    // Legible enough to navigate by. Text nobody can read is text nobody
    // reports a layout problem in.
    let out = transform("Paytable", plain());
    assert!(out.contains("Páýtáblé"));
}

#[test]
fn every_string_is_bracketed_so_a_glued_label_is_obvious() {
    // The fault a translator cannot work around: word order is not
    // universal, and a sentence assembled from fragments cannot be
    // reordered. Two brackets in a row is the tell.
    let glued = format!(
        "{}{}",
        transform("Level ", plain()),
        transform("5", plain())
    );
    assert!(glued.contains("]["), "{}", glued);
}

#[test]
fn placeholders_are_left_alone() {
    // `{}` mangled into `{}` with an accent would stop formatting and panic
    // at the call site rather than showing a layout problem.
    let out = transform("Balance {}", plain());
    assert!(out.contains("{}"), "{}", out);
    // Padding after the placeholder is fine — it is the tail of the string.
    // What must never happen is padding or an accent landing *between* the
    // braces, which would stop the format and panic at the call site.
    let inside: String = out
        .split('{')
        .skip(1)
        .filter_map(|part| part.split('}').next())
        .collect();
    assert!(inside.is_empty(), "placeholder contains {:?}", inside);
}

#[test]
fn named_and_formatted_placeholders_survive() {
    for source in ["{:.1}% of play", "{name} won {:>4}", "{{literal}}"] {
        let out = transform(source, plain());
        let braces = |s: &str| s.matches(['{', '}']).count();
        assert_eq!(braces(&out), braces(source), "{} -> {}", source, out);
    }
}

#[test]
fn digits_stay_digits() {
    // A figure that came out accented would read as a different number.
    let out = transform("1,009,419 credits", plain());
    assert!(out.contains("1,009,419"), "{}", out);
}

#[test]
fn expansion_happens_once_however_many_helpers_are_involved() {
    // A block is laid out once and drawn a line at a time, and both steps
    // go through helpers that expand. Doubly marked text measures a width
    // no translation would ever produce.
    enable(plain());
    let outer = apply("Session Limits").unwrap();
    {
        let _once = Once::new();
        assert_eq!(apply(&outer), None);
        assert_eq!(apply("anything at all"), None);
    }
    // And it comes back afterwards.
    assert!(apply("Session Limits").is_some());
    disable();
}

#[test]
fn the_guard_nests() {
    enable(plain());
    {
        let _a = Once::new();
        {
            let _b = Once::new();
            assert_eq!(apply("x"), None);
        }
        // Still suppressed: the outer guard is alive.
        assert_eq!(apply("x"), None);
    }
    assert!(apply("x").is_some());
    disable();
}

#[test]
fn an_empty_string_still_gets_a_marker() {
    // So a label that turned out to be empty is visible as empty rather
    // than as absent.
    let out = transform("", plain());
    assert!(out.starts_with('['));
    assert!(out.ends_with(']'));
}

#[test]
fn the_transform_can_be_turned_down() {
    // Accents off is for a font with no diacritics; brackets off is for a
    // screenshot someone wants to read.
    let quiet = Pseudo {
        expansion: 0.4,
        brackets: false,
        accents: false,
    };
    let out = transform("Settings", quiet);
    assert!(out.starts_with("Settings"), "{}", out);
    assert!(!out.contains('['));
}

#[test]
fn expansion_of_zero_still_marks_the_string() {
    // Zero expansion is for hunting glued labels without the width noise.
    let out = transform(
        "Ok",
        Pseudo {
            expansion: 0.0,
            ..plain()
        },
    );
    assert!(out.starts_with('[') && out.ends_with(']'));
}
