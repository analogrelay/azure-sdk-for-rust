// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Cosmos DB driver runtime environment.

use azure_core::http::ClientOptions;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::{
    models::{
        AccountEndpoint, ContainerProperties, ContainerReference, ThroughputControlGroupName,
        UserAgent,
    },
    options::{
        ConnectionPoolOptions, CorrelationId, DiagnosticsOptions, SharedRuntimeOptions,
        ThroughputControlGroupOptions, ThroughputControlGroupRegistry, UserAgentSuffix, WorkloadId,
    },
    system::{AzureVmMetadata, CpuMemoryHistory, CpuMemoryMonitor, VmMetadataService},
};

use super::{
    cache::{AccountMetadataCache, ContainerCache},
    transport::CosmosTransport,
    CosmosDriver, CosmosDriverRuntimeBuilder,
};

/// The Cosmos DB driver runtime environment.
///
/// A runtime represents the global configuration shared across all drivers
/// and connections. It includes connection pool settings, default operation options,
/// and manages singleton driver instances per account.
///
/// # Thread Safety
///
/// The runtime is thread-safe and can be shared across threads. Driver instances
/// are managed as singletons per account endpoint, ensuring efficient resource usage.
///
/// # Example
///
/// ```no_run
/// use azure_data_cosmos_driver::driver::{
///     CosmosDriverRuntime, CosmosDriverRuntimeBuilder,
/// };
/// use azure_data_cosmos_driver::options::{RuntimeOptions, ContentResponseOnWrite};
/// use azure_data_cosmos_driver::models::AccountReference;
/// use url::Url;
///
/// # async fn example() -> azure_core::Result<()> {
/// let runtime = RuntimeOptions::builder()
///     .content_response_on_write(ContentResponseOnWrite::DISABLED)
///     .build();
///
/// let cosmos_runtime = CosmosDriverRuntimeBuilder::new()
///     .runtime_options(runtime)
///     .build()
///     .await?;
///
/// // Get or create a driver for an account
/// let account = AccountReference::new(
///     Url::parse("https://myaccount.documents.azure.com:443/").unwrap(),
/// ).with_master_key("my-key");
///
/// let driver = cosmos_runtime.get_or_create_driver(account, None).await?;
///
/// // Later, modify defaults at runtime
/// cosmos_runtime.runtime_options().set_content_response_on_write(Some(ContentResponseOnWrite::ENABLED));
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct CosmosDriverRuntime {
    /// Core HTTP client options from azure_core.
    pub(crate) client_options: ClientOptions,

    /// Connection pool configuration for managing TCP connections.
    pub(crate) connection_pool: ConnectionPoolOptions,

    /// HTTP transport manager with connection pools.
    ///
    /// Manages separate pools for metadata and data plane operations,
    /// with lazy initialization of emulator-specific pools.
    pub(crate) transport: Arc<CosmosTransport>,

    /// Diagnostics configuration for output verbosity and size limits.
    pub(crate) diagnostics_options: Arc<DiagnosticsOptions>,

    /// Thread-safe runtime options for operation options.
    pub(crate) runtime_options: SharedRuntimeOptions,

    /// Computed user agent string for HTTP requests.
    ///
    /// This is automatically computed from the SDK version, platform info,
    /// and optional suffix (from user_agent_suffix, workload_id, or correlation_id).
    pub(crate) user_agent: UserAgent,

    /// Workload identifier for resource governance (1-50 if set).
    pub(crate) workload_id: Option<WorkloadId>,

    /// Correlation ID for client-side metrics.
    ///
    /// Used as a dimension for client-side metrics. If cardinality is too high,
    /// this may be ignored by metrics aggregation.
    pub(crate) correlation_id: Option<CorrelationId>,

    /// User agent suffix appended to identify request source.
    ///
    /// If `correlation_id` is not set, this suffix is used as the correlation
    /// dimension for client-side metrics. Server-side cardinality enforcement
    /// is more strict for this field.
    pub(crate) user_agent_suffix: Option<UserAgentSuffix>,

    /// Process-wide CPU and memory monitor singleton.
    ///
    /// Provides access to historical CPU/memory snapshots for client telemetry.
    /// The monitor runs in a background thread and samples every 5 seconds.
    pub(crate) cpu_memory_monitor: CpuMemoryMonitor,

    /// Process-wide Azure VM metadata service singleton.
    ///
    /// Provides access to VM metadata from the Instance Metadata Service (IMDS).
    /// Metadata is fetched once on first access and cached for the process lifetime.
    pub(crate) vm_metadata_service: VmMetadataService,

    /// Registry of throughput control groups.
    ///
    /// Groups are registered during builder construction and are immutable after
    /// runtime creation (except for mutable target values within each group).
    pub(crate) throughput_control_groups: ThroughputControlGroupRegistry,

    /// Registry of driver instances keyed by account endpoint.
    ///
    /// Ensures singleton driver per account reference.
    pub(crate) driver_registry: Arc<RwLock<HashMap<String, Arc<CosmosDriver>>>>,

    /// Cache for account metadata (regions, capabilities).
    ///
    /// Entries are populated on first access to an account and used for routing.
    /// Wrapped in `Arc` for cheap cloning.
    pub(crate) account_metadata_cache: Arc<AccountMetadataCache>,

    /// Cache for container metadata (partition key definition, indexing policy).
    ///
    /// Entries are populated on first access to a container and used for
    /// partition key extraction and routing. Wrapped in `Arc` for cheap cloning.
    pub(crate) container_cache: Arc<ContainerCache>,
}

