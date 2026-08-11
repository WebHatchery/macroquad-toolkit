use super::*;

/// `SeededRng::new` xors its seed with the golden-ratio constant, so that
/// exact seed used to leave the state at zero — and an xorshift at zero
/// never leaves it. Every draw came back 0 with nothing to indicate a fault.
#[test]
fn the_one_seed_that_zeroes_the_state_still_generates() {
    let mut rng = SeededRng::new(0x9E37_79B9_7F4A_7C15);
    let draws: Vec<u64> = (0..8).map(|_| rng.next_u64()).collect();

    assert!(
        draws.iter().any(|value| *value != 0),
        "the generator is stuck at zero"
    );
    assert!(
        draws.windows(2).any(|pair| pair[0] != pair[1]),
        "the generator repeats"
    );
}

#[test]
fn no_seed_leaves_the_generator_stuck() {
    for seed in (0..2_000u64).chain([u64::MAX, 0x9E37_79B9_7F4A_7C15]) {
        let mut rng = SeededRng::new(seed);
        let a = rng.next_u64();
        let b = rng.next_u64();
        assert!(a != 0 || b != 0, "seed {} produced a dead stream", seed);
    }
}

#[test]
fn ordinary_seeds_are_unchanged_by_the_zero_state_guard() {
    // The fix must not move any existing game's stream. These are the first
    // draws for a handful of everyday seeds, recorded from the behaviour
    // before the guard was added.
    for seed in [0u64, 1, 42, 1234, 0xD2A6_0F1E] {
        let expected = {
            let init = seed ^ 0x9E3779B97F4A7C15;
            let mut x = init;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            x.wrapping_mul(0x2545F4914F6CDD1D)
        };
        assert_eq!(SeededRng::new(seed).next_u64(), expected, "seed {}", seed);
    }
}
