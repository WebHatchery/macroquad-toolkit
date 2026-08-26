use super::*;

fn config() -> AnalyticsConfig {
    AnalyticsConfig::web(
        "https://example.test/events",
        "public-write-key",
        "test_game",
        "0.1.0",
        "webhatchery",
    )
}

#[test]
fn starts_with_one_session_event() {
    let client = AnalyticsClient::with_ids(config(), "install".into(), "session".into());
    assert_eq!(client.queued_count(), 1);
    assert_eq!(client.queued[0].event, AnalyticsEventName::SessionStarted);
}

#[test]
fn heartbeat_counts_only_active_time() {
    let mut client = AnalyticsClient::with_ids(config(), "install".into(), "session".into());
    client.config.heartbeat_seconds = 5.0;
    client.config.flush_seconds = 999.0;
    for _ in 0..3 {
        client.update(1.0, false);
    }
    for _ in 0..5 {
        client.update(1.0, true);
    }
    assert_eq!(client.queued_count(), 2);
    assert_eq!(client.queued[1].active_seconds, 5);
}

#[test]
fn end_is_idempotent() {
    let mut client = AnalyticsClient::with_ids(config(), "install".into(), "session".into());
    client.end_session();
    client.end_session();
    assert_eq!(client.queued_count(), 2);
}
