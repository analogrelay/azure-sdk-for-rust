// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! The main diagnostics context for tracking operation-level diagnostics.

use crate::{
    models::{ActivityId, SubStatusCode},
    options::{DiagnosticsOptions, DiagnosticsVerbosity, Region},
};
use azure_core::http::StatusCode;
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use super::{
    serialization::{
        DeduplicatedGroup, DetailedDiagnosticsOutput, RegionSummary, RequestSummary,
        SummaryDiagnosticsOutput, TruncatedOutput,
    },
    ExecutionContext, RequestDiagnostics, RequestEvent, RequestHandle,
};

/// Internal mutable builder for constructing a [`DiagnosticsContext`].
///
/// This type is used during operation execution to collect diagnostic data.
/// Once the operation completes, call [`complete`](Self::complete) to produce
/// an immutable [`DiagnosticsContext`].
///
/// All methods on this builder are `pub(crate)` as it is an internal type.
#[derive(Debug)]
pub(crate) struct DiagnosticsContextBuilder {
    /// Operation-level activity ID.
    activity_id: ActivityId,

    /// When this operation started.
    started_at: Instant,

    /// All request diagnostics collected during this operation.
    requests: Vec<RequestDiagnostics>,

    /// Operation-level HTTP status code (final status after retries).
    status_code: Option<StatusCode>,

    /// Operation-level sub-status code (final sub-status after retries).
    sub_status_code: Option<SubStatusCode>,

    /// Reference to diagnostics configuration.
    options: Arc<DiagnosticsOptions>,
}

impl DiagnosticsContextBuilder {
    /// Creates a new diagnostics context builder for an operation.
    pub(crate) fn new(activity_id: ActivityId, options: Arc<DiagnosticsOptions>) -> Self {
        Self {
            activity_id,
            started_at: Instant::now(),
            requests: Vec::with_capacity(4), // Expect 1-4 requests in most cases
            status_code: None,
            sub_status_code: None,
            options,
        }
    }

    /// Returns the operation's activity ID.
    pub(crate) fn activity_id(&self) -> &ActivityId {
        &self.activity_id
    }

    /// Sets the operation-level status codes.
    ///
    /// This should be called when the operation completes to record the
    /// final HTTP status and sub-status codes.
    pub(crate) fn set_operation_status(
        &mut self,
        status_code: StatusCode,
        sub_status_code: Option<SubStatusCode>,
    ) {
        self.status_code = Some(status_code);
        self.sub_status_code = sub_status_code;
    }

    /// Starts tracking a new request and returns a handle for updates.
    ///
    /// This should be called at the beginning of each HTTP request.
    /// The returned [`RequestHandle`] is used to record completion or timeout.
    pub(crate) fn start_request(
        &mut self,
        execution_context: ExecutionContext,
        region: Region,
        endpoint: String,
    ) -> RequestHandle {
        let request = RequestDiagnostics::new(execution_context, region, endpoint);
        let handle = RequestHandle(self.requests.len());
        self.requests.push(request);
        handle
    }

    /// Records completion of a request.
    ///
    /// Should be called when the HTTP response is received.
    pub(crate) fn complete_request(&mut self, handle: RequestHandle, status_code: StatusCode) {
        if let Some(request) = self.requests.get_mut(handle.0) {
            request.complete(status_code);
        }
    }

    /// Records timeout of a request.
    ///
    /// Should be called when a request times out before receiving a response.
    pub(crate) fn timeout_request(&mut self, handle: RequestHandle) {
        if let Some(request) = self.requests.get_mut(handle.0) {
            request.timeout();
        }
    }

    /// Records failure of a request with an error message.
    ///
    /// Should be called when a transport-level error occurs (connection failure,
    /// DNS error, TLS error, etc.) and no HTTP response was received.
    ///
    /// # Parameters
    ///
    /// - `handle`: The request handle from [`start_request`](Self::start_request)
    /// - `error`: The error message describing the failure
    /// - `request_sent`: Whether the request was sent on the wire before failure.
    ///   This is critical for retry safety - see [`RequestDiagnostics::fail`].
    pub(crate) fn fail_request(
        &mut self,
        handle: RequestHandle,
        error: impl Into<String>,
        request_sent: bool,
    ) {
        if let Some(request) = self.requests.get_mut(handle.0) {
            request.fail(error, request_sent);
        }
    }

