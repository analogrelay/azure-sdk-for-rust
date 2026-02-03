// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Request options for Cosmos DB operations.

use azure_core::http::headers::Headers;

use crate::{
    models::{
        ContentResponseOnWrite, DedicatedGatewayOptions, DiagnosticsThresholds,
        EndToEndOperationLatencyPolicy, ETagCondition, ExcludedRegions, NonIdempotentWriteRetries,
        PartitionKey, QuotaInfoEnabled, ScriptLoggingEnabled, ThroughputControlGroupName,
    },
    options::ReadConsistencyStrategy,
};

use super::{PriorityLevel, SessionToken, TriggerOptions};

/// Options that can be applied to Cosmos DB operations.
///
/// This struct provides a fluent builder interface for configuring request options
/// such as consistency levels, session tokens, triggers, and other policies.
///
/// # Example
///
/// ```ignore
/// use azure_data_cosmos_driver::models::{
///     OperationOptions, PartitionKey, PriorityLevel, ContentResponseOnWrite,
/// };
///
/// let options = OperationOptions::new()
///     .partition_key(PartitionKey::from("my-partition"))
///     .priority_level(PriorityLevel::LOW)
///     .content_response_on_write(ContentResponseOnWrite::DISABLED);
/// ```
#[derive(Clone, Debug, Default)]
pub struct OperationOptions {
    triggers: Option<TriggerOptions>,
    read_consistency_strategy: Option<ReadConsistencyStrategy>,
    session_token: Option<SessionToken>,
    etag_condition: Option<ETagCondition>,
    partition_key: Option<PartitionKey>,
    content_response_on_write: Option<ContentResponseOnWrite>,
    throughput_control_group_name: Option<ThroughputControlGroupName>,
    dedicated_gateway_options: Option<DedicatedGatewayOptions>,
    diagnostics_thresholds: Option<DiagnosticsThresholds>,
    non_idempotent_write_retries: Option<NonIdempotentWriteRetries>,
    end_to_end_latency_policy: Option<EndToEndOperationLatencyPolicy>,
    excluded_regions: Option<ExcludedRegions>,
    priority_level: Option<PriorityLevel>,
    script_logging_enabled: Option<ScriptLoggingEnabled>,
    quota_info_enabled: Option<QuotaInfoEnabled>,
    custom_headers: Option<Headers>,
}

impl OperationOptions {
    /// Creates a new empty `OperationOptions`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the trigger options for this operation.
    pub fn triggers(mut self, triggers: TriggerOptions) -> Self {
        self.triggers = Some(triggers);
        self
    }

    /// Gets the trigger options.
    pub fn get_triggers(&self) -> Option<&TriggerOptions> {
        self.triggers.as_ref()
    }

    /// Sets the read consistency strategy for this operation.
    pub fn read_consistency_strategy(mut self, strategy: ReadConsistencyStrategy) -> Self {
        self.read_consistency_strategy = Some(strategy);
        self
    }

    /// Gets the read consistency strategy.
    pub fn get_read_consistency_strategy(&self) -> Option<&ReadConsistencyStrategy> {
        self.read_consistency_strategy.as_ref()
    }

    /// Sets the session token for session consistency.
    pub fn session_token(mut self, token: SessionToken) -> Self {
        self.session_token = Some(token);
        self
    }

    /// Gets the session token.
    pub fn get_session_token(&self) -> Option<&SessionToken> {
        self.session_token.as_ref()
    }

    /// Sets the ETag condition for optimistic concurrency.
    pub fn etag_condition(mut self, condition: ETagCondition) -> Self {
        self.etag_condition = Some(condition);
        self
    }

    /// Gets the ETag condition.
    pub fn get_etag_condition(&self) -> Option<&ETagCondition> {
        self.etag_condition.as_ref()
    }

    /// Sets the partition key for this operation.
    pub fn partition_key(mut self, key: PartitionKey) -> Self {
        self.partition_key = Some(key);
        self
    }

    /// Gets the partition key.
    pub fn get_partition_key(&self) -> Option<&PartitionKey> {
        self.partition_key.as_ref()
    }

    /// Sets whether the response should include the content after write operations.
    pub fn content_response_on_write(mut self, value: ContentResponseOnWrite) -> Self {
        self.content_response_on_write = Some(value);
        self
    }

