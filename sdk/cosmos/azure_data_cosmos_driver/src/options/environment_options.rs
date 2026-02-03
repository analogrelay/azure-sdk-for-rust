// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Environment-level configuration options.

use azure_core::http::{headers::Headers, ClientOptions};
use std::sync::{Arc, RwLock};

use crate::{
    models::ThroughputControlGroupName,
    options::{
        ConnectionPoolOptions, ContentResponseOnWrite, DedicatedGatewayOptions,
        DiagnosticsThresholds, EndToEndOperationLatencyPolicy, ExcludedRegions,
        ReadConsistencyStrategy,
    },
};

/// Thread-safe mutable defaults for operation options.
///
/// These defaults can be modified at runtime and will be applied to all operations
/// that don't explicitly override them.
#[derive(Clone, Debug, Default)]
pub struct MutableDefaults {
    /// Default throughput control group name.
    pub throughput_control_group_name: Option<ThroughputControlGroupName>,
    /// Default dedicated gateway options.
    pub dedicated_gateway_options: Option<DedicatedGatewayOptions>,
    /// Default diagnostics thresholds.
    pub diagnostics_thresholds: Option<DiagnosticsThresholds>,
    /// Default end-to-end latency policy.
    pub end_to_end_latency_policy: Option<EndToEndOperationLatencyPolicy>,
    /// Default custom headers.
    pub custom_headers: Option<Headers>,
    /// Default excluded regions.
    pub excluded_regions: Option<ExcludedRegions>,
    /// Default read consistency strategy.
    pub read_consistency_strategy: Option<ReadConsistencyStrategy>,
    /// Default content response on write setting.
    pub content_response_on_write: Option<ContentResponseOnWrite>,
}

/// Thread-safe wrapper for mutable defaults.
///
/// Provides interior mutability for runtime configuration changes.
#[derive(Clone, Debug, Default)]
pub struct SharedDefaults(Arc<RwLock<MutableDefaults>>);

impl SharedDefaults {
    /// Creates a new empty shared defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates shared defaults from existing mutable defaults.
    pub fn from_defaults(defaults: MutableDefaults) -> Self {
        Self(Arc::new(RwLock::new(defaults)))
    }

    /// Returns a snapshot of the current defaults.
    ///
    /// # Panics
    ///
    /// Panics if the lock is poisoned.
    pub fn snapshot(&self) -> MutableDefaults {
        self.0.read().expect("lock poisoned").clone()
    }

    /// Sets the default throughput control group name.
    pub fn set_throughput_control_group_name(&self, name: Option<ThroughputControlGroupName>) {
        self.0
            .write()
            .expect("lock poisoned")
            .throughput_control_group_name = name;
    }

    /// Sets the default dedicated gateway options.
    pub fn set_dedicated_gateway_options(&self, options: Option<DedicatedGatewayOptions>) {
        self.0
            .write()
            .expect("lock poisoned")
            .dedicated_gateway_options = options;
    }

    /// Sets the default diagnostics thresholds.
    pub fn set_diagnostics_thresholds(&self, thresholds: Option<DiagnosticsThresholds>) {
        self.0
            .write()
            .expect("lock poisoned")
            .diagnostics_thresholds = thresholds;
    }

    /// Sets the default end-to-end latency policy.
    pub fn set_end_to_end_latency_policy(&self, policy: Option<EndToEndOperationLatencyPolicy>) {
        self.0
            .write()
            .expect("lock poisoned")
            .end_to_end_latency_policy = policy;
    }

    /// Sets the default custom headers.
    pub fn set_custom_headers(&self, headers: Option<Headers>) {
        self.0.write().expect("lock poisoned").custom_headers = headers;
    }

    /// Sets the default excluded regions.
    pub fn set_excluded_regions(&self, regions: Option<ExcludedRegions>) {
        self.0.write().expect("lock poisoned").excluded_regions = regions;
    }

    /// Sets the default read consistency strategy.
    pub fn set_read_consistency_strategy(&self, strategy: Option<ReadConsistencyStrategy>) {
        self.0
            .write()
            .expect("lock poisoned")
            .read_consistency_strategy = strategy;
    }

    /// Sets the default content response on write setting.
    pub fn set_content_response_on_write(&self, value: Option<ContentResponseOnWrite>) {
        self.0
            .write()
            .expect("lock poisoned")
            .content_response_on_write = value;
    }
}

/// Configuration options for a Cosmos DB environment.
///
/// An environment represents the global configuration shared across all drivers
/// and connections. It includes connection pool settings and default operation options.
///
/// # Thread Safety
///
/// The mutable defaults can be modified at runtime via the `defaults()` accessor.
/// Changes are thread-safe and will be applied to subsequent operations.
///
/// # Example
///
/// ```
/// use azure_data_cosmos_driver::options::{
///     EnvironmentOptions, EnvironmentOptionsBuilder, ContentResponseOnWrite,
/// };
///
/// let options = EnvironmentOptionsBuilder::new()
///     .default_content_response_on_write(ContentResponseOnWrite::DISABLED)
///     .build();
///
/// // Later, modify defaults at runtime
/// options.defaults().set_content_response_on_write(Some(ContentResponseOnWrite::ENABLED));
/// ```
#[derive(Clone, Debug, Default)]
pub struct EnvironmentOptions {
    /// Core HTTP client options from azure_core.
    client_options: ClientOptions,

