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

/// Something wrong with a piece of text where it was drawn.
#[derive(Debug, Clone, PartialEq)]
pub enum Finding {
    /// It ran past the edge of its region.
    Overflow {
        text: String,
        needed: f32,
        available: f32,
    },
    /// It cannot be read off what it was drawn on (see `ui::contrast`).
    LowContrast {
        text: String,
        ratio: f32,
        required: f32,
        font_size: f32,
    },
    /// It is drawn over something else in the same region (§5.47).
    Collision {
        text: String,
        /// What it ran into — another string, or a control it does not belong to.
        other: String,
        overlap: f32,
    },
}

impl Finding {
    pub fn text(&self) -> &str {
        match self {
            Finding::Overflow { text, .. }
            | Finding::LowContrast { text, .. }
            | Finding::Collision { text, .. } => text,
        }
    }

    /// A one-line description, for a report.
    pub fn describe(&self) -> String {
        match self {
            Finding::Overflow {
                needed, available, ..
            } => format!("{:.0}px past the edge", needed - available),
            Finding::LowContrast {
                ratio,
                required,
                font_size,
                ..
            } => format!(
                "contrast {:.1}:1 against {:.1} needed at {:.0}px",
                ratio, required, font_size
            ),
            Finding::Collision { other, overlap, .. } => {
                format!("overlaps {:?} by {:.0}px²", other, overlap)
            }
        }
    }
}

/// A bounded area, and what is behind it.
#[derive(Debug, Clone, Copy)]
struct Bounds {
    rect: Rect,
    /// What text drawn here sits on. `None` where the caller did not say, in
    /// which case contrast cannot be judged and is not guessed at.
    behind: Option<Color>,
}

/// Something occupying space on screen, for the collision check.
#[derive(Debug, Clone)]
struct Drawn {
    rect: Rect,
    what: String,
    /// A control rather than a string. Text sitting wholly inside one is its
    /// label and is not a collision.
    is_control: bool,
}

#[derive(Default)]
struct State {
    stack: Vec<Bounds>,
    auditing: bool,
    findings: Vec<Finding>,
    drawn: Vec<Drawn>,
    /// Collisions are only meaningful on a screen the game can actually show.
    /// The all-panels audit scene opens twelve overlays at once and they
    /// genuinely overlap, which says nothing about the game (§5.47).
    collisions: bool,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
    /// Nesting depth of [`Decorative`]; contrast is skipped above zero.
    static DECORATIVE: RefCell<u32> = const { RefCell::new(0) };
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
    /// Bounds only. Text drawn here is measured for fit but not for contrast,
    /// because nothing has said what it sits on.
    pub fn new(rect: Rect) -> Self {
        Self::push(rect, None)
    }

    /// Bounds and the surface behind them, so text is checked for both fit and
    /// legibility (`ui::contrast`).
    pub fn on(rect: Rect, behind: Color) -> Self {
        Self::push(rect, Some(behind))
    }

    /// Has this surface hidden that text, or cut across it?
    ///
    /// Both are "partly covered", and only one is a fault. A panel whose left
    /// edge lands mid-label hides the tail of it and shows whole glyphs up to
    /// the edge — ordinary layering, and reporting it buries the real findings
    /// in noise. A panel whose *top or bottom* edge lands inside the line cuts
    /// every glyph in half, and the player reads a row of severed letters.
    ///
    /// Centring was the old proxy for this and got both wrong: a title sliced
    /// through the middle by an overlay went unreported for as long as its
    /// centre stayed outside the panel, which is exactly how three overlays
    /// came to be drawn across the cabinet name.
    fn hidden_by(surface: Rect, text: Rect) -> bool {
        let touches = surface.x < text.right()
            && surface.right() > text.x
            && surface.y < text.bottom()
            && surface.bottom() > text.y;
        if !touches {
            return false;
        }
        let cuts_across = (surface.y > text.y && surface.y < text.bottom())
            || (surface.bottom() > text.y && surface.bottom() < text.bottom());
        !cuts_across
    }

