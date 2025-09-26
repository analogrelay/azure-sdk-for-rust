// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Local session token implementation for Cosmos DB operations.

use super::Error;
use crate::{session::VectorSessionToken, PartitionKeyRangeId};
use std::fmt;
use std::str::FromStr;

/// A partition-local session token that combines a partition key range ID with a vector session token.
///
/// The string format is: `{pkrange_id}:{vector_session_token}`
/// For example: `42:1#123#4=500#5=600`
#[derive(Debug, Clone, PartialEq)]
pub struct PartitionSessionToken {
    /// The partition key range ID this token applies to.
    pub pkrange_id: PartitionKeyRangeId,

    /// The vector session token containing version, global LSN, and regional LSNs.
    pub vector_token: VectorSessionToken,
}

impl FromStr for PartitionSessionToken {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(Error::EmptyInput);
        }

        // Find the first ':' separator
        let colon_pos = s.find(':').ok_or(Error::MissingComponents)?;

        if colon_pos == 0 {
            return Err(Error::MissingComponents);
        }

        let pkrange_part = &s[..colon_pos];
        let vector_part = &s[colon_pos + 1..];

        if vector_part.is_empty() {
            return Err(Error::MissingComponents);
        }

        let pkrange_id = PartitionKeyRangeId::new(pkrange_part.to_string());
        let vector_token = VectorSessionToken::from_str(vector_part)?;

        Ok(PartitionSessionToken {
            pkrange_id,
            vector_token,
        })
    }
}

impl fmt::Display for PartitionSessionToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.pkrange_id.value(), self.vector_token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_local_token_with_regions() {
        let token: PartitionSessionToken = "42:1#123#4=500#5=600".parse().unwrap();

        assert_eq!(token.pkrange_id.value(), "42");
        assert_eq!(token.vector_token.version, 1);
        assert_eq!(token.vector_token.global_lsn.value(), 123);
        assert_eq!(token.vector_token.regional_lsns.len(), 2);
    }

    #[test]
    fn parse_local_token_minimal() {
        let token: PartitionSessionToken = "0:2#456".parse().unwrap();

        assert_eq!(token.pkrange_id.value(), "0");
        assert_eq!(token.vector_token.version, 2);
        assert_eq!(token.vector_token.global_lsn.value(), 456);
        assert!(token.vector_token.regional_lsns.is_empty());
    }

    #[test]
    fn parse_empty_string_fails() {
        let result: Result<PartitionSessionToken, _> = "".parse();
        assert_eq!(result.unwrap_err(), Error::EmptyInput);
    }

    #[test]
    fn parse_missing_colon_fails() {
        let result: Result<PartitionSessionToken, _> = "42#1#123".parse();
        assert_eq!(result.unwrap_err(), Error::MissingComponents);
    }

    #[test]
    fn parse_empty_pkrange_fails() {
        let result: Result<PartitionSessionToken, _> = ":1#123".parse();
        assert_eq!(result.unwrap_err(), Error::MissingComponents);
    }

    #[test]
    fn parse_empty_vector_part_fails() {
        let result: Result<PartitionSessionToken, _> = "42:".parse();
        assert_eq!(result.unwrap_err(), Error::MissingComponents);
    }

    #[test]
    fn parse_invalid_vector_token_fails() {
        let result: Result<PartitionSessionToken, _> = "42:invalid".parse();
        assert_eq!(result.unwrap_err(), Error::MissingComponents);
    }

    #[test]
    fn display_local_token() {
        let token: PartitionSessionToken = "42:1#123#4=500#5=600".parse().unwrap();
        let displayed = token.to_string();

        // Should maintain the same format (regions may be reordered due to HashMap)
        assert!(displayed.starts_with("42:1#123#"));
        assert!(displayed.contains("4=500"));
        assert!(displayed.contains("5=600"));
    }

    #[test]
    fn roundtrip_parsing() {
        let original = "test-range:2#789#100=1000#200=2000";
        let token: PartitionSessionToken = original.parse().unwrap();
        let reparsed: PartitionSessionToken = token.to_string().parse().unwrap();

        assert_eq!(token, reparsed);
    }
}
