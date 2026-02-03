// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Account reference and authentication types.

use azure_core::credentials::{Secret, TokenCredential};
use std::sync::Arc;
use url::Url;

/// A master key for authenticating with a Cosmos DB account.
///
/// Wraps the account's primary or secondary key as a secret.
#[derive(Clone, Debug)]
pub struct MasterKey(Secret);

impl MasterKey {
    /// Creates a new master key from the provided key string.
    pub fn new(key: impl Into<Secret>) -> Self {
        Self(key.into())
    }

    /// Returns the secret key value.
    pub fn secret(&self) -> &str {
        self.0.secret()
    }
}

impl From<&'static str> for MasterKey {
    fn from(key: &'static str) -> Self {
        Self::new(key)
    }
}

impl From<String> for MasterKey {
    fn from(key: String) -> Self {
        Self::new(key)
    }
}

impl From<Secret> for MasterKey {
    fn from(secret: Secret) -> Self {
        Self(secret)
    }
}

/// Authentication options for connecting to a Cosmos DB account.
///
/// Either key-based authentication using a master key, or token-based
/// authentication using an Azure credential (e.g., managed identity, service principal).
#[derive(Clone)]
pub enum AuthOptions {
    /// Key-based authentication using the account's primary or secondary master key.
    MasterKey(MasterKey),
    /// Token-based authentication using an Azure credential.
    TokenCredential(Arc<dyn TokenCredential>),
}

impl std::fmt::Debug for AuthOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MasterKey(key) => f.debug_tuple("MasterKey").field(key).finish(),
            Self::TokenCredential(_) => f.debug_tuple("TokenCredential").field(&"...").finish(),
        }
    }
}

impl From<MasterKey> for AuthOptions {
    fn from(key: MasterKey) -> Self {
        Self::MasterKey(key)
    }
}

impl From<Arc<dyn TokenCredential>> for AuthOptions {
    fn from(credential: Arc<dyn TokenCredential>) -> Self {
        Self::TokenCredential(credential)
    }
}

/// A reference to a Cosmos DB account.
///
/// Contains the service endpoint (required) and optional authentication credentials.
/// This is the root reference from which database and container references are derived.
///
/// # Examples
///
/// ```
/// use azure_data_cosmos_driver::models::{AccountReference, MasterKey};
/// use url::Url;
///
/// // With master key authentication
/// let account = AccountReference::new(
///     Url::parse("https://myaccount.documents.azure.com:443/").unwrap(),
/// ).with_master_key("my-master-key");
///
/// // Without authentication (for operations that don't require it)
/// let account = AccountReference::new(
///     Url::parse("https://myaccount.documents.azure.com:443/").unwrap(),
/// );
/// ```
#[derive(Clone, Debug)]
pub struct AccountReference {
    /// The service endpoint URL (required).
    endpoint: Url,
    /// Optional authentication credentials.
    auth: Option<AuthOptions>,
}

// Manual PartialEq implementation because AuthOptions contains Arc<dyn TokenCredential>
// which doesn't implement PartialEq. We compare by endpoint only.
impl PartialEq for AccountReference {
    fn eq(&self, other: &Self) -> bool {
        self.endpoint == other.endpoint
    }
}

impl AccountReference {
    /// Creates a new account reference with the specified endpoint.
    ///
    /// Authentication can be added using `with_master_key` or `with_credential`.
    pub fn new(endpoint: Url) -> Self {
        Self {
            endpoint,
            auth: None,
        }
    }

    /// Returns the service endpoint URL.
    pub fn endpoint(&self) -> &Url {
        &self.endpoint
    }

    /// Returns the authentication options, if configured.
    pub fn auth(&self) -> Option<&AuthOptions> {
        self.auth.as_ref()
    }

    /// Sets master key authentication.
    #[must_use]
    pub fn with_master_key(mut self, key: impl Into<MasterKey>) -> Self {
        self.auth = Some(AuthOptions::MasterKey(key.into()));
        self
    }

    /// Sets token credential authentication.
    #[must_use]
    pub fn with_credential(mut self, credential: Arc<dyn TokenCredential>) -> Self {
        self.auth = Some(AuthOptions::TokenCredential(credential));
        self
    }

    /// Sets authentication options directly.
    #[must_use]
    pub fn with_auth(mut self, auth: AuthOptions) -> Self {
        self.auth = Some(auth);
        self
    }
}
