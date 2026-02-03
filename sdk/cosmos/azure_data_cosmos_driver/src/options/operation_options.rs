// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Request options for Cosmos DB operations.

use azure_core::http::headers::Headers;

use crate::{
    models::{ETagCondition, PartitionKey, SessionToken, ThroughputControlGroupName},
    options::{
        ContentResponseOnWrite, DedicatedGatewayOptions, DiagnosticsThresholds,
        EndToEndOperationLatencyPolicy, ExcludedRegions, FilterPredicate,
        NonIdempotentWriteRetries, PriorityLevel, QuotaInfoEnabled, ReadConsistencyStrategy,
        RuntimeOptions, ScriptLoggingEnabled, TriggerOptions,
    },
};

/// Options that can be applied to Cosmos DB operations.
///
/// This struct provides a fluent builder interface for configuring request options
/// such as consistency levels, session tokens, triggers, and other policies.
///
/// # Runtime Options
///
/// Many settings (like `throughput_control_group_name`, `dedicated_gateway_options`, etc.)
/// are shared with `EnvironmentOptions` and `DriverOptions` via [`RuntimeOptions`].
/// Operation-level settings override driver-level, which override environment-level defaults.
///
/// # Example
///
/// ```ignore
/// use azure_data_cosmos_driver::options::OperationOptions;
/// use azure_data_cosmos_driver::models::{PartitionKey, PriorityLevel, ContentResponseOnWrite};
///
/// let options = OperationOptions::new()
///     .partition_key(PartitionKey::from("my-partition"))
///     .priority_level(PriorityLevel::Low)
///     .content_response_on_write(ContentResponseOnWrite::DISABLED);
/// ```
#[derive(Clone, Debug, Default)]
pub struct OperationOptions {
    // Shared runtime options (can be set at environment/driver/operation level)
    runtime: RuntimeOptions,

    // Operation-specific options (not shared with environment/driver)
    session_token: Option<SessionToken>,
    partition_key: Option<PartitionKey>,
    quota_info_enabled: Option<QuotaInfoEnabled>,
    priority_level: Option<PriorityLevel>,

    // Just read operations
    etag_condition: Option<ETagCondition>,

    // Just write operations
    triggers: Option<TriggerOptions>,
    non_idempotent_write_retries: Option<NonIdempotentWriteRetries>,

    // Only patch operations
    filter_predicate: Option<FilterPredicate>,

    // Only StoredProc executions
    script_logging_enabled: Option<ScriptLoggingEnabled>,
}

impl OperationOptions {
    /// Creates a new empty `OperationOptions`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the embedded runtime options.
    ///
    /// These are the options shared with environment and driver levels.
    pub fn runtime(&self) -> &RuntimeOptions {
        &self.runtime
    }

    /// Returns a mutable reference to the embedded runtime options.
    pub fn runtime_mut(&mut self) -> &mut RuntimeOptions {
        &mut self.runtime
    }

    /// Creates effective runtime options by merging with a base.
    ///
    /// Operation-level settings take precedence over the base settings.
    pub fn effective_runtime(&self, base: &RuntimeOptions) -> RuntimeOptions {
        self.runtime.merge_with_base(base)
    }

    /// Sets the trigger options for this operation.
    #[must_use]
    pub fn triggers(mut self, triggers: TriggerOptions) -> Self {
        self.triggers = Some(triggers);
        self
    }

    /// Gets the trigger options.
    pub fn triggers_ref(&self) -> Option<&TriggerOptions> {
        self.triggers.as_ref()
    }

    /// Sets the read consistency strategy for this operation.
    #[must_use]
    pub fn read_consistency_strategy(mut self, strategy: ReadConsistencyStrategy) -> Self {
        self.runtime.read_consistency_strategy = Some(strategy);
        self
    }

    /// Gets the read consistency strategy.
    pub fn read_consistency_strategy_ref(&self) -> Option<&ReadConsistencyStrategy> {
        self.runtime.read_consistency_strategy.as_ref()
    }

    /// Sets the session token for session consistency.
    #[must_use]
    pub fn session_token(mut self, token: SessionToken) -> Self {
        self.session_token = Some(token);
        self
    }

    /// Gets the session token.
    pub fn session_token_ref(&self) -> Option<&SessionToken> {
        self.session_token.as_ref()
    }

    /// Sets the ETag condition for optimistic concurrency.
    #[must_use]
    pub fn etag_condition(mut self, condition: ETagCondition) -> Self {
        self.etag_condition = Some(condition);
        self
    }

    /// Gets the ETag condition.
    pub fn etag_condition_ref(&self) -> Option<&ETagCondition> {
        self.etag_condition.as_ref()
    }

    /// Sets the partition key for this operation.
    #[must_use]
    pub fn partition_key(mut self, key: PartitionKey) -> Self {
        self.partition_key = Some(key);
        self
    }

    /// Gets the partition key.
    pub fn partition_key_ref(&self) -> Option<&PartitionKey> {
        self.partition_key.as_ref()
    }

    /// Sets whether the response should include the content after write operations.
    #[must_use]
    pub fn content_response_on_write(mut self, value: ContentResponseOnWrite) -> Self {
        self.runtime.content_response_on_write = Some(value);
        self
    }

