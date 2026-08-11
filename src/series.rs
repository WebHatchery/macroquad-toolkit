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
mod tests;
