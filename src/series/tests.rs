use super::*;

#[test]
fn an_empty_series_reports_nothing_rather_than_zero() {
    let series = Series::new(16);
    assert!(series.is_empty());
    assert_eq!(series.min(), None);
    assert_eq!(series.max(), None);
    assert_eq!(series.last(), None);
}

#[test]
fn memory_is_bounded_however_long_the_session_runs() {
    let mut series = Series::new(64);
    for i in 0..100_000 {
        series.push(i as f32);
    }
    assert!(series.len() <= 66, "{} buckets", series.len());
    assert_eq!(series.count(), 100_000);
}

/// The property the whole module exists for.
///
/// Averaging into buckets would have smoothed these away, and a graph of
/// variance that hides its spikes is worse than no graph.
#[test]
fn extremes_survive_any_amount_of_decimation_exactly() {
    let mut series = Series::new(8);
    let mut lowest = f32::MAX;
    let mut highest = f32::MIN;

    for i in 0..10_000 {
        // A slow drift with occasional violent spikes, which is the shape
        // this is meant to plot.
        let value = if i % 977 == 0 {
            40_000.0 - i as f32
        } else if i % 613 == 0 {
            -9_000.0 - i as f32
        } else {
            (i as f32 * 0.01).sin() * 50.0
        };
        lowest = lowest.min(value);
        highest = highest.max(value);
        series.push(value);
    }

    assert_eq!(series.min(), Some(lowest));
    assert_eq!(series.max(), Some(highest));
}

#[test]
fn a_single_spike_is_never_lost() {
    // One sample in ten thousand, which is exactly the case a ring buffer
    // drops and an average dissolves.
    let mut series = Series::new(4);
    for i in 0..10_000 {
        series.push(if i == 17 { 1_000.0 } else { 0.0 });
    }
    assert_eq!(series.max(), Some(1_000.0));
}

#[test]
fn the_last_value_is_always_the_most_recent_push() {
    let mut series = Series::new(8);
    for i in 0..5_000 {
        series.push(i as f32);
        assert_eq!(series.last(), Some(i as f32), "after {} pushes", i);
    }
}

#[test]
fn the_beginning_of_the_session_is_never_dropped() {
    // The other failure mode: a ring buffer would have discarded this.
    let mut series = Series::new(8);
    series.push(-500.0);
    for i in 0..5_000 {
        series.push(i as f32);
    }
    assert_eq!(series.min(), Some(-500.0));
    assert_eq!(series.buckets()[0].min, -500.0);
}

#[test]
fn resolution_doubles_rather_than_drifting() {
    let mut series = Series::new(8);
    assert_eq!(series.resolution(), 1);
    for _ in 0..5_000 {
        series.push(1.0);
    }
    // Every decimation doubles it, so it is always a power of two.
    assert!(series.resolution().is_power_of_two());
    assert!(series.resolution() > 1);
}

#[test]
fn buckets_stay_ordered_in_time() {
    let mut series = Series::new(8);
    for i in 0..1_000 {
        series.push(i as f32);
    }
    // The input rises monotonically, so every bucket's range must too.
    for pair in series.buckets().windows(2) {
        assert!(pair[1].min >= pair[0].min);
        assert!(pair[1].max >= pair[0].max);
        assert!(pair[1].last > pair[0].last);
    }
}

#[test]
fn every_bucket_is_internally_consistent() {
    let mut series = Series::new(16);
    for i in 0..3_000 {
        series.push(((i * 37) % 500) as f32 - 250.0);
    }
    for bucket in series.buckets() {
        assert!(bucket.min <= bucket.max);
        assert!(bucket.last >= bucket.min && bucket.last <= bucket.max);
    }
}

#[test]
fn an_odd_capacity_is_rounded_so_pairs_always_match() {
    // A runt bucket covering half the span of its neighbours would skew
    // every plot drawn from the series.
    let mut series = Series::new(7);
    for i in 0..1_000 {
        series.push(i as f32);
    }
    // Rounded up to 8, so decimation always merges whole pairs.
    assert!(series.len() <= 8);
    // And every bucket really does cover the same span: the input rises by
    // one per push, so a runt bucket would show a smaller range than its
    // neighbours.
    let ranges: Vec<f32> = series
        .buckets()
        .iter()
        .map(|bucket| bucket.max - bucket.min)
        .collect();
    let widest = ranges.iter().fold(0.0f32, |w, r| w.max(*r));
    for range in &ranges[..ranges.len() - 1] {
        assert!(
            (*range - widest).abs() < 1.5,
            "{} against {}",
            range,
            widest
        );
    }
}

#[test]
fn a_non_finite_sample_cannot_poison_the_extremes() {
    // One NaN would make every later min/max NaN, and a series that has
    // stopped tracking looks just like one with nothing to track.
    let mut series = Series::new(8);
    series.push(5.0);
    series.push(f32::NAN);
    series.push(f32::INFINITY);
    series.push(-3.0);

    assert_eq!(series.min(), Some(-3.0));
    assert_eq!(series.max(), Some(5.0));
    assert_eq!(series.count(), 2);
}

#[test]
fn clearing_returns_it_to_new() {
    let mut series = Series::new(8);
    for i in 0..1_000 {
        series.push(i as f32);
    }
    series.clear();
    assert!(series.is_empty());
    assert_eq!(series.resolution(), 1);
    assert_eq!(series.count(), 0);
    assert_eq!(series.max(), None);
}

#[test]
fn a_tiny_capacity_still_behaves() {
    let mut series = Series::new(0);
    for i in 0..500 {
        series.push(i as f32);
    }
    assert!(!series.is_empty());
    assert_eq!(series.max(), Some(499.0));
    assert_eq!(series.min(), Some(0.0));
}
