use super::{finite, GameSettings};

/// Timed trial for fullscreen/UI changes. Tick with real elapsed time even
/// while gameplay is paused. Confirm before saving; revert on exit or timeout.
#[derive(Debug)]
pub struct DisplayPreview {
    original: GameSettings,
    remaining: f32,
    active: bool,
}

impl DisplayPreview {
    pub fn begin(original: &GameSettings, candidate: &GameSettings) -> Self {
        let preview = Self::new(original.clone());
        candidate.apply_display();
        preview
    }

    fn new(original: GameSettings) -> Self {
        Self {
            original,
            remaining: 15.0,
            active: true,
        }
    }

    pub fn remaining(&self) -> f32 {
        self.remaining
    }
    pub fn confirm(&mut self) {
        self.active = false;
    }

    /// Returns true when the trial expired and restored the original settings.
    pub fn update(&mut self, real_dt: f32) -> bool {
        if self.elapse(real_dt) {
            self.original.apply_display();
            return true;
        }
        false
    }

    fn elapse(&mut self, dt: f32) -> bool {
        if !self.active {
            return false;
        }
        self.remaining -= finite(dt, 0.0, f32::MAX, 0.0);
        if self.remaining <= 0.0 {
            self.active = false;
            return true;
        }
        false
    }

    pub fn revert(&mut self) {
        if self.active {
            self.original.apply_display();
            self.active = false;
        }
    }
}

#[cfg(test)]
mod tests;
