// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Types for working with Cosmos DB feed results.
//!
//! This module includes query scopes, query definitions, paged response types,
//! and async iterators for consuming query results item by item or page by page.

// =========================================================================
// Public API
// =========================================================================

#[doc(inline)]
pub use azure_data_cosmos_driver::models::{ContinuationToken, FeedRange};
pub use iterator::{QueryItemIterator, QueryPageIterator};
pub use page::FeedPage;
pub use query::{FeedScope, Query};
pub use query_page::QueryFeedPage;

// =========================================================================
// Crate-internal re-exports
// =========================================================================

pub(crate) use page::FeedBody;

// =========================================================================
// Internal modules
// =========================================================================

mod iterator;
mod page;
mod query;
mod query_page;