    /// Gets the content response on write setting.
    pub fn get_content_response_on_write(&self) -> Option<&ContentResponseOnWrite> {
        self.content_response_on_write.as_ref()
    }

    /// Sets the throughput control group name for this operation.
    pub fn throughput_control_group_name(mut self, name: ThroughputControlGroupName) -> Self {
        self.throughput_control_group_name = Some(name);
        self
    }

    /// Gets the throughput control group name.
    pub fn get_throughput_control_group_name(&self) -> Option<&ThroughputControlGroupName> {
        self.throughput_control_group_name.as_ref()
    }

    /// Sets the dedicated gateway options for integrated cache.
    pub fn dedicated_gateway_options(mut self, options: DedicatedGatewayOptions) -> Self {
        self.dedicated_gateway_options = Some(options);
        self
    }

    /// Gets the dedicated gateway options.
    pub fn get_dedicated_gateway_options(&self) -> Option<&DedicatedGatewayOptions> {
        self.dedicated_gateway_options.as_ref()
    }

    /// Sets the diagnostics thresholds for this operation.
    pub fn diagnostics_thresholds(mut self, thresholds: DiagnosticsThresholds) -> Self {
        self.diagnostics_thresholds = Some(thresholds);
        self
    }

    /// Gets the diagnostics thresholds.
    pub fn get_diagnostics_thresholds(&self) -> Option<&DiagnosticsThresholds> {
        self.diagnostics_thresholds.as_ref()
    }

    /// Sets whether non-idempotent write retries are enabled.
    pub fn non_idempotent_write_retries(mut self, value: NonIdempotentWriteRetries) -> Self {
        self.non_idempotent_write_retries = Some(value);
        self
    }

    /// Gets the non-idempotent write retries setting.
    pub fn get_non_idempotent_write_retries(&self) -> Option<&NonIdempotentWriteRetries> {
        self.non_idempotent_write_retries.as_ref()
    }

    /// Sets the end-to-end operation latency policy.
    pub fn end_to_end_latency_policy(mut self, policy: EndToEndOperationLatencyPolicy) -> Self {
        self.end_to_end_latency_policy = Some(policy);
        self
    }

    /// Gets the end-to-end operation latency policy.
    pub fn get_end_to_end_latency_policy(&self) -> Option<&EndToEndOperationLatencyPolicy> {
        self.end_to_end_latency_policy.as_ref()
    }

    /// Sets the regions to exclude from routing.
    pub fn excluded_regions(mut self, regions: ExcludedRegions) -> Self {
        self.excluded_regions = Some(regions);
        self
    }

    /// Gets the excluded regions.
    pub fn get_excluded_regions(&self) -> Option<&ExcludedRegions> {
        self.excluded_regions.as_ref()
    }

    /// Sets the priority level for this operation.
    pub fn priority_level(mut self, level: PriorityLevel) -> Self {
        self.priority_level = Some(level);
        self
    }

    /// Gets the priority level.
    pub fn get_priority_level(&self) -> Option<&PriorityLevel> {
        self.priority_level.as_ref()
    }

    /// Sets whether script logging is enabled.
    pub fn script_logging_enabled(mut self, value: ScriptLoggingEnabled) -> Self {
        self.script_logging_enabled = Some(value);
        self
    }

    /// Gets the script logging enabled setting.
    pub fn get_script_logging_enabled(&self) -> Option<&ScriptLoggingEnabled> {
        self.script_logging_enabled.as_ref()
    }

    /// Sets whether quota info is included in responses.
    pub fn quota_info_enabled(mut self, value: QuotaInfoEnabled) -> Self {
        self.quota_info_enabled = Some(value);
        self
    }

    /// Gets the quota info enabled setting.
    pub fn get_quota_info_enabled(&self) -> Option<&QuotaInfoEnabled> {
        self.quota_info_enabled.as_ref()
    }

    /// Sets custom HTTP headers to include in the request.
    pub fn custom_headers(mut self, headers: Headers) -> Self {
        self.custom_headers = Some(headers);
        self
    }

    /// Gets the custom headers.
    pub fn get_custom_headers(&self) -> Option<&Headers> {
        self.custom_headers.as_ref()
    }
}
