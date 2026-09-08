use super::GameSettings;

/// Transactional settings editor. Changes stay in the draft until persisted.
/// Games call runtime apply methods only after `commit` succeeds.
#[derive(Debug, Clone)]
pub struct SettingsSession {
    pub draft: GameSettings,
    saved: GameSettings,
    defaults: GameSettings,
}

impl SettingsSession {
    pub fn new(mut saved: GameSettings, mut defaults: GameSettings) -> Self {
        saved.sanitize();
        defaults.sanitize();
        Self {
            draft: saved.clone(),
            saved,
            defaults,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.draft != self.saved
    }

    pub fn reset_defaults(&mut self) {
        self.draft = self.defaults.clone();
    }

    pub fn cancel(&mut self) {
        self.draft = self.saved.clone();
    }

    pub fn commit(&mut self, game_name: &str) -> Result<(), String> {
        self.commit_with(|settings| settings.save(game_name))
    }

    /// Inject storage for integrations and tests. A failed write retains the
    /// draft for retry and leaves the committed snapshot untouched.
    pub fn commit_with(
        &mut self,
        save: impl FnOnce(&GameSettings) -> Result<(), String>,
    ) -> Result<(), String> {
        self.draft.sanitize();
        save(&self.draft)?;
        self.saved = self.draft.clone();
        Ok(())
    }
}
