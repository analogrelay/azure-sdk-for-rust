// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Newtype wrapper for Request Units (RU) charges.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::iter::Sum;
use std::ops::Add;

/// Request charge measured in Request Units (RU).
///
/// All Cosmos DB operations consume Request Units (RU), which represent
/// the compute, memory, and I/O resources consumed by the operation.
/// This newtype wraps `f64` to provide type safety and clarity.
///
/// # Examples
///
/// ```
/// use azure_data_cosmos_driver::models::RequestCharge;
///
/// let charge = RequestCharge::new(3.5);
/// assert_eq!(charge.value(), 3.5);
///
/// // Supports addition
/// let total = charge + RequestCharge::new(2.0);
/// assert_eq!(total.value(), 5.5);
///
/// // Supports summing iterators
/// let charges = vec![RequestCharge::new(1.0), RequestCharge::new(2.0), RequestCharge::new(3.0)];
/// let sum: RequestCharge = charges.into_iter().sum();
/// assert_eq!(sum.value(), 6.0);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequestCharge(f64);

impl RequestCharge {
    /// Creates a new `RequestCharge` from a raw `f64` value.
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    /// Returns the raw `f64` value of this request charge.
    pub const fn value(self) -> f64 {
        self.0
    }
}

impl fmt::Display for RequestCharge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Add for RequestCharge {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sum for RequestCharge {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), |acc, x| acc + x)
    }
}

impl From<f64> for RequestCharge {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl From<RequestCharge> for f64 {
    fn from(charge: RequestCharge) -> Self {
        charge.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_zero() {
        let charge = RequestCharge::default();
        assert_eq!(charge.value(), 0.0);
    }

    #[test]
    fn new_and_value() {
        let charge = RequestCharge::new(3.5);
        assert_eq!(charge.value(), 3.5);
    }

    #[test]
    fn add() {
        let a = RequestCharge::new(1.5);
        let b = RequestCharge::new(2.5);
        assert_eq!((a + b).value(), 4.0);
    }

    #[test]
    fn sum_iterator() {
        let charges = vec![
            RequestCharge::new(1.0),
            RequestCharge::new(2.0),
            RequestCharge::new(3.0),
        ];
        let total: RequestCharge = charges.into_iter().sum();
        assert_eq!(total.value(), 6.0);
    }

    #[test]
    fn sum_empty_iterator() {
        let charges: Vec<RequestCharge> = vec![];
        let total: RequestCharge = charges.into_iter().sum();
        assert_eq!(total.value(), 0.0);
    }

    #[test]
    fn display() {
        let charge = RequestCharge::new(5.5);
        assert_eq!(format!("{}", charge), "5.5");
    }

    #[test]
    fn from_f64() {
        let charge: RequestCharge = 3.5.into();
        assert_eq!(charge.value(), 3.5);
    }

    #[test]
    fn into_f64() {
        let charge = RequestCharge::new(3.5);
        let val: f64 = charge.into();
        assert_eq!(val, 3.5);
    }

    #[test]
    fn partial_ord() {
        let a = RequestCharge::new(1.0);
        let b = RequestCharge::new(2.0);
        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn serialization() {
        let charge = RequestCharge::new(3.5);
        let json = serde_json::to_string(&charge).unwrap();
        assert_eq!(json, "3.5");
    }

    #[test]
    fn deserialization() {
        let charge: RequestCharge = serde_json::from_str("3.5").unwrap();
        assert_eq!(charge.value(), 3.5);
    }

    #[test]
    fn copy_semantics() {
        let a = RequestCharge::new(1.0);
        let b = a;
        assert_eq!(a.value(), b.value()); // `a` is still usable (Copy)
    }
}
