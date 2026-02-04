// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Execution context for categorizing request attempts.

use serde::Serialize;

/// Context in which a request was executed.
///
/// This categorizes why a request was made, which is useful for understanding
/// operation patterns and debugging retry/hedging behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionContext {
    /// Initial request attempt (first try).
    Initial,
    /// Retry due to transient error (e.g., 429, 503).
    Retry,
    /// Hedged request for latency reduction.
    Hedging,
    /// Region failover attempt.
    RegionFailover,
    /// Circuit breaker recovery probe.
    CircuitBreakerProbe,
}

impl std::fmt::Display for ExecutionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionContext::Initial => write!(f, "initial"),
            ExecutionContext::Retry => write!(f, "retry"),
            ExecutionContext::Hedging => write!(f, "hedging"),
            ExecutionContext::RegionFailover => write!(f, "region_failover"),
            ExecutionContext::CircuitBreakerProbe => write!(f, "circuit_breaker_probe"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display() {
        assert_eq!(ExecutionContext::Initial.to_string(), "initial");
        assert_eq!(ExecutionContext::Retry.to_string(), "retry");
        assert_eq!(ExecutionContext::Hedging.to_string(), "hedging");
        assert_eq!(
            ExecutionContext::RegionFailover.to_string(),
            "region_failover"
        );
        assert_eq!(
            ExecutionContext::CircuitBreakerProbe.to_string(),
            "circuit_breaker_probe"
        );
    }
}
