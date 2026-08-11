//! Generic Entity-Component System for game development
//!
//! Provides a lightweight entity management system suitable for grid-based games,
//! strategy games, and other scenarios where a simple entity system is sufficient.
//!
//! # Example
//! ```
//! use macroquad_toolkit::entities::{EntityManager, EntityId};
//!
//! // Define your own component types
//! #[derive(Clone)]
//! struct Position { x: i32, y: i32 }
//!
//! #[derive(Clone)]
//! struct Health { current: f32, max: f32 }
//!
//! // Create a custom entity type
//! #[derive(Clone)]
//! struct GameEntity {
//!     pos: Position,
//!     health: Option<Health>,
//! }
//!
//! let mut entities: EntityManager<GameEntity> = EntityManager::new();
//! let id = entities.spawn(GameEntity {
//!     pos: Position { x: 5, y: 5 },
//!     health: Some(Health { current: 100.0, max: 100.0 }),
//! });
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for an entity
pub type EntityId = usize;

/// Generic entity manager that stores entities of type T
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityManager<T> {
    entities: HashMap<EntityId, T>,
    next_id: EntityId,
}

impl<T> EntityManager<T> {
    /// Create a new entity manager
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            next_id: 1,
        }
    }

    /// Spawn a new entity and return its ID
    pub fn spawn(&mut self, entity: T) -> EntityId {
        let id = self.next_id;
        self.next_id += 1;
        self.entities.insert(id, entity);
        id
    }

    /// Spawn an entity with a specific ID (useful for loading saves)
    pub fn spawn_with_id(&mut self, id: EntityId, entity: T) -> EntityId {
        self.entities.insert(id, entity);
        if id >= self.next_id {
            self.next_id = id + 1;
        }
        id
    }

    /// Get an entity by ID
    pub fn get(&self, id: EntityId) -> Option<&T> {
        self.entities.get(&id)
    }

    /// Get a mutable reference to an entity by ID
    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut T> {
        self.entities.get_mut(&id)
    }

    /// Remove an entity and return it
    pub fn remove(&mut self, id: EntityId) -> Option<T> {
        self.entities.remove(&id)
    }

    /// Check if an entity exists
    pub fn contains(&self, id: EntityId) -> bool {
        self.entities.contains_key(&id)
    }

    /// Get all entities as an iterator
    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &T)> {
        self.entities.iter().map(|(id, e)| (*id, e))
    }

    /// Get all entities as a mutable iterator
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (EntityId, &mut T)> {
        self.entities.iter_mut().map(|(id, e)| (*id, e))
    }

    /// Get just the entities (without IDs)
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.entities.values()
    }

    /// Get mutable references to just the entities
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.entities.values_mut()
    }

    /// Get just the entity IDs
    pub fn ids(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.entities.keys().copied()
    }

    /// Get the internal HashMap (for advanced operations)
    pub fn entities(&self) -> &HashMap<EntityId, T> {
        &self.entities
    }

    /// Get the internal HashMap mutably
    pub fn entities_mut(&mut self) -> &mut HashMap<EntityId, T> {
        &mut self.entities
    }

    /// Get the number of entities
    pub fn count(&self) -> usize {
        self.entities.len()
    }

    /// Check if there are no entities
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Clear all entities
    pub fn clear(&mut self) {
        self.entities.clear();
    }

    /// Retain only entities that satisfy the predicate
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&EntityId, &mut T) -> bool,
    {
        self.entities.retain(f);
    }

    /// Get the next ID that would be assigned
    pub fn peek_next_id(&self) -> EntityId {
        self.next_id
    }
}

impl<T> Default for EntityManager<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> EntityManager<T> {
    /// Clone an entity if it exists
    pub fn clone_entity(&self, id: EntityId) -> Option<T> {
        self.entities.get(&id).cloned()
    }
}

/// Trait for entities that have a position
pub trait HasPosition {
    fn position(&self) -> (i32, i32);
    fn set_position(&mut self, x: i32, y: i32);
}

/// Trait for entities that have health
pub trait HasHealth {
    fn health(&self) -> f32;
    fn max_health(&self) -> f32;
    fn set_health(&mut self, health: f32);

