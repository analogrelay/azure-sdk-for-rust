// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! [`ResponseBody`] for Azure Cosmos DB operation responses.

use azure_core::{fmt::SafeDebug, Bytes};
use azure_data_cosmos_driver::models::ResponseBody as DriverResponseBody;
use serde::de::DeserializeOwned;

/// The body of a Cosmos DB operation response.
///
/// Returned by [`ItemResponse::into_body`](crate::models::ItemResponse::into_body),
/// [`ResourceResponse::into_body`](crate::models::ResourceResponse::into_body), and
/// [`BatchResponse::into_body`](crate::models::BatchResponse::into_body). The
/// body can hold either a single payload or multiple item payloads.
#[derive(Clone, Default, SafeDebug)]
#[non_exhaustive]
pub struct ResponseBody(DriverResponseBody);

impl ResponseBody {
    /// Returns `true` if the body carries no readable content.
    ///
    /// True for the no-payload response shape, for a single-payload body of
    /// zero bytes, and for a feed envelope with zero items.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the single payload.
    ///
    /// # Errors
    ///
    /// Returns an error if the body contains multiple item payloads instead of
    /// a single payload.
    pub fn single(self) -> crate::Result<Bytes> {
        self.0.single().map_err(Into::into)
    }

    /// Returns the payloads as raw item buffers.
    ///
    /// A single payload is returned as a one-element `Vec`, and a no-payload
    /// response returns an empty `Vec`.
    pub fn items(self) -> crate::Result<Vec<Bytes>> {
        self.0.items().map_err(Into::into)
    }

    /// Deserializes a single payload as JSON of type `T`.
    ///
    /// # Errors
    ///
    /// Returns an error if the body does not contain exactly one payload or if
    /// that payload cannot be deserialized as `T`.
    pub fn into_single<T: DeserializeOwned>(self) -> crate::Result<T> {
        self.0.into_single().map_err(Into::into)
    }

    /// Deserializes every payload as JSON of type `T`.
    ///
    /// # Errors
    ///
    /// Returns an error if any payload cannot be deserialized as `T`.
    pub fn into_items<T: DeserializeOwned>(self) -> crate::Result<Vec<T>> {
        self.0.into_items().map_err(Into::into)
    }
}

impl From<DriverResponseBody> for ResponseBody {
    fn from(inner: DriverResponseBody) -> Self {
        Self(inner)
    }
}
