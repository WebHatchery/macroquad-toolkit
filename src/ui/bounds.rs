//! Knowing whether text fitted, without asking every call site.
//!
//! # Four defects, all found by eye
//!
//! Text that runs past its panel is the one UI fault that no ordinary test sees.
//! The draw call succeeds, the frame renders, nothing is out of range — the
//! sentence is simply cut off, or drawn over the thing beside it. It is found by
//! looking, which means it is found late and only if someone looks at that
//! screen.
//!
//! # Bound the region, not the call
//!
//! `draw_ui_text_ex` takes a position and no width, and there are a hundred of
//! them in a game of any size. Giving every one an explicit box is a hundred
//! edits and a hundred chances to write the wrong number.
//!
//! But text is always drawn *inside something* — a panel, a row, a column — and
//! that something already knows how wide it is, because it was drawn from a
//! `Rect`. So a [`Region`] pushes those bounds for as long as it is alive, and
//! every text draw inside compares what it measured against what it had. One
//! guard per panel covers everything in it, including code written later that
//! never heard of this module.
//!
//! # Recording rather than clipping
//!
//! Nothing here changes what is drawn. Overflowing text still overflows, because
//! silently shrinking or truncating it would replace a visible bug with an
//! invisible one — a sentence quietly losing its last three words is worse than
//! one that obviously collides. The audit **reports**, and the fix is a decision
//! someone makes about that specific piece of text.
//!
//! Recording is off unless [`begin_audit`] has been called, so a shipped frame
//! pays for one thread-local read per text draw and nothing else.

use macroquad::prelude::*;
use std::cell::RefCell;

/// One piece of text that did not fit where it was drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct Overflow {
    pub text: String,
    /// How wide the text measured.
    pub needed: f32,
    /// How much room it had from its position to the edge of its region.
    pub available: f32,
}

impl Overflow {
    pub fn excess(&self) -> f32 {
        self.needed - self.available
    }
}

#[derive(Default)]
struct State {
    stack: Vec<Rect>,
    auditing: bool,
    overflows: Vec<Overflow>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// Bounds that text drawn inside them is measured against.
///
/// An RAII guard: the region is in force until it is dropped, so an early
/// `return` out of a draw function cannot leave the stack unbalanced — which it
/// certainly would, because half the panels in a game return early when they
/// have nothing to show.
pub struct Region {
    _private: (),
}

impl Region {
    pub fn new(rect: Rect) -> Self {
        STATE.with(|state| state.borrow_mut().stack.push(rect));
        Self { _private: () }
    }

    /// A region inset from the current one, for a row or a column inside a
    /// panel. Falls back to `rect` when there is nothing to inset from.
    pub fn inset(rect: Rect) -> Self {
        Self::new(current().map_or(rect, |outer| intersect(outer, rect)))
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        STATE.with(|state| {
            state.borrow_mut().stack.pop();
        });
    }
}

fn intersect(a: Rect, b: Rect) -> Rect {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    Rect::new(
        x,
        y,
        (a.right().min(b.right()) - x).max(0.0),
        (a.bottom().min(b.bottom()) - y).max(0.0),
    )
}

/// The innermost region, if any.
pub fn current() -> Option<Rect> {
    STATE.with(|state| state.borrow().stack.last().copied())
}

/// Start recording. Clears anything already recorded.
pub fn begin_audit() {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.auditing = true;
        state.overflows.clear();
    });
}

/// Stop recording and take what was found.
pub fn take_audit() -> Vec<Overflow> {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.auditing = false;
        std::mem::take(&mut state.overflows)
    })
}

pub fn auditing() -> bool {
    STATE.with(|state| state.borrow().auditing)
}

