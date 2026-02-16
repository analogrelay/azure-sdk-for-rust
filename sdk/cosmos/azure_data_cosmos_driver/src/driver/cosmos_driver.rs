// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Cosmos DB driver instance.

use crate::{
    diagnostics::{DiagnosticsContextBuilder, ExecutionContext, PipelineType, TransportSecurity},
    models::{
        AccountEndpoint, AccountReference, ActivityId, ContainerReference, CosmosHeaders,
        CosmosOperation, CosmosResult,
    },
    options::{
        DriverOptions, OperationOptions, Region, RuntimeOptions, ThroughputControlGroupSnapshot,
    },
};
use azure_core::http::{Context, Request};
use std::sync::Arc;

use super::{
    transport::{
        event_channel, is_emulator_host, uses_dataplane_pipeline, AuthorizationContext,
        EventEmitter, TrackedRequestState,
    },
    CosmosDriverRuntime,
};

/// Cosmos DB driver instance.
///
/// A driver represents a connection to a specific Cosmos DB account. It is created
/// via [`CosmosDriverRuntime::get_or_create_driver()`] and is managed as a singleton
/// per account endpoint.
///
/// The driver handles executing operations against Cosmos DB, merging options from
/// operation, driver, and runtime levels.
#[non_exhaustive]
#[derive(Debug)]
pub struct CosmosDriver {
    /// Reference to the parent runtime.
    runtime: CosmosDriverRuntime,
    /// Driver-level options including account reference.
    options: DriverOptions,
}

impl CosmosDriver {
    /// Creates a new driver instance.
    ///
    /// This is internal - use [`CosmosDriverRuntime::get_or_create_driver()`] instead.
    pub(crate) fn new(runtime: CosmosDriverRuntime, options: DriverOptions) -> Self {
        Self { runtime, options }
    }

    /// Returns the account reference.
    pub fn account(&self) -> &AccountReference {
        self.options.account()
    }

    /// Returns the runtime.
    pub fn runtime(&self) -> &CosmosDriverRuntime {
        &self.runtime
    }

    /// Returns the driver options.
    pub fn options(&self) -> &DriverOptions {
        &self.options
    }

    /// Computes the effective runtime options by merging operation, driver, and runtime options.
    ///
    /// The merge order is (highest to lowest priority):
    /// 1. `OperationOptions` - operation-specific overrides
    /// 2. `DriverOptions` - driver-level defaults
    /// 3. `CosmosDriverRuntime` - global defaults
    ///
    /// For each property in `RuntimeOptions`, the first defined value is used.
    pub fn effective_runtime_options(
        &self,
        operation_options: &OperationOptions,
    ) -> RuntimeOptions {
        // Start with operation-level options (highest priority)
        let operation_runtime = operation_options.runtime();

        // Get driver-level options
        let driver_runtime = self.options.runtime_options().snapshot();

        // Get runtime-level options (lowest priority)
        let global_runtime = self.runtime.runtime_options().snapshot();

        // Merge: operation -> driver -> runtime
        // First merge operation with driver
        let merged = operation_runtime.merge_with_base(&driver_runtime);
        // Then merge result with runtime defaults
        merged.merge_with_base(&global_runtime)
    }

    /// Computes the effective throughput control group for an operation.
    ///
    /// Resolution order (first match wins):
    /// 1. Explicit group name from effective runtime options + operation's container
    /// 2. Default group for the operation's container
    ///
    /// Returns `None` if no applicable control group is found.
    ///
    /// # Parameters
    ///
    /// - `effective_options`: The merged runtime options (use `effective_runtime_options()`)
    /// - `container`: The container reference for the operation
    pub(crate) fn effective_throughput_control_group(
        &self,
        effective_options: &RuntimeOptions,
        container: &ContainerReference,
    ) -> Option<ThroughputControlGroupSnapshot> {
        // First, check if an explicit group name is specified in options
        if let Some(group_name) = &effective_options.throughput_control_group_name {
            if let Some(group) = self
                .runtime
                .get_throughput_control_group(container, group_name)
            {
                return Some(ThroughputControlGroupSnapshot::from(group.as_ref()));
            }
        }

        // Fall back to the default group for the container
        self.runtime
            .get_default_throughput_control_group(container)
            .map(|group| ThroughputControlGroupSnapshot::from(group.as_ref()))
    }

