// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Diagnostics threshold types.

use std::time::Duration;

/// Thresholds for controlling when diagnostics are captured/logged.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DiagnosticsThresholds {
    /// Latency threshold above which diagnostics are captured for point operations.
    pub point_operation_latency_threshold: Option<Duration>,
    /// Latency threshold above which diagnostics are captured for non-point operations.
    pub non_point_operation_latency_threshold: Option<Duration>,
    /// Request charge (RU) threshold above which diagnostics are captured.
    pub request_charge_threshold: Option<f64>,
    /// Payload size threshold (in bytes) above which diagnostics are captured.
    pub payload_size_threshold: Option<usize>,
}

impl DiagnosticsThresholds {
    /// Creates new diagnostics thresholds with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the latency threshold for point operations.
    pub fn with_point_operation_latency_threshold(mut self, threshold: Duration) -> Self {
        self.point_operation_latency_threshold = Some(threshold);
        self
    }

    /// Sets the latency threshold for non-point operations.
    pub fn with_non_point_operation_latency_threshold(mut self, threshold: Duration) -> Self {
        self.non_point_operation_latency_threshold = Some(threshold);
        self
    }

    /// Sets the request charge threshold.
    pub fn with_request_charge_threshold(mut self, threshold: f64) -> Self {
        self.request_charge_threshold = Some(threshold);
        self
    }

    /// Sets the payload size threshold.
    pub fn with_payload_size_threshold(mut self, threshold: usize) -> Self {
        self.payload_size_threshold = Some(threshold);
        self
    }
}
