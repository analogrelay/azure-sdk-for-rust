// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Builder for [`CosmosDriverRuntime`].

use azure_core::http::ClientOptions;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::{
    models::AccountReference,
    options::{
        ConnectionPoolOptions, CorrelationId, DriverOptions, RuntimeOptions,
        SharedRuntimeOptions, ThroughputControlGroupOptions,
        ThroughputControlGroupRegistrationError, ThroughputControlGroupRegistry, UserAgent,
        UserAgentSuffix, WorkloadId,
    },
    system::{CpuMemoryMonitor, VmMetadataService},
};

use super::{CosmosDriver, CosmosDriverRuntime};

/// Builder for creating [`CosmosDriverRuntime`].
///
/// Use [`RuntimeOptions::builder()`] to create runtime options, then pass them
/// to this builder via [`runtime_options()`](Self::runtime_options).
///
/// # User Agent
///
/// The user agent string is automatically computed with a static prefix containing
/// SDK version and platform info. The suffix is derived from (in priority order):
/// 1. [`user_agent_suffix()`](Self::user_agent_suffix) if set
/// 2. [`workload_id()`](Self::workload_id) if set (formatted as `w{id}`)
/// 3. [`correlation_id()`](Self::correlation_id) if set
/// 4. No suffix (base user agent only)
///
/// # Throughput Control Groups
///
/// Throughput control groups must be registered during builder construction.
/// Once `build()` is called, the set of groups is immutable (though mutable
/// values within each group can still be updated).
#[derive(Clone, Debug, Default)]
pub struct CosmosDriverRuntimeBuilder {
    client_options: Option<ClientOptions>,
    connection_pool: Option<ConnectionPoolOptions>,
    runtime_options: Option<RuntimeOptions>,
    workload_id: Option<WorkloadId>,
    correlation_id: Option<CorrelationId>,
    user_agent_suffix: Option<UserAgentSuffix>,
    throughput_control_groups: ThroughputControlGroupRegistry,
}

impl CosmosDriverRuntimeBuilder {
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

    /// Sets the runtime options (defaults for operations).
    ///
    /// Use [`RuntimeOptions::builder()`] to create the runtime options.
    #[must_use]
    pub fn runtime_options(mut self, options: RuntimeOptions) -> Self {
        self.runtime_options = Some(options);
        self
    }

    /// Sets the workload identifier (must be 1-50 if set).
    ///
    /// The workload ID is used as a fallback for the user agent suffix
    /// if [`user_agent_suffix()`](Self::user_agent_suffix) is not set.
    #[must_use]
    pub fn workload_id(mut self, workload_id: WorkloadId) -> Self {
        self.workload_id = Some(workload_id);
        self
    }

    /// Sets the correlation ID for client-side metrics.
    ///
    /// The correlation ID is used as a fallback for the user agent suffix
    /// if neither [`user_agent_suffix()`](Self::user_agent_suffix) nor
    /// [`workload_id()`](Self::workload_id) is set.
    ///
    /// # Cardinality Warning
    ///
    /// If the cardinality of correlation IDs is too high, metrics aggregation
    /// may ignore this dimension. Choose values with moderate cardinality
    /// (e.g., cluster names, environment identifiers, deployment IDs).
    #[must_use]
    pub fn correlation_id(mut self, correlation_id: CorrelationId) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Sets the user agent suffix.
    ///
    /// If `correlation_id` is not set, this suffix is used as the correlation
    /// dimension for client-side metrics.
    ///
    /// # Server-Side Enforcement
    ///
    /// The Cosmos DB service enforces cardinality limits more strictly for
    /// user agent suffixes. High-cardinality suffixes may be rejected.
    ///
    /// Good examples: AKS cluster name, Azure VM ID (if limited nodes),
    /// app name with region.
    #[must_use]
    pub fn user_agent_suffix(mut self, suffix: UserAgentSuffix) -> Self {
        self.user_agent_suffix = Some(suffix);
        self
    }

