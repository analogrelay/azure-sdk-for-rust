// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Account reference types for Azure Cosmos DB.

use crate::{AccountEndpoint, CosmosCredential};

#[cfg(feature = "key_auth")]
use azure_core::credentials::Secret;
use azure_core::credentials::TokenCredential;
use std::sync::Arc;

/// A Cosmos DB account endpoint paired with credentials.
///
/// Use [`AccountReference::with_credential`] for Microsoft Entra ID or
/// [`AccountReference::with_authentication_key`] for key-based authentication.
///
/// # Examples
///
/// Using Microsoft Entra ID authentication:
///
/// ```rust,no_run
/// use azure_data_cosmos::{AccountReference, AccountEndpoint};
/// use std::sync::Arc;
///
/// let credential: Arc<dyn azure_core::credentials::TokenCredential> =
///     azure_identity::DeveloperToolsCredential::new(None).unwrap();
/// let endpoint: AccountEndpoint = "https://myaccount.documents.azure.com/".parse().unwrap();
/// let account = AccountReference::with_credential(endpoint, credential);
/// ```
///
/// Using key authentication (requires `key_auth` feature):
///
/// ```rust,ignore
/// use azure_data_cosmos::{AccountReference, AccountEndpoint};
/// use azure_core::credentials::Secret;
///
/// let endpoint: AccountEndpoint = "https://myaccount.documents.azure.com/".parse().unwrap();
/// let account = AccountReference::with_authentication_key(endpoint, Secret::from("my_account_key"));
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct AccountReference {
    endpoint: AccountEndpoint,
    credential: CosmosCredential,
}

impl AccountReference {
    /// Creates an account reference that uses a Microsoft Entra ID token credential.
    pub fn with_credential(
        endpoint: AccountEndpoint,
        credential: Arc<dyn TokenCredential>,
    ) -> Self {
        Self {
            endpoint,
            credential: CosmosCredential::from(credential),
        }
    }

    /// Creates an account reference that uses a Cosmos DB account key.
    #[cfg(feature = "key_auth")]
    pub fn with_authentication_key(endpoint: AccountEndpoint, key: impl Into<Secret>) -> Self {
        Self {
            endpoint,
            credential: CosmosCredential::from(key.into()),
        }
    }

    /// Returns the endpoint and credential as a tuple.
    ///
    /// This is used internally by the builder to extract the components.
    pub(crate) fn into_parts(self) -> (AccountEndpoint, CosmosCredential) {
        (self.endpoint, self.credential)
    }
}
