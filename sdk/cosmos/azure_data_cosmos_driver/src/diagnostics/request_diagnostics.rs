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
    pub(crate) fn new(execution_context: ExecutionContext, region: Region, endpoint: String) -> Self {
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
            error: None,
        }
    }

    /// Records completion of this request.
    pub(crate) fn complete(&mut self, status_code: StatusCode) {
        self.completed_at = Some(Instant::now());
        self.status_code = status_code;
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
}

/// Handle for tracking a request within [`DiagnosticsContext`](super::DiagnosticsContext).
///
/// This is an opaque index used to reference a specific request's diagnostics
/// for updates during request execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestHandle(pub(crate) usize);