    /// Updates a request's diagnostics with additional data.
    ///
    /// Use this to add response headers data (charge, activity ID, etc.).
    ///
    /// # Panics (debug builds)
    ///
    /// Panics if the request has already been completed via [`complete_request`](Self::complete_request).
    /// In release builds, the update is silently ignored.
    pub(crate) fn update_request<F>(&mut self, handle: RequestHandle, f: F)
    where
        F: FnOnce(&mut RequestDiagnostics),
    {
        if let Some(request) = self.requests.get_mut(handle.0) {
            debug_assert!(
                !request.is_completed(),
                "update_request called after complete_request - updates should occur before completion"
            );
            if !request.is_completed() {
                f(request);
            }
        }
    }

    /// Adds a pipeline event to a request.
    pub(crate) fn add_event(&mut self, handle: RequestHandle, event: RequestEvent) {
        if let Some(request) = self.requests.get_mut(handle.0) {
            request.add_event(event);
        }
    }

    /// Returns the total request charge (RU) across all requests.
    pub(crate) fn total_request_charge(&self) -> f64 {
        self.requests.iter().map(|r| r.request_charge).sum()
    }

    /// Returns the number of requests made during this operation.
    pub(crate) fn request_count(&self) -> usize {
        self.requests.len()
    }

    /// Completes the builder and returns an immutable [`DiagnosticsContext`].
    ///
    /// This consumes the builder and creates a finalized diagnostics context
    /// with all data frozen. The `DiagnosticsContext` can then be safely
    /// shared via `Arc` without any locking overhead.
    pub(crate) fn complete(self) -> DiagnosticsContext {
        let duration = self.started_at.elapsed();
        DiagnosticsContext {
            activity_id: self.activity_id,
            duration,
            requests: Arc::new(self.requests),
            status_code: self.status_code,
            sub_status_code: self.sub_status_code,
            options: self.options,
            cached_json_detailed: OnceLock::new(),
            cached_json_summary: OnceLock::new(),
        }
    }
}

/// Diagnostic context for a Cosmos DB operation.
///
/// This is an **immutable** type containing detailed information about request execution
/// including RU consumption, regions contacted, retry attempts, and timing information.
///
/// # Immutability
///
/// Once created from a `DiagnosticsContextBuilder`, a `DiagnosticsContext` is fully
/// immutable. All data is frozen at completion time, and no further mutations are possible.
/// This enables lock-free access and efficient sharing via `Arc`.
///
/// # Efficient Multi-Read
///
/// The [`requests`](Self::requests) method returns `Arc<Vec<RequestDiagnostics>>`,
/// allowing multiple readers to share the same allocation without cloning. This is
/// efficient for repeated access patterns.
///
/// # JSON Caching
///
/// JSON serialization via [`to_json_string`](Self::to_json_string) is lazily cached.
/// The first call computes the JSON; subsequent calls return the cached string.
///
/// # JSON Verbosity Levels
///
/// - **Summary**: Optimized for size constraints, deduplicates similar requests
/// - **Detailed**: Full information about every request
#[derive(Debug)]
pub struct DiagnosticsContext {
    /// Operation-level activity ID.
    activity_id: ActivityId,

    /// Total duration of the operation (from start to completion).
    duration: Duration,

    /// All request diagnostics (shared via Arc for efficient multi-read).
    requests: Arc<Vec<RequestDiagnostics>>,

    /// Operation-level HTTP status code (final status after retries).
    status_code: Option<StatusCode>,

    /// Operation-level sub-status code (final sub-status after retries).
    sub_status_code: Option<SubStatusCode>,

    /// Reference to diagnostics configuration.
    options: Arc<DiagnosticsOptions>,

    /// Cached JSON string for detailed verbosity.
    cached_json_detailed: OnceLock<String>,

    /// Cached JSON string for summary verbosity.
    cached_json_summary: OnceLock<String>,
}

impl DiagnosticsContext {
    /// Returns the operation's activity ID.
    pub fn activity_id(&self) -> &ActivityId {
        &self.activity_id
    }

