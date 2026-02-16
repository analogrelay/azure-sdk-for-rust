// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Diagnostics for individual HTTP request/response pairs.

use crate::{
    models::{ActivityId, CosmosStatus, RequestCharge, SubStatusCode},
    options::Region,
};
use azure_core::http::StatusCode;
use serde::Serialize;
use std::time::Instant;

use super::{ExecutionContext, RequestEvent};

// =============================================================================
// Pipeline Classification Types
// =============================================================================

/// The type of pipeline used to execute a request.
///
/// Cosmos DB operations are routed through different pipelines based on their
/// resource type and operation type. This enum captures which pipeline was used,
/// which is useful for debugging and understanding request behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PipelineType {
    /// Metadata pipeline for control plane operations.
    ///
    /// Used for database, container, throughput, and other management operations.
    /// Has a higher timeout (65 seconds) to accommodate operations that may take
    /// longer to complete.
    Metadata,

    /// Data plane pipeline for document operations.
    ///
    /// Used for CRUD operations on items/documents and queries.
    /// Has a lower timeout (6 seconds) optimized for high-throughput scenarios.
    DataPlane,
}

impl PipelineType {
    /// Returns true if this is a metadata (control plane) pipeline.
    pub fn is_metadata(self) -> bool {
        matches!(self, PipelineType::Metadata)
    }

    /// Returns true if this is a data plane pipeline.
    pub fn is_data_plane(self) -> bool {
        matches!(self, PipelineType::DataPlane)
    }
}

/// The transport security mode used for a request.
///
/// This captures whether the request was made with full TLS certificate
/// validation or with relaxed validation for emulator scenarios.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum TransportSecurity {
    /// Standard secure transport with full certificate validation.
    ///
    /// Used for production endpoints with valid TLS certificates.
    #[default]
    Secure,

    /// Emulator transport with insecure certificate acceptance.
    ///
    /// Used when connecting to the local Cosmos DB emulator, which uses
    /// self-signed certificates that would fail standard validation.
    EmulatorWithInsecureCertificates,
}

impl TransportSecurity {
    /// Returns true if this is a secure transport.
    pub fn is_secure(self) -> bool {
        matches!(self, TransportSecurity::Secure)
    }

    /// Returns true if this is an emulator transport with insecure certificates.
    pub fn is_emulator(self) -> bool {
        matches!(self, TransportSecurity::EmulatorWithInsecureCertificates)
    }
}

// =============================================================================
// Request Sent Status
// =============================================================================

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
#[non_exhaustive]
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
///
/// This type is non-exhaustive and new fields may be added in future releases.
/// Use the getter methods to access field values.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct RequestDiagnostics {
    /// Context describing why this request was made.
    pub(super) execution_context: ExecutionContext,

    /// The pipeline type used for this request.
    pub(super) pipeline_type: PipelineType,

    /// The transport security mode used for this request.
    pub(super) transport_security: TransportSecurity,

    /// Region this request was sent to.
    pub(super) region: Region,

    /// Endpoint URI contacted.
    pub(super) endpoint: String,

    /// Combined HTTP status code and Cosmos sub-status code.
    #[serde(flatten)]
    pub(super) status: CosmosStatus,

    /// Request charge (RU) for this individual request.
    pub(crate) request_charge: RequestCharge,

    /// Activity ID from response headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) activity_id: Option<ActivityId>,

    /// Session token from response (for session consistency).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) session_token: Option<String>,

    /// When this request was started.
    #[serde(skip)]
    pub(super) started_at: Instant,

    /// When this request completed (response received or error).
    #[serde(skip)]
    pub(super) completed_at: Option<Instant>,

    /// Duration in milliseconds (computed from started_at/completed_at).
    pub(super) duration_ms: u64,

    /// Pipeline events during this request.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) events: Vec<RequestEvent>,

    /// Whether this request timed out.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub(super) timed_out: bool,

    /// Whether the request was sent on the wire.
    ///
    /// This is critical for retry decisions:
    /// - `Sent`: Request was transmitted; don't retry non-idempotent operations.
    /// - `NotSent`: Safe to retry any operation.
    /// - `Unknown`: Treat as potentially sent for safety.
    #[serde(skip_serializing_if = "RequestSentStatus::definitely_not_sent")]
    pub(super) request_sent: RequestSentStatus,

    /// Error message if the request failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<String>,
}

