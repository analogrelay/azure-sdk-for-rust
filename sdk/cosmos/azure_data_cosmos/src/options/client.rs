// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Options used when building a [`CosmosClient`](crate::CosmosClient).

use azure_data_cosmos_driver::options::{OperationOptions, UserAgentSuffix};

/// Options applied when creating a [`CosmosClient`](crate::CosmosClient).
///
/// Most apps set these values through [`CosmosClient::builder`](crate::CosmosClient::builder).
#[derive(Clone, Default, Debug)]
#[non_exhaustive]
pub struct CosmosClientOptions {
    /// Default [`OperationOptions`] applied to requests from this client unless a
    /// per-request options type overrides them.
    pub operation: OperationOptions,
    pub(crate) user_agent_suffix: Option<UserAgentSuffix>,
}

impl CosmosClientOptions {
    /// Adds a custom suffix to the `User-Agent` sent with this client's requests.
    pub fn with_user_agent_suffix(mut self, suffix: UserAgentSuffix) -> Self {
        self.user_agent_suffix = Some(suffix);
        self
    }

    /// Sets the default [`OperationOptions`] for requests from this client.
    pub fn with_operation_options(mut self, operation: OperationOptions) -> Self {
        self.operation = operation;
        self
    }
}