/// Note a piece of text drawn at `x` that measured `width` wide.
///
/// Called by the text helpers. A draw outside any region is not a finding: the
/// reels and the celebration cards paint over the whole screen on purpose.
pub fn note(text: &str, x: f32, width: f32) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if !state.auditing {
            return;
        }
        let Some(region) = state.stack.last().copied() else {
            return;
        };
        let available = region.right() - x;
        // A hair over is rounding, not a defect. Anything a reader would notice
        // is at least a character wide.
        if width > available + 2.0 {
            let overflow = Overflow {
                text: text.to_owned(),
                needed: width,
                available,
            };
            if !state.overflows.contains(&overflow) {
                state.overflows.push(overflow);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel() -> Rect {
        Rect::new(100.0, 50.0, 400.0, 300.0)
    }

    #[test]
    fn nothing_is_recorded_until_the_audit_starts() {
        let _ = take_audit();
        let _region = Region::new(panel());
        note("far too wide for this", 110.0, 9_000.0);
        assert!(take_audit().is_empty());
    }

    #[test]
    fn text_that_fits_is_not_a_finding() {
        begin_audit();
        let _region = Region::new(panel());
        note("short", 110.0, 60.0);
        assert!(take_audit().is_empty());
    }

    #[test]
    fn text_past_the_edge_is_recorded_with_how_far() {
        begin_audit();
        {
            let _region = Region::new(panel());
            // 110 + 500 runs 110 past a right edge of 500.
            note("a long line", 110.0, 500.0);
        }
        let found = take_audit();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].text, "a long line");
        assert!((found[0].available - 390.0).abs() < 1e-3);
        assert!((found[0].excess() - 110.0).abs() < 1e-3);
    }

    #[test]
    fn a_hair_over_is_rounding_rather_than_a_defect() {
        begin_audit();
        let _region = Region::new(panel());
        note("just about", 110.0, 391.0);
        assert!(take_audit().is_empty());
    }

    #[test]
    fn a_draw_outside_every_region_is_not_a_finding() {
        // The reels and the celebration cards paint the whole screen on purpose.
        begin_audit();
        note("full bleed", 0.0, 5_000.0);
        assert!(take_audit().is_empty());
    }

    #[test]
    fn the_innermost_region_is_the_one_that_counts() {
        begin_audit();
        let _outer = Region::new(panel());
        {
            let _inner = Region::new(Rect::new(100.0, 50.0, 120.0, 40.0));
            note("inside the column", 110.0, 200.0);
        }
        // And the outer bound applies again once the inner one is gone.
        note("inside the panel", 110.0, 200.0);

        let found = take_audit();
        assert_eq!(found.len(), 1, "{:?}", found);
        assert_eq!(found[0].text, "inside the column");
    }

    #[test]
    fn a_region_is_popped_even_when_the_panel_returns_early() {
        // Half the panels in a game bail out when they have nothing to show. A
        // stack that leaked would bound every later draw by a dead panel.
        fn draws_nothing() {
            let _region = Region::new(panel());
            #[allow(clippy::needless_return)]
            return;
        }
        let before = STATE.with(|state| state.borrow().stack.len());
        draws_nothing();
        assert_eq!(STATE.with(|state| state.borrow().stack.len()), before);
        assert!(current().is_none());
    }

    #[test]
    fn an_inset_region_is_clipped_to_the_one_around_it() {
        let _outer = Region::new(panel());
        // Asking for more room than the panel has must not grant it.
        let _inner = Region::inset(Rect::new(100.0, 50.0, 9_000.0, 40.0));
        assert!((current().unwrap().right() - panel().right()).abs() < 1e-3);
    }

    #[test]
    fn an_inset_with_no_region_around_it_stands_alone() {
        let _ = take_audit();
        let rect = Rect::new(0.0, 0.0, 80.0, 20.0);
        let _inner = Region::inset(rect);
        assert!((current().unwrap().w - 80.0).abs() < 1e-3);
    }

    #[test]
    fn the_same_overflow_is_reported_once_rather_than_every_frame() {
        // The audit runs while the game is drawing, which redraws everything
        // sixty times a second.
        begin_audit();
        let _region = Region::new(panel());
        for _ in 0..60 {
            note("a long line", 110.0, 500.0);
        }
        assert_eq!(take_audit().len(), 1);
    }

    #[test]
    fn taking_the_audit_stops_it() {
        begin_audit();
        let _region = Region::new(panel());
        note("a long line", 110.0, 500.0);
        assert_eq!(take_audit().len(), 1);

        note("another long line", 110.0, 500.0);
        assert!(take_audit().is_empty());
        assert!(!auditing());
    }
}
