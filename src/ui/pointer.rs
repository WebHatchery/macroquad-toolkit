//! One place a control asks "was I pressed", whatever pressed it.
//!
//! # Mouse and finger are the same question
//!
//! A game built for a mouse checks `is_mouse_button_released` at every control.
//! Adding touch that way means finding all of them and remembering the
//! differences: a touch has no hover, arrives and leaves in one gesture, and
//! there may be several at once. Miss one control and it is simply dead on a
//! phone, silently, because nothing errors.
//!
//! So both become a [`Pointer`]: a position, and whether it was released this
//! frame. A control asks that and nothing else.
//!
//! # Hover is a mouse idea
//!
//! [`Pointer::hovering`] is false for touch. A finger is not *over* anything
//! until it is *on* it, and a UI that lights up under a finger is a UI showing
//! the player what they have already committed to. Controls that only look
//! different on hover are controls a touch player never sees respond — which is
//! worth knowing about rather than papering over.
//!
//! # How big is big enough
//!
//! Every guideline puts a touch target near **44 CSS pixels**: WCAG 2.5.5 asks
//! 44×44, Apple 44pt, Google 48dp. A game drawn at a fixed logical size does not
//! have a size in those units until it is on a screen — a 42-pixel button is
//! comfortable on a monitor and a speck on a phone.
//!
//! So [`smallest_touchable_width`] turns that around and asks the one question
//! with an actionable answer: **how wide does the window have to be before every
//! control clears the standard?** One number, derived from the layout, that goes
//! down when a control is made bigger.

use macroquad::prelude::*;
use std::cell::RefCell;

/// The size a touch target should reach, in CSS pixels.
///
/// WCAG 2.5.5 (AAA) and Apple both say 44; Google says 48dp. 44 is the common
/// floor and the one worth failing against.
pub const MIN_TARGET: f32 = 44.0;

/// Where the interaction is, and whether it just ended.
#[derive(Debug, Clone, Copy, Default)]
pub struct Pointer {
    pub position: Vec2,
    /// A click or a lifted finger, this frame.
    pub released: bool,
    /// Held down, or a finger currently on the glass.
    pub down: bool,
    /// True only for a mouse. A finger is not over anything until it is on it.
    pub hovering: bool,
}

/// Put a touch in the same units as the cursor.
///
/// macroquad reports the two differently and says so nowhere:
/// `mouse_position()` divides by the DPI scale before returning, and
/// `touches()` hands back the raw framebuffer coordinate the platform gave it.
/// On a desktop at 100% the scale is 1 and the difference does not exist, which
/// is why this survived every test on the machine it was written on.
///
/// A tablet is the case that finds it. An iPad reports `devicePixelRatio` 2, so
/// a finger on a control at logical (300, 200) arrives as (600, 400) — off the
/// control, usually off the screen. Every hit test then fails while the *drawn*
/// UI, which is laid out in logical pixels, looks perfectly fine.
///
/// The symptom is worth writing down because it points away from the cause: the
/// button lights up anyway. macroquad synthesises a mouse from the touch, and
/// that path *is* divided, so the frame after the finger lifts the control is
/// hovered at the right place and stays lit — a press that visibly registers
/// and does nothing.
fn logical_touch(position: Vec2, dpi: f32) -> Vec2 {
    if dpi > 0.0 {
        position / dpi
    } else {
        position
    }
}

/// Is the finger still there? A touch has no separate "button down" to read, so
/// being on the glass is being pressed.
///
/// The phase that is easy to forget is `Cancelled` — the system taking the
/// gesture away, which iOS does for a pinch, an edge swipe, or a notification
/// sliding in. It is not a lift and must not act like one, but it is not a
/// finger either: treating it as held left the last thing under it drawn as
/// pressed for a frame after the player had lost the gesture entirely.
fn finger_is_on_the_glass(phase: TouchPhase) -> bool {
    !matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled)
}

impl Pointer {
    /// Read this frame's input, preferring a touch when one is present.
    ///
    /// `to_logical` maps screen coordinates into the game's own space, so this
    /// does not need to know how the virtual frame is scaled.
    pub fn read(to_logical: impl Fn(Vec2) -> Vec2) -> Self {
        Self::read_at_dpi(to_logical, screen_dpi_scale())
    }

