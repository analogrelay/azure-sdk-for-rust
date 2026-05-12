// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Account endpoint types for Azure Cosmos DB.

use azure_core::http::Url;

/// The endpoint URL for a Cosmos DB account.
///
/// This is a newtype wrapper around [`Url`] that provides a strongly-typed representation
/// of a Cosmos DB account endpoint, such as `https://myaccount.documents.azure.com/`.
///
/// Cosmos DB endpoints must use the `https` scheme. Constructing a
/// `CosmosAccountEndpoint` from a non-HTTPS URL fails so that credentials
/// cannot accidentally be sent over an unencrypted channel.
///
/// # Examples
///
/// Parsing from a string:
///
/// ```rust
/// use azure_data_cosmos::CosmosAccountEndpoint;
///
/// let endpoint: CosmosAccountEndpoint = "https://myaccount.documents.azure.com/".parse().unwrap();
/// ```
///
/// Converting from a [`Url`](azure_core::http::Url):
///
/// ```rust
/// use azure_data_cosmos::CosmosAccountEndpoint;
/// use azure_core::http::Url;
///
/// let url: Url = "https://myaccount.documents.azure.com/".parse().unwrap();
/// let endpoint = CosmosAccountEndpoint::try_from(url).unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CosmosAccountEndpoint(Url);

impl CosmosAccountEndpoint {
    /// Returns a reference to the underlying [`Url`].
    pub fn url(&self) -> &Url {
        &self.0
    }

    /// Consumes this endpoint and returns the underlying [`Url`].
    pub fn into_url(self) -> Url {
        self.0
    }

    fn https_only(url: Url) -> Result<Self, azure_core::Error> {
        if !url.scheme().eq_ignore_ascii_case("https") {
            return Err(azure_core::Error::with_message(
                azure_core::error::ErrorKind::Other,
                format!(
                    "Cosmos DB account endpoints must use the 'https' scheme; got '{}' for '{}'",
                    url.scheme(),
                    url
                ),
            ));
        }
        Ok(Self(url))
    }
}

impl std::str::FromStr for CosmosAccountEndpoint {
    type Err = azure_core::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url: Url = s.parse().map_err(|e: url::ParseError| {
            azure_core::Error::new(azure_core::error::ErrorKind::Other, e)
        })?;
        Self::https_only(url)
    }
}

impl TryFrom<Url> for CosmosAccountEndpoint {
    type Error = azure_core::Error;

    fn try_from(url: Url) -> Result<Self, Self::Error> {
        Self::https_only(url)
    }
}

impl std::fmt::Display for CosmosAccountEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::CosmosAccountEndpoint;
    use azure_core::http::Url;

    #[test]
    fn from_str_accepts_https() {
        let endpoint: CosmosAccountEndpoint =
            "https://myaccount.documents.azure.com/".parse().unwrap();
        assert_eq!(
            endpoint.url().as_str(),
            "https://myaccount.documents.azure.com/"
        );
    }

    #[test]
    fn from_str_accepts_https_localhost() {
        let endpoint: CosmosAccountEndpoint = "https://localhost:8081/".parse().unwrap();
        assert_eq!(endpoint.url().host_str(), Some("localhost"));
    }

    #[test]
    fn from_str_rejects_http() {
        let err = "http://myaccount.documents.azure.com/"
            .parse::<CosmosAccountEndpoint>()
            .unwrap_err();
        assert!(err.to_string().contains("https"), "got: {err}");
    }

    #[test]
    fn from_str_rejects_http_localhost() {
        let err = "http://localhost:8081/"
            .parse::<CosmosAccountEndpoint>()
            .unwrap_err();
        assert!(err.to_string().contains("https"), "got: {err}");
    }

    #[test]
    fn try_from_url_rejects_http() {
        let url = Url::parse("http://myaccount.documents.azure.com/").unwrap();
        let err = CosmosAccountEndpoint::try_from(url).unwrap_err();
        assert!(err.to_string().contains("https"), "got: {err}");
    }

    #[test]
    fn try_from_url_rejects_non_http_scheme() {
        let url = Url::parse("file:///etc/passwd").unwrap();
        let err = CosmosAccountEndpoint::try_from(url).unwrap_err();
        assert!(err.to_string().contains("https"), "got: {err}");
    }

    #[test]
    fn try_from_url_accepts_https() {
        let url = Url::parse("https://myaccount.documents.azure.com/").unwrap();
        let endpoint = CosmosAccountEndpoint::try_from(url).unwrap();
        assert_eq!(
            endpoint.url().as_str(),
            "https://myaccount.documents.azure.com/"
        );
    }
}
