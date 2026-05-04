// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Variant-specific request payload data for [`CosmosOperation`].
//!
//! [`CosmosOperation`]: crate::models::CosmosOperation

/// Operation-specific request payload data.
///
/// Replaces the generic `body: Option<Vec<u8>>` field on
/// [`CosmosOperation`](crate::models::CosmosOperation). Each variant carries
/// exactly the data needed for its kind of operation.
///
/// New variants will be added as additional operation kinds (Query,
/// ReadMany, ChangeFeed, ...) are introduced.
#[derive(Clone, Debug, Default)]
pub enum OperationPayload {
    /// No payload (e.g. ReadItem, DeleteItem, ReadContainer).
    #[default]
    None,

    /// Pre-serialized request body (e.g. CreateItem, UpsertItem, ReplaceItem).
    Body(Vec<u8>),
}

impl OperationPayload {
    /// Returns the body bytes if this payload carries any.
    pub fn as_body(&self) -> Option<&[u8]> {
        match self {
            OperationPayload::None => None,
            OperationPayload::Body(body) => Some(body),
        }
    }
}