impl CosmosDriverRuntime {
    /// Returns a new builder for creating a runtime.
    pub fn builder() -> CosmosDriverRuntimeBuilder {
        CosmosDriverRuntimeBuilder::new()
    }

    /// Returns the HTTP client options.
    pub fn client_options(&self) -> &ClientOptions {
        &self.client_options
    }

    /// Returns the connection pool options.
    pub fn connection_pool(&self) -> &ConnectionPoolOptions {
        &self.connection_pool
    }

    /// Returns the HTTP transport manager.
    ///
    /// The transport provides access to connection pools configured for
    /// metadata and data plane operations, with automatic emulator detection.
    pub(crate) fn transport(&self) -> &Arc<CosmosTransport> {
        &self.transport
    }

    /// Returns the diagnostics options.
    ///
    /// Use this to access verbosity and size settings for diagnostic output.
    pub fn diagnostics_options(&self) -> &Arc<DiagnosticsOptions> {
        &self.diagnostics_options
    }

    /// Returns the thread-safe runtime options.
    ///
    /// Use this to modify default operation options at runtime.
    pub fn runtime_options(&self) -> &SharedRuntimeOptions {
        &self.runtime_options
    }

    /// Returns the computed user agent string.
    ///
    /// The user agent is automatically computed with a static prefix containing
    /// SDK version and platform info, plus an optional suffix derived from
    /// `user_agent_suffix`, `workload_id`, or `correlation_id` (in priority order).
    pub fn user_agent(&self) -> &UserAgent {
        &self.user_agent
    }

    /// Returns the workload identifier.
    pub fn workload_id(&self) -> Option<WorkloadId> {
        self.workload_id
    }

    /// Returns the correlation ID for client-side metrics.
    pub fn correlation_id(&self) -> Option<&CorrelationId> {
        self.correlation_id.as_ref()
    }

    /// Returns the user agent suffix.
    pub fn user_agent_suffix(&self) -> Option<&UserAgentSuffix> {
        self.user_agent_suffix.as_ref()
    }

    /// Returns the effective correlation dimension.
    ///
    /// Returns `correlation_id` if set, otherwise falls back to `user_agent_suffix`.
    pub fn effective_correlation(&self) -> Option<&str> {
        self.correlation_id
            .as_ref()
            .map(|c| c.as_str())
            .or_else(|| self.user_agent_suffix.as_ref().map(|s| s.as_str()))
    }

