// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Stable plug points for replacing the HTTP transport and async runtime
//! used by [`CosmosClient`](crate::CosmosClient).
//!
//! Pass a [`CosmosDriverRuntimeBuilder`] pre-configured with
//! [`CosmosDriverRuntimeBuilder::with_http_client_factory`] and/or
//! [`CosmosDriverRuntimeBuilder::with_async_runtime`] to
//! [`CosmosClientBuilder::with_driver_runtime_builder`](crate::CosmosClientBuilder::with_driver_runtime_builder).
//! The SDK's own settings (connection pool, wrapping SDK identifier,
//! PPCB default, fault-injection rules, throughput-control groups) are
//! layered onto the supplied builder per the documented field-interaction
//! rules; see the setter for full details.
//!
#![doc = include_str!("../docs/pluggable-runtime-warning.md")]

pub use azure_core::async_runtime::{AbortableTask, AsyncRuntime, SpawnedTask, TaskFuture};
pub use azure_data_cosmos_driver::transport::{
    HttpClientConfig, HttpClientFactory, HttpRequest, HttpResponse, HttpVersionPolicy,
    TransportClient, TransportError,
};
pub use azure_data_cosmos_driver::{CosmosDriverRuntime, CosmosDriverRuntimeBuilder};
