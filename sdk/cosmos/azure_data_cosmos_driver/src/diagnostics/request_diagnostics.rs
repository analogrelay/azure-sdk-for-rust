// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Diagnostics for individual HTTP request/response pairs.

use crate::{
    models::{ActivityId, SubStatusCode},
    options::Region,
};
use azure_core::http::StatusCode;
use serde::Serialize;
use std::time::Instant;

use super::{ExecutionContext, RequestEvent};

/// Tri-state indicating whether a request was sent on the wire.
///
/// This is critical for retry decisions:
/// - `Sent`: The request was definitely transmitted; non-idempotent operations
///   should not be retried without additional safeguards (etag checks).
/// - `NotSent`: The request definitely was NOT transmitted; safe to retry.
/// - `Unknown`: Cannot determine if request was sent; treat as potentially sent
///   for safety (don't retry non-idempotent operations).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestSentStatus {
    /// Request was definitely sent on the wire.
    /// This is confirmed when we receive response headers or the transport
    /// completes successfully.
    Sent,

    /// Request was definitely NOT sent on the wire.
    /// This is confirmed for errors that occur before transmission
    /// (e.g., DNS resolution failure, connection refused).
    NotSent,

    /// Cannot determine if request was sent.
    /// Treat as potentially sent for retry safety.
    #[default]
    Unknown,
}

impl RequestSentStatus {
    /// Returns `true` if the request may have been sent.
    ///
    /// This is conservative: returns `true` for both `Sent` and `Unknown`,
    /// since we must assume `Unknown` might have been sent for retry safety.
    pub fn may_have_been_sent(&self) -> bool {
        !matches!(self, RequestSentStatus::NotSent)
    }

    /// Returns `true` if we know for certain the request was sent.
    pub fn definitely_sent(&self) -> bool {
        matches!(self, RequestSentStatus::Sent)
    }

    /// Returns `true` if we know for certain the request was NOT sent.
    pub fn definitely_not_sent(&self) -> bool {
        matches!(self, RequestSentStatus::NotSent)
    }
}

/// Diagnostics for a single HTTP request/response pair.
///
/// Each retry, hedged request, or failover produces a separate `RequestDiagnostics`
/// entry in the [`DiagnosticsContext`](super::DiagnosticsContext).
#[derive(Clone, Debug, Serialize)]
pub struct RequestDiagnostics {
    /// Context describing why this request was made.
    pub execution_context: ExecutionContext,

    /// Region this request was sent to.
    pub region: Region,

    /// Endpoint URI contacted.
    pub endpoint: String,

    /// HTTP status code from response.
    #[serde(serialize_with = "serialize_status_code")]
    pub status_code: StatusCode,

    /// Cosmos sub-status code (for detailed error classification).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_status_code: Option<SubStatusCode>,

    /// Request charge (RU) for this individual request.
    pub request_charge: f64,

    /// Activity ID from response headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity_id: Option<ActivityId>,

    /// Session token from response (for session consistency).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_token: Option<String>,

    /// When this request was started.
    #[serde(skip)]
    pub started_at: Instant,

    /// When this request completed (response received or error).
    #[serde(skip)]
    pub completed_at: Option<Instant>,

    /// Duration in milliseconds (computed from started_at/completed_at).
    pub duration_ms: u64,

    /// Pipeline events during this request.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<RequestEvent>,

    /// Whether this request timed out.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub timed_out: bool,

    /// Whether the request was sent on the wire.
    ///
    /// This is critical for retry decisions:
    /// - `Sent`: Request was transmitted; don't retry non-idempotent operations.
    /// - `NotSent`: Safe to retry any operation.
    /// - `Unknown`: Treat as potentially sent for safety.
    #[serde(skip_serializing_if = "RequestSentStatus::definitely_not_sent")]
    pub request_sent: RequestSentStatus,

    /// Error message if the request failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn serialize_status_code<S>(status: &StatusCode, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u16((*status).into())
}

impl RequestDiagnostics {
    /// Creates a new request diagnostics entry for a request being started.
    pub(crate) fn new(
        execution_context: ExecutionContext,
        region: Region,
        endpoint: String,
    ) -> Self {
        Self {
            execution_context,
            region,
            endpoint,
            // Status code is set when the request completes via `complete()`.
            // Using 0 as sentinel value for "not yet completed".
            status_code: StatusCode::from(0),
            sub_status_code: None,
            request_charge: 0.0,
            activity_id: None,
            session_token: None,
            started_at: Instant::now(),
            completed_at: None,
            duration_ms: 0,
            events: Vec::new(),
            timed_out: false,
            request_sent: RequestSentStatus::Unknown,
            error: None,
        }
    }

    /// Records completion of this request.
    ///
    /// Since we received a response, the request was definitely sent.
    pub(crate) fn complete(&mut self, status_code: StatusCode) {
        self.completed_at = Some(Instant::now());
        self.status_code = status_code;
        self.request_sent = RequestSentStatus::Sent;
        self.duration_ms = self
            .completed_at
            .unwrap()
            .duration_since(self.started_at)
            .as_millis() as u64;
    }

    /// Records timeout of this request.
    pub(crate) fn timeout(&mut self) {
        self.completed_at = Some(Instant::now());
        self.timed_out = true;
        self.duration_ms = self
            .completed_at
            .unwrap()
            .duration_since(self.started_at)
            .as_millis() as u64;
    }

    /// Records failure of this request with an error message.
    ///
    /// Use this for transport-level failures (connection errors, DNS failures, etc.)
    /// where no HTTP response was received.
    ///
    /// # Note on retry safety
    ///
    /// The `request_sent` parameter indicates whether the request bytes were
    /// written to the network. This is critical for determining retry safety:
    /// - `NotSent`: Safe to retry any operation
    /// - `Sent`: Only safe to retry idempotent operations
    /// - `Unknown`: Treat as potentially sent (conservative)
    pub(crate) fn fail(&mut self, error: impl Into<String>, request_sent: RequestSentStatus) {
        self.completed_at = Some(Instant::now());
        self.error = Some(error.into());
        self.request_sent = request_sent;
        self.duration_ms = self
            .completed_at
            .unwrap()
            .duration_since(self.started_at)
            .as_millis() as u64;
    }

    /// Records an error for this request.
    pub(crate) fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Sets the sub-status code.
    pub(crate) fn with_sub_status(mut self, sub_status: SubStatusCode) -> Self {
        self.sub_status_code = Some(sub_status);
        self
    }

    /// Sets the request charge.
    pub(crate) fn with_charge(mut self, charge: f64) -> Self {
        self.request_charge = charge;
        self
    }

    /// Sets the activity ID.
    pub(crate) fn with_activity_id(mut self, activity_id: ActivityId) -> Self {
        self.activity_id = Some(activity_id);
        self
    }

    /// Sets the session token.
    pub(crate) fn with_session_token(mut self, token: String) -> Self {
        self.session_token = Some(token);
        self
    }

    /// Adds a pipeline event.
    pub(crate) fn add_event(&mut self, event: RequestEvent) {
        self.events.push(event);
    }

    /// Returns whether this request has been completed.
    pub(crate) fn is_completed(&self) -> bool {
        self.completed_at.is_some()
    }

}

/// Handle for tracking a request within [`DiagnosticsContext`](super::DiagnosticsContext).
///
/// This is an opaque index used to reference a specific request's diagnostics
/// for updates during request execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestHandle(pub(crate) usize);
