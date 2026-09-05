//! Measured literal lines, for paths and identifiers where whitespace is data.
use super::{measure_text_size, TextStyle};

/// Wrap without deleting or normalizing any characters. Concatenating the lines
/// reproduces the input exactly. An individual glyph wider than the budget is
/// retained on its own line rather than lost.
pub fn wrap_literal_text(text: &str, width: f32, style: TextStyle<'_>) -> Vec<String> {
    wrap_literal_text_with_measure(text, width, |line| measure_text_size(line, style).width)
}

/// Pure counterpart for alternate text renderers and deterministic layout checks.
pub fn wrap_literal_text_with_measure(
    text: &str,
    width: f32,
    measure: impl Fn(&str) -> f32,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        let mut candidate = current.clone();
        candidate.push(character);
        if !current.is_empty() && measure(&candidate) > width.max(0.0) {
            lines.push(std::mem::take(&mut current));
        }
        current.push(character);
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests;