impl RequestDiagnostics {
    /// Creates a new request diagnostics entry for a request being started.
    pub(crate) fn new(
        execution_context: ExecutionContext,
        pipeline_type: PipelineType,
        transport_security: TransportSecurity,
        region: Region,
        endpoint: String,
    ) -> Self {
        Self {
            execution_context,
            pipeline_type,
            transport_security,
            region,
            endpoint,
            // Status is set when the request completes via `complete()`.
            // Using 0 as sentinel value for "not yet completed".
            status: CosmosStatus::default(),
            request_charge: RequestCharge::default(),
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
    pub(crate) fn complete(&mut self, status_code: StatusCode, sub_status: Option<SubStatusCode>) {
        self.completed_at = Some(Instant::now());
        self.status = CosmosStatus::from_parts(status_code, sub_status);
        self.request_sent = RequestSentStatus::Sent;
        self.duration_ms = self
            .completed_at
            .unwrap()
            .duration_since(self.started_at)
            .as_millis() as u64;
    }

    /// Records end-to-end timeout of this request.
    ///
    /// Sets the status to 408 (Request Timeout) with sub-status
    /// [`SubStatusCode::CLIENT_OPERATION_TIMEOUT`] to indicate an end-to-end
    /// operation timeout from the client side.
    pub(crate) fn timeout(&mut self) {
        self.completed_at = Some(Instant::now());
        self.timed_out = true;
        self.status = CosmosStatus::with_sub_status(
            StatusCode::RequestTimeout,
            SubStatusCode::CLIENT_OPERATION_TIMEOUT,
        );
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
        self.status = CosmosStatus::from_parts(self.status.status_code(), Some(sub_status));
        self
    }

    /// Sets the request charge.
    pub(crate) fn with_charge(mut self, charge: RequestCharge) -> Self {
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

    // Public getters for read-only access to fields

    /// Returns the execution context describing why this request was made.
    pub fn execution_context(&self) -> ExecutionContext {
        self.execution_context
    }

    /// Returns the pipeline type used for this request.
    pub fn pipeline_type(&self) -> PipelineType {
        self.pipeline_type
    }

    /// Returns the transport security mode used for this request.
    pub fn transport_security(&self) -> TransportSecurity {
        self.transport_security
    }

    /// Returns the region this request was sent to.
    pub fn region(&self) -> &Region {
        &self.region
    }

    /// Returns the endpoint URI contacted.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Returns the combined HTTP status and sub-status code.
    pub fn status(&self) -> &CosmosStatus {
        &self.status
    }

    /// Returns the request charge (RU) for this individual request.
    pub fn request_charge(&self) -> RequestCharge {
        self.request_charge
    }

    /// Returns the activity ID from response headers, if present.
    pub fn activity_id(&self) -> Option<&ActivityId> {
        self.activity_id.as_ref()
    }

    /// Returns the session token from response, if present.
    pub fn session_token(&self) -> Option<&str> {
        self.session_token.as_deref()
    }

    /// Returns when this request was started.
    pub fn started_at(&self) -> Instant {
        self.started_at
    }

    /// Returns when this request completed, if it has completed.
    pub fn completed_at(&self) -> Option<Instant> {
        self.completed_at
    }

    /// Returns the duration in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        self.duration_ms
    }

    /// Returns the pipeline events during this request.
    pub fn events(&self) -> &[RequestEvent] {
        &self.events
    }

    /// Returns whether this request timed out.
    pub fn timed_out(&self) -> bool {
        self.timed_out
    }

    /// Returns whether the request was sent on the wire.
    pub fn request_sent(&self) -> RequestSentStatus {
        self.request_sent
    }

    /// Returns the error message if the request failed.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

/// Handle for tracking a request within [`DiagnosticsContext`](super::DiagnosticsContext).
///
/// This is an opaque index used to reference a specific request's diagnostics
/// for updates during request execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestHandle(pub(crate) usize);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_type_classification() {
        assert!(PipelineType::Metadata.is_metadata());
        assert!(!PipelineType::Metadata.is_data_plane());
        assert!(PipelineType::DataPlane.is_data_plane());
        assert!(!PipelineType::DataPlane.is_metadata());
    }

    #[test]
    fn transport_security_classification() {
        assert!(TransportSecurity::Secure.is_secure());
        assert!(!TransportSecurity::Secure.is_emulator());
        assert!(TransportSecurity::EmulatorWithInsecureCertificates.is_emulator());
        assert!(!TransportSecurity::EmulatorWithInsecureCertificates.is_secure());
    }

    #[test]
    fn transport_security_default() {
        assert_eq!(TransportSecurity::default(), TransportSecurity::Secure);
    }

    #[test]
    fn pipeline_type_serialization() {
        assert_eq!(
            serde_json::to_string(&PipelineType::Metadata).unwrap(),
            "\"metadata\""
        );
        assert_eq!(
            serde_json::to_string(&PipelineType::DataPlane).unwrap(),
            "\"data_plane\""
        );
    }

    #[test]
    fn transport_security_serialization() {
        assert_eq!(
            serde_json::to_string(&TransportSecurity::Secure).unwrap(),
            "\"secure\""
        );
        assert_eq!(
            serde_json::to_string(&TransportSecurity::EmulatorWithInsecureCertificates).unwrap(),
            "\"emulator_with_insecure_certificates\""
        );
    }
}
