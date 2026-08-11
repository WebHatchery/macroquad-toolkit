//! Timer-based autosave helper for immediate-mode game loops.

/// Timer-based autosave helper for immediate-mode game loops.
#[derive(Debug, Clone)]
pub struct AutoSaveManager {
    interval_seconds: f32,
    elapsed_seconds: f32,
    enabled: bool,
}

impl AutoSaveManager {
    pub fn new(interval_seconds: f32) -> Self {
        Self {
            interval_seconds: interval_seconds.max(0.0),
            elapsed_seconds: 0.0,
            enabled: true,
        }
    }

    pub fn interval_seconds(&self) -> f32 {
        self.interval_seconds
    }

    pub fn set_interval_seconds(&mut self, interval_seconds: f32) {
        self.interval_seconds = interval_seconds.max(0.0);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn reset_timer(&mut self) {
        self.elapsed_seconds = 0.0;
    }

    /// Advance the autosave timer and run `save` when the interval elapses.
    ///
    /// Returns `Ok(true)` only when a save was actually performed.
    pub fn update<F>(&mut self, dt: f32, should_save: bool, save: F) -> Result<bool, String>
    where
        F: FnOnce() -> Result<(), String>,
    {
        if !self.enabled || !should_save {
            return Ok(false);
        }

        self.elapsed_seconds += dt.max(0.0);
        if self.elapsed_seconds < self.interval_seconds {
            return Ok(false);
        }

        save()?;
        self.reset_timer();
        Ok(true)
    }

    /// Run `save` immediately and reset the timer.
    pub fn force<F>(&mut self, save: F) -> Result<(), String>
    where
        F: FnOnce() -> Result<(), String>,
    {
        save()?;
        self.reset_timer();
        Ok(())
    }
}

impl Default for AutoSaveManager {
    fn default() -> Self {
        Self::new(60.0)
    }
}

#[cfg(test)]
mod tests;
