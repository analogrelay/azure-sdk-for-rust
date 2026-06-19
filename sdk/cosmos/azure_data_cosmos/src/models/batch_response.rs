// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! [`BatchResponse`] for transactional batch operation responses.

use std::sync::Arc;

use crate::diagnostics::DiagnosticsContext;
use crate::models::CosmosStatus;
use crate::models::TransactionalBatchResponse;
use crate::models::{CosmosResponse, ResponseBody, ResponseHeaders};
use azure_core::fmt::SafeDebug;

/// A response from a transactional batch operation.
///
/// Includes the batch status, response headers, diagnostics, and response body.
/// The ETag in [`ResponseHeaders`] applies to the batch response as a whole, not
/// to an individual operation result. For per-operation ETags, inspect the
/// [`TransactionalBatchOperationResult`](crate::models::TransactionalBatchOperationResult)
/// values in the deserialized [`TransactionalBatchResponse`].
#[derive(SafeDebug)]
#[safe(true)]
#[non_exhaustive]
pub struct BatchResponse {
    response: CosmosResponse,
}

impl BatchResponse {
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

    /// Deserializes the response body into a [`TransactionalBatchResponse`].
    ///
    /// # Errors
    ///
    /// Returns an error if the response body cannot be deserialized as a batch
    /// response.
    pub fn into_model(self) -> crate::Result<TransactionalBatchResponse> {
        self.response.into_model()
    }
}
