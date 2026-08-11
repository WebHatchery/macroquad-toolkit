//! Spinning strips: reels, wheels, and anything else that scrolls to a decided
//! stop.
//!
//! # The decided-stop rule
//!
//! A strip is told where it is going *before* it starts moving, and its position
//! is a **closed-form function of elapsed time** rather than an integrated
//! velocity. That is the whole design. It means a strip lands on exactly the
//! symbol it was told to regardless of how the frame times fall — a stutter, a
//! debugger pause, a 240Hz monitor, none of it can move the result by a pixel,
//! and the animation consumes no randomness of its own.
//!
//! It also means the game decides outcomes and the animation only ever *reveals*
//! them, which is the property any game with money or score on the line
//! actually needs.
//!
//! # Travel must be a whole number of revolutions
//!
//! [`StripFeel::revolutions`] plus the per-strip stagger has to come out as a
//! whole number of strip lengths, or a strip lands somewhere other than its
//! stop. The game this was lifted from shipped `2.0 + index * 0.5` for five
//! iterations: strips 2 and 4 turned two and a half times and came to rest
//! exactly half a strip out, popping to the right symbols the instant the
//! resting draw took over. Every landing test happened to use an even index.
//! [`StripAnimation::travel_symbols`] is exposed so a test can assert the
//! invariant directly.
//!
//! ```no_run
//! use macroquad_toolkit::strip::{StripFeel, StripSpinner};
//!
//! let feel = StripFeel::default();
//! // Five strips of 40 symbols, each moving from its old stop to a new one.
//! let mut spinner = StripSpinner::new(&[40; 5], &[0; 5], &[7, 12, 3, 28, 31], 1.0, &[false; 5], &feel);
//! for stopped in spinner.tick(1.0 / 60.0) {
//!     let _ = stopped; // this strip just came to rest
//! }
//! ```

use crate::math::ease_out_cubic;

/// How a strip moves. Every duration here is in seconds and every distance in
/// symbols.
#[derive(Debug, Clone, Copy)]
pub struct StripFeel {
    /// How long the first strip spins for.
    pub base_time: f32,
    /// Extra time per strip, which is what produces the left-to-right stop.
    pub stagger: f32,
    /// Whole strip revolutions before landing, plus one more every second strip
    /// so later ones visibly spin faster rather than merely longer.
    pub revolutions: usize,
    /// Depth of the landing bounce, in symbols. Small: a strip should settle,
    /// not wobble.
    pub bounce_depth: f32,
    /// Fraction of the travel spent bouncing at the end.
    pub bounce_tail: f32,
    /// A strip slower than this many symbols per second is drawn crisply.
    pub blur_speed: f32,
    /// Ceiling on the smear, so a fast strip streaks rather than dissolving
    /// into a flat band.
    pub blur_cap: f32,
    /// How much longer a *held* strip turns for. Used for near-miss
    /// anticipation: the caller decides which strips to hold and why, because
    /// that is game logic, but the stretch itself is not.
    pub hold_stretch: f32,
}

impl Default for StripFeel {
    fn default() -> Self {
        Self {
            base_time: 0.95,
            stagger: 0.30,
            revolutions: 2,
            bounce_depth: 0.16,
            bounce_tail: 0.18,
            blur_speed: 6.0,
            blur_cap: 0.85,
            hold_stretch: 2.6,
        }
    }
}

/// One strip travelling from its previous stop to its decided stop.
#[derive(Debug, Clone)]
pub struct StripAnimation {
    start: f32,
    travel: f32,
    duration: f32,
    elapsed: f32,
    strip_len: f32,
    held: bool,
    feel: StripFeel,
}

impl StripAnimation {
    /// `time_scale` shortens the reveal without touching `travel`, so a fast
    /// setting lands on exactly the same symbol as a slow one.
    pub fn new(
        strip_len: usize,
        from: usize,
        to: usize,
        index: usize,
        time_scale: f32,
        held: bool,
        feel: &StripFeel,
    ) -> Self {
        let len = strip_len.max(1);
        let from = from % len;
        let to = to % len;
        let delta = (to + len - from) % len;
        let revolutions = feel.revolutions + index / 2;
        // Never zero: a duration of 0 would land the strips in the frame they
        // start and skip every stop notification.
        let scale = time_scale.clamp(0.05, 4.0);
        let stretch = if held { feel.hold_stretch } else { 1.0 };

        Self {
            start: from as f32,
            travel: (revolutions * len + delta) as f32,
            duration: (feel.base_time + index as f32 * feel.stagger) * scale * stretch,
            elapsed: 0.0,
            strip_len: len as f32,
            held,
            feel: *feel,
        }
    }

    pub fn is_held(&self) -> bool {
        self.held
    }

    /// Total distance in symbols. Exposed so a caller can assert it is a whole
    /// number of strip lengths — see the module note.
    pub fn travel_symbols(&self) -> f32 {
        self.travel
    }

    pub fn strip_len(&self) -> f32 {
        self.strip_len
    }

