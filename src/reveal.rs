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
mod tests {
    use super::*;

    #[test]
    fn a_countup_starts_at_nothing_and_arrives_at_the_target() {
        let mut count = Countup::new(1_000, 0.5, 1.0);
        assert_eq!(count.value(), 0);
        assert!(!count.finished());

        assert!(count.tick(0.5), "the tick that completes it says so");
        assert_eq!(count.value(), 1_000);
        assert!(count.finished());
    }

    /// Eased, not linear: halfway through the time is past halfway to the
    /// number. A linear count reads as a progress bar rather than a payout.
    #[test]
    fn a_countup_is_fastest_at_the_start() {
        let mut count = Countup::new(1_000, 1.0, 1.0);
        count.tick(0.5);
        assert!(
            count.value() > 500,
            "halfway through the time it had only reached {}",
            count.value()
        );
        assert!(count.value() < 1_000);
    }

    /// The failure this guards: a scale of zero is a timer that never finishes,
    /// so the number would sit at nothing forever.
    #[test]
    fn a_countup_survives_a_nonsense_scale() {
        for scale in [0.0, -3.0, 1e9] {
            let mut count = Countup::new(500, 0.5, scale);
            for _ in 0..200 {
                count.tick(0.1);
            }
            assert!(count.finished(), "scale {} never finished", scale);
            assert_eq!(count.value(), 500);
        }
    }

    /// A big target must not overflow on the way. `i64::MAX` through an `f32`
    /// multiply was the shape of it.
    #[test]
    fn a_countup_handles_a_large_target() {
        let mut count = Countup::new(9_000_000_000, 0.2, 1.0);
        count.tick(0.2);
        assert_eq!(count.value(), 9_000_000_000);
    }

    /// Exactly once, however many ticks arrive after it.
    ///
    /// The return value is what makes a noise or fires an event, so a counter
    /// that kept saying "finished" on every later frame would play the payout
    /// sound forever.
    #[test]
    fn a_countup_reports_finishing_exactly_once() {
        let mut count = Countup::new(10, 0.5, 1.0);
        let mut finishes = 0;
        for _ in 0..200 {
            if count.tick(1.0 / 60.0) {
                finishes += 1;
            }
        }
        assert_eq!(finishes, 1);
    }

    #[test]
    fn a_stepper_visits_every_stage_in_order() {
        let mut stepper = Stepper::new(4, 0.5, 1.0);
        let mut visited = vec![stepper.step()];
        for _ in 0..40 {
            if stepper.tick(0.1) {
                visited.push(stepper.step());
            }
        }
        assert_eq!(visited, vec![0, 1, 2, 3]);
    }

    /// The distinction the doc comment is about: the last stage gets a beat of
    /// its own before the sequence is over.
    #[test]
    fn the_last_stage_is_held_before_the_sequence_ends() {
        let mut stepper = Stepper::new(2, 0.5, 1.0);
        assert!(stepper.tick(0.5), "moved to the last stage");
        assert_eq!(stepper.step(), 1);
        assert!(
            !stepper.finished(),
            "the last stage was over the moment it was reached, so nobody saw it"
        );

        assert!(!stepper.tick(0.5), "no new stage, but the beat has passed");
        assert!(stepper.finished());
    }

    /// An empty sequence is over before it starts, so callers need no special
    /// case for it.
    #[test]
    fn a_stepper_with_nothing_to_show_is_already_finished() {
        let mut stepper = Stepper::new(0, 0.5, 1.0);
        assert!(stepper.finished());
        assert!(!stepper.tick(10.0));
        assert_eq!(stepper.step(), 0);
    }

    /// One stage is shown once and then done — the degenerate case either side
    /// of the one above.
    #[test]
    fn a_stepper_with_one_stage_shows_it_and_stops() {
        let mut stepper = Stepper::new(1, 0.5, 1.0);
        assert!(!stepper.finished());
        assert!(!stepper.tick(0.5), "there is no second stage to move to");
        assert!(stepper.finished());
        assert_eq!(stepper.step(), 0);
    }
}