    /// Returns the operation duration.
    ///
    /// This is the total time from operation start to completion.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Returns the operation-level HTTP status code.
    ///
    /// This is the final status code after all retries and failovers.
    pub fn status_code(&self) -> Option<StatusCode> {
        self.status_code
    }

    /// Returns the operation-level sub-status code.
    ///
    /// Sub-status codes provide more specific error classification than
    /// HTTP status codes alone.
    pub fn sub_status_code(&self) -> Option<SubStatusCode> {
        self.sub_status_code
    }

    /// Returns the total request charge (RU) across all requests.
    pub fn total_request_charge(&self) -> f64 {
        self.requests.iter().map(|r| r.request_charge).sum()
    }

    /// Returns the number of requests made during this operation.
    pub fn request_count(&self) -> usize {
        self.requests.len()
    }

    /// Returns all regions contacted during this operation.
    pub fn regions_contacted(&self) -> Vec<Region> {
        let mut regions: Vec<Region> = self.requests.iter().map(|r| r.region.clone()).collect();
        regions.sort();
        regions.dedup();
        regions
    }

    /// Returns a shared reference to all request diagnostics.
    ///
    /// This returns an `Arc<Vec<RequestDiagnostics>>`, enabling efficient
    /// sharing without cloning the entire vector. Cloning the `Arc` is
    /// a cheap atomic increment (~5 CPU cycles).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let requests = diagnostics.requests();
    /// for req in requests.iter() {
    ///     println!("Request to {} took {}ms", req.endpoint, req.duration_ms);
    /// }
    /// // requests can be stored or passed elsewhere cheaply
    /// ```
    pub fn requests(&self) -> Arc<Vec<RequestDiagnostics>> {
        Arc::clone(&self.requests)
    }

    /// Serializes diagnostics to a JSON string.
    ///
    /// The result is lazily cached - the first call computes the JSON,
    /// subsequent calls return the cached string (for the same verbosity level).
    ///
    /// # Arguments
    ///
    /// * `verbosity` - Output verbosity level. Pass `None` to use the default from options.
    ///
    /// # Returns
    ///
    /// JSON string representation of diagnostics, truncated in Summary mode to fit
    /// within configured size limits.
    pub fn to_json_string(&self, verbosity: Option<DiagnosticsVerbosity>) -> &str {
        let effective_verbosity = match verbosity.unwrap_or(self.options.default_verbosity()) {
            DiagnosticsVerbosity::Default => self.options.default_verbosity(),
            v => v,
        };

        match effective_verbosity {
            DiagnosticsVerbosity::Default | DiagnosticsVerbosity::Detailed => self
                .cached_json_detailed
                .get_or_init(|| self.compute_json_detailed()),
            DiagnosticsVerbosity::Summary => self
                .cached_json_summary
                .get_or_init(|| self.compute_json_summary(self.options.max_summary_size_bytes())),
        }
    }

    fn compute_json_detailed(&self) -> String {
        let total_duration_ms = self.duration.as_millis() as u64;
        let output = DetailedDiagnosticsOutput {
            activity_id: &self.activity_id,
            total_duration_ms,
            total_request_charge: self.requests.iter().map(|r| r.request_charge).sum(),
            request_count: self.requests.len(),
            requests: &self.requests,
        };
        serde_json::to_string(&output).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
    }

    fn compute_json_summary(&self, max_size: usize) -> String {
        let total_duration_ms = self.duration.as_millis() as u64;

        // Group requests by region
        let mut region_groups = HashMap::<Region, Vec<&RequestDiagnostics>>::new();
        for req in self.requests.iter() {
            region_groups
                .entry(req.region.clone())
                .or_default()
                .push(req);
        }

        // Build summary for each region
        let mut region_summaries = Vec::new();
        for (region, requests) in region_groups {
            region_summaries.push(build_region_summary(region, requests));
        }

        // Sort by region name for deterministic output
        region_summaries.sort_by(|a, b| a.region.cmp(&b.region));

        let output = SummaryDiagnosticsOutput {
            activity_id: &self.activity_id,
            total_duration_ms,
            total_request_charge: self.requests.iter().map(|r| r.request_charge).sum(),
            request_count: self.requests.len(),
            regions: region_summaries,
        };

        let json =
            serde_json::to_string(&output).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));

        // Truncate if needed
        if json.len() <= max_size {
            json
        } else {
            // Return a truncated indicator
            let truncated = TruncatedOutput {
                activity_id: &self.activity_id,
                total_duration_ms,
                request_count: self.requests.len(),
                truncated: true,
                message:
                    "Output truncated to fit size limit. Use Detailed verbosity for full diagnostics.",
            };
            serde_json::to_string(&truncated)
                .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
        }
    }
}

