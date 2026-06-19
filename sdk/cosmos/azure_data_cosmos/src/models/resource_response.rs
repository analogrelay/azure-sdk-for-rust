// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! [`ResourceResponse`] for resource management operations.

use std::{marker::PhantomData, sync::Arc};

use crate::diagnostics::DiagnosticsContext;
use crate::models::CosmosStatus;
use crate::models::{CosmosResponse, ResponseBody, ResponseHeaders};
use azure_core::fmt::SafeDebug;
use serde::de::DeserializeOwned;

/// A response from a resource management operation.
///
/// Includes the operation status, response headers, diagnostics, and resource
/// payload. The type parameter `T` is the model type returned by
/// [`into_model`](Self::into_model).
#[derive(SafeDebug)]
#[safe(true)]
#[non_exhaustive]
pub struct ResourceResponse<T> {
    response: CosmosResponse,
    _marker: PhantomData<fn() -> T>,
}

impl<T> ResourceResponse<T> {
    pub(crate) fn new(response: CosmosResponse) -> Self {
        Self {
            response,
            _marker: PhantomData,
        }
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
}

impl<T: DeserializeOwned> ResourceResponse<T> {
    /// Deserializes the response body into `T`.
    ///
    /// # Errors
    ///
    /// Returns an error if the response body cannot be deserialized as `T`.
    pub fn into_model(self) -> crate::Result<T> {
        self.response.into_model::<T>()
    }
}