    /// Returns a snapshot of the current CPU and memory usage history.
    ///
    /// The history contains the most recent CPU load and memory usage samples,
    /// typically covering the last 30 seconds (6 samples at 5-second intervals).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use azure_data_cosmos_driver::driver::CosmosDriverRuntime;
    ///
    /// # async fn example() -> azure_core::Result<()> {
    /// let runtime = CosmosDriverRuntime::builder().build().await?;
    /// let history = runtime.cpu_memory_snapshot();
    ///
    /// if let Some(cpu) = history.latest_cpu() {
    ///     println!("Latest CPU: {:.1}%", cpu.value);
    /// }
    ///
    /// if history.is_cpu_overloaded() {
    ///     println!("Warning: CPU is overloaded");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn cpu_memory_snapshot(&self) -> CpuMemoryHistory {
        self.cpu_memory_monitor.snapshot()
    }

    /// Returns the cached Azure VM metadata, if available.
    ///
    /// Returns `None` if:
    /// - Not running on an Azure VM
    /// - The `COSMOS_DISABLE_IMDS` environment variable is set
    /// - The IMDS endpoint is unreachable
    ///
    /// # Example
    ///
    /// ```no_run
    /// use azure_data_cosmos_driver::driver::CosmosDriverRuntime;
    ///
    /// # async fn example() -> azure_core::Result<()> {
    /// let runtime = CosmosDriverRuntime::builder().build().await?;
    /// if let Some(metadata) = runtime.vm_metadata() {
    ///     println!("VM ID: {}", metadata.vm_id());
    ///     println!("Location: {}", metadata.location());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn vm_metadata(&self) -> Option<&AzureVmMetadata> {
        self.vm_metadata_service.metadata()
    }

    /// Returns the unique machine ID.
    ///
    /// This is always available:
    /// - On Azure VMs: "vmId_{vm-id}" from IMDS
    /// - Off Azure: "uuid_{generated-uuid}" (stable for process lifetime)
    pub fn machine_id(&self) -> &str {
        self.vm_metadata_service.machine_id()
    }

    /// Returns `true` if running on an Azure VM with accessible IMDS.
    pub fn is_on_azure(&self) -> bool {
        self.vm_metadata_service.is_on_azure()
    }

    /// Returns the throughput control group registry.
    ///
    /// The registry contains all groups registered during runtime construction.
    /// Groups are identified by the combination of container reference and group name.
    pub fn throughput_control_groups(&self) -> &ThroughputControlGroupRegistry {
        &self.throughput_control_groups
    }

    /// Returns a throughput control group by container and name.
    ///
    /// This is a convenience method for looking up a specific group.
    pub fn get_throughput_control_group(
        &self,
        container: &ContainerReference,
        name: &ThroughputControlGroupName,
    ) -> Option<&Arc<ThroughputControlGroupOptions>> {
        self.throughput_control_groups
            .get_by_container_and_name(container, name)
    }

    /// Returns the default throughput control group for a container.
    ///
    /// Returns `None` if no default group is registered for the container.
    pub fn get_default_throughput_control_group(
        &self,
        container: &ContainerReference,
    ) -> Option<&Arc<ThroughputControlGroupOptions>> {
        self.throughput_control_groups
            .get_default_for_container(container)
    }

    // ===== Cache Access Methods =====

    /// Returns cached container properties if available.
    ///
    /// Returns `None` if the container hasn't been cached yet.
    /// Use [`get_or_fetch_container_properties`] to fetch and cache if needed.
    pub(crate) async fn get_cached_container_properties(
        &self,
        container: &ContainerReference,
    ) -> Option<Arc<ContainerProperties>> {
        self.container_cache.get(container).await
    }

    /// Gets container properties, fetching and caching if not already cached.
    ///
    /// The `fetch_fn` is only called if the container is not in the cache.
    /// Concurrent requests for the same container share the same fetch operation.
    pub(crate) async fn get_or_fetch_container_properties<F, Fut>(
        &self,
        container: ContainerReference,
        fetch_fn: F,
    ) -> Arc<ContainerProperties>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ContainerProperties>,
    {
        self.container_cache.get_or_fetch(container, fetch_fn).await
    }

    /// Invalidates cached container properties.
    ///
    /// Call this when container properties may have changed (e.g., after
    /// updating indexing policy).
    pub(crate) async fn invalidate_container_cache(&self, container: &ContainerReference) {
        self.container_cache.invalidate(container).await;
    }

    /// Invalidates cached account metadata.
    ///
    /// Call this when account configuration may have changed (e.g., after
    /// adding/removing regions).
    pub(crate) async fn invalidate_account_cache(&self, endpoint: &AccountEndpoint) {
        self.account_metadata_cache.invalidate(endpoint).await;
    }

    /// Clears all caches.
    ///
    /// This is primarily useful for testing or when the connection needs
    /// to be fully refreshed.
    pub(crate) async fn clear_all_caches(&self) {
        self.account_metadata_cache.clear().await;
        self.container_cache.clear().await;
    }
}