    /// A damped wobble over the last stretch of the travel, in symbols.
    ///
    /// Exactly zero at `t == 1`, so the strip still comes to rest on the symbol
    /// it was told to — the bounce is presentation, never a change of mind.
    fn bounce(&self, t: f32) -> f32 {
        let tail = self.feel.bounce_tail;
        if tail <= 0.0 || t <= 1.0 - tail {
            return 0.0;
        }
        let u = ((t - (1.0 - tail)) / tail).clamp(0.0, 1.0);
        let decay = 1.0 - u;
        self.feel.bounce_depth * decay * (u * std::f32::consts::PI * 2.0).sin()
    }

    /// Fractional strip position of the top visible cell.
    pub fn position(&self) -> f32 {
        let t = self.progress();
        (self.start + self.travel * ease_out_cubic(t) + self.bounce(t)).rem_euclid(self.strip_len)
    }

    /// Symbols per second, used to decide how hard to blur.
    pub fn speed(&self) -> f32 {
        if self.settled() || self.duration <= 0.0 {
            return 0.0;
        }
        // Derivative of the cubic ease-out, scaled by the travel distance.
        let t = self.progress();
        3.0 * (1.0 - t).powi(2) * self.travel / self.duration
    }

    pub fn progress(&self) -> f32 {
        if self.duration <= 0.0 {
            1.0
        } else {
            (self.elapsed / self.duration).clamp(0.0, 1.0)
        }
    }

    pub fn settled(&self) -> bool {
        self.elapsed >= self.duration
    }

    pub fn tick(&mut self, dt: f32) {
        self.elapsed = (self.elapsed + dt).min(self.duration);
    }
}

/// A row of strips, staggered so they stop left to right.
#[derive(Debug, Clone)]
pub struct StripSpinner {
    strips: Vec<StripAnimation>,
    announced: usize,
    feel: StripFeel,
}

impl StripSpinner {
    /// `held` marks the strips to hold back. Deciding *which* is game logic —
    /// a near miss, a bonus about to land — and stays with the caller.
    pub fn new(
        strip_lengths: &[usize],
        from: &[usize],
        to: &[usize],
        time_scale: f32,
        held: &[bool],
        feel: &StripFeel,
    ) -> Self {
        let strips = strip_lengths
            .iter()
            .enumerate()
            .map(|(index, len)| {
                StripAnimation::new(
                    *len,
                    from.get(index).copied().unwrap_or(0),
                    to.get(index).copied().unwrap_or(0),
                    index,
                    time_scale,
                    held.get(index).copied().unwrap_or(false),
                    feel,
                )
            })
            .collect();

        Self {
            strips,
            announced: 0,
            feel: *feel,
        }
    }

    /// Advance every strip and return the indices that came to rest this tick,
    /// in stop order. Each index is reported exactly once.
    pub fn tick(&mut self, dt: f32) -> Vec<usize> {
        for strip in &mut self.strips {
            strip.tick(dt);
        }

        let mut stopped = Vec::new();
        while self.announced < self.strips.len() && self.strips[self.announced].settled() {
            stopped.push(self.announced);
            self.announced += 1;
        }
        stopped
    }

    pub fn len(&self) -> usize {
        self.strips.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strips.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&StripAnimation> {
        self.strips.get(index)
    }

    pub fn all_settled(&self) -> bool {
        self.strips.iter().all(StripAnimation::settled)
    }

    pub fn position(&self, index: usize) -> f32 {
        self.strips
            .get(index)
            .map(StripAnimation::position)
            .unwrap_or(0.0)
    }

    pub fn is_moving(&self, index: usize) -> bool {
        self.strips.get(index).is_some_and(|s| !s.settled())
    }

    pub fn is_held(&self, index: usize) -> bool {
        self.strips.get(index).is_some_and(StripAnimation::is_held)
    }

    /// True while the strip is moving fast enough to warrant motion blur.
    pub fn is_blurred(&self, index: usize) -> bool {
        self.strips
            .get(index)
            .is_some_and(|s| s.speed() > self.feel.blur_speed)
    }

    /// Symbols this strip covers in a single 60 Hz frame — the distance motion
    /// blur has to smear over.
    pub fn blur_symbols(&self, index: usize) -> f32 {
        self.strips
            .get(index)
            .map_or(0.0, |s| (s.speed() / 60.0).min(self.feel.blur_cap))
    }
}

/// Sub-frame offsets for drawing a blurred strip, spread either side of its
/// current position.
///
/// Draw the strip once per offset at `1.0 / passes` alpha and the symbols
/// streak. **Draw only the art that way** — repeating whatever the symbols sit
/// *on* stacks that many translucent backgrounds and washes the whole thing
/// toward white, which is exactly what happened the first time.
pub fn blur_offsets(smear: f32, passes: usize) -> Vec<f32> {
    if passes <= 1 {
        return vec![0.0];
    }
    (0..passes)
        .map(|pass| smear * (pass as f32 / (passes - 1) as f32 - 0.5))
        .collect()
}

#[cfg(test)]
mod tests;
