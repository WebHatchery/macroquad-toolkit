//! A time series that never grows and never loses an extreme.
//!
//! # The problem
//!
//! Plotting something across a whole session means an unbounded number of
//! samples and a bounded amount of memory. There are two usual answers and both
//! are wrong for a graph whose *point* is the shape of the variation:
//!
//! - **A ring buffer** keeps the last N and drops the beginning. The player's
//!   first hour vanishes, and with it any sense of where they started.
//! - **Averaging into buckets** keeps the whole span and destroys the detail.
//!   Two samples of `+40,000` and `-200` average to a shrug. The spike *is* the
//!   information; smoothing it away leaves a graph that says the session was
//!   uneventful when it was anything but.
//!
//! # What this does instead
//!
//! Each slot is a **bucket** holding the minimum, the maximum and the most
//! recent value over the span it covers. When the series fills, adjacent pairs
//! are merged — min of mins, max of maxes, last of the later — which halves the
//! count and doubles the time each slot represents.
//!
//! Merging that way is **lossless in the extremes**. However many times a series
//! has been decimated, [`Series::min`] and [`Series::max`] are still exactly the
//! smallest and largest values ever pushed. Resolution decays; the envelope
//! never does. A plot drawn from the buckets keeps showing every spike that ever
//! happened, at a coarser and coarser position in time.
//!
//! That trade is the right way round for a graph of variance, and it is the
//! opposite of what averaging gives you.

/// One slot: the range covered and where it ended.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bucket {
    pub min: f32,
    pub max: f32,
    /// The most recent value in this bucket. What a line plot connects.
    pub last: f32,
}

impl Bucket {
    pub fn point(value: f32) -> Self {
        Self {
            min: value,
            max: value,
            last: value,
        }
    }

    /// Fold a later bucket into this one. Order matters for `last` and not for
    /// the rest, which is the whole reason the extremes survive.
    fn absorb(&mut self, later: &Bucket) {
        self.min = self.min.min(later.min);
        self.max = self.max.max(later.max);
        self.last = later.last;
    }
}

/// A bounded series of buckets covering an unbounded span.
#[derive(Debug, Clone)]
pub struct Series {
    buckets: Vec<Bucket>,
    capacity: usize,
    /// Samples that go into one bucket at the current resolution. Doubles on
    /// every decimation.
    span: usize,
    /// Samples pushed into the bucket currently being filled.
    filling: usize,
    /// Total ever pushed, which is the series' own clock.
    pushed: u64,
}

impl Series {
    /// `capacity` is rounded up to an even number: decimation merges pairs, and
    /// an odd capacity would leave a runt bucket covering half the span of its
    /// neighbours and quietly skew every plot drawn from it.
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(2) + capacity % 2;
        Self {
            buckets: Vec::with_capacity(capacity),
            capacity,
            span: 1,
            filling: 0,
            pushed: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buckets.is_empty()
    }

    pub fn len(&self) -> usize {
        self.buckets.len()
    }

    pub fn buckets(&self) -> &[Bucket] {
        &self.buckets
    }

    /// Samples pushed since the series was created.
    pub fn count(&self) -> u64 {
        self.pushed
    }

    /// Samples each bucket now covers. Grows as the session does.
    pub fn resolution(&self) -> usize {
        self.span
    }

    /// The smallest value ever pushed. Exact, whatever the resolution.
    pub fn min(&self) -> Option<f32> {
        self.buckets
            .iter()
            .map(|bucket| bucket.min)
            .fold(None, |worst: Option<f32>, value| {
                Some(worst.map_or(value, |w| w.min(value)))
            })
    }

    /// The largest value ever pushed. Exact, whatever the resolution.
    pub fn max(&self) -> Option<f32> {
        self.buckets
            .iter()
            .map(|bucket| bucket.max)
            .fold(None, |best: Option<f32>, value| {
                Some(best.map_or(value, |b| b.max(value)))
            })
    }

    /// The most recent value.
    pub fn last(&self) -> Option<f32> {
        self.buckets.last().map(|bucket| bucket.last)
    }

    pub fn clear(&mut self) {
        self.buckets.clear();
        self.span = 1;
        self.filling = 0;
        self.pushed = 0;
    }

    /// Add a sample.
    pub fn push(&mut self, value: f32) {
        // NaN would poison min/max for the rest of the session, and a series
        // that has silently stopped tracking its extremes looks exactly like one
        // that has had no extremes.
        if !value.is_finite() {
            return;
        }
        self.pushed += 1;

        if self.filling == 0 {
            self.buckets.push(Bucket::point(value));
        } else if let Some(current) = self.buckets.last_mut() {
            current.absorb(&Bucket::point(value));
        }

        self.filling += 1;
        if self.filling >= self.span {
            self.filling = 0;
        }
        if self.buckets.len() > self.capacity {
            self.decimate();
        }
    }

    /// Halve the bucket count by merging adjacent pairs, doubling the span.
    ///
    /// The last bucket may be partly filled. It is merged with its predecessor
    /// like any other; the alternative — carrying it forward alone — would leave
    /// the newest slot covering a different span from the rest, which is the one
    /// place a plot is most closely read.
    fn decimate(&mut self) {
        let mut merged = Vec::with_capacity(self.capacity);
        for pair in self.buckets.chunks(2) {
            let mut bucket = pair[0];
            if let Some(later) = pair.get(1) {
                bucket.absorb(later);
            }
            merged.push(bucket);
        }
        self.buckets = merged;
        self.span *= 2;
        // The tail bucket now holds however many samples the two it came from
        // did. Anything else would make the next bucket boundary fall in the
        // wrong place and the resolution drift away from `span`.
        self.filling %= self.span;
    }
}

#[cfg(test)]
mod tests {
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
}
