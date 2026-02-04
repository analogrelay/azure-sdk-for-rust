// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! HTTP transport layer for Cosmos DB driver.
//!
//! This module provides connection pooling and transport management for HTTP
//! requests to Azure Cosmos DB. It maintains separate connection pools for:
//!
//! - **Metadata operations**: REST/JSON requests for account, database, and
//!   container management. Uses HTTP/2 when allowed.
//! - **Data plane operations**: Point read/write operations and queries.
//!   Uses HTTP/2 when allowed.
//! - **Emulator operations**: Lazily-initialized pools for local emulator
//!   with certificate validation disabled.

mod emulator;
mod headers_policy;

use crate::{models::AccountEndpoint, options::ConnectionPoolOptions};
use azure_core::http::{ClientOptions, Transport};
use headers_policy::CosmosHeadersPolicy;
use std::sync::{Arc, OnceLock};

pub(crate) use emulator::is_emulator_host;

/// HTTP transport manager for Cosmos DB connections.
///
/// Manages connection pools with separate settings for metadata and data plane
/// operations. Supports both production endpoints and local emulator with
/// lazy initialization of emulator-specific pools.
///
/// # Connection Pools
///
/// - **Metadata pool**: For REST/JSON operations (account/database/container
///   management). Prefers HTTP/2 multiplexing when enabled.
/// - **Data plane pool**: For point operations and queries. Will support RNTBD
///   envelope encapsulation in future versions.
/// - **Emulator pools**: Lazily created when connecting to emulator hosts with
///   certificate validation disabled.
///
/// # Thread Safety
///
/// All pools are thread-safe and can be accessed concurrently. The transport
/// is designed to be shared across all drivers in a runtime.
#[derive(Debug)]
pub(crate) struct CosmosTransport {
    /// Connection pool configuration.
    connection_pool: ConnectionPoolOptions,

    /// Headers policy for setting Cosmos-specific headers.
    headers_policy: Arc<CosmosHeadersPolicy>,

    /// Pipeline for metadata operations (REST/JSON).
    metadata_options: ClientOptions,

    /// Pipeline for data plane operations.
    dataplane_options: ClientOptions,

    /// Lazily-initialized pipeline for emulator metadata operations.
    emulator_metadata_options: OnceLock<ClientOptions>,

    /// Lazily-initialized pipeline for emulator data plane operations.
    emulator_dataplane_options: OnceLock<ClientOptions>,
}

impl CosmosTransport {
    /// Creates a new transport with the given connection pool configuration.
    ///
    /// # Arguments
    ///
    /// * `connection_pool` - Connection pool settings for HTTP clients
    /// * `user_agent` - User agent string to use for all requests
    pub(crate) fn new(
        connection_pool: ConnectionPoolOptions,
        user_agent: impl Into<String>,
    ) -> azure_core::Result<Self> {
        let headers_policy = Arc::new(CosmosHeadersPolicy::new(user_agent));
        let metadata_options = Self::create_metadata_options(&connection_pool, false)?;
        let dataplane_options = Self::create_dataplane_options(&connection_pool, false)?;

        Ok(Self {
            connection_pool,
            headers_policy,
            metadata_options,
            dataplane_options,
            emulator_metadata_options: OnceLock::new(),
            emulator_dataplane_options: OnceLock::new(),
        })
    }

    /// Returns the client options for metadata operations.
    ///
    /// If the endpoint is an emulator host and emulator certificate validation
    /// is disabled, returns options with TLS validation disabled.
    pub(crate) fn metadata_options(&self, endpoint: &AccountEndpoint) -> &ClientOptions {
        if self.should_use_emulator_transport(endpoint) {
            self.emulator_metadata_options.get_or_init(|| {
                Self::create_metadata_options(&self.connection_pool, true)
                    .expect("failed to create emulator metadata options")
            })
        } else {
            &self.metadata_options
        }
    }

    /// Returns the client options for data plane operations.
    ///
    /// If the endpoint is an emulator host and emulator certificate validation
    /// is disabled, returns options with TLS validation disabled.
    pub(crate) fn dataplane_options(&self, endpoint: &AccountEndpoint) -> &ClientOptions {
        if self.should_use_emulator_transport(endpoint) {
            self.emulator_dataplane_options.get_or_init(|| {
                Self::create_dataplane_options(&self.connection_pool, true)
                    .expect("failed to create emulator dataplane options")
            })
        } else {
            &self.dataplane_options
        }
    }

    /// Returns the connection pool options.
    pub(crate) fn connection_pool(&self) -> &ConnectionPoolOptions {
        &self.connection_pool
    }

    /// Returns the headers policy for use in pipeline construction.
    ///
    /// This policy sets Cosmos-specific headers on every request including
    /// API version, SDK capabilities, and user agent.
    pub(crate) fn headers_policy(&self) -> Arc<CosmosHeadersPolicy> {
        Arc::clone(&self.headers_policy)
    }