/// Builds a summary for requests in a single region.
fn build_region_summary(region: Region, requests: Vec<&RequestDiagnostics>) -> RegionSummary {
    let count = requests.len();
    let total_charge: f64 = requests.iter().map(|r| r.request_charge).sum();

    // Keep first and last in full detail
    let first = requests.first().map(|r| RequestSummary::from(*r));
    let last = if count > 1 {
        requests.last().map(|r| RequestSummary::from(*r))
    } else {
        None
    };

    // Deduplicate middle requests
    let middle_requests: Vec<_> = if count > 2 {
        requests[1..count - 1].to_vec()
    } else {
        Vec::new()
    };

    let deduped_groups = deduplicate_requests(middle_requests);

    RegionSummary {
        region: region.to_string(),
        request_count: count,
        total_request_charge: total_charge,
        first,
        last,
        deduplicated_groups: deduped_groups,
    }
}

/// Key for deduplicating requests.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct DeduplicationKey {
    endpoint: String,
    status_code: StatusCode,
    sub_status_code: Option<SubStatusCode>,
    execution_context: ExecutionContext,
}

/// Deduplicates requests by grouping similar ones.
fn deduplicate_requests(requests: Vec<&RequestDiagnostics>) -> Vec<DeduplicatedGroup> {
    let mut groups = HashMap::<DeduplicationKey, Vec<&RequestDiagnostics>>::new();

    for req in requests {
        let key = DeduplicationKey {
            endpoint: req.endpoint.clone(),
            status_code: req.status_code,
            sub_status_code: req.sub_status_code,
            execution_context: req.execution_context,
        };
        groups.entry(key).or_default().push(req);
    }

    groups
        .into_iter()
        .map(|(key, reqs)| {
            let durations: Vec<u64> = reqs.iter().map(|r| r.duration_ms).collect();
            let total_charge: f64 = reqs.iter().map(|r| r.request_charge).sum();

            DeduplicatedGroup {
                endpoint: key.endpoint,
                status_code: key.status_code,
                sub_status_code: key.sub_status_code,
                execution_context: key.execution_context,
                count: reqs.len(),
                total_request_charge: total_charge,
                min_duration_ms: *durations.iter().min().unwrap_or(&0),
                max_duration_ms: *durations.iter().max().unwrap_or(&0),
                p50_duration_ms: percentile(&durations, 50),
            }
        })
        .collect()
}