    /// Executes a Cosmos DB operation.
    ///
    /// This method computes effective options by merging the provided operation options
    /// with driver and runtime defaults, then executes the operation.
    ///
    /// # Parameters
    ///
    /// - `operation`: The operation to execute
    /// - `options`: Operation-specific options that override driver and runtime defaults
    ///
    /// # Returns
    ///
    /// Returns a [`CosmosResult`] containing the response body, headers, and diagnostics.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The account has no authentication configured
    /// - The resource reference cannot produce a valid path
    /// - The HTTP request fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use azure_data_cosmos_driver::driver::CosmosDriverRuntime;
    /// use azure_data_cosmos_driver::options::{OperationOptions, ContentResponseOnWrite};
    /// use azure_data_cosmos_driver::models::AccountReference;
    /// use url::Url;
    ///
    /// # async fn example() -> azure_core::Result<()> {
    /// let runtime = CosmosDriverRuntime::builder().build().await?;
    ///
    /// let account = AccountReference::new(
    ///     Url::parse("https://myaccount.documents.azure.com:443/").unwrap(),
    /// ).with_master_key("my-key");
    ///
    /// let driver = runtime.get_or_create_driver(account, None).await?;
    ///
    /// // Execute operations with operation-specific options that override defaults
    /// let options = OperationOptions::new()
    ///     .content_response_on_write(ContentResponseOnWrite::DISABLED);
    ///
    /// // let result = driver.execute_operation(operation, options).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_operation(
        &self,
        operation: CosmosOperation,
        options: OperationOptions,
    ) -> azure_core::Result<CosmosResult> {
        // Step 1: Derive effective runtime options
        let effective_options = self.effective_runtime_options(&options);

        // Step 2: Get effective throughput control group (if any)
        let _effective_control_group = operation.container().and_then(|container| {
            self.effective_throughput_control_group(&effective_options, container)
        });

        // Step 3: Initialize diagnostics
        let activity_id = ActivityId::new_uuid();
        let mut diagnostics_builder = DiagnosticsContextBuilder::new(
            activity_id.clone(),
            Arc::clone(self.runtime.diagnostics_options()),
        );

        // Step 4: Get authentication (guaranteed to be present by AccountReference)
        let account = operation.resource_reference().account();
        let auth = account.auth();

        // Step 5: Build resource link for authorization
        let resource_ref = operation.resource_reference();
        let resource_link = resource_ref.link_for_signing();

        // Step 6: Build request URL
        let request_path = resource_ref.request_path();
        let endpoint = AccountEndpoint::from(account);
        let url = endpoint.join_path(&request_path);

        // Step 7: Determine HTTP method and create request
        let operation_type = operation.operation_type();
        let resource_type = operation.resource_type();
        let method = operation_type.http_method();
        let mut request = Request::new(url, method);

        // Step 8: Add body if present
        if let Some(body) = operation.body() {
            request.set_body(body.to_vec());
        }

        // Step 9: Add operation headers
        for (name, value) in operation.headers().iter() {
            request.insert_header(name.clone(), value.clone());
        }

        // Step 9b: Add partition key header if set
        if let Some(pk) = operation.partition_key() {
            use azure_core::http::headers::AsHeaders;
            for (name, value) in pk.as_headers()? {
                request.insert_header(name, value);
            }
        }

        // Step 10: Create authorization context
        // Strip leading slash from resource link for signing
        let signing_link = resource_link.trim_start_matches('/');
        let auth_context = AuthorizationContext::new(method, resource_type, signing_link);

        // Step 11: Select and create appropriate pipeline
        let transport = self.runtime.transport();
        let is_dataplane = uses_dataplane_pipeline(resource_type, operation_type);
        let pipeline = if is_dataplane {
            transport.create_dataplane_pipeline(&endpoint, auth)
        } else {
            transport.create_metadata_pipeline(&endpoint, auth)
        };

        // Determine pipeline type and transport security for diagnostics
        let pipeline_type = if is_dataplane {
            PipelineType::DataPlane
        } else {
            PipelineType::Metadata
        };
        let transport_security = if is_emulator_host(&endpoint) {
            TransportSecurity::EmulatorWithInsecureCertificates
        } else {
            TransportSecurity::Secure
        };

        // Step 12: Build context with authorization info and event emitter
        let mut ctx = Context::default();
        ctx.insert(auth_context);

        // Set up event channel for transport tracking
        let (event_sender, event_receiver) = event_channel();
        ctx.insert(EventEmitter::new(event_sender));

        // Step 13: Start request tracking in diagnostics
        // For now, use a placeholder region - proper region routing will come later
        let region = Region::new("Unknown");
        let request_handle = diagnostics_builder.start_request(
            ExecutionContext::Initial,
            pipeline_type,
            transport_security,
            region,
            endpoint.host().to_owned(),
        );

        // Step 14: Execute request
        let result = pipeline.send(&ctx, &mut request).await;

        // Step 15: Collect events from transport tracking
        let tracked_state = TrackedRequestState::collect(event_receiver);

        // Step 16: Handle response or error
        match result {
            Ok(response) => {
                let status_code = response.status();

                // Extract sub-status from headers if present
                let sub_status = response
                    .headers()
                    .get_optional_str(&azure_core::http::headers::HeaderName::from_static(
                        "x-ms-substatus",
                    ))
                    .and_then(|s| s.parse::<u32>().ok());

                // Update request with response data (before completing to keep it mutable)
                if let Some(charge) = response
                    .headers()
                    .get_optional_str(&azure_core::http::headers::HeaderName::from_static(
                        "x-ms-request-charge",
                    ))
                    .and_then(|s| s.parse::<f64>().ok())
                {
                    diagnostics_builder.update_request(request_handle, |req| {
                        req.request_charge = charge;
                    });
                }

                // Add all transport events to diagnostics
                for event in tracked_state.into_events() {
                    diagnostics_builder.add_event(request_handle, event);
                }

                // Complete request tracking (makes request info immutable)
                diagnostics_builder.complete_request(request_handle, status_code);

                // Set operation status
                diagnostics_builder.set_operation_status(status_code, sub_status);

                // Extract headers and body
                let cosmos_headers = CosmosHeaders::from_headers(response.headers());
                let body = response.into_body();

                // Complete diagnostics
                let diagnostics = Arc::new(diagnostics_builder.complete());

                Ok(CosmosResult::new(
                    body.as_ref().to_vec(),
                    cosmos_headers,
                    diagnostics,
                ))
            }
            Err(e) => {
                // Request failed at transport level - no HTTP response received.
                //
                // Determine request sent status using both events and error analysis:
                // - Sent: If we received ResponseHeadersReceived (definitive)
                // - NotSent: If error indicates pre-send failure (DNS, connect refused)
                // - Unknown: Otherwise (can't determine)
                let request_sent = tracked_state.request_sent_status_with_error(&e);

                // Add all transport events to diagnostics
                for event in tracked_state.into_events() {
                    diagnostics_builder.add_event(request_handle, event);
                }

                diagnostics_builder.fail_request(request_handle, e.to_string(), request_sent);

                // Complete diagnostics with error state
                diagnostics_builder.set_operation_status(
                    azure_core::http::StatusCode::ServiceUnavailable,
                    Some(20003),
                );

                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use url::Url;

    use crate::{
        driver::CosmosDriverRuntimeBuilder,
        models::AccountReference,
        options::{
            ContentResponseOnWrite, CorrelationId, RuntimeOptions, UserAgentSuffix, WorkloadId,
        },
    };

    use super::*;

    fn test_account() -> AccountReference {
        AccountReference::with_master_key(
            Url::parse("https://test.documents.azure.com:443/").unwrap(),
            "test-key",
        )
    }

    #[tokio::test]
    async fn default_runtime_options() {
        let runtime = CosmosDriverRuntimeBuilder::new().build().await.unwrap();
        let snapshot = runtime.runtime_options().snapshot();
        assert!(snapshot.throughput_control_group_name.is_none());
        assert!(snapshot.content_response_on_write.is_none());
        // user_agent is always available with base prefix
        assert!(runtime
            .user_agent()
            .as_str()
            .starts_with("azsdk-rust-cosmos-driver/"));
        assert!(runtime.user_agent().suffix().is_none());
        assert!(runtime.workload_id().is_none());
        assert!(runtime.correlation_id().is_none());
        assert!(runtime.user_agent_suffix().is_none());
        // machine_id is always available
        assert!(!runtime.machine_id().is_empty());
    }

    #[tokio::test]
    async fn builder_sets_runtime_options() {
        let opts = RuntimeOptions::builder()
            .content_response_on_write(ContentResponseOnWrite::DISABLED)
            .build();

        let runtime = CosmosDriverRuntimeBuilder::new()
            .runtime_options(opts)
            .build()
            .await
            .unwrap();

        let snapshot = runtime.runtime_options().snapshot();
        assert_eq!(
            snapshot.content_response_on_write,
            Some(ContentResponseOnWrite::DISABLED)
        );
    }

    #[tokio::test]
    async fn builder_sets_identity_fields() {
        let runtime = CosmosDriverRuntimeBuilder::new()
            .workload_id(WorkloadId::new(25))
            .correlation_id(CorrelationId::new("aks-prod-eastus"))
            .user_agent_suffix(UserAgentSuffix::new("myapp-westus2"))
            .build()
            .await
            .unwrap();

        // user_agent_suffix takes priority for user agent computation
        assert!(runtime.user_agent().as_str().contains("myapp-westus2"));
        assert_eq!(runtime.user_agent().suffix(), Some("myapp-westus2"));
        assert_eq!(runtime.workload_id().unwrap().value(), 25);
        assert_eq!(
            runtime.correlation_id().unwrap().as_str(),
            "aks-prod-eastus"
        );
        assert_eq!(
            runtime.user_agent_suffix().unwrap().as_str(),
            "myapp-westus2"
        );
    }

    #[tokio::test]
    async fn user_agent_computed_from_suffix() {
        let runtime = CosmosDriverRuntimeBuilder::new()
            .user_agent_suffix(UserAgentSuffix::new("my-suffix"))
            .build()
            .await
            .unwrap();

        assert!(runtime
            .user_agent()
            .as_str()
            .starts_with("azsdk-rust-cosmos-driver/"));
        assert!(runtime.user_agent().as_str().contains("my-suffix"));
        assert_eq!(runtime.user_agent().suffix(), Some("my-suffix"));
    }

    #[tokio::test]
    async fn user_agent_computed_from_workload_id() {
        let runtime = CosmosDriverRuntimeBuilder::new()
            .workload_id(WorkloadId::new(42))
            .build()
            .await
            .unwrap();

        assert!(runtime
            .user_agent()
            .as_str()
            .starts_with("azsdk-rust-cosmos-driver/"));
        assert!(runtime.user_agent().as_str().contains("w42"));
    }

    #[tokio::test]
    async fn user_agent_computed_from_correlation_id() {
        let runtime = CosmosDriverRuntimeBuilder::new()
            .correlation_id(CorrelationId::new("my-correlation"))
            .build()
            .await
            .unwrap();

        assert!(runtime
            .user_agent()
            .as_str()
            .starts_with("azsdk-rust-cosmos-driver/"));
        assert!(runtime.user_agent().as_str().contains("my-correlation"));
    }

    #[tokio::test]
    async fn user_agent_suffix_takes_priority_over_workload_id() {
        let runtime = CosmosDriverRuntimeBuilder::new()
            .user_agent_suffix(UserAgentSuffix::new("suffix"))
            .workload_id(WorkloadId::new(25))
            .correlation_id(CorrelationId::new("correlation"))
            .build()
            .await
            .unwrap();

        // suffix should be used, not workload_id or correlation_id
        assert!(runtime.user_agent().as_str().contains("suffix"));
        assert!(!runtime.user_agent().as_str().contains("w25"));
        assert!(!runtime.user_agent().as_str().contains("correlation"));
    }

    #[tokio::test]
    async fn workload_id_takes_priority_over_correlation_id() {
        let runtime = CosmosDriverRuntimeBuilder::new()
            .workload_id(WorkloadId::new(25))
            .correlation_id(CorrelationId::new("correlation"))
            .build()
            .await
            .unwrap();

        // workload_id should be used, not correlation_id
        assert!(runtime.user_agent().as_str().contains("w25"));
        assert!(!runtime.user_agent().as_str().contains("correlation"));
    }

    #[tokio::test]
    async fn effective_correlation_prefers_correlation_id() {
        let runtime = CosmosDriverRuntimeBuilder::new()
            .correlation_id(CorrelationId::new("correlation"))
            .user_agent_suffix(UserAgentSuffix::new("suffix"))
            .build()
            .await
            .unwrap();

        assert_eq!(runtime.effective_correlation(), Some("correlation"));
    }

    #[tokio::test]
    async fn effective_correlation_falls_back_to_suffix() {
        let runtime = CosmosDriverRuntimeBuilder::new()
            .user_agent_suffix(UserAgentSuffix::new("suffix"))
            .build()
            .await
            .unwrap();

        assert_eq!(runtime.effective_correlation(), Some("suffix"));
    }

    #[tokio::test]
    async fn effective_correlation_none_when_both_unset() {
        let runtime = CosmosDriverRuntimeBuilder::new().build().await.unwrap();
        assert!(runtime.effective_correlation().is_none());
    }

    #[tokio::test]
    async fn runtime_modification() {
        let runtime = CosmosDriverRuntimeBuilder::new().build().await.unwrap();

        // Initially none
        assert!(runtime
            .runtime_options()
            .snapshot()
            .content_response_on_write
            .is_none());

        // Modify at runtime
        runtime
            .runtime_options()
            .set_content_response_on_write(Some(ContentResponseOnWrite::ENABLED));

        // Now set
        assert_eq!(
            runtime
                .runtime_options()
                .snapshot()
                .content_response_on_write,
            Some(ContentResponseOnWrite::ENABLED)
        );
    }

    #[tokio::test]
    async fn effective_options_merge_priority() {
        // Runtime has ENABLED
        let cosmos_runtime = CosmosDriverRuntimeBuilder::new()
            .runtime_options(
                RuntimeOptions::builder()
                    .content_response_on_write(ContentResponseOnWrite::ENABLED)
                    .build(),
            )
            .build()
            .await
            .unwrap();

        // Driver has DISABLED
        let driver_options = DriverOptions::builder(test_account())
            .runtime_options(
                RuntimeOptions::builder()
                    .content_response_on_write(ContentResponseOnWrite::DISABLED)
                    .build(),
            )
            .build();

        let driver = CosmosDriver::new(cosmos_runtime, driver_options);

        // Operation has no override - should get driver's DISABLED
        let op_options = OperationOptions::new();
        let effective = driver.effective_runtime_options(&op_options);
        assert_eq!(
            effective.content_response_on_write,
            Some(ContentResponseOnWrite::DISABLED)
        );

        // Operation overrides to ENABLED - should get ENABLED
        let op_options =
            OperationOptions::new().content_response_on_write(ContentResponseOnWrite::ENABLED);
        let effective = driver.effective_runtime_options(&op_options);
        assert_eq!(
            effective.content_response_on_write,
            Some(ContentResponseOnWrite::ENABLED)
        );
    }

    #[tokio::test]
    async fn effective_options_falls_back_to_runtime() {
        // Runtime has ENABLED
        let cosmos_runtime = CosmosDriverRuntimeBuilder::new()
            .runtime_options(
                RuntimeOptions::builder()
                    .content_response_on_write(ContentResponseOnWrite::ENABLED)
                    .build(),
            )
            .build()
            .await
            .unwrap();

        // Driver has no override
        let driver_options = DriverOptions::builder(test_account()).build();

        let driver = CosmosDriver::new(cosmos_runtime, driver_options);

        // Operation has no override - should fall back to runtime's ENABLED
        let op_options = OperationOptions::new();
        let effective = driver.effective_runtime_options(&op_options);
        assert_eq!(
            effective.content_response_on_write,
            Some(ContentResponseOnWrite::ENABLED)
        );
    }

    #[tokio::test]
    async fn machine_id_always_available() {
        let runtime = CosmosDriverRuntimeBuilder::new().build().await.unwrap();

        // machine_id is always available (either VM ID or generated UUID)
        let machine_id = runtime.machine_id();
        assert!(!machine_id.is_empty());

        // It should have one of the known prefixes
        assert!(
            machine_id.starts_with("vmId_") || machine_id.starts_with("uuid_"),
            "machine_id should start with 'vmId_' or 'uuid_', got: {}",
            machine_id
        );
    }
}
