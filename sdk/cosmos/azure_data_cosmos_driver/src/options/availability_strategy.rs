// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Availability strategy types for cross-region hedging.
//!
//! These types model **what** the SDK should do when a primary request is slow
//! to respond; they do **not** by themselves trigger any behavior. The hedging
//! pipeline stage consumes a resolved [`AvailabilityStrategy`] to decide
//! whether to issue a hedged request to a secondary region.

use std::time::Duration;

/// Minimum time the SDK waits before sending a hedged request to another
/// region.
///
/// This wrapper around [`Duration`] guarantees that the threshold is greater
/// than zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HedgeThreshold(Duration);

impl HedgeThreshold {
    /// Creates a new [`HedgeThreshold`] from a [`Duration`].
    ///
    /// Returns `None` if `duration` is zero, since a zero threshold has no
    /// meaningful semantics (it would mean "hedge before the primary request
    /// has had any time to complete").
    pub const fn new(duration: Duration) -> Option<Self> {
        if duration.is_zero() {
            None
        } else {
            Some(Self(duration))
        }
    }

    /// Returns the underlying [`Duration`].
    pub const fn get(self) -> Duration {
        self.0
    }
}

/// Configuration for the parallel hedging strategy.
///
/// Currently the only knob is the threshold; additional fields (e.g. retry
/// caps, hedged-request limits) may be added in future revisions. Construct
/// via [`HedgingStrategy::new`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct HedgingStrategy {
    threshold: HedgeThreshold,
}

impl HedgingStrategy {
    /// Creates a [`HedgingStrategy`] with the given threshold.
    pub const fn new(threshold: HedgeThreshold) -> Self {
        Self { threshold }
    }

    /// Returns the configured hedge threshold.
    pub const fn threshold(&self) -> HedgeThreshold {
        self.threshold
    }
}

/// Strategy controlling whether the SDK issues hedged cross-region requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AvailabilityStrategy {
    /// Hedge eligible requests using the supplied [`HedgingStrategy`].
    Hedging(HedgingStrategy),

    /// Hedging is explicitly disabled for the scope at which this value is set.
    Disabled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hedge_threshold_rejects_zero() {
        assert!(HedgeThreshold::new(Duration::ZERO).is_none());
    }

    #[test]
    fn hedge_threshold_accepts_positive() {
        let t = HedgeThreshold::new(Duration::from_millis(1)).expect("1ms is a valid threshold");
        assert_eq!(t.get(), Duration::from_millis(1));
    }

    #[test]
    fn hedge_threshold_get_roundtrip() {
        let original = Duration::from_secs(2);
        let t = HedgeThreshold::new(original).expect("non-zero");
        assert_eq!(t.get(), original);
    }

    #[test]
    fn hedging_strategy_exposes_threshold() {
        let t = HedgeThreshold::new(Duration::from_millis(750)).unwrap();
        let s = HedgingStrategy::new(t);
        assert_eq!(s.threshold(), t);
    }

    #[test]
    fn availability_strategy_equality() {
        let t = HedgeThreshold::new(Duration::from_millis(500)).unwrap();
        let a = AvailabilityStrategy::Hedging(HedgingStrategy::new(t));
        let b = AvailabilityStrategy::Hedging(HedgingStrategy::new(t));
        assert_eq!(a, b);
        assert_ne!(a, AvailabilityStrategy::Disabled);
    }
}
