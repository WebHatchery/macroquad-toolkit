use super::*;

#[derive(Clone)]
struct TestEntity {
    x: i32,
    y: i32,
    hp: f32,
    max_hp: f32,
}

impl HasPosition for TestEntity {
    fn position(&self) -> (i32, i32) {
        (self.x, self.y)
    }
    fn set_position(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }
}

impl HasHealth for TestEntity {
    fn health(&self) -> f32 {
        self.hp
    }
    fn max_health(&self) -> f32 {
        self.max_hp
    }
    fn set_health(&mut self, health: f32) {
        self.hp = health;
    }
}

#[test]
fn test_entity_manager_basic() {
    let mut manager: EntityManager<TestEntity> = EntityManager::new();

    let id1 = manager.spawn(TestEntity {
        x: 0,
        y: 0,
        hp: 100.0,
        max_hp: 100.0,
    });
    let id2 = manager.spawn(TestEntity {
        x: 5,
        y: 5,
        hp: 50.0,
        max_hp: 100.0,
    });

    assert_eq!(manager.count(), 2);
    assert!(manager.contains(id1));
    assert!(manager.contains(id2));

    let entity = manager.get(id1).unwrap();
    assert_eq!(entity.position(), (0, 0));
}

#[test]
fn test_position_query() {
    let mut manager: EntityManager<TestEntity> = EntityManager::new();

    manager.spawn(TestEntity {
        x: 5,
        y: 5,
        hp: 100.0,
        max_hp: 100.0,
    });
    manager.spawn(TestEntity {
        x: 5,
        y: 5,
        hp: 50.0,
        max_hp: 100.0,
    });
    manager.spawn(TestEntity {
        x: 10,
        y: 10,
        hp: 75.0,
        max_hp: 100.0,
    });

    let at_5_5 = manager.at_position(5, 5);
    assert_eq!(at_5_5.len(), 2);

    let at_10_10 = manager.at_position(10, 10);
    assert_eq!(at_10_10.len(), 1);
}

#[test]
fn test_remove_dead() {
    let mut manager: EntityManager<TestEntity> = EntityManager::new();

    manager.spawn(TestEntity {
        x: 0,
        y: 0,
        hp: 100.0,
        max_hp: 100.0,
    });
    manager.spawn(TestEntity {
        x: 1,
        y: 1,
        hp: 0.0,
        max_hp: 100.0,
    }); // Dead
    manager.spawn(TestEntity {
        x: 2,
        y: 2,
        hp: 50.0,
        max_hp: 100.0,
    });

    assert_eq!(manager.count(), 3);

    let removed = manager.remove_dead();
    assert_eq!(removed, 1);
    assert_eq!(manager.count(), 2);
    assert_eq!(manager.count_living(), 2);
}
