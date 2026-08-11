//! Showing a decided outcome a piece at a time.
//!
//! # The shape both of these have
//!
//! A game works out what happened, and then has to *show* it. Those are
//! different jobs and mixing them is how a reveal ends up able to change the
//! result — which is why the game this came from fixes every outcome at commit
//! and lets nothing in the animation touch it.
//!
//! What is left for the animation is a cursor and a clock. Two shapes cover
//! almost all of it:
//!
//! - [`Countup`] — a number that arrives over a moment rather than snapping on.
//!   A win, a score, a resource total, damage dealt.
//! - [`Stepper`] — a cursor through a fixed number of stages on a beat. A
//!   cascade chain, a card deal, a queue of match-3 collapses, a dialogue
//!   reveal.
//!
//! Neither holds any of the content. `Stepper` does not know what stage three
//! looks like and `Countup` does not know what the number means. They are the
//! timing, and the caller keeps the truth — which is the property that makes
//! them safe to reuse.
//!
//! # Both scale
//!
//! Every constructor takes a `scale` multiplying the duration, because a game
//! that lets a player speed animations up (or a headless test that wants them
//! instant) needs one number to turn. A scale of zero would make a `Timer` that
//! never finishes, so it is clamped rather than trusted.

use crate::math::ease_out_quad;
use crate::timing::Timer;

/// Sensible bounds for a speed multiplier.
///
/// Zero is the dangerous one: it means "no time at all", which reads as a
/// division by zero to anything measuring progress, and gives an animation that
/// never completes rather than one that completes instantly.
const MIN_SCALE: f32 = 0.05;
const MAX_SCALE: f32 = 4.0;

/// A number that arrives over a moment instead of snapping on.
#[derive(Debug, Clone)]
pub struct Countup {
    target: i64,
    timer: Timer,
}

impl Countup {
    /// Count to `target` over `seconds`, adjusted by `scale`.
    pub fn new(target: i64, seconds: f32, scale: f32) -> Self {
        Self {
            target,
            timer: Timer::new(seconds * scale.clamp(MIN_SCALE, MAX_SCALE)),
        }
    }

    /// Advance. Returns `true` on the tick that finishes the count.
    pub fn tick(&mut self, dt: f32) -> bool {
        self.timer.tick(dt)
    }

    /// Where the number is now.
    ///
    /// Eased out, so it moves fastest at the start and settles rather than
    /// stopping dead. A linear count-up reads as a progress bar; this reads as
    /// a number arriving.
    pub fn value(&self) -> i64 {
        let progress = self.timer.progress().clamp(0.0, 1.0);
        (self.target as f64 * ease_out_quad(progress) as f64) as i64
    }

    pub fn target(&self) -> i64 {
        self.target
    }

    pub fn finished(&self) -> bool {
        self.timer.progress() >= 1.0
    }
}

/// A cursor through a fixed number of stages, moving on a beat.
#[derive(Debug, Clone)]
pub struct Stepper {
    step: usize,
    steps: usize,
    /// Set when the *last* stage has had its beat, not when it is reached.
    ///
    /// This distinction is the whole reason this type is worth sharing. Without
    /// it the final stage of a sequence is considered done the instant the
    /// cursor arrives at it, so it is never actually shown — the animation ends
    /// on the second-to-last thing and the last one appears only because
    /// whatever comes next draws it.
    done: bool,
    beat: Timer,
    seconds: f32,
}

impl Stepper {
    /// Step through `steps` stages, one every `seconds`, adjusted by `scale`.
    pub fn new(steps: usize, seconds: f32, scale: f32) -> Self {
        let seconds = seconds * scale.clamp(MIN_SCALE, MAX_SCALE);
        Self {
            step: 0,
            steps,
            // A sequence with nothing in it is over before it starts, and
            // saying so here saves every caller a special case.
            done: steps == 0,
            beat: Timer::new(seconds),
            seconds,
        }
    }

    pub fn step(&self) -> usize {
        self.step
    }

    pub fn steps(&self) -> usize {
        self.steps
    }

    /// Advance on the beat. Returns `true` on the tick that moves to a new
    /// stage, so a caller can make a noise exactly once per step.
    pub fn tick(&mut self, dt: f32) -> bool {
        if self.done || !self.beat.tick(dt) {
            return false;
        }
        if self.step + 1 >= self.steps {
            self.done = true;
            return false;
        }
        self.step += 1;
        self.beat = Timer::new(self.seconds);
        true
    }

    pub fn finished(&self) -> bool {
        self.done
    }
}

#[cfg(test)]
mod tests;