    /// [`Self::read`] with the DPI scale injected, so the conversion can be
    /// tested without a live macroquad context.
    pub fn read_at_dpi(to_logical: impl Fn(Vec2) -> Vec2, dpi: f32) -> Self {
        // A touch wins when both are present: a browser that synthesises mouse
        // events from taps would otherwise fire the same control twice.
        if let Some(touch) = touches().first() {
            return Self {
                position: to_logical(logical_touch(touch.position, dpi)),
                released: matches!(touch.phase, TouchPhase::Ended),
                down: finger_is_on_the_glass(touch.phase),
                hovering: false,
            };
        }
        let (x, y) = mouse_position();
        Self {
            position: to_logical(vec2(x, y)),
            released: is_mouse_button_released(MouseButton::Left),
            down: is_mouse_button_down(MouseButton::Left),
            hovering: true,
        }
    }

    /// Is the pointer inside this control, and is it being held down?
    ///
    /// Distinct from `released` because a button should look pressed while it is
    /// held and act when it is let go — which is also what lets a player slide
    /// off a control they did not mean to hit.
    ///
    /// Reads only its own fields. Reaching for `is_mouse_button_down` here would
    /// make every control untestable without a window, which is how a UI ends up
    /// with no tests at all.
    pub fn pressing(&self, rect: Rect) -> bool {
        self.down && rect.contains(self.position)
    }

    /// Is the pointer over this control without having committed to it? Always
    /// false for touch.
    pub fn hovering_over(&self, rect: Rect) -> bool {
        self.hovering && rect.contains(self.position)
    }

    /// Did an interaction end on this control?
    pub fn released_on(&self, rect: Rect) -> bool {
        self.released && rect.contains(self.position)
    }

    /// The same pointer with its press taken away — still somewhere, no longer
    /// pressing anything.
    ///
    /// For when something upstream has claimed the gesture: a
    /// [`ScrollArea`](crate::ui::ScrollArea) that turned it into a drag, or a
    /// panel drawn over the top. Passing this down means the controls underneath
    /// draw exactly as they would have and simply do not fire, instead of each
    /// of them having to know what else might be going on.
    ///
    /// Hover survives, because the cursor really is still there.
    pub fn suppressed(&self) -> Self {
        Self {
            released: false,
            down: false,
            ..*self
        }
    }
}

#[derive(Default)]
struct Audit {
    recording: bool,
    /// Every distinct control seen: (hit side, drawn side, label), smallest
    /// hit side first. Both are kept because they answer different questions —
    /// the hit side is what a finger can reach, the drawn side is what an eye
    /// can find, and expansion fixes only the first.
    seen: Vec<(f32, f32, String)>,
    /// Grown hit areas, in draw order, for the overlap check.
    areas: Vec<(Rect, String)>,
}

thread_local! {
    static AUDIT: RefCell<Audit> = RefCell::new(Audit::default());
}

/// Grow a control's *hit* area to the touch standard, leaving what is drawn
/// alone.
///
/// The alternative is enlarging every button, which changes a layout that was
/// designed at those sizes and pushes panels past their bounds. A small, precise
/// control with a generous invisible margin is the usual answer and the right
/// one: the visual weight is a design decision and the target size is an
/// accessibility one, and they do not have to be the same number.
///
/// The catch is neighbours. Two buttons eight pixels apart, each grown to
/// forty-four, now overlap — and a press landing on the wrong control is worse
/// than one landing on nothing. [`overlapping_targets`] is what stops that being
/// silent.
pub fn touch_area(rect: Rect) -> Rect {
    NEIGHBOURS.with(|slot| touch_area_among(rect, &slot.borrow().last))
}

