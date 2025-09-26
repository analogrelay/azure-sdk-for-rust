// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Session error types for Cosmos DB operations.

use std::fmt;

/// Errors that can occur when working with session tokens.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// The input string is empty.
    EmptyInput,
    /// The input string does not contain the required minimum components.
    MissingComponents,
    /// The version component could not be parsed as a u64.
    InvalidVersion(String),
    /// The global LSN component could not be parsed as a u64.
    InvalidGlobalLsn(String),
    /// A region ID component could not be parsed as a u32.
    InvalidRegionId(String),
    /// A region LSN component could not be parsed as a u64.
    InvalidRegionLsn(String),
    /// A regional component is missing the required '=' separator.
    MalformedRegionalComponent(String),
    /// Invalid regions in session token comparison.
    InvalidRegions { current: String, other: String },
    /// Session tokens cannot be merged due to incompatible regions.
    TokensCannotBeMerged(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::EmptyInput => write!(f, "input string is empty"),
            Error::MissingComponents => {
                write!(f, "missing required components (version and global LSN)")
            }
            Error::InvalidVersion(s) => write!(f, "invalid version: '{}'", s),
            Error::InvalidGlobalLsn(s) => write!(f, "invalid global LSN: '{}'", s),
            Error::InvalidRegionId(s) => write!(f, "invalid region ID: '{}'", s),
            Error::InvalidRegionLsn(s) => write!(f, "invalid region LSN: '{}'", s),
            Error::MalformedRegionalComponent(s) => {
                write!(f, "malformed regional component: '{}'", s)
            }
            Error::InvalidRegions { current, other } => {
                write!(
                    f,
                    "invalid regions in session token comparison: current='{}', other='{}'",
                    current, other
                )
            }
            Error::TokensCannotBeMerged(s) => {
                write!(f, "incompatible tokens: {}", s)
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<Error> for azure_core::Error {
    fn from(error: Error) -> Self {
        azure_core::Error::full(
            azure_core::error::ErrorKind::Other,
            error,
            "session token invalid",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_converts_to_azure_core_error() {
        let session_error = Error::EmptyInput;
        let core_error: azure_core::Error = session_error.into();

        // Just verify the conversion works without panicking
        println!("Azure core error: {}", core_error);
        // The conversion itself is what we're testing - it should not panic
    }
}
