// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Pipeline events for request lifecycle tracking.

use serde::Serialize;
use std::time::{Duration, Instant};

/// An event in the request pipeline lifecycle.
///
/// Events are recorded at key points during request processing to enable
/// detailed timing analysis and debugging.
#[derive(Clone, Debug, Serialize)]
pub struct RequestEvent {
    /// Name of the pipeline stage (e.g., "send_request", "receive_response").
    pub stage: String,

    /// When this event occurred.
    #[serde(skip)]
    pub timestamp: Instant,

    /// Duration of this stage, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,

    /// Additional context for this event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl RequestEvent {
    /// Creates a new request event.
    pub(crate) fn new(stage: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            timestamp: Instant::now(),
            duration_ms: None,
            details: None,
        }
    }

    /// Creates a request event with duration.
    pub(crate) fn with_duration(stage: impl Into<String>, duration: Duration) -> Self {
        Self {
            stage: stage.into(),
            timestamp: Instant::now(),
            duration_ms: Some(duration.as_millis() as u64),
            details: None,
        }
    }

    /// Adds details to the event.
    pub(crate) fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}