/// Grow a control's hit area as far as its neighbours allow.
///
/// The unconstrained version was wrong and the audit said so: on a narrow
/// screen the controls sit closer together, and growing every one to
/// forty-four made them overlap by thousands of square pixels. **A press
/// landing on the wrong control is worse than one landing on nothing**, which
/// was the stated rule from the beginning and was only ever checked at the
/// design width.
///
/// So each side grows at most halfway to whatever is next to it. A control with
/// room takes the full standard; one in a tight row takes what is going and
/// stays unambiguous.
pub fn touch_area_among(rect: Rect, others: &[Rect]) -> Rect {
    let mut left = ((MIN_TARGET - rect.w) * 0.5).max(0.0);
    let mut right = left;
    let mut up = ((MIN_TARGET - rect.h) * 0.5).max(0.0);
    let mut down = up;

    for other in others {
        if other.x == rect.x && other.y == rect.y && other.w == rect.w && other.h == rect.h {
            continue;
        }
        // Only neighbours that actually share a band can be run into.
        let rows_overlap = rect.y < other.bottom() && other.y < rect.bottom();
        let cols_overlap = rect.x < other.right() && other.x < rect.right();

        if rows_overlap {
            if other.right() <= rect.x {
                left = left.min((rect.x - other.right()) * 0.5);
            }
            if other.x >= rect.right() {
                right = right.min((other.x - rect.right()) * 0.5);
            }
        }
        if cols_overlap {
            if other.bottom() <= rect.y {
                up = up.min((rect.y - other.bottom()) * 0.5);
            }
            if other.y >= rect.bottom() {
                down = down.min((other.y - rect.bottom()) * 0.5);
            }
        }
    }

    Rect::new(
        rect.x - left,
        rect.y - up,
        rect.w + left + right,
        rect.h + up + down,
    )
}

#[derive(Default)]
struct Neighbours {
    /// Controls seen last frame. The set is stable between frames in an
    /// immediate-mode UI as long as the same panels are open, which is the same
    /// property the keyboard focus ring already depends on.
    last: Vec<Rect>,
    building: Vec<Rect>,
}

thread_local! {
    static NEIGHBOURS: RefCell<Neighbours> = RefCell::new(Neighbours::default());
}

/// Has a full frame of controls been seen yet?
///
/// The first frame of a scene grows every hit area without limits, because the
/// limits come from the frame before it. A report taken then describes a state
/// the game is never actually in.
pub fn neighbours_warm() -> bool {
    NEIGHBOURS.with(|slot| !slot.borrow().last.is_empty())
}

/// Note a control for next frame's growth limits. Called by the widgets.
pub fn note_neighbour(rect: Rect) {
    NEIGHBOURS.with(|slot| slot.borrow_mut().building.push(rect));
}

/// Roll this frame's controls into next frame's limits. Call once per frame,
/// after everything has drawn.
pub fn end_frame_neighbours() {
    NEIGHBOURS.with(|slot| {
        let mut n = slot.borrow_mut();
        n.last = std::mem::take(&mut n.building);
    });
}

/// Pairs of grown hit areas that overlap, and by how much.
///
/// Recorded in the order the controls drew, so the report names them the same
/// way the layout does.
pub fn overlapping_targets() -> Vec<(String, String, f32)> {
    AUDIT.with(|audit| {
        let audit = audit.borrow();
        let mut found = Vec::new();
        for (index, (a_rect, a_label)) in audit.areas.iter().enumerate() {
            for (b_rect, b_label) in audit.areas.iter().skip(index + 1) {
                let w = (a_rect.right().min(b_rect.right()) - a_rect.x.max(b_rect.x)).max(0.0);
                let h = (a_rect.bottom().min(b_rect.bottom()) - a_rect.y.max(b_rect.y)).max(0.0);
                // A hairline touch is rounding; anything a finger could land in
                // is a real ambiguity.
                if w > 2.0 && h > 2.0 {
                    found.push((a_label.clone(), b_label.clone(), w * h));
                }
            }
        }
        found
    })
}

/// Start measuring control sizes.
pub fn begin_target_audit() {
    AUDIT.with(|audit| {
        let mut audit = audit.borrow_mut();
        audit.recording = true;
        audit.seen.clear();
        audit.areas.clear();
    });
}

/// Forget controls covered by a panel painted over them.
///
/// The same rule the collision check needed (§5.47): a control under an opaque
/// overlay cannot be pressed, so it cannot be ambiguous with anything. Without
/// this the settings panel’s buttons “overlap” the wager panel’s underneath it,
/// which is thirty-two findings about a screen the player never sees.
pub fn occlude(rect: Rect) {
    AUDIT.with(|audit| {
        let mut audit = audit.borrow_mut();
        if !audit.recording {
            return;
        }
        audit.areas.retain(|(area, _)| !hidden(rect, *area));
    });
    NEIGHBOURS.with(|slot| {
        let mut n = slot.borrow_mut();
        n.building.retain(|r| !hidden(rect, *r));
    });
}