/// Calculates the Nth percentile of a sorted list.
fn percentile(values: &[u64], p: u8) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let index = ((p as f64 / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[index.min(sorted.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_options() -> Arc<DiagnosticsOptions> {
        Arc::new(DiagnosticsOptions::default())
    }

    /// Helper to create a completed DiagnosticsContext from a builder.
    fn make_context_with<F>(activity_id: ActivityId, f: F) -> DiagnosticsContext
    where
        F: FnOnce(&mut DiagnosticsContextBuilder),
    {
        let mut builder = DiagnosticsContextBuilder::new(activity_id, make_options());
        f(&mut builder);
        builder.complete()
    }

    #[test]
    fn builder_new_context_has_activity_id() {
        let activity_id = ActivityId::new_uuid();
        let ctx = make_context_with(activity_id.clone(), |_| {});
        assert_eq!(ctx.activity_id(), &activity_id);
    }

    #[test]
    fn builder_start_and_complete_request() {
        let ctx = make_context_with(ActivityId::new_uuid(), |builder| {
            let handle = builder.start_request(
                ExecutionContext::Initial,
                Region::WEST_US_2,
                "https://test.documents.azure.com".to_string(),
            );

            std::thread::sleep(std::time::Duration::from_millis(10));
            builder.complete_request(handle, StatusCode::Ok);
        });

        let requests = ctx.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].status_code, StatusCode::Ok);
        assert!(requests[0].duration_ms >= 10);
        assert!(requests[0].completed_at.is_some());
    }

    #[test]
    fn builder_timeout_request() {
        let ctx = make_context_with(ActivityId::new_uuid(), |builder| {
            let handle = builder.start_request(
                ExecutionContext::Initial,
                Region::WEST_US_2,
                "https://test.documents.azure.com".to_string(),
            );
            builder.timeout_request(handle);
        });

        let requests = ctx.requests();
        assert!(requests[0].timed_out);
    }

    #[test]
    fn builder_update_request_with_charge() {
        let ctx = make_context_with(ActivityId::new_uuid(), |builder| {
            let handle = builder.start_request(
                ExecutionContext::Initial,
                Region::WEST_US_2,
                "https://test.documents.azure.com".to_string(),
            );
            builder.update_request(handle, |req| {
                req.request_charge = 5.5;
            });
        });

        assert_eq!(ctx.total_request_charge(), 5.5);
    }

    #[test]
    fn total_charge_sums_all_requests() {
        let ctx = make_context_with(ActivityId::new_uuid(), |builder| {
            let h1 = builder.start_request(
                ExecutionContext::Initial,
                Region::WEST_US_2,
                "https://test.documents.azure.com".to_string(),
            );
            builder.update_request(h1, |req| req.request_charge = 3.0);

            let h2 = builder.start_request(
                ExecutionContext::Retry,
                Region::WEST_US_2,
                "https://test.documents.azure.com".to_string(),
            );
            builder.update_request(h2, |req| req.request_charge = 2.5);
        });

        assert!((ctx.total_request_charge() - 5.5).abs() < f64::EPSILON);
    }

    #[test]
    fn regions_contacted_deduplicates() {
        let ctx = make_context_with(ActivityId::new_uuid(), |builder| {
            builder.start_request(
                ExecutionContext::Initial,
                Region::WEST_US_2,
                "https://test.westus2.documents.azure.com".to_string(),
            );
            builder.start_request(
                ExecutionContext::Retry,
                Region::WEST_US_2,
                "https://test.westus2.documents.azure.com".to_string(),
            );
            builder.start_request(
                ExecutionContext::RegionFailover,
                Region::EAST_US_2,
                "https://test.eastus2.documents.azure.com".to_string(),
            );
        });

        let regions = ctx.regions_contacted();
        assert_eq!(regions.len(), 2);
    }

    #[test]
    fn to_json_detailed() {
        let ctx = make_context_with(ActivityId::from_string("test-id".to_string()), |builder| {
            let handle = builder.start_request(
                ExecutionContext::Initial,
                Region::WEST_US_2,
                "https://test.documents.azure.com".to_string(),
            );
            builder.update_request(handle, |req| req.request_charge = 1.0);
            builder.complete_request(handle, StatusCode::Ok);
        });

        let json = ctx.to_json_string(Some(DiagnosticsVerbosity::Detailed));
        assert!(json.contains("test-id"));
        assert!(json.contains("westus2")); // Region serializes to normalized name
    }

    #[test]
    fn to_json_summary() {
        let ctx = make_context_with(ActivityId::from_string("test-id".to_string()), |builder| {
            // Add several requests to trigger deduplication
            for i in 0..5 {
                let handle = builder.start_request(
                    ExecutionContext::Retry,
                    Region::WEST_US_2,
                    "https://test.documents.azure.com".to_string(),
                );
                builder.update_request(handle, |req| req.request_charge = i as f64);
                builder.complete_request(handle, StatusCode::TooManyRequests);
            }
        });

        let json = ctx.to_json_string(Some(DiagnosticsVerbosity::Summary));
        assert!(json.contains("test-id"));
        assert!(json.contains("request_count"));
    }

    #[test]
    fn json_caching_detailed() {
        let ctx = make_context_with(
            ActivityId::from_string("cache-test".to_string()),
            |builder| {
                let handle = builder.start_request(
                    ExecutionContext::Initial,
                    Region::WEST_US_2,
                    "https://test.documents.azure.com".to_string(),
                );
                builder.complete_request(handle, StatusCode::Ok);
            },
        );

        // First call computes
        let json1 = ctx.to_json_string(Some(DiagnosticsVerbosity::Detailed));
        // Second call should return cached
        let json2 = ctx.to_json_string(Some(DiagnosticsVerbosity::Detailed));

        // Both should be identical (pointer comparison proves caching)
        assert_eq!(json1, json2);
        assert!(std::ptr::eq(json1, json2)); // Same string reference
    }

    #[test]
    fn requests_returns_arc() {
        let ctx = make_context_with(ActivityId::new_uuid(), |builder| {
            builder.start_request(
                ExecutionContext::Initial,
                Region::WEST_US_2,
                "https://test.documents.azure.com".to_string(),
            );
        });

        let requests1 = ctx.requests();
        let requests2 = ctx.requests();

        // Both should point to the same allocation (Arc::ptr_eq)
        assert!(Arc::ptr_eq(&requests1, &requests2));
    }

    #[test]
    fn duration_is_captured() {
        let ctx = make_context_with(ActivityId::new_uuid(), |builder| {
            std::thread::sleep(std::time::Duration::from_millis(10));
            builder.start_request(
                ExecutionContext::Initial,
                Region::WEST_US_2,
                "https://test.documents.azure.com".to_string(),
            );
        });

        assert!(ctx.duration().as_millis() >= 10);
    }

    #[test]
    fn status_codes_stored() {
        let mut builder = DiagnosticsContextBuilder::new(ActivityId::new_uuid(), make_options());
        builder.set_operation_status(
            StatusCode::NotFound,
            Some(SubStatusCode::READ_SESSION_NOT_AVAILABLE),
        );
        let ctx = builder.complete();

        assert_eq!(ctx.status_code(), Some(StatusCode::NotFound));
        assert_eq!(
            ctx.sub_status_code(),
            Some(SubStatusCode::READ_SESSION_NOT_AVAILABLE)
        );
    }

    #[test]
    fn percentile_calculation() {
        assert_eq!(percentile(&[], 50), 0);
        assert_eq!(percentile(&[100], 50), 100);
        assert_eq!(percentile(&[10, 20, 30, 40, 50], 50), 30);
        assert_eq!(percentile(&[10, 20, 30, 40, 50], 0), 10);
        assert_eq!(percentile(&[10, 20, 30, 40, 50], 100), 50);
    }

    #[test]
    fn update_before_complete_succeeds() {
        let mut builder = DiagnosticsContextBuilder::new(ActivityId::new_uuid(), make_options());
        let handle = builder.start_request(
            ExecutionContext::Initial,
            Region::WEST_US_2,
            "https://test.documents.azure.com".to_string(),
        );

        // Update before complete - should work
        builder.update_request(handle, |req| {
            req.request_charge = 5.5;
        });

        // Now complete
        builder.complete_request(handle, StatusCode::Ok);

        let ctx = builder.complete();
        let requests = ctx.requests();
        assert_eq!(requests[0].request_charge, 5.5);
    }

    #[test]
    fn update_after_complete_is_ignored_in_release() {
        let mut builder = DiagnosticsContextBuilder::new(ActivityId::new_uuid(), make_options());
        let handle = builder.start_request(
            ExecutionContext::Initial,
            Region::WEST_US_2,
            "https://test.documents.azure.com".to_string(),
        );

        // Update with initial value
        builder.update_request(handle, |req| {
            req.request_charge = 5.5;
        });

        // Complete the request
        builder.complete_request(handle, StatusCode::Ok);

        // In release builds, this update should be silently ignored
        // In debug builds, this would panic (tested separately)
        #[cfg(not(debug_assertions))]
        {
            builder.update_request(handle, |req| {
                req.request_charge = 10.0; // Attempt to change after completion
            });

            let ctx = builder.complete();
            let requests = ctx.requests();
            // Value should remain 5.5, not 10.0
            assert_eq!(requests[0].request_charge, 5.5);
        }
    }
}