    /// Determines if emulator transport should be used for the given endpoint.
    fn should_use_emulator_transport(&self, endpoint: &AccountEndpoint) -> bool {
        self.connection_pool
            .is_emulator_server_cert_validation_disabled()
            && is_emulator_host(endpoint)
    }

    /// Creates client options for metadata operations.
    fn create_metadata_options(
        pool: &ConnectionPoolOptions,
        for_emulator: bool,
    ) -> azure_core::Result<ClientOptions> {
        let client = Self::create_reqwest_client(pool, true, for_emulator)?;
        Ok(ClientOptions {
            transport: Some(Transport::new(Arc::new(client))),
            ..Default::default()
        })
    }

    /// Creates client options for data plane operations.
    fn create_dataplane_options(
        pool: &ConnectionPoolOptions,
        for_emulator: bool,
    ) -> azure_core::Result<ClientOptions> {
        let client = Self::create_reqwest_client(pool, false, for_emulator)?;
        Ok(ClientOptions {
            transport: Some(Transport::new(Arc::new(client))),
            ..Default::default()
        })
    }

    /// Creates a reqwest client with the appropriate settings.
    ///
    /// # Arguments
    ///
    /// * `pool` - Connection pool configuration
    /// * `is_metadata` - Whether this is for metadata operations (uses different timeouts)
    /// * `for_emulator` - Whether to disable TLS certificate validation
    fn create_reqwest_client(
        pool: &ConnectionPoolOptions,
        is_metadata: bool,
        for_emulator: bool,
    ) -> azure_core::Result<reqwest::Client> {
        let mut builder = reqwest::ClientBuilder::new();

        // Connection pool settings
        builder = builder.pool_max_idle_per_host(pool.max_idle_connections_per_endpoint());

        if let Some(idle_timeout) = pool.idle_connection_timeout() {
            builder = builder.pool_idle_timeout(idle_timeout);
        }

        // Connect timeout
        builder = builder.connect_timeout(pool.max_connect_timeout());

        // Request timeout (different for metadata vs data plane)
        let request_timeout = if is_metadata {
            pool.max_metadata_request_timeout()
        } else {
            pool.max_dataplane_request_timeout()
        };
        builder = builder.timeout(request_timeout);

        // HTTP/2 settings
        // Note: reqwest's http2_prior_knowledge() forces HTTP/2 only.
        // For now, we let reqwest negotiate (ALPN) which prefers HTTP/2 when available.
        // If we need strict HTTP/2-only, we'd use http2_prior_knowledge().

        // Proxy settings
        if !pool.is_proxy_allowed() {
            builder = builder.no_proxy();
        }
        // When proxy is allowed, reqwest automatically respects HTTP_PROXY/HTTPS_PROXY env vars

        // Local address binding
        if let Some(local_addr) = pool.local_address() {
            builder = builder.local_address(local_addr);
        }

        // Emulator settings - disable TLS validation
        if for_emulator {
            builder = builder.danger_accept_invalid_certs(true);
        }

        builder.build().map_err(|e| {
            azure_core::Error::with_message(
                azure_core::error::ErrorKind::Other,
                format!("Failed to create HTTP client: {e}"),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::ConnectionPoolOptionsBuilder;

    #[test]
    fn transport_creates_with_default_options() {
        let pool = ConnectionPoolOptionsBuilder::new().build().unwrap();
        let transport = CosmosTransport::new(pool, "test-user-agent").unwrap();

        // Should not be using emulator transport for regular endpoints
        let endpoint =
            AccountEndpoint::try_from("https://myaccount.documents.azure.com:443/").unwrap();
        assert!(!transport.should_use_emulator_transport(&endpoint));
    }

    #[test]
    fn transport_detects_emulator_when_disabled() {
        let pool = ConnectionPoolOptionsBuilder::new()
            .dangerous_emulator_server_cert_validation_disabled(true)
            .build()
            .unwrap();
        let transport = CosmosTransport::new(pool, "test-user-agent").unwrap();

        // localhost is an emulator host
        let endpoint = AccountEndpoint::try_from("https://localhost:8081/").unwrap();
        assert!(transport.should_use_emulator_transport(&endpoint));

        // 127.0.0.1 is an emulator host
        let endpoint = AccountEndpoint::try_from("https://127.0.0.1:8081/").unwrap();
        assert!(transport.should_use_emulator_transport(&endpoint));

        // Production endpoint is not an emulator host
        let endpoint =
            AccountEndpoint::try_from("https://myaccount.documents.azure.com:443/").unwrap();
        assert!(!transport.should_use_emulator_transport(&endpoint));
    }

    #[test]
    fn transport_ignores_emulator_hosts_when_validation_enabled() {
        let pool = ConnectionPoolOptionsBuilder::new().build().unwrap();
        let transport = CosmosTransport::new(pool, "test-user-agent").unwrap();

        // Even localhost should not use emulator transport if validation is enabled
        let endpoint = AccountEndpoint::try_from("https://localhost:8081/").unwrap();
        assert!(!transport.should_use_emulator_transport(&endpoint));
    }
}
