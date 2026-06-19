// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! [`ItemResponse`] for point item operations.

use std::sync::Arc;

use crate::diagnostics::DiagnosticsContext;
use crate::models::CosmosStatus;
use crate::models::{CosmosResponse, ResponseBody, ResponseHeaders};
use azure_core::fmt::SafeDebug;
use serde::de::DeserializeOwned;

/// A response from a point item operation.
///
/// Includes the operation status, response headers, diagnostics, and item
/// payload.
#[derive(SafeDebug)]
#[safe(true)]
#[non_exhaustive]
pub struct ItemResponse {
    response: CosmosResponse,
}

impl ItemResponse {
    pub(crate) fn new(response: CosmosResponse) -> Self {
        Self { response }
    }

    /// Returns the operation status.
    pub fn status(&self) -> CosmosStatus {
        self.response.status()
    }

    /// Returns the response headers.
    pub fn headers(&self) -> &ResponseHeaders {
        self.response.cosmos_headers()
    }

    /// Consumes the response and returns the response body.
    ///
    /// Use [`ResponseBody::into_single`] to deserialize the contained
    /// item, or [`into_model::<T>`](Self::into_model) for a one-shot convenience.
    pub fn into_body(self) -> ResponseBody {
        self.response.into_body()
    }

    /// Returns diagnostics for this operation.
    ///
    /// The returned [`DiagnosticsContext`] includes details such as request
    /// timing, retries, contacted regions, request charges, and status.
    pub fn diagnostics(&self) -> Arc<DiagnosticsContext> {
        self.response.diagnostics()
    }

    /// Deserializes the response body into `T`.
    ///
    /// # Errors
    ///
    /// Returns an error if the response body cannot be deserialized as `T`.
    pub fn into_model<T: DeserializeOwned>(self) -> crate::Result<T> {
        self.response.into_model::<T>()
    }
}
