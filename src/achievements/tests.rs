use super::*;

fn sample() -> Achievements {
    Achievements::from_definitions(vec![
        Achievement::new("first_win", "First Win", "Win a run."),
        Achievement::new("no_damage", "Untouchable", "Win without damage."),
    ])
}

#[test]
fn unlock_fires_once() {
    let mut achievements = sample();
    assert!(achievements.unlock("first_win"));
    assert!(
        !achievements.unlock("first_win"),
        "second unlock is a no-op"
    );
    assert!(achievements.is_unlocked("first_win"));
    assert!(!achievements.is_unlocked("no_damage"));
    assert_eq!(achievements.progress(), (1, 2));
}

#[test]
fn unknown_id_does_not_unlock() {
    let mut achievements = sample();
    assert!(!achievements.unlock("missing"));
    assert_eq!(achievements.progress(), (0, 2));
}

#[test]
fn sync_definitions_preserves_unlocks_and_adds_new() {
    let mut achievements = sample();
    achievements.unlock_with_date("first_win", Some("2026-07-14".into()));

    achievements.sync_definitions(vec![
        Achievement::new("first_win", "First Victory", "Win a run."),
        Achievement::new("speedrun", "Speedrun", "Win fast."),
    ]);

    assert_eq!(achievements.len(), 2);
    let first = achievements.get("first_win").unwrap();
    assert!(first.unlocked);
    assert_eq!(first.name, "First Victory", "definition text wins");
    assert_eq!(first.unlock_date.as_deref(), Some("2026-07-14"));
    assert!(!achievements.is_unlocked("speedrun"));
    assert!(achievements.get("no_damage").is_none(), "removed defs drop");
}

#[test]
fn round_trips_through_json() {
    let mut achievements = sample();
    achievements.unlock("no_damage");
    let json = serde_json::to_string(&achievements).unwrap();
    let back: Achievements = serde_json::from_str(&json).unwrap();
    assert_eq!(achievements, back);
}
