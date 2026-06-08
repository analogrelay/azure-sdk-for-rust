// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! **Unsupported internal API — no stability guarantees.**
//!
//! This module is only available when the `__internal_mocking` feature flag
//! is enabled. It is intended exclusively for use in benchmarks and test
//! harnesses that need to replace the reqwest transport layer with an
//! in-memory mock.
//!
//! Breaking changes may be made to this module at any time without a semver
//! bump. Do **not** depend on this module in production code.
//!
//! New callers should use the stable
//! [`crate::transport`] module exposed under the
//! `pluggable_runtime` feature flag, together with
//! [`CosmosDriverRuntimeBuilder::with_http_client_factory`](crate::CosmosDriverRuntimeBuilder::with_http_client_factory).
//! The setters and types in this module are now thin re-exports of the
//! stable surface, kept only for backwards compatibility with existing
//! benchmark and mock harnesses.

pub use crate::options::ConnectionPoolOptions;
pub use crate::transport::{
    HttpClientConfig, HttpClientFactory, HttpRequest, HttpResponse, TransportClient, TransportError,
};