    /// Connection pool configuration for managing TCP connections.
    connection_pool: ConnectionPoolOptions,

    /// Thread-safe mutable defaults for operation options.
    defaults: SharedDefaults,
}

impl EnvironmentOptions {
    /// Returns a new builder for creating environment options.
    pub fn builder() -> EnvironmentOptionsBuilder {
        EnvironmentOptionsBuilder::new()
    }

    /// Returns the HTTP client options.
    pub fn client_options(&self) -> &ClientOptions {
        &self.client_options
    }

    /// Returns the connection pool options.
    pub fn connection_pool(&self) -> &ConnectionPoolOptions {
        &self.connection_pool
    }

    /// Returns the thread-safe mutable defaults.
    ///
    /// Use this to modify default operation options at runtime.
    pub fn defaults(&self) -> &SharedDefaults {
        &self.defaults
    }
}

/// Builder for creating [`EnvironmentOptions`].
///
/// Only mutable default properties can be set through this builder.
/// Connection pool and client options are set directly.
#[derive(Clone, Debug, Default)]
pub struct EnvironmentOptionsBuilder {
    client_options: Option<ClientOptions>,
    connection_pool: Option<ConnectionPoolOptions>,
    defaults: MutableDefaults,
}

impl EnvironmentOptionsBuilder {
    /// Creates a new builder with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the HTTP client options.
    #[must_use]
    pub fn client_options(mut self, options: ClientOptions) -> Self {
        self.client_options = Some(options);
        self
    }

    /// Sets the connection pool options.
    #[must_use]
    pub fn connection_pool(mut self, options: ConnectionPoolOptions) -> Self {
        self.connection_pool = Some(options);
        self
    }

    /// Sets the default throughput control group name.
    #[must_use]
    pub fn default_throughput_control_group_name(
        mut self,
        name: ThroughputControlGroupName,
    ) -> Self {
        self.defaults.throughput_control_group_name = Some(name);
        self
    }

    /// Sets the default dedicated gateway options.
    #[must_use]
    pub fn default_dedicated_gateway_options(mut self, options: DedicatedGatewayOptions) -> Self {
        self.defaults.dedicated_gateway_options = Some(options);
        self
    }

    /// Sets the default diagnostics thresholds.
    #[must_use]
    pub fn default_diagnostics_thresholds(mut self, thresholds: DiagnosticsThresholds) -> Self {
        self.defaults.diagnostics_thresholds = Some(thresholds);
        self
    }

    /// Sets the default end-to-end latency policy.
    #[must_use]
    pub fn default_end_to_end_latency_policy(
        mut self,
        policy: EndToEndOperationLatencyPolicy,
    ) -> Self {
        self.defaults.end_to_end_latency_policy = Some(policy);
        self
    }

    /// Sets the default custom headers.
    #[must_use]
    pub fn default_custom_headers(mut self, headers: Headers) -> Self {
        self.defaults.custom_headers = Some(headers);
        self
    }

    /// Sets the default excluded regions.
    #[must_use]
    pub fn default_excluded_regions(mut self, regions: ExcludedRegions) -> Self {
        self.defaults.excluded_regions = Some(regions);
        self
    }

    /// Sets the default read consistency strategy.
    #[must_use]
    pub fn default_read_consistency_strategy(mut self, strategy: ReadConsistencyStrategy) -> Self {
        self.defaults.read_consistency_strategy = Some(strategy);
        self
    }

    /// Sets the default content response on write setting.
    #[must_use]
    pub fn default_content_response_on_write(mut self, value: ContentResponseOnWrite) -> Self {
        self.defaults.content_response_on_write = Some(value);
        self
    }

    /// Builds the [`EnvironmentOptions`].
    pub fn build(self) -> EnvironmentOptions {
        EnvironmentOptions {
            client_options: self.client_options.unwrap_or_default(),
            connection_pool: self.connection_pool.unwrap_or_default(),
            defaults: SharedDefaults::from_defaults(self.defaults),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_environment_options() {
        let options = EnvironmentOptions::default();
        let defaults = options.defaults().snapshot();
        assert!(defaults.throughput_control_group_name.is_none());
        assert!(defaults.content_response_on_write.is_none());
    }

    #[test]
    fn builder_sets_defaults() {
        let options = EnvironmentOptionsBuilder::new()
            .default_content_response_on_write(ContentResponseOnWrite::DISABLED)
            .build();

        let defaults = options.defaults().snapshot();
        assert_eq!(
            defaults.content_response_on_write,
            Some(ContentResponseOnWrite::DISABLED)
        );
    }

    #[test]
    fn runtime_modification() {
        let options = EnvironmentOptions::default();

        // Initially none
        assert!(options
            .defaults()
            .snapshot()
            .content_response_on_write
            .is_none());

        // Modify at runtime
        options
            .defaults()
            .set_content_response_on_write(Some(ContentResponseOnWrite::ENABLED));

        // Now set
        assert_eq!(
            options.defaults().snapshot().content_response_on_write,
            Some(ContentResponseOnWrite::ENABLED)
        );
    }
}
