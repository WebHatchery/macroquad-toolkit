use super::SeededRng;

#[test]
fn seeded_rng_is_repeatable() {
    let mut a = SeededRng::new(42);
    let mut b = SeededRng::new(42);

    assert_eq!(a.next_u64(), b.next_u64());
    assert_eq!(a.next_u64(), b.next_u64());
}

#[test]
fn seeded_rng_below_stays_in_range() {
    let mut rng = SeededRng::new(11);
    assert_eq!(rng.below(0), 0);
    for _ in 0..256 {
        assert!(rng.below(7) < 7);
    }
}

#[test]
fn seeded_rng_chance_extremes() {
    let mut rng = SeededRng::new(3);
    assert!(!rng.chance(0.0));
    assert!(rng.chance(1.0));
}

#[test]
fn seeded_rng_float_is_unit_interval() {
    let mut rng = SeededRng::new(7);
    for _ in 0..16 {
        let value = rng.next_f32();
        assert!((0.0..=1.0).contains(&value));
    }
}

#[test]
fn range_i32_respects_bounds_and_survives_empty_ranges() {
    let mut rng = SeededRng::new(5);
    assert_eq!(rng.range_i32(3, 3), 3);
    assert_eq!(rng.range_i32(5, 2), 5);
    for _ in 0..128 {
        let value = rng.range_i32(-2, 4);
        assert!((-2..4).contains(&value));
    }
}

#[test]
fn shuffle_is_a_repeatable_permutation() {
    let mut first: Vec<u32> = (0..10).collect();
    let mut second: Vec<u32> = (0..10).collect();
    SeededRng::new(9).shuffle(&mut first);
    SeededRng::new(9).shuffle(&mut second);
    assert_eq!(first, second, "the same seed must shuffle the same way");

    let mut sorted = first.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, (0..10).collect::<Vec<u32>>(), "elements were lost");
}