/// Does this surface hide that control, or *is* it that control?
///
/// A button paints its own background and then declares a region over itself so
/// its label is contrast-checked against the fill it actually sits on. That
/// region has exactly the button's rect — so the plain containment test
/// removed every control in the game the instant it drew, one at a time.
///
/// The damage was silent and total. `building` was empty at the end of every
/// frame, so `last` was always empty, so [`neighbours_warm`] was always false.
/// Every report gated on it — the smallest touchable window, the undersized
/// list, the overlapping pairs — has therefore never printed, and the gate that
/// runs them passed by saying nothing. Hit areas were also being grown to the
/// full standard with no neighbour limits at all, which is the one thing the
/// growth was careful about.
///
/// So: a surface that *is* a control does not hide it. Anything else does.
fn hidden(surface: Rect, control: Rect) -> bool {
    if is_the_same(surface, control) {
        return false;
    }
    surface.contains(control.center())
}

/// Same rectangle, to within a rounding error.
fn is_the_same(a: Rect, b: Rect) -> bool {
    const SLACK: f32 = 0.5;
    (a.x - b.x).abs() < SLACK
        && (a.y - b.y).abs() < SLACK
        && (a.w - b.w).abs() < SLACK
        && (a.h - b.h).abs() < SLACK
}

/// Start a fresh frame of measurements.
///
/// Growth is limited by the *previous* frame’s controls, so the first frame of
/// a scene has no limits and its numbers are wrong. Measuring per frame rather
/// than accumulating means the report describes a frame that had neighbours to
/// work with (§5.48).
pub fn begin_target_frame() {
    AUDIT.with(|audit| {
        let mut audit = audit.borrow_mut();
        if audit.recording {
            audit.seen.clear();
            audit.areas.clear();
        }
    });
}

/// Note a control that can be pressed. Called by the widgets.
pub fn note_target(label: &str, rect: Rect) {
    AUDIT.with(|audit| {
        let mut audit = audit.borrow_mut();
        if !audit.recording {
            return;
        }
        // The *smaller* side of the area that is actually hit-tested, which is
        // the grown one: what a player can press is the question, not what was
        // drawn. Before expansion this reported a requirement no window could
        // meet and gave nothing to act on.
        let drawn = rect.w.min(rect.h);
        if drawn <= 0.0 {
            return;
        }
        let area = touch_area(rect);
        let side = area.w.min(area.h);
        audit.areas.push((area, label.to_owned()));
        let entry = (side, drawn, label.to_owned());
        if !audit.seen.contains(&entry) {
            audit.seen.push(entry);
            audit.seen.sort_by(|a, b| a.0.total_cmp(&b.0));
        }
    });
}

/// The window width at which every control seen would clear [`MIN_TARGET`],
/// and the control that decides it.
///
/// A control `side` logical pixels across is drawn `side * width / logical`
/// CSS pixels wide, so it clears the standard once
/// `width >= MIN_TARGET * logical / side`. The widest such requirement is the
/// answer, and the control that set it is the one to make bigger.
pub fn smallest_touchable_width(logical_width: f32) -> Option<(f32, String)> {
    AUDIT.with(|audit| {
        let audit = audit.borrow();
        audit
            .seen
            .first()
            .map(|(side, _, label)| (MIN_TARGET * logical_width / side, label.clone()))
    })
}

/// Controls **drawn** smaller than [`MIN_TARGET`], worst first.
///
/// Their hit areas are already grown, so these are reachable — but a control
/// too small to find is a control nobody presses, and that is a separate fault
/// from one too small to hit. This is the list of things worth enlarging for
/// the eye rather than for the finger.
pub fn undersized_targets() -> Vec<(f32, String)> {
    AUDIT.with(|audit| {
        let mut out: Vec<(f32, String)> = audit
            .borrow()
            .seen
            .iter()
            .filter(|(_, drawn, _)| *drawn < MIN_TARGET)
            .map(|(_, drawn, label)| (*drawn, label.clone()))
            .collect();
        out.sort_by(|a, b| a.0.total_cmp(&b.0));
        out
    })
}