    fn is_alive(&self) -> bool {
        self.health() > 0.0
    }

    fn health_percentage(&self) -> f32 {
        self.health() / self.max_health()
    }

    fn heal(&mut self, amount: f32) {
        let new_health = (self.health() + amount).min(self.max_health());
        self.set_health(new_health);
    }

    fn damage(&mut self, amount: f32) {
        let new_health = (self.health() - amount).max(0.0);
        self.set_health(new_health);
    }
}

/// Extension methods for EntityManager when T implements HasPosition
impl<T: HasPosition> EntityManager<T> {
    /// Find all entities at a specific position
    pub fn at_position(&self, x: i32, y: i32) -> Vec<(EntityId, &T)> {
        self.entities
            .iter()
            .filter(|(_, e)| e.position() == (x, y))
            .map(|(id, e)| (*id, e))
            .collect()
    }

    /// Find the first entity at a position
    pub fn first_at_position(&self, x: i32, y: i32) -> Option<(EntityId, &T)> {
        self.entities
            .iter()
            .find(|(_, e)| e.position() == (x, y))
            .map(|(id, e)| (*id, e))
    }

    /// Get all entity positions
    pub fn positions(&self) -> Vec<(EntityId, (i32, i32))> {
        self.entities
            .iter()
            .map(|(id, e)| (*id, e.position()))
            .collect()
    }
}

/// Extension methods for EntityManager when T implements HasHealth
impl<T: HasHealth> EntityManager<T> {
    /// Remove all dead entities, returning count removed
    pub fn remove_dead(&mut self) -> usize {
        let dead_ids: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|(_, e)| !e.is_alive())
            .map(|(id, _)| *id)
            .collect();

        let count = dead_ids.len();
        for id in dead_ids {
            self.entities.remove(&id);
        }
        count
    }

    /// Get all living entities
    pub fn living(&self) -> impl Iterator<Item = (EntityId, &T)> {
        self.entities
            .iter()
            .filter(|(_, e)| e.is_alive())
            .map(|(id, e)| (*id, e))
    }

    /// Count living entities
    pub fn count_living(&self) -> usize {
        self.entities.values().filter(|e| e.is_alive()).count()
    }
}

/// Simple status effect that can be applied to entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEffect {
    /// Identifier for the effect type (e.g., "poison", "burning", "frozen")
    pub effect_type: String,
    /// Duration remaining in seconds
    pub duration: f32,
    /// Strength/intensity of the effect
    pub strength: f32,
}

impl StatusEffect {
    /// Create a new status effect
    pub fn new(effect_type: impl Into<String>, duration: f32, strength: f32) -> Self {
        Self {
            effect_type: effect_type.into(),
            duration,
            strength,
        }
    }

    /// Check if the effect has expired
    pub fn is_expired(&self) -> bool {
        self.duration <= 0.0
    }

    /// Update the effect (reduce duration)
    pub fn tick(&mut self, dt: f32) {
        self.duration -= dt;
    }
}

/// Trait for entities that can have status effects
pub trait HasStatusEffects {
    fn status_effects(&self) -> &[StatusEffect];
    fn status_effects_mut(&mut self) -> &mut Vec<StatusEffect>;

    /// Add a status effect
    fn add_effect(&mut self, effect: StatusEffect) {
        self.status_effects_mut().push(effect);
    }

    /// Remove expired effects
    fn remove_expired_effects(&mut self) {
        self.status_effects_mut().retain(|e| !e.is_expired());
    }

    /// Check if entity has a specific effect type
    fn has_effect(&self, effect_type: &str) -> bool {
        self.status_effects()
            .iter()
            .any(|e| e.effect_type == effect_type)
    }

    /// Get the total strength of an effect type (sum of all matching effects)
    fn effect_strength(&self, effect_type: &str) -> f32 {
        self.status_effects()
            .iter()
            .filter(|e| e.effect_type == effect_type)
            .map(|e| e.strength)
            .sum()
    }

    /// Update all effects
    fn tick_effects(&mut self, dt: f32) {
        for effect in self.status_effects_mut() {
            effect.tick(dt);
        }
        self.remove_expired_effects();
    }
}

#[cfg(test)]
mod tests;
