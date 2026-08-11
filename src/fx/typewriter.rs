//! Terminal-style character-by-character text reveal.
//!
//! Pure, GL-free helpers: given the full text, seconds elapsed since the reveal
//! began, and a characters-per-second rate, they return how much of the string
//! should be visible. Callers draw the returned prefix however they like (word
//! wrap, cursor, etc.). Counting is by `char`, so multi-byte UTF-8 never splits.
//!
//! A non-positive rate reveals everything immediately, which is handy for
//! disabling the effect (accessibility, screenshot capture) without branching at
//! every call site.

/// Number of `char`s of `text` visible after `elapsed` seconds at
/// `chars_per_sec`. Saturates at the full length; a non-positive rate returns
/// the full length immediately.
pub fn typed_char_count(text: &str, elapsed: f32, chars_per_sec: f32) -> usize {
    let total = text.chars().count();
    if chars_per_sec <= 0.0 {
        return total;
    }
    let shown = elapsed.max(0.0) * chars_per_sec;
    if !shown.is_finite() || shown >= total as f32 {
        return total;
    }
    shown.floor() as usize
}

/// The first `n` `char`s of `text` as a slice on a valid `char` boundary,
/// saturating at the full string. Pairs with a visible-count from
/// [`typed_char_count`] or [`BlockReveal::shown`].
pub fn prefix_chars(text: &str, n: usize) -> &str {
    match text.char_indices().nth(n) {
        Some((byte_idx, _)) => &text[..byte_idx],
        None => text,
    }
}

/// The visible prefix of `text` — always a valid slice on a `char` boundary.
pub fn typed_prefix(text: &str, elapsed: f32, chars_per_sec: f32) -> &str {
    prefix_chars(text, typed_char_count(text, elapsed, chars_per_sec))
}

/// True once every `char` of `text` is visible.
pub fn is_fully_typed(text: &str, elapsed: f32, chars_per_sec: f32) -> bool {
    typed_char_count(text, elapsed, chars_per_sec) >= text.chars().count()
}

/// Per-line reveal state for a block of text streamed as one shared character
/// budget — a terminal boot log, scrolling console output, a killfeed. Unlike
/// [`typed_char_count`], which reveals each string independently, the budget is
/// continuous: line `N` only starts revealing once every `char` of lines
/// `0..N` is visible, so the block reads as one uninterrupted stream. Build it
/// with [`reveal_block`].
#[derive(Debug, Clone)]
pub struct BlockReveal {
    /// Visible `char` count for each input line, in input order.
    pub shown: Vec<usize>,
    /// The line the write-cursor sits on: the first not-yet-complete line, or
    /// the last line once the block is fully revealed. `0` when `lines` is
    /// empty. Draw a blinking cursor at the end of this line's visible prefix
    /// and it parks at the stream head while typing, then at the final glyph.
    pub cursor_line: usize,
    /// True once every `char` of every line is visible.
    pub complete: bool,
}

/// Compute the [`BlockReveal`] for `lines` after `elapsed` seconds at
/// `chars_per_sec`. The budget is shared across lines (see [`BlockReveal`]);
/// empty lines cost nothing and are stepped over instantly. A non-positive
/// rate reveals everything immediately.
pub fn reveal_block(lines: &[&str], elapsed: f32, chars_per_sec: f32) -> BlockReveal {
    let counts: Vec<usize> = lines.iter().map(|l| l.chars().count()).collect();
    let total: usize = counts.iter().sum();
    let mut budget = if chars_per_sec <= 0.0 {
        total
    } else {
        let shown = elapsed.max(0.0) * chars_per_sec;
        if !shown.is_finite() || shown >= total as f32 {
            total
        } else {
            shown.floor() as usize
        }
    };
    let mut shown = Vec::with_capacity(lines.len());
    for &n in &counts {
        let take = budget.min(n);
        budget -= take;
        shown.push(take);
    }
    let complete = shown.iter().zip(&counts).all(|(s, n)| s >= n);
    let cursor_line = if complete {
        lines.len().saturating_sub(1)
    } else {
        shown
            .iter()
            .zip(&counts)
            .position(|(s, n)| s < n)
            .unwrap_or(0)
    };
    BlockReveal {
        shown,
        cursor_line,
        complete,
    }
}

#[cfg(test)]
mod tests;