    fn push(rect: Rect, behind: Option<Color>) -> Self {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            // A region that says what is behind it has painted a surface, and
            // that surface hides whatever was already there. Without this, an
            // overlay panel's text "collides" with the wager panel underneath
            // it — which it does not, because the player cannot see the wager
            // panel at all (§5.47).
            if behind.is_some() {
                super::pointer::occlude(rect);
                state.drawn.retain(|d| !Self::hidden_by(rect, d.rect));
            }
            state.stack.push(Bounds { rect, behind });
        });
        Self { _private: () }
    }

    /// A region inset from the current one, for a row or a column inside a
    /// panel. Inherits the surface unless told otherwise, since a row inside a
    /// panel sits on the panel.
    pub fn inset(rect: Rect) -> Self {
        let outer = STATE.with(|state| state.borrow().stack.last().copied());
        match outer {
            Some(outer) => Self::push(intersect(outer.rect, rect), outer.behind),
            None => Self::push(rect, None),
        }
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

/// The innermost region's bounds, if any.
pub fn current() -> Option<Rect> {
    STATE.with(|state| state.borrow().stack.last().map(|bounds| bounds.rect))
}

/// What the innermost region sits on, if it said.
pub fn current_surface() -> Option<Color> {
    STATE.with(|state| state.borrow().stack.last().and_then(|bounds| bounds.behind))
}

/// Start recording. Clears anything already recorded.
pub fn begin_audit() {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.auditing = true;
        state.findings.clear();
        state.drawn.clear();
        state.collisions = false;
    });
}

/// Stop recording and take what was found.
pub fn take_audit() -> Vec<Finding> {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.auditing = false;
        std::mem::take(&mut state.findings)
    })
}

fn record(finding: Finding) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        // The audit runs while the game is drawing, which redraws everything
        // sixty times a second.
        if !state.findings.contains(&finding) {
            state.findings.push(finding);
        }
    });
}

pub fn auditing() -> bool {
    STATE.with(|state| state.borrow().auditing)
}

/// Note a piece of text drawn at `x` that measured `width` wide.
///
/// Called by the text helpers. A draw outside any region is not a finding: the
/// reels and the celebration cards paint over the whole screen on purpose.
/// Note a control's footprint, so text drawn over it can be caught (§5.47).
pub fn note_control(label: &str, rect: Rect) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if !state.auditing {
            return;
        }
        state.drawn.push(Drawn {
            rect,
            what: label.to_owned(),
            is_control: true,
        });
    });
}

/// Note a piece of text's footprint and report anything it lands on.
///
/// The fault §5.46 exposed: the layout audit checked text against its region's
/// *edge* and nothing else, so a title drawn straight through a button was
/// "clean". Overflow past a boundary and collision with a sibling are different
/// questions, and only the first was being asked.
///
/// Text lying wholly inside a control is that control's own label and is
/// skipped. Text that only *partly* covers one is a collision, which is exactly
/// the shape the header fault took.
/// Also record what lands on what. Only meaningful one screen at a time.
pub fn begin_collision_audit() {
    STATE.with(|state| state.borrow_mut().collisions = true);
}

pub fn note_extent(text: &str, rect: Rect) {
    // A stroke is drawn four times at the same place on purpose; reporting it
    // as colliding with the label it exists to make readable would be reporting
    // the fix as the fault, exactly as it would for contrast.
    if DECORATIVE.with(|depth| *depth.borrow()) > 0 {
        return;
    }
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if !state.auditing || !state.collisions {
            return;
        }
        let mut hits = Vec::new();
        for other in &state.drawn {
            let w = (rect.right().min(other.rect.right()) - rect.x.max(other.rect.x)).max(0.0);
            let h = (rect.bottom().min(other.rect.bottom()) - rect.y.max(other.rect.y)).max(0.0);
            if w <= 1.0 || h <= 1.0 {
                continue;
            }
            // Inside a control means it is that control's label.
            if other.is_control
                && rect.x >= other.rect.x - 1.0
                && rect.right() <= other.rect.right() + 1.0
                && rect.y >= other.rect.y - 1.0
                && rect.bottom() <= other.rect.bottom() + 1.0
            {
                continue;
            }
            hits.push((other.what.clone(), w * h));
        }
        state.drawn.push(Drawn {
            rect,
            what: text.to_owned(),
            is_control: false,
        });
        for (other, overlap) in hits {
            let finding = Finding::Collision {
                text: text.to_owned(),
                other,
                overlap,
            };
            if !state.findings.contains(&finding) {
                state.findings.push(finding);
            }
        }
    });
}

pub fn note(text: &str, x: f32, width: f32) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if !state.auditing {
            return;
        }
        let Some(region) = state.stack.last().copied() else {
            return;
        };
        let available = region.rect.right() - x;
        // A hair over is rounding, not a defect. Anything a reader would notice
        // is at least a character wide.
        if width > available + 2.0 {
            let finding = Finding::Overflow {
                text: text.to_owned(),
                needed: width,
                available,
            };
            if !state.findings.contains(&finding) {
                state.findings.push(finding);
            }
        }
    });
}

/// Marks draws that are not meant to be read.
///
/// A text outline — the same string drawn four times in near-black behind a
/// light label, to keep it legible over a bright fill — is deliberately
/// invisible against a dark panel, and reporting it as low contrast is reporting
/// the fix as the fault. Anything held under this is skipped for contrast.
///
/// Not for turning off an inconvenient finding. It says *this is a stroke, not a
/// word*, and the only things that should hold it are the ones drawing twice.
pub struct Decorative {
    _private: (),
}

