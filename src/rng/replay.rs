use super::*;

/// The property a commitment scheme rests on: the state is the future.
#[test]
fn a_captured_state_replays_the_same_stream() {
    let mut live = SeededRng::new(0xD2A6_0F1E);
    for _ in 0..17 {
        live.next_u64();
    }

    let captured = live.state();
    let ahead: Vec<u64> = (0..64).map(|_| live.next_u64()).collect();

    let mut replayed = SeededRng::from_state(captured);
    let again: Vec<u64> = (0..64).map(|_| replayed.next_u64()).collect();
    assert_eq!(
        ahead, again,
        "a restored generator diverged from the original"
    );
}

/// `from_state` must not mix. Feeding a seed to `new` and the same number
/// to `from_state` are different requests and have to stay different.
#[test]
fn from_state_does_not_mix_the_way_new_does() {
    let seeded = SeededRng::new(12_345);
    assert_ne!(seeded.state(), 12_345, "new stopped mixing its seed");
    assert_eq!(SeededRng::from_state(12_345).state(), 12_345);
    assert_eq!(
        SeededRng::from_state(seeded.state()).state(),
        seeded.state(),
        "a round trip through from_state must be the identity"
    );
}

/// Zero is the fixed point, on this door as much as on the other one.
#[test]
fn a_zero_state_is_refused_here_too() {
    let mut zeroed = SeededRng::from_state(0);
    assert_ne!(zeroed.state(), 0);
    let draws: Vec<u64> = (0..8).map(|_| zeroed.next_u64()).collect();
    assert!(
        draws.iter().any(|value| *value != 0),
        "a zero state produced a dead stream"
    );
}
