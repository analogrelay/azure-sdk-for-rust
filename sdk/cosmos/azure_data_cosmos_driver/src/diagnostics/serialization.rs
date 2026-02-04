// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Internal JSON serialization structures for diagnostics output.

use crate::models::{ActivityId, SubStatusCode};
use azure_core::http::StatusCode;
use serde::Serialize;

use super::{ExecutionContext, RequestDiagnostics};

/// Detailed diagnostics output structure.
#[derive(Serialize)]
pub(super) struct DetailedDiagnosticsOutput<'a> {
    pub activity_id: &'a ActivityId,
    pub total_duration_ms: u64,
    pub total_request_charge: f64,
    pub request_count: usize,
    pub requests: &'a [RequestDiagnostics],
}

/// Summary diagnostics output structure.
#[derive(Serialize)]
pub(super) struct SummaryDiagnosticsOutput<'a> {
    pub activity_id: &'a ActivityId,
    pub total_duration_ms: u64,
    pub total_request_charge: f64,
    pub request_count: usize,
    pub regions: Vec<RegionSummary>,
}

/// Summary of requests in a single region.
#[derive(Serialize)]
pub(super) struct RegionSummary {
    pub region: String,
    pub request_count: usize,
    pub total_request_charge: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first: Option<RequestSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<RequestSummary>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub deduplicated_groups: Vec<DeduplicatedGroup>,
}

/// Summary of a single request.
#[derive(Serialize)]
pub(super) struct RequestSummary {
    pub execution_context: ExecutionContext,
    pub endpoint: String,
    #[serde(serialize_with = "serialize_status_code")]
    pub status_code: StatusCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_status_code: Option<SubStatusCode>,
    pub request_charge: f64,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub timed_out: bool,
}

fn serialize_status_code<S>(status: &StatusCode, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u16((*status).into())
}

impl From<&RequestDiagnostics> for RequestSummary {
    fn from(req: &RequestDiagnostics) -> Self {
        Self {
            execution_context: req.execution_context,
            endpoint: req.endpoint.clone(),
            status_code: req.status_code,
            sub_status_code: req.sub_status_code,
            request_charge: req.request_charge,
            duration_ms: req.duration_ms,
            timed_out: req.timed_out,
        }
    }
}

/// Group of deduplicated similar requests.
#[derive(Serialize)]
pub(super) struct DeduplicatedGroup {
    pub endpoint: String,
    #[serde(serialize_with = "serialize_status_code")]
    pub status_code: StatusCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_status_code: Option<SubStatusCode>,
    pub execution_context: ExecutionContext,
    pub count: usize,
    pub total_request_charge: f64,
    pub min_duration_ms: u64,
    pub max_duration_ms: u64,
    pub p50_duration_ms: u64,
}

/// Truncated output indicator.
#[derive(Serialize)]
pub(super) struct TruncatedOutput<'a> {
    pub activity_id: &'a ActivityId,
    pub total_duration_ms: u64,
    pub request_count: usize,
    pub truncated: bool,
    pub message: &'static str,
}