impl Decorative {
    pub fn new() -> Self {
        DECORATIVE.with(|depth| *depth.borrow_mut() += 1);
        Self { _private: () }
    }
}

impl Default for Decorative {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Decorative {
    fn drop(&mut self) {
        DECORATIVE.with(|depth| *depth.borrow_mut() -= 1);
    }
}

/// Note the colour and size a piece of text was drawn at.
///
/// Separate from [`note`] because a caller may know one and not the other, and
/// because contrast is only judged where a region has said what is behind it.
pub fn note_contrast(text: &str, color: Color, font_size: f32) {
    if !auditing() || DECORATIVE.with(|depth| *depth.borrow()) > 0 {
        return;
    }
    let Some(behind) = current_surface() else {
        return;
    };
    let ratio = super::contrast::ratio(color, behind);
    let required = super::contrast::Level::for_size(font_size).ratio();
    if ratio < required {
        record(Finding::LowContrast {
            text: text.to_owned(),
            ratio,
            required,
            font_size,
        });
    }
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
        begin_collision_audit();
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
        let Finding::Overflow {
            text, available, ..
        } = &found[0]
        else {
            panic!("expected an overflow, got {:?}", found[0]);
        };
        assert_eq!(text, "a long line");
        assert!((available - 390.0).abs() < 1e-3);
        assert!(
            found[0].describe().contains("110px"),
            "{}",
            found[0].describe()
        );
    }

    #[test]
    fn a_hair_over_is_rounding_rather_than_a_defect() {
        begin_audit();
        begin_collision_audit();
        let _region = Region::new(panel());
        note("just about", 110.0, 391.0);
        assert!(take_audit().is_empty());
    }

    /// The fault §5.46 exposed and §5.47 exists for.
    ///
    /// A title drawn straight through a button never crossed its *region's*
    /// edge, so the overflow check called it clean. Overflow past a boundary and
    /// collision with a sibling are different questions.
    #[test]
    fn text_drawn_across_a_control_is_reported() {
        begin_audit();
        begin_collision_audit();
        let _region = Region::new(panel());
        note_control("a button", Rect::new(200.0, 100.0, 100.0, 40.0));
        note_extent("a title", Rect::new(150.0, 105.0, 120.0, 30.0));

        let found = take_audit();
        assert_eq!(found.len(), 1, "{:?}", found);
        assert!(
            found[0].describe().contains("overlaps"),
            "{}",
            found[0].describe()
        );
    }

    #[test]
    fn a_controls_own_label_is_not_a_collision() {
        // Every button draws its text inside itself. Reporting that would make
        // the check useless from the first frame.
        begin_audit();
        begin_collision_audit();
        let _region = Region::new(panel());
        note_control("a button", Rect::new(200.0, 100.0, 100.0, 40.0));
        note_extent("Press", Rect::new(220.0, 112.0, 60.0, 16.0));
        assert!(take_audit().is_empty());
    }

    #[test]
    fn two_strings_over_one_another_are_reported() {
        begin_audit();
        begin_collision_audit();
        let _region = Region::new(panel());
        note_extent("first", Rect::new(150.0, 100.0, 100.0, 20.0));
        note_extent("second", Rect::new(200.0, 105.0, 100.0, 20.0));
        assert_eq!(take_audit().len(), 1);
    }

    #[test]
    fn text_that_merely_touches_is_not_a_collision() {
        // Adjacent labels share an edge constantly; a hairline is rounding.
        begin_audit();
        begin_collision_audit();
        let _region = Region::new(panel());
        note_extent("left", Rect::new(150.0, 100.0, 100.0, 20.0));
        note_extent("right", Rect::new(250.0, 100.0, 100.0, 20.0));
        assert!(take_audit().is_empty());
    }

    /// The five findings this check produced on its first real run were all
    /// this: an overlay's text "colliding" with a panel underneath that the
    /// player cannot see.
    #[test]
    fn a_panel_hides_what_is_underneath_it() {
        begin_audit();
        begin_collision_audit();
        {
            let _behind = Region::on(panel(), Color::new(0.1, 0.1, 0.1, 1.0));
            note_extent("underneath", Rect::new(150.0, 100.0, 200.0, 20.0));
        }
        {
            // An overlay painting its own surface over the same place.
            let _over = Region::on(
                Rect::new(140.0, 90.0, 240.0, 60.0),
                Color::new(0.1, 0.1, 0.1, 1.0),
            );
            note_extent("on top", Rect::new(150.0, 105.0, 200.0, 20.0));
        }
        assert!(take_audit().is_empty());
    }

