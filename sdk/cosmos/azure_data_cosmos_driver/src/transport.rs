// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Stable HTTP transport plug points for the Cosmos driver.
//!
//! Re-exports the types a caller needs to supply a custom
//! [`HttpClientFactory`] to [`CosmosDriverRuntimeBuilder::with_http_client_factory`].
//! Available under the `pluggable_runtime` feature (and the existing
//! `__internal_in_memory_emulator` / `__internal_mocking` internal flags).
//!
#![doc = include_str!("../docs/pluggable-runtime-warning.md")]
//!
//! [`CosmosDriverRuntimeBuilder::with_http_client_factory`]: crate::CosmosDriverRuntimeBuilder::with_http_client_factory

pub use crate::driver::transport::cosmos_transport_client::{
    HttpRequest, HttpResponse, TransportClient, TransportError,
};
pub use crate::driver::transport::http_client_factory::{
    HttpClientConfig, HttpClientFactory, HttpVersionPolicy,
};
