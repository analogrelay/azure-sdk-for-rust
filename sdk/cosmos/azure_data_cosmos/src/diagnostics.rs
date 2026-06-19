// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Diagnostics captured for Cosmos DB operations.
//!
//! [`DiagnosticsContext`] records request tracking, retries, contacted regions,
//! and other details about an operation. You can access it from
//! [`CosmosError`](crate::CosmosError) on failure and from response types such
//! as [`FeedPage`](crate::feed::FeedPage) and [`ItemResponse`](crate::models::ItemResponse)
//! on success.

// =========================================================================
// Public API
// =========================================================================

/// Diagnostics for a single Cosmos DB operation.
#[doc(inline)]
pub use azure_data_cosmos_driver::diagnostics::DiagnosticsContext;
