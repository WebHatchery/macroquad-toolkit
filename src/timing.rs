//! Timers, cooldowns, interval tickers, and phase timelines.
//!
//! Replaces the bare `x = (x - dt).max(0.0)` cooldown fields, `*_accum`
//! interval accumulators, and hand-stepped phase state machines repeated
//! across carriage_run, dungeon_core, dungeon_manager, scrapyard, sentience,
//! feast_frenzy, and kaiju_sim.

use serde::{Deserialize, Serialize};

/// A count-down cooldown. Starts ready; [`Cooldown::trigger`] arms it.
///
/// ```
/// use macroquad_toolkit::timing::Cooldown;
///
/// let mut fire = Cooldown::new(0.5);
/// assert!(fire.try_trigger());
/// assert!(!fire.try_trigger());
/// fire.tick(0.5);
/// assert!(fire.is_ready());
/// ```
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Cooldown {
    duration: f32,
    remaining: f32,
}

impl Cooldown {
    /// Creates a ready cooldown with the given duration in seconds.
    pub fn new(duration: f32) -> Self {
        Self {
            duration: duration.max(0.0),
            remaining: 0.0,
        }
    }

    /// Creates a cooldown that starts armed (must tick down before first use).
    pub fn new_armed(duration: f32) -> Self {
        let mut cooldown = Self::new(duration);
        cooldown.trigger();
        cooldown
    }

    /// Counts down by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        self.remaining = (self.remaining - dt).max(0.0);
    }

    /// True when the cooldown has fully elapsed.
    pub fn is_ready(&self) -> bool {
        self.remaining <= 0.0
    }

    /// Arms the cooldown to its full duration.
    pub fn trigger(&mut self) {
        self.remaining = self.duration;
    }

    /// If ready, triggers and returns true; otherwise returns false.
    pub fn try_trigger(&mut self) -> bool {
        if self.is_ready() {
            self.trigger();
            true
        } else {
            false
        }
    }

    /// Forces the cooldown to be ready immediately.
    pub fn reset(&mut self) {
        self.remaining = 0.0;
    }

    /// Seconds left until ready.
    pub fn remaining(&self) -> f32 {
        self.remaining
    }

    /// Configured duration in seconds.
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Fraction still remaining, `1.0` just after trigger down to `0.0` when ready.
    pub fn fraction_remaining(&self) -> f32 {
        if self.duration <= 0.0 {
            0.0
        } else {
            (self.remaining / self.duration).clamp(0.0, 1.0)
        }
    }

    /// Fraction elapsed, `0.0` just after trigger up to `1.0` when ready.
    pub fn fraction_elapsed(&self) -> f32 {
        1.0 - self.fraction_remaining()
    }
}

/// A count-up one-shot timer with `0..=1` progress, e.g. for hit flashes,
/// travel lerps, and fade lifetimes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Timer {
    duration: f32,
    elapsed: f32,
}

impl Timer {
    /// Creates a timer that finishes after `duration` seconds.
    pub fn new(duration: f32) -> Self {
        Self {
            duration: duration.max(0.0),
            elapsed: 0.0,
        }
    }

    /// Advances the timer. Returns true on the tick that finishes it.
    pub fn tick(&mut self, dt: f32) -> bool {
        if self.finished() {
            return false;
        }
        self.elapsed = (self.elapsed + dt).min(self.duration);
        self.finished()
    }

    /// True once the full duration has elapsed.
    pub fn finished(&self) -> bool {
        self.elapsed >= self.duration
    }

    /// Progress from `0.0` to `1.0`. A zero-duration timer reports `1.0`.
    pub fn progress(&self) -> f32 {
        if self.duration <= 0.0 {
            1.0
        } else {
            (self.elapsed / self.duration).clamp(0.0, 1.0)
        }
    }

    /// Remaining fraction from `1.0` down to `0.0` (handy for fade alpha).
    pub fn fraction_remaining(&self) -> f32 {
        1.0 - self.progress()
    }

    /// Seconds elapsed so far.
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// Configured duration in seconds.
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Restarts the timer from zero.
    pub fn reset(&mut self) {
        self.elapsed = 0.0;
    }

    /// Jumps the timer to its finished state.
    pub fn finish(&mut self) {
        self.elapsed = self.duration;
    }
}

/// Fires every `interval` seconds, accumulating fractional time so no fire
/// is lost on slow frames ("fire every N ms" spawn/decay/autosave loops).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct IntervalTimer {
    interval: f32,
    accumulated: f32,
}

