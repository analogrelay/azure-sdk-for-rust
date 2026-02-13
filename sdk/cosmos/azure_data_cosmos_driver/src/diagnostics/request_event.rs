// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Pipeline events for request lifecycle tracking.
//!
//! # Reqwest Limitations
//!
//! Unlike Reactor Netty (used in the Java SDK), reqwest does not expose fine-grained
//! connection lifecycle callbacks. We cannot directly track:
//! - DNS resolution time (separate from connection time)
//! - Connection pool acquisition vs new connection creation
//! - TLS handshake time
//! - Time to first byte after request sent
//!
//! What we **can** track:
//! - Request start/end timing
//! - Total elapsed time
//! - Error categorization (connection refused, DNS failure, timeout, etc.)
//! - Whether the request was likely sent before failure (for retry safety)
//!
//! # Future Improvements
//!
//! To get more granular metrics, we would need to either:
//! 1. Use `hyper` directly with custom connectors
//! 2. Subscribe to `tracing` events emitted by hyper/reqwest internals
//! 3. Implement a custom `tower::Service` layer via `connector_layer`

use serde::Serialize;
use std::time::{Duration, Instant};

/// The type of event in the request lifecycle.
///
/// These events track key milestones during HTTP request processing.
/// Note: Due to reqwest's high-level abstraction, we cannot track fine-grained
/// connection events (DNS, TLS handshake) separately. We track what we can observe.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RequestEventType {
    /// Request sent to transport - we're now waiting for the HTTP client.
    /// From here, reqwest handles DNS, connection, TLS, and sending internally.
    /// We cannot distinguish these phases with reqwest's current API.
    TransportStart,

    /// Response headers received from the server.
    /// Emitted when `transport.send().await` returns `Ok(response)`.
    /// At this point, the response body is still a stream - not yet buffered.
    ResponseHeadersReceived,

    /// Transport fully completed - response headers received AND body buffered.
    /// Emitted after `try_into_raw_response().await` succeeds.
    TransportComplete,

    /// Transport failed - an error occurred during the request.
    /// The `details` field contains the error message.
    /// Use error analysis to determine if the request was likely sent.
    TransportFailed,
}

impl RequestEventType {
    /// Returns the string representation of the event type.
    pub fn as_str(&self) -> &str {
        match self {
            Self::TransportStart => "transport_start",
            Self::ResponseHeadersReceived => "response_headers_received",
            Self::TransportComplete => "transport_complete",
            Self::TransportFailed => "transport_failed",
        }
    }

    /// Returns true if this event indicates the request was sent on the wire.
    ///
    /// For retry safety:
    /// - `ResponseHeadersReceived`, `TransportComplete` = definitely sent
    /// - `TransportFailed` = depends on error analysis (handled separately)
    /// - `TransportStart` = not yet sent (in progress)
    pub fn indicates_request_sent(&self) -> bool {
        matches!(
            self,
            Self::ResponseHeadersReceived | Self::TransportComplete
        )
    }
}

/// An event in the request pipeline lifecycle.
///
/// Events are recorded at key points during request processing to enable
/// detailed timing analysis and debugging.
///
/// This type is non-exhaustive and new fields may be added in future releases.
/// Use the getter methods to access field values.
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct RequestEvent {
    /// Type of the pipeline event.
    pub(super) event_type: RequestEventType,

    /// When this event occurred.
    #[serde(skip)]
    pub(super) timestamp: Instant,

    /// Duration of this stage, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) duration_ms: Option<u64>,

    /// Additional context for this event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) details: Option<String>,
}

impl RequestEvent {
    /// Creates a new request event.
    pub(crate) fn new(event_type: RequestEventType) -> Self {
        Self {
            event_type,
            timestamp: Instant::now(),
            duration_ms: None,
            details: None,
        }
    }

    /// Creates a request event with duration.
    pub(crate) fn with_duration(event_type: RequestEventType, duration: Duration) -> Self {
        Self {
            event_type,
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

    /// Returns the stage name for backwards compatibility.
    #[deprecated(note = "Use event_type() instead")]
    pub fn stage(&self) -> &str {
        self.event_type.as_str()
    }

    // Public getters for read-only access to fields

    /// Returns the type of the pipeline event.
    pub fn event_type(&self) -> &RequestEventType {
        &self.event_type
    }

    /// Returns when this event occurred.
    pub fn timestamp(&self) -> Instant {
        self.timestamp
    }

    /// Returns the duration of this stage in milliseconds, if applicable.
    pub fn duration_ms(&self) -> Option<u64> {
        self.duration_ms
    }

    /// Returns additional context for this event, if present.
    pub fn details(&self) -> Option<&str> {
        self.details.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_type_indicates_sent() {
        // Before/during sending - not confirmed sent
        assert!(!RequestEventType::TransportStart.indicates_request_sent());

        // TransportFailed is ambiguous - requires error analysis
        assert!(!RequestEventType::TransportFailed.indicates_request_sent());

        // After headers received or transport complete - definitely sent
        assert!(RequestEventType::ResponseHeadersReceived.indicates_request_sent());
        assert!(RequestEventType::TransportComplete.indicates_request_sent());
    }

    #[test]
    fn event_creation() {
        let event = RequestEvent::new(RequestEventType::TransportStart);
        assert_eq!(event.event_type, RequestEventType::TransportStart);
        assert!(event.duration_ms.is_none());
        assert!(event.details.is_none());
    }

    #[test]
    fn event_with_details() {
        let event = RequestEvent::new(RequestEventType::TransportFailed)
            .with_details("connection reset by peer");
        assert_eq!(event.details, Some("connection reset by peer".to_string()));
    }

    #[test]
    fn event_with_duration() {
        let event = RequestEvent::with_duration(
            RequestEventType::TransportComplete,
            Duration::from_millis(50),
        );
        assert_eq!(event.duration_ms, Some(50));
    }
}
