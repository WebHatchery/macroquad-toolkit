use super::*;

#[test]
fn history_keeps_what_the_toasts_let_go_of() {
    let mut manager = NotificationManager::new();
    manager.success("first");
    manager.warning("second");
    // Long enough that both toasts are gone from the screen.
    manager.update(100.0);

    assert!(manager.is_empty(), "the toasts should have faded");
    assert_eq!(manager.history().len(), 2);
    assert_eq!(manager.history()[0].message, "first");
    assert_eq!(
        manager.history()[1].notification_type,
        NotificationType::Warning
    );
}

#[test]
fn history_drops_the_oldest_entry_rather_than_the_newest() {
    let mut manager = NotificationManager::new();
    manager.set_history_limit(2);
    for message in ["one", "two", "three"] {
        manager.info(message);
    }

    let messages: Vec<&str> = manager
        .history()
        .iter()
        .map(|entry| entry.message.as_str())
        .collect();
    assert_eq!(messages, ["two", "three"]);
}

#[test]
fn a_manager_told_to_keep_nothing_keeps_nothing() {
    let mut manager = NotificationManager::new();
    manager.set_history_limit(0);
    manager.info("gone");
    assert!(manager.history().is_empty());
    assert_eq!(manager.count(), 1, "it is still a toast");
}

#[test]
fn dismissing_the_toasts_does_not_forget_what_happened() {
    let mut manager = NotificationManager::new();
    manager.info("something");
    manager.clear();
    assert!(manager.is_empty());
    assert_eq!(manager.history().len(), 1);
}

#[test]
fn test_notification_opacity() {
    let notif = Notification::new("Test".to_string(), NotificationType::Info, 4.0);
    assert!((notif.opacity() - 1.0).abs() < 0.001);

    let mut notif2 = Notification::new("Test".to_string(), NotificationType::Info, 4.0);
    notif2.time_remaining = 0.5;
    assert!((notif2.opacity() - 0.5).abs() < 0.001);
}

#[test]
fn test_notification_manager() {
    let mut manager = NotificationManager::new();

    manager.success("Test 1");
    manager.warning("Test 2");

    assert_eq!(manager.count(), 2);

    // Update to expire first notification
    manager.update(5.0);
    assert_eq!(manager.count(), 0);
}

#[test]
fn test_max_notifications() {
    let mut manager = NotificationManager::with_settings(3, 4.0);

    for i in 0..5 {
        manager.info(format!("Test {}", i));
    }

    assert_eq!(manager.count(), 3);
    // Should have Test 2, 3, 4 (oldest removed)
    assert_eq!(manager.get_notifications()[0].message, "Test 2");
}