impl IntervalTimer {
    /// Creates a timer that fires every `interval` seconds (minimum 1ms).
    pub fn new(interval: f32) -> Self {
        Self {
            interval: interval.max(0.001),
            accumulated: 0.0,
        }
    }

    /// Advances by `dt`, returning how many complete intervals fired.
    pub fn tick(&mut self, dt: f32) -> u32 {
        self.accumulated += dt.max(0.0);
        // Small relative tolerance so accumulated float error (e.g. two 0.05
        // ticks against a 0.1 interval) still fires on time.
        let epsilon = self.interval * 1e-4;
        let mut fires = 0;
        while self.accumulated + epsilon >= self.interval {
            self.accumulated = (self.accumulated - self.interval).max(0.0);
            fires += 1;
        }
        fires
    }

    /// Advances by `dt`, returning true if at least one interval fired.
    /// Excess intervals beyond the first are discarded.
    pub fn tick_once(&mut self, dt: f32) -> bool {
        let fired = self.tick(dt) > 0;
        if fired {
            self.accumulated = 0.0;
        }
        fired
    }

    /// Interval length in seconds.
    pub fn interval(&self) -> f32 {
        self.interval
    }

    /// Changes the interval, keeping accumulated time.
    pub fn set_interval(&mut self, interval: f32) {
        self.interval = interval.max(0.001);
    }

    /// Clears accumulated time.
    pub fn reset(&mut self) {
        self.accumulated = 0.0;
    }
}

/// An ordered sequence of timed phases, each yielding a local `0..=1` progress.
///
/// Extracted from feast_frenzy's cinematic phase table and kaiju_sim's
/// hand-stepped animation state machine.
///
/// ```
/// use macroquad_toolkit::timing::Timeline;
///
/// #[derive(Clone, Copy, PartialEq, Debug)]
/// enum Swing { WindUp, Strike, Recover }
///
/// let mut swing = Timeline::new(vec![
///     (Swing::WindUp, 0.2),
///     (Swing::Strike, 0.1),
///     (Swing::Recover, 0.3),
/// ]);
/// swing.advance(0.25);
/// let (phase, progress) = swing.current().unwrap();
/// assert_eq!(*phase, Swing::Strike);
/// assert!((progress - 0.5).abs() < 1e-4);
/// ```
#[derive(Debug, Clone)]
pub struct Timeline<T> {
    phases: Vec<(T, f32)>,
    elapsed: f32,
}

impl<T> Timeline<T> {
    /// Creates a timeline from `(phase, duration_seconds)` pairs.
    pub fn new(phases: Vec<(T, f32)>) -> Self {
        Self {
            phases,
            elapsed: 0.0,
        }
    }

    /// Advances the timeline by `dt` seconds, clamping at the end.
    pub fn advance(&mut self, dt: f32) {
        self.elapsed = (self.elapsed + dt.max(0.0)).min(self.total_duration());
    }

    /// The active phase and its local `0..=1` progress, or `None` when finished
    /// (or the timeline is empty).
    pub fn current(&self) -> Option<(&T, f32)> {
        if self.finished() {
            return None;
        }
        let mut start = 0.0;
        for (phase, duration) in &self.phases {
            let end = start + duration.max(0.0);
            if self.elapsed < end || *duration <= 0.0 && self.elapsed <= start {
                let progress = if *duration <= 0.0 {
                    1.0
                } else {
                    ((self.elapsed - start) / duration).clamp(0.0, 1.0)
                };
                return Some((phase, progress));
            }
            start = end;
        }
        None
    }

    /// True once all phases have elapsed. An empty timeline is finished.
    pub fn finished(&self) -> bool {
        self.elapsed >= self.total_duration()
    }

    /// Sum of all phase durations.
    pub fn total_duration(&self) -> f32 {
        self.phases.iter().map(|(_, d)| d.max(0.0)).sum()
    }

    /// Seconds elapsed since the timeline started.
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// Overall `0..=1` progress across the whole timeline.
    pub fn overall_progress(&self) -> f32 {
        let total = self.total_duration();
        if total <= 0.0 {
            1.0
        } else {
            (self.elapsed / total).clamp(0.0, 1.0)
        }
    }

    /// Restarts the timeline from the first phase.
    pub fn reset(&mut self) {
        self.elapsed = 0.0;
    }

    /// Jumps to the end of the timeline.
    pub fn skip_to_end(&mut self) {
        self.elapsed = self.total_duration();
    }
}

#[cfg(test)]
mod tests;