    /// Gets the content response on write setting.
    pub fn content_response_on_write_ref(&self) -> Option<&ContentResponseOnWrite> {
        self.runtime.content_response_on_write.as_ref()
    }

    /// Sets the throughput control group name for this operation.
    #[must_use]
    pub fn throughput_control_group_name(mut self, name: ThroughputControlGroupName) -> Self {
        self.runtime.throughput_control_group_name = Some(name);
        self
    }

    /// Gets the throughput control group name.
    pub fn throughput_control_group_name_ref(&self) -> Option<&ThroughputControlGroupName> {
        self.runtime.throughput_control_group_name.as_ref()
    }

    /// Sets the dedicated gateway options for integrated cache.
    #[must_use]
    pub fn dedicated_gateway_options(mut self, options: DedicatedGatewayOptions) -> Self {
        self.runtime.dedicated_gateway_options = Some(options);
        self
    }

    /// Gets the dedicated gateway options.
    pub fn dedicated_gateway_options_ref(&self) -> Option<&DedicatedGatewayOptions> {
        self.runtime.dedicated_gateway_options.as_ref()
    }

    /// Sets the diagnostics thresholds for this operation.
    #[must_use]
    pub fn diagnostics_thresholds(mut self, thresholds: DiagnosticsThresholds) -> Self {
        self.runtime.diagnostics_thresholds = Some(thresholds);
        self
    }

    /// Gets the diagnostics thresholds.
    pub fn diagnostics_thresholds_ref(&self) -> Option<&DiagnosticsThresholds> {
        self.runtime.diagnostics_thresholds.as_ref()
    }

    /// Sets whether non-idempotent write retries are enabled.
    #[must_use]
    pub fn non_idempotent_write_retries(mut self, value: NonIdempotentWriteRetries) -> Self {
        self.non_idempotent_write_retries = Some(value);
        self
    }

    /// Gets the non-idempotent write retries setting.
    pub fn non_idempotent_write_retries_ref(&self) -> Option<&NonIdempotentWriteRetries> {
        self.non_idempotent_write_retries.as_ref()
    }

    /// Sets the end-to-end operation latency policy.
    #[must_use]
    pub fn end_to_end_latency_policy(mut self, policy: EndToEndOperationLatencyPolicy) -> Self {
        self.runtime.end_to_end_latency_policy = Some(policy);
        self
    }

    /// Gets the end-to-end operation latency policy.
    pub fn end_to_end_latency_policy_ref(&self) -> Option<&EndToEndOperationLatencyPolicy> {
        self.runtime.end_to_end_latency_policy.as_ref()
    }

    /// Sets the regions to exclude from routing.
    #[must_use]
    pub fn excluded_regions(mut self, regions: ExcludedRegions) -> Self {
        self.runtime.excluded_regions = Some(regions);
        self
    }

    /// Gets the excluded regions.
    pub fn excluded_regions_ref(&self) -> Option<&ExcludedRegions> {
        self.runtime.excluded_regions.as_ref()
    }

    /// Sets the priority level for this operation.
    #[must_use]
    pub fn priority_level(mut self, level: PriorityLevel) -> Self {
        self.priority_level = Some(level);
        self
    }

    /// Gets the priority level.
    pub fn priority_level_ref(&self) -> Option<&PriorityLevel> {
        self.priority_level.as_ref()
    }

    /// Sets whether script logging is enabled.
    #[must_use]
    pub fn script_logging_enabled(mut self, value: ScriptLoggingEnabled) -> Self {
        self.script_logging_enabled = Some(value);
        self
    }

    /// Gets the script logging enabled setting.
    pub fn script_logging_enabled_ref(&self) -> Option<&ScriptLoggingEnabled> {
        self.script_logging_enabled.as_ref()
    }

    /// Sets whether quota info is included in responses.
    #[must_use]
    pub fn quota_info_enabled(mut self, value: QuotaInfoEnabled) -> Self {
        self.quota_info_enabled = Some(value);
        self
    }

    /// Gets the quota info enabled setting.
    pub fn quota_info_enabled_ref(&self) -> Option<&QuotaInfoEnabled> {
        self.quota_info_enabled.as_ref()
    }

    /// Sets custom HTTP headers to include in the request.
    #[must_use]
    pub fn custom_headers(mut self, headers: Headers) -> Self {
        self.runtime.custom_headers = Some(headers);
        self
    }

    /// Gets the custom headers.
    pub fn custom_headers_ref(&self) -> Option<&Headers> {
        self.runtime.custom_headers.as_ref()
    }

    /// Sets the filter predicate for conditional patch operations.
    ///
    /// The filter predicate is a SQL-like condition that must evaluate to true
    /// for the patch operation to be applied. Only used with patch operations.
    #[must_use]
    pub fn filter_predicate(mut self, predicate: impl Into<FilterPredicate>) -> Self {
        self.filter_predicate = Some(predicate.into());
        self
    }

    /// Gets the filter predicate.
    pub fn filter_predicate_ref(&self) -> Option<&FilterPredicate> {
        self.filter_predicate.as_ref()
    }
}
