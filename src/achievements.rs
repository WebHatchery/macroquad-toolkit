//! A small achievement registry: definitions, unlock tracking, and
//! persistence-friendly state.
//!
//! Extracted from the parallel implementations in nightmare_shift
//! (unlock dates, init/check helpers) and nanite_swarm (unlock + progress
//! counts).

use serde::{Deserialize, Serialize};

/// One achievement definition plus its unlock state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub unlocked: bool,
    /// Optional caller-formatted timestamp captured at unlock time.
    #[serde(default)]
    pub unlock_date: Option<String>,
}

impl Achievement {
    /// Creates a locked achievement definition.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            unlocked: false,
            unlock_date: None,
        }
    }
}

/// An ordered set of achievements with id-based unlocking.
///
/// Serialize the whole registry into your save; on load, call
/// [`sync_definitions`](Self::sync_definitions) with the current definition
/// list so renamed text updates and newly added achievements appear while
/// unlock state is preserved.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Achievements {
    achievements: Vec<Achievement>,
}

impl Achievements {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a registry from definitions.
    pub fn from_definitions(definitions: Vec<Achievement>) -> Self {
        Self {
            achievements: definitions,
        }
    }

    /// Reconciles saved state with the current definitions: definition
    /// order/name/description win, unlock state and dates are preserved,
    /// and achievements no longer defined are dropped.
    pub fn sync_definitions(&mut self, definitions: Vec<Achievement>) {
        let previous = std::mem::take(&mut self.achievements);
        self.achievements = definitions
            .into_iter()
            .map(|mut definition| {
                if let Some(saved) = previous.iter().find(|saved| saved.id == definition.id) {
                    definition.unlocked = saved.unlocked;
                    definition.unlock_date.clone_from(&saved.unlock_date);
                }
                definition
            })
            .collect();
    }

    /// Unlocks by id. Returns true only when newly unlocked (so callers can
    /// fire a notification exactly once).
    pub fn unlock(&mut self, id: &str) -> bool {
        self.unlock_with_date(id, None)
    }

    /// Unlocks by id with a caller-formatted date string.
    pub fn unlock_with_date(&mut self, id: &str, date: Option<String>) -> bool {
        if let Some(achievement) = self
            .achievements
            .iter_mut()
            .find(|achievement| achievement.id == id)
        {
            if !achievement.unlocked {
                achievement.unlocked = true;
                achievement.unlock_date = date;
                return true;
            }
        }
        false
    }

    /// True when the achievement exists and is unlocked.
    pub fn is_unlocked(&self, id: &str) -> bool {
        self.achievements
            .iter()
            .any(|achievement| achievement.id == id && achievement.unlocked)
    }

    /// Looks up an achievement by id.
    pub fn get(&self, id: &str) -> Option<&Achievement> {
        self.achievements
            .iter()
            .find(|achievement| achievement.id == id)
    }

    /// `(unlocked, total)` counts for progress displays.
    pub fn progress(&self) -> (usize, usize) {
        let unlocked = self
            .achievements
            .iter()
            .filter(|achievement| achievement.unlocked)
            .count();
        (unlocked, self.achievements.len())
    }

    /// Iterates all achievements in definition order.
    pub fn iter(&self) -> impl Iterator<Item = &Achievement> {
        self.achievements.iter()
    }

    /// Number of defined achievements.
    pub fn len(&self) -> usize {
        self.achievements.len()
    }

    /// True when no achievements are defined.
    pub fn is_empty(&self) -> bool {
        self.achievements.is_empty()
    }
}

#[cfg(test)]
mod tests;