    #[test]
    fn a_region_that_names_no_surface_hides_nothing() {
        // `Region::new` says where, not what on — it has painted nothing, so it
        // cannot be covering anything.
        begin_audit();
        begin_collision_audit();
        let _behind = Region::new(panel());
        note_extent("underneath", Rect::new(150.0, 100.0, 200.0, 20.0));
        {
            let _over = Region::new(Rect::new(140.0, 90.0, 240.0, 60.0));
            note_extent("on top", Rect::new(150.0, 105.0, 200.0, 20.0));
        }
        assert_eq!(take_audit().len(), 1);
    }

    #[test]
    fn a_decorative_stroke_does_not_collide_with_the_label_it_serves() {
        // The meter draws its label four times in near-black behind itself to
        // keep it readable. Reporting that is reporting the fix as the fault.
        begin_audit();
        begin_collision_audit();
        let _region = Region::new(panel());
        {
            let _stroke = Decorative::new();
            note_extent("Hoard 0/15", Rect::new(150.0, 100.0, 100.0, 20.0));
        }
        note_extent("Hoard 0/15", Rect::new(151.0, 101.0, 100.0, 20.0));
        assert!(take_audit().is_empty());
    }

    #[test]
    fn a_decorative_draw_is_not_judged_on_contrast() {
        // An outline is deliberately invisible against a dark panel. Reporting
        // it would be reporting the fix as the fault.
        begin_audit();
        begin_collision_audit();
        let _region = Region::on(panel(), Color::new(0.07, 0.06, 0.07, 1.0));
        {
            let _stroke = Decorative::new();
            note_contrast("outlined", Color::new(0.0, 0.0, 0.0, 0.75), 14.0);
        }
        assert!(take_audit().is_empty());
    }

    /// A modal's edge landing mid-label hides the tail of it. That is what an
    /// overlay is for, and it must not be reported.
    #[test]
    fn a_panel_edge_that_hides_the_tail_of_a_label_is_not_a_finding() {
        let label = Rect::new(30.0, 100.0, 110.0, 20.0);
        let panel = Rect::new(90.0, 96.0, 1100.0, 500.0);
        assert!(Region::hidden_by(panel, label));
    }

    /// A modal's *top* edge landing inside the line severs every glyph, which
    /// is what three overlays were doing to the cabinet name unnoticed.
    #[test]
    fn a_panel_edge_that_cuts_across_a_line_is_a_finding() {
        let title = Rect::new(26.0, 28.0, 304.0, 38.0);
        let panel = Rect::new(180.0, 44.0, 920.0, 632.0);
        assert!(!Region::hidden_by(panel, title));
        // And from below, which severs the tops of the letters instead.
        let from_below = Rect::new(180.0, -200.0, 920.0, 240.0);
        assert!(!Region::hidden_by(from_below, title));
    }

    #[test]
    fn a_surface_that_covers_a_label_outright_hides_it() {
        let label = Rect::new(200.0, 300.0, 80.0, 18.0);
        assert!(Region::hidden_by(
            Rect::new(100.0, 200.0, 600.0, 400.0),
            label
        ));
        // And one nowhere near it neither hides nor cuts.
        assert!(!Region::hidden_by(Rect::new(0.0, 0.0, 50.0, 50.0), label));
    }

    #[test]
    fn the_label_itself_is_still_judged() {
        // The guard covers the stroke and nothing after it.
        begin_audit();
        begin_collision_audit();
        let _region = Region::on(panel(), Color::new(0.07, 0.06, 0.07, 1.0));
        {
            let _stroke = Decorative::new();
            note_contrast("outlined", Color::new(0.0, 0.0, 0.0, 0.75), 14.0);
        }
        note_contrast("label", Color::new(0.10, 0.09, 0.10, 1.0), 14.0);
        assert_eq!(take_audit().len(), 1);
    }

    #[test]
    fn contrast_is_not_judged_where_no_surface_was_declared() {
        // `Region::new` says where, not what on. Guessing would invent findings.
        begin_audit();
        begin_collision_audit();
        let _region = Region::new(panel());
        note_contrast("anything", Color::new(0.1, 0.1, 0.1, 1.0), 14.0);
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
        assert_eq!(found[0].text(), "inside the column");
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
        begin_collision_audit();
        let _region = Region::new(panel());
        for _ in 0..60 {
            note("a long line", 110.0, 500.0);
        }
        assert_eq!(take_audit().len(), 1);
    }

    #[test]
    fn taking_the_audit_stops_it() {
        begin_audit();
        begin_collision_audit();
        let _region = Region::new(panel());
        note("a long line", 110.0, 500.0);
        assert_eq!(take_audit().len(), 1);

        note("another long line", 110.0, 500.0);
        assert!(take_audit().is_empty());
        assert!(!auditing());
    }
}
