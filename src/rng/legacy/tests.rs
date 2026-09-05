use super::*;

fn check<const INC: u64>(seed: u64, expected: [u32; 6]) {
    let mut rng = LegacyLcg64::<INC>::new(seed);
    assert_eq!(std::array::from_fn::<_, 6, _>(|_| rng.next_u32()), expected);
}

#[test]
fn shipped_streams_match_fixed_pre_migration_vectors() {
    check::<1>(
        0,
        [
            1481765933, 3232861391, 3417699910, 3338875177, 812669700, 553475508,
        ],
    );
    check::<1>(
        42,
        [
            2104627054, 2013331137, 2406144595, 107061148, 317460219, 2811415527,
        ],
    );
    check::<1>(
        u64::MAX,
        [
            2813201362, 4025637771, 3048022872, 3496524642, 1570113359, 1071863892,
        ],
    );
    check::<1_442_695_040_888_963_407>(
        0,
        [
            1817669548, 2187888307, 2784682393, 1644385741, 3416422068, 2149679590,
        ],
    );
    check::<1_442_695_040_888_963_407>(
        42,
        [
            2440530669, 968358053, 1773127077, 2707539007, 2921212588, 112652313,
        ],
    );
    check::<1_442_695_040_888_963_407>(
        u64::MAX,
        [
            3149104977, 2980664687, 2415005355, 1802035205, 4173865728, 2668067974,
        ],
    );
}

#[test]
fn restoring_and_serializing_preserve_the_future_including_zero() {
    let mut original = LegacyLcg64::<1>::new(42);
    for _ in 0..71 {
        original.next_u32();
    }
    let mut restored = LegacyLcg64::<1>::from_state(original.state());
    let json = serde_json::to_string(&original).unwrap();
    let mut decoded: LegacyLcg64<1> = serde_json::from_str(&json).unwrap();
    for _ in 0..100 {
        let expected = original.next_u32();
        assert_eq!(restored.next_u32(), expected);
        assert_eq!(decoded.next_u32(), expected);
    }
    let mut zero = LegacyLcg64::<1>::from_state(0);
    assert_eq!(zero.state(), 0);
    assert_eq!(zero.next_u32(), 0);
    assert_eq!(zero.state(), 1);
    assert_eq!(LegacyLcg64::<1>::new(0), LegacyLcg64::<1>::new(1));
}