    /// Registers a throughput control group.
    ///
    /// Groups are identified by the combination of container reference and group name.
    /// At most one group per container can be marked as `is_default = true`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - A group with the same (container, name) key already exists
    /// - Another group is already marked as default for the same container
    ///
    /// # Example
    ///
    /// ```no_run
    /// use azure_data_cosmos_driver::driver::CosmosDriverRuntimeBuilder;
    /// use azure_data_cosmos_driver::options::{ThroughputControlGroupOptions, ThroughputTarget};
    /// use azure_data_cosmos_driver::models::{AccountReference, DatabaseReference, ContainerReference};
    /// use url::Url;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let account = AccountReference::new(
    ///     Url::parse("https://myaccount.documents.azure.com:443/").unwrap(),
    /// );
    /// let database = DatabaseReference::from_name(account, "mydb");
    /// let container = ContainerReference::from_name(database, "mycollection");
    ///
    /// // Register a default group for the container
    /// let runtime = CosmosDriverRuntimeBuilder::new()
    ///     .register_throughput_control_group(
    ///         ThroughputControlGroupOptions::client_side(
    ///             "default-group",
    ///             container.clone(),
    ///             ThroughputTarget::Threshold(0.5),
    ///             None,
    ///             true, // is_default
    ///         )
    ///     )?
    ///     .build()
    ///     .await;
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::result_large_err)]
    pub fn register_throughput_control_group(
        mut self,
        group: ThroughputControlGroupOptions,
    ) -> Result<Self, ThroughputControlGroupRegistrationError> {
        self.throughput_control_groups.register(group)?;
        Ok(self)
    }

    /// Builds the [`CosmosDriverRuntime`].
    ///
    /// This automatically initializes the process-wide CPU/memory monitor and
    /// VM metadata service singletons if they haven't been initialized already.
    ///
    /// The user agent is computed from (in priority order):
    /// 1. `user_agent_suffix` if set
    /// 2. `workload_id` if set (formatted as `w{id}`)
    /// 3. `correlation_id` if set
    /// 4. No suffix (base user agent only)
    ///
    /// # Note
    ///
    /// This method is async because it may need to fetch Azure VM metadata from
    /// the Instance Metadata Service (IMDS) on first initialization.
    pub async fn build(self) -> CosmosDriverRuntime {
        // Compute user agent from suffix/workloadId/correlationId (in priority order)
        let user_agent = if let Some(ref suffix) = self.user_agent_suffix {
            UserAgent::from_suffix(suffix)
        } else if let Some(workload_id) = self.workload_id {
            UserAgent::from_workload_id(workload_id)
        } else if let Some(ref correlation_id) = self.correlation_id {
            UserAgent::from_correlation_id(correlation_id)
        } else {
            UserAgent::default()
        };

        CosmosDriverRuntime {
            client_options: self.client_options.unwrap_or_default(),
            connection_pool: self.connection_pool.unwrap_or_default(),
            runtime_options: SharedRuntimeOptions::from_options(
                self.runtime_options.unwrap_or_default(),
            ),
            user_agent,
            workload_id: self.workload_id,
            correlation_id: self.correlation_id,
            user_agent_suffix: self.user_agent_suffix,
            cpu_memory_monitor: CpuMemoryMonitor::get_or_init(),
            vm_metadata_service: VmMetadataService::get_or_init().await,
            throughput_control_groups: self.throughput_control_groups,
            driver_registry: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl CosmosDriverRuntime {
    /// Gets or creates a driver for the specified account.
    ///
    /// This method ensures singleton behavior - only one driver instance exists
    /// per account endpoint. Subsequent calls with the same account endpoint
    /// return the existing driver.
    ///
    /// # Parameters
    ///
    /// - `account`: The account reference (endpoint + credentials)
    /// - `driver_options`: Optional driver-level options. If not provided, defaults are used.
    ///
    /// # Note
    ///
    /// If a driver already exists for the account, the `driver_options` parameter is ignored.
    /// The existing driver with its original options is returned.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use azure_data_cosmos_driver::driver::CosmosDriverRuntime;
    /// use azure_data_cosmos_driver::options::DriverOptions;
    /// use azure_data_cosmos_driver::models::AccountReference;
    /// use url::Url;
    ///
    /// # async fn example() -> azure_core::Result<()> {
    /// let runtime = CosmosDriverRuntime::builder().build().await;
    ///
    /// let account = AccountReference::new(
    ///     Url::parse("https://myaccount.documents.azure.com:443/").unwrap(),
    /// ).with_master_key("my-key");
    ///
    /// // First call creates the driver
    /// let driver = runtime.get_or_create_driver(account.clone(), None).await?;
    ///
    /// // Subsequent calls return the same driver instance
    /// let driver2 = runtime.get_or_create_driver(account, None).await?;
    /// // driver and driver2 point to the same instance
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_or_create_driver(
        &self,
        account: AccountReference,
        driver_options: Option<DriverOptions>,
    ) -> azure_core::Result<Arc<CosmosDriver>> {
        let key = account.endpoint().to_string();

        // Check if driver already exists (read lock)
        {
            let registry = self.driver_registry.read().unwrap();
            if let Some(driver) = registry.get(&key) {
                return Ok(driver.clone());
            }
        }

        // Create new driver (write lock)
        let mut registry = self.driver_registry.write().unwrap();

        // Double-check after acquiring write lock
        if let Some(driver) = registry.get(&key) {
            return Ok(driver.clone());
        }

        // Build driver options if not provided
        let options = driver_options.unwrap_or_else(|| DriverOptions::builder(account).build());

        let driver = Arc::new(CosmosDriver::new(self.clone(), options));
        registry.insert(key, driver.clone());

        Ok(driver)
    }
}
