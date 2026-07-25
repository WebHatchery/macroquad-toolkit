//! Pseudolocalisation: finding out what translation will break, before there is
//! any translation.
//!
//! # The expensive part of localising is not the words
//!
//! Translating a game is a bounded job someone can be paid to do. What is not
//! bounded is discovering, afterwards, that half the panels were laid out to the
//! exact width of their English text — because German runs about 35% longer,
//! and the fix is a redesign rather than a retranslation.
//!
//! Pseudolocalisation is the standard answer and it does not need a translator.
//! Every string is transformed on its way to the screen into something that is
//! still readable as English but has the properties translated text has:
//!
//! - **Longer.** Padded by [`Pseudo::expansion`], so a panel that only just fits
//!   its own copy fails immediately rather than in the German build.
//! - **Accented.** `Settings` becomes `Śéttíñgś`, which is legible enough to
//!   navigate by and instantly shows a font that has no glyph for `é` — a
//!   missing glyph draws as a blank or a box, and finding that after shipping a
//!   language is finding it late.
//! - **Bracketed.** `[Śéttíñgś··]` marks the whole string, so **a label built by
//!   gluing two strings together shows up as `[..][..]`** and can be fixed. That
//!   is the fault a translator cannot work around: word order is not universal,
//!   and a sentence assembled from fragments cannot be reordered.
//!
//! Anything the brackets do not touch never went through the text helpers at
//! all, which is its own finding.
//!
//! # Not a locale
//!
//! This ships no translations and claims none. It is a **measurement**, run with
//! the layout audit to produce a list of what would have to change; the game
//! stays in English until someone who speaks the target language writes it.

use std::cell::RefCell;

/// How pseudolocalised text is built.
#[derive(Debug, Clone, Copy)]
pub struct Pseudo {
    /// Fraction of the original length added as padding. 0.35 is the usual
    /// figure for English into German or Finnish; 0.4 leaves a margin.
    pub expansion: f32,
    /// Wrap each string in markers, so a glued-together label is visible.
    pub brackets: bool,
    /// Substitute accented look-alikes.
    pub accents: bool,
}

impl Default for Pseudo {
    fn default() -> Self {
        Self {
            expansion: 0.4,
            brackets: true,
            accents: true,
        }
    }
}

thread_local! {
    static ACTIVE: RefCell<Option<Pseudo>> = const { RefCell::new(None) };
    /// Nesting depth; expansion happens only at depth zero.
    static DEPTH: RefCell<u32> = const { RefCell::new(0) };
}

pub fn enable(pseudo: Pseudo) {
    ACTIVE.with(|slot| *slot.borrow_mut() = Some(pseudo));
}

pub fn disable() {
    ACTIVE.with(|slot| *slot.borrow_mut() = None);
}

pub fn active() -> Option<Pseudo> {
    ACTIVE.with(|slot| *slot.borrow())
}

/// Transform a string for display, or hand it back untouched.
///
/// Returns `Cow`-like ownership only when it has to: with pseudolocalisation off
/// this is one thread-local read and no allocation, which is what lets the text
/// helpers call it unconditionally.
pub fn apply(text: &str) -> Option<String> {
    if DEPTH.with(|depth| *depth.borrow()) > 0 {
        return None;
    }
    let pseudo = active()?;
    Some(transform(text, pseudo))
}

/// Suppress further expansion for as long as it is alive.
///
/// A block of text is laid out once and then drawn a line at a time, and both
/// steps go through helpers that expand. Without this the first line comes out
/// `[[doubly marked]]` and every line is padded twice, which measures a width
/// no translation would ever produce. The unit that gets expanded is the whole
/// block, so the guard spans layout *and* drawing.
pub struct Once;

impl Once {
    pub fn new() -> Self {
        DEPTH.with(|depth| *depth.borrow_mut() += 1);
        Self
    }
}

impl Default for Once {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Once {
    fn drop(&mut self) {
        DEPTH.with(|depth| *depth.borrow_mut() -= 1);
    }
}

pub fn transform(text: &str, pseudo: Pseudo) -> String {
    // A placeholder that has already been substituted cannot be protected, but
    // the format string itself often reaches here intact. Leaving `{}` alone
    // keeps a caller that formats *after* this from breaking.
    let mut out = String::with_capacity(text.len() * 2 + 4);
    if pseudo.brackets {
        out.push('[');
    }

    let mut in_placeholder = false;
    for ch in text.chars() {
        match ch {
            '{' => {
                in_placeholder = true;
                out.push(ch);
            }
            '}' => {
                in_placeholder = false;
                out.push(ch);
            }
            _ if in_placeholder || !pseudo.accents => out.push(ch),
            _ => out.push(accent(ch)),
        }
    }

    // Padding after the text rather than inside it, so the words stay readable
    // and the growth is still measured. A middle dot is in every Latin font
    // worth using and cannot be mistaken for content.
    let letters = text.chars().filter(|ch| ch.is_alphanumeric()).count();
    let pad = ((letters as f32 * pseudo.expansion).round() as usize).max(1);
    for _ in 0..pad {
        out.push('\u{00B7}');
    }
    if pseudo.brackets {
        out.push(']');
    }
    out
}

/// A look-alike with a diacritic. Only Latin letters are touched; digits and
/// punctuation are left alone so a figure stays a figure.
fn accent(ch: char) -> char {
    match ch {
        'a' => 'á',
        'e' => 'é',
        'i' => 'í',
        'o' => 'ó',
        'u' => 'ú',
        'n' => 'ñ',
        'c' => 'ç',
        's' => 'ś',
        'y' => 'ý',
        'A' => 'Á',
        'E' => 'É',
        'I' => 'Í',
        'O' => 'Ó',
        'U' => 'Ú',
        'N' => 'Ñ',
        'C' => 'Ç',
        'S' => 'Ś',
        'Y' => 'Ý',
        other => other,
    }
}

#[cfg(test)]
mod tests {
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
}