/// Stop measuring.
pub fn end_target_audit() {
    AUDIT.with(|audit| audit.borrow_mut().recording = false);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset() {
        begin_target_audit();
    }

    #[test]
    fn nothing_is_measured_until_the_audit_starts() {
        end_target_audit();
        AUDIT.with(|audit| {
            let mut audit = audit.borrow_mut();
            audit.seen.clear();
            audit.areas.clear();
        });
        note_target("button", Rect::new(0.0, 0.0, 10.0, 10.0));
        assert!(smallest_touchable_width(1280.0).is_none());
    }

    #[test]
    fn the_smaller_side_is_the_target() {
        // A control 200 wide and 20 tall is a 20-pixel target however generous
        // it looks lying down.
        reset();
        note_target("wide and flat", Rect::new(0.0, 0.0, 200.0, 20.0));
        let (width, label) = smallest_touchable_width(1280.0).unwrap();
        assert_eq!(label, "wide and flat");
        // Grown to 44 tall, so it asks for the design width and no more.
        assert!((width - 1280.0).abs() < 1.0, "{}", width);
    }

    #[test]
    fn the_worst_control_sets_the_number() {
        reset();
        note_target("generous", Rect::new(0.0, 0.0, 120.0, 90.0));
        note_target("mean", Rect::new(0.0, 0.0, 30.0, 24.0));
        note_target("middling", Rect::new(0.0, 0.0, 80.0, 40.0));
        // Everything below the standard is grown to exactly it, so they tie —
        // and the answer is the design width, which is the point.
        let (width, _) = smallest_touchable_width(1280.0).unwrap();
        assert!((width - 1280.0).abs() < 1.0, "{}", width);
    }

    #[test]
    fn a_bigger_control_asks_for_a_smaller_window() {
        // The property that makes the number actionable: it goes down when the
        // layout is improved.
        reset();
        note_target("small", Rect::new(0.0, 0.0, 30.0, 30.0));
        let (tight, _) = smallest_touchable_width(1280.0).unwrap();

        reset();
        note_target("large", Rect::new(0.0, 0.0, 60.0, 60.0));
        let (roomy, _) = smallest_touchable_width(1280.0).unwrap();

        assert!(roomy < tight, "{} against {}", roomy, tight);
    }

    #[test]
    fn a_control_already_at_the_standard_asks_for_the_logical_width() {
        // A control exactly MIN_TARGET across needs the window to be exactly the
        // logical size — one logical pixel to one CSS pixel.
        reset();
        note_target("just right", Rect::new(0.0, 0.0, MIN_TARGET, MIN_TARGET));
        let (width, _) = smallest_touchable_width(1280.0).unwrap();
        assert!((width - 1280.0).abs() < 0.01, "{}", width);
    }

    #[test]
    fn everything_below_the_standard_is_listed_worst_first() {
        // One number says how bad it is; this says what to fix, in order.
        reset();
        note_target("fine", Rect::new(0.0, 0.0, 60.0, 60.0));
        note_target("small", Rect::new(0.0, 0.0, 90.0, 30.0));
        note_target("smallest", Rect::new(0.0, 0.0, 20.0, 90.0));

        let under = undersized_targets();
        assert_eq!(under.len(), 2, "{:?}", under);
        assert_eq!(under[0].1, "smallest");
        assert_eq!(under[1].1, "small");
    }

    #[test]
    fn a_small_control_gets_a_generous_hit_area_around_its_centre() {
        let drawn = Rect::new(100.0, 100.0, 20.0, 20.0);
        let touch = touch_area(drawn);
        assert!((touch.w - MIN_TARGET).abs() < 1e-3);
        assert!((touch.h - MIN_TARGET).abs() < 1e-3);
        // Grown around the middle, not from a corner, or the target would sit
        // off to one side of the thing it belongs to.
        assert!((touch.center().x - drawn.center().x).abs() < 1e-3);
        assert!((touch.center().y - drawn.center().y).abs() < 1e-3);
    }

    #[test]
    fn a_control_already_large_enough_is_untouched() {
        let drawn = Rect::new(0.0, 0.0, 200.0, 70.0);
        let touch = touch_area(drawn);
        assert!((touch.w - 200.0).abs() < 1e-3);
        assert!((touch.h - 70.0).abs() < 1e-3);
    }

    #[test]
    fn only_the_short_side_grows() {
        let touch = touch_area(Rect::new(0.0, 0.0, 300.0, 28.0));
        assert!((touch.w - 300.0).abs() < 1e-3);
        assert!((touch.h - MIN_TARGET).abs() < 1e-3);
    }

    #[test]
    fn growth_stops_halfway_to_a_neighbour() {
        // The fault the verification harness found: on a narrow screen the
        // controls sit closer together, and growing every one to the standard
        // made them overlap by thousands of square pixels.
        let a = Rect::new(0.0, 0.0, 100.0, 28.0);
        let b = Rect::new(0.0, 36.0, 100.0, 28.0);
        let grown = touch_area_among(a, &[a, b]);
        // 8px gap, so 4px each way, not the 8 the standard would want.
        assert!((grown.bottom() - 32.0).abs() < 0.01, "{:?}", grown);
        assert!(grown.bottom() <= b.y);
    }

    #[test]
    fn a_control_with_room_still_takes_the_whole_standard() {
        let lonely = Rect::new(0.0, 0.0, 20.0, 20.0);
        let far = Rect::new(0.0, 400.0, 20.0, 20.0);
        let grown = touch_area_among(lonely, &[lonely, far]);
        assert!((grown.h - MIN_TARGET).abs() < 0.01, "{:?}", grown);
    }

    #[test]
    fn neighbours_that_share_no_band_do_not_constrain() {
        // A control in another column cannot be run into vertically.
        let here = Rect::new(0.0, 0.0, 20.0, 20.0);
        let elsewhere = Rect::new(500.0, 22.0, 20.0, 20.0);
        let grown = touch_area_among(here, &[here, elsewhere]);
        assert!((grown.h - MIN_TARGET).abs() < 0.01, "{:?}", grown);
    }

    #[test]
    fn grown_areas_never_overlap_however_tight_the_row() {
        // The property, stated directly: whatever the spacing, no two hit areas
        // may claim the same pixel.
        for gap in [0.0f32, 2.0, 6.0, 20.0, 60.0] {
            let rects: Vec<Rect> = (0..5)
                .map(|i| Rect::new(0.0, i as f32 * (28.0 + gap), 100.0, 28.0))
                .collect();
            let grown: Vec<Rect> = rects.iter().map(|r| touch_area_among(*r, &rects)).collect();
            for (i, a) in grown.iter().enumerate() {
                for b in grown.iter().skip(i + 1) {
                    let h = (a.bottom().min(b.bottom()) - a.y.max(b.y)).max(0.0);
                    assert!(h <= 0.01, "gap {} left {:?} over {:?}", gap, a, b);
                }
            }
        }
    }

    /// The cost of growing hit areas, made visible.
    ///
    /// A press landing on the wrong control is worse than one landing on
    /// nothing, so this must not be something anyone has to remember to check.
    #[test]
    fn neighbours_that_now_overlap_are_reported() {
        reset();
        // Two 28-tall controls eight pixels apart: fine as drawn, overlapping
        // once both are grown to forty-four.
        note_target("upper", Rect::new(0.0, 0.0, 100.0, 28.0));
        note_target("lower", Rect::new(0.0, 36.0, 100.0, 28.0));
        let clashes = overlapping_targets();
        assert_eq!(clashes.len(), 1, "{:?}", clashes);
        assert_eq!(clashes[0].0, "upper");
        assert_eq!(clashes[0].1, "lower");
    }

    #[test]
    fn controls_with_room_between_them_do_not_clash() {
        reset();
        note_target("upper", Rect::new(0.0, 0.0, 100.0, 28.0));
        note_target("lower", Rect::new(0.0, 60.0, 100.0, 28.0));
        assert!(overlapping_targets().is_empty());
    }

    #[test]
    fn a_control_with_no_size_is_ignored_rather_than_dividing_by_zero() {
        reset();
        note_target("collapsed", Rect::new(0.0, 0.0, 0.0, 40.0));
        assert!(smallest_touchable_width(1280.0).is_none());
    }

    #[test]
    fn a_mouse_pointer_hovers_and_a_finger_does_not() {
        // Not a behaviour test — there is no input here — but the contract is
        // worth pinning: a UI that lights up under a finger is showing the
        // player what they have already committed to.
        let mouse = Pointer {
            hovering: true,
            ..Pointer::default()
        };
        let finger = Pointer::default();
        assert!(mouse.hovering);
        assert!(!finger.hovering);
    }

    /// The fault a tablet found: a touch arrives in physical pixels and the
    /// cursor in logical ones, so on a 2x screen every finger landed twice as
    /// far into the layout as it really was.
    #[test]
    fn a_touch_is_brought_into_the_same_units_as_the_cursor() {
        // What the platform hands over on an iPad for a finger at (300, 200).
        assert_eq!(logical_touch(vec2(600.0, 400.0), 2.0), vec2(300.0, 200.0));
        // A 3x phone screen, and a 150% desktop.
        assert_eq!(logical_touch(vec2(900.0, 600.0), 3.0), vec2(300.0, 200.0));
        assert_eq!(logical_touch(vec2(450.0, 300.0), 1.5), vec2(300.0, 200.0));
    }

    #[test]
    fn an_unscaled_display_is_left_exactly_alone() {
        // The case every desktop is in, and the reason this went unnoticed.
        let raw = vec2(317.0, 42.5);
        assert_eq!(logical_touch(raw, 1.0), raw);
    }

    /// A scale of zero would silently teleport every touch to infinity. Nothing
    /// should report one, but a hit test is not the place to find out.
    #[test]
    fn a_nonsense_scale_is_ignored_rather_than_dividing_by_zero() {
        let raw = vec2(100.0, 50.0);
        assert_eq!(logical_touch(raw, 0.0), raw);
    }

    #[test]
    fn a_finger_is_held_until_it_lifts_or_is_taken_away() {
        assert!(finger_is_on_the_glass(TouchPhase::Started));
        assert!(finger_is_on_the_glass(TouchPhase::Moved));
        assert!(finger_is_on_the_glass(TouchPhase::Stationary));
        assert!(!finger_is_on_the_glass(TouchPhase::Ended));
        // The one that is easy to miss: the system took the gesture, so there
        // is nothing on the glass even though nobody lifted anything.
        assert!(!finger_is_on_the_glass(TouchPhase::Cancelled));
    }

    #[test]
    fn a_finger_inside_a_control_counts_as_pressing_it() {
        // A touch has no separate "button down" to read, so being there is
        // being on it.
        let rect = Rect::new(10.0, 10.0, 100.0, 40.0);
        let finger = Pointer {
            position: vec2(50.0, 30.0),
            released: false,
            down: true,
            hovering: false,
        };
        assert!(finger.pressing(rect));

        let elsewhere = Pointer {
            position: vec2(500.0, 500.0),
            ..finger
        };
        assert!(!elsewhere.pressing(rect));
    }
}

