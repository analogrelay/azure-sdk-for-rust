// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Response body shape for [`CosmosResponse`].
//!
//! [`CosmosResponse`]: crate::models::CosmosResponse

/// The body of a Cosmos DB response.
///
/// Mirrors [`OperationPayload`](crate::models::OperationPayload) on the
/// request side: each variant carries exactly the data shape produced
/// by its kind of operation. The driver does not deserialize item content.
///
/// New variants will be added as additional operation kinds (e.g. feed
/// operations that aggregate items across partitions) are introduced.
#[derive(Clone, Debug, Default)]
pub enum ResponseBody {
    /// No body (e.g. 204 No Content).
    #[default]
    None,

    /// Raw response body bytes — the driver passes the server response
    /// through verbatim. Used for any operation where the driver does not
    /// need to parse the body.
    Bytes(Vec<u8>),
}

impl ResponseBody {
    /// Returns the body bytes if this response carries any.
    ///
    /// Returns an empty slice for [`ResponseBody::None`].
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            ResponseBody::None => &[],
            ResponseBody::Bytes(body) => body,
        }
    }

    /// Consumes the response body and returns owned bytes.
    ///
    /// Returns an empty `Vec` for [`ResponseBody::None`].
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            ResponseBody::None => Vec::new(),
            ResponseBody::Bytes(body) => body,
        }
    }
}
