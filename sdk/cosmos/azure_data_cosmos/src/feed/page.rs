// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! A single page returned by a Cosmos DB feed operation.

use std::sync::Arc;

use serde::Deserialize;

use crate::{diagnostics::DiagnosticsContext, models::ResponseHeaders};

/// A single page returned by a Cosmos DB feed operation.
///
/// Feed operations include queries and other list-style APIs. Each page
/// contains the returned items, parsed response headers, and diagnostics for
/// the work needed to produce that page.
#[derive(Debug)]
pub struct FeedPage<T> {
    /// The items in the response.
    items: Vec<T>,

    /// Parsed Cosmos-specific response headers.
    headers: ResponseHeaders,

    /// Diagnostics for this page.
    diagnostics: Arc<DiagnosticsContext>,
}

impl<T> FeedPage<T> {
    /// Creates a new `FeedPage` instance.
    pub(crate) fn new(
        items: Vec<T>,
        headers: ResponseHeaders,
        diagnostics: Arc<DiagnosticsContext>,
    ) -> Self {
        Self {
            items,
            headers,
            diagnostics,
        }
    }

    /// Gets the items in this page of results.
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// Consumes the page and returns a vector of the items.
    pub fn into_items(self) -> Vec<T> {
        self.items
    }

    /// Returns the parsed Cosmos-specific response headers for this page.
    pub fn headers(&self) -> &ResponseHeaders {
        &self.headers
    }

    /// Returns diagnostics for this page.
    ///
    /// The returned [`DiagnosticsContext`] includes request tracking, retries,
    /// contacted regions, and other details about the operation.
    pub fn diagnostics(&self) -> Arc<DiagnosticsContext> {
        Arc::clone(&self.diagnostics)
    }
}

/// Internal wire-format wrapper for a feed body returned by the service.
#[derive(Deserialize)]
pub(crate) struct FeedBody<T> {
    #[serde(alias = "Documents")]
    #[serde(alias = "DocumentCollections")]
    #[serde(alias = "Databases")]
    #[serde(alias = "Offers")]
    pub(crate) items: Vec<T>,
}