#[cfg(test)]
mod self_occlusion {
    use super::*;

    /// The fault, reproduced: a button declaring a region over itself used to
    /// erase itself from the neighbour list, every button, every frame.
    #[test]
    fn a_button_does_not_hide_itself() {
        let button = Rect::new(100.0, 200.0, 108.0, 28.0);
        begin_target_audit();
        begin_target_frame();
        note_neighbour(button);
        note_target("button", button);

        // Exactly what `virtual_button` does after drawing its fill.
        occlude(button);

        end_frame_neighbours();
        assert!(
            neighbours_warm(),
            "a button erased itself from the neighbour list, so nothing is ever warm"
        );
        assert!(
            !overlapping_targets().is_empty() || smallest_touchable_width(1280.0).is_some(),
            "the button also vanished from the target audit"
        );
    }

    /// And the rule it must not break: a panel painted over a control really
    /// does hide it, which is why occlusion exists at all.
    #[test]
    fn a_panel_still_hides_what_is_under_it() {
        let button = Rect::new(100.0, 200.0, 108.0, 28.0);
        let panel = Rect::new(0.0, 0.0, 640.0, 480.0);
        begin_target_audit();
        begin_target_frame();
        note_neighbour(button);
        note_target("button", button);

        occlude(panel);

        end_frame_neighbours();
        assert!(
            !neighbours_warm(),
            "a panel covering a control left it in the neighbour list"
        );
    }

    /// Half a pixel of rounding is the same rectangle; ten is not.
    #[test]
    fn the_same_rectangle_is_recognised_through_rounding() {
        let a = Rect::new(10.0, 20.0, 100.0, 40.0);
        assert!(is_the_same(a, Rect::new(10.2, 19.8, 100.1, 40.2)));
        assert!(!is_the_same(a, Rect::new(10.0, 20.0, 110.0, 40.0)));
    }
}
