// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Container session state management for Cosmos DB operations.

use crate::session::{Error, PartitionSessionToken};
use crate::PartitionKeyRangeId;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::RwLock;

/// Represents the current session for a specific Cosmos DB Container
///
/// This type maintains a mapping from partition key range IDs to their corresponding
/// local session tokens, allowing for proper session consistency tracking across
/// all partitions in a container.
#[derive(Debug)]
pub struct ContainerSession {
    /// Session tokens indexed by partition key range ID.
    partition_tokens: RwLock<HashMap<PartitionKeyRangeId, PartitionSessionToken>>,
}

impl ContainerSession {
    /// Creates a new empty container session.
    pub fn new() -> Self {
        Self {
            partition_tokens: RwLock::new(HashMap::new()),
        }
    }

    /// Updates the internal HashMap entries for each partition mentioned in the Container Session Token.
    ///
    /// A Container Session Token is a comma-separated list of Partition Session Tokens.
    /// For example: `"42:1#123#4=500,43:1#124#4=501"`
    pub fn set_session_token(&self, token: &str) -> Result<(), Error> {
        if token.is_empty() {
            return Err(Error::EmptyInput);
        }

        let mut partition_tokens = self.partition_tokens.write().unwrap();

        // Parse comma-separated partition session tokens
        for token_str in token.split(',') {
            let token_str = token_str.trim();
            if token_str.is_empty() {
                continue;
            }

            let partition_token = PartitionSessionToken::from_str(token_str)?;
            partition_tokens.insert(partition_token.pkrange_id.clone(), partition_token);
        }

        Ok(())
    }

    /// Serializes the current set of tokens into a single Container Session Token string.
    ///
    /// Returns `None` if there are no partition tokens.
    /// The format is a comma-separated list of Partition Session Tokens.
    pub fn get_session_token(&self) -> Option<String> {
        let partition_tokens = self.partition_tokens.read().unwrap();

        if partition_tokens.is_empty() {
            return None;
        }

        let mut tokens: Vec<String> = partition_tokens
            .values()
            .map(|token| token.to_string())
            .collect();

        // Sort for consistent output
        tokens.sort();

        Some(tokens.join(","))
    }

    /// Retrieves the session token for the given partition.
    ///
    /// Returns `None` if no such partition exists.
    pub fn get_partition_session_token(&self, pk_range_id: &PartitionKeyRangeId) -> Option<String> {
        let partition_tokens = self.partition_tokens.read().unwrap();
        partition_tokens
            .get(pk_range_id)
            .map(|token| token.to_string())
    }

    /// Clears the internal hashmap, removing all existing session tokens.
    pub fn clear_session(&self) {
        let mut partition_tokens = self.partition_tokens.write().unwrap();
        partition_tokens.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_container_session_is_empty() {
        let session = ContainerSession::new();
        assert!(session.get_session_token().is_none());
    }

    #[test]
    fn set_single_partition_token() {
        let session = ContainerSession::new();
        session.set_session_token("42:1#123#4=500").unwrap();

        let token = session.get_session_token().unwrap();
        assert_eq!(token, "42:1#123#4=500");
    }

    #[test]
    fn set_multiple_partition_tokens() {
        let session = ContainerSession::new();
        session
            .set_session_token("42:1#123#4=500,43:1#124#4=501")
            .unwrap();

        let token = session.get_session_token().unwrap();
        // Should contain both tokens (order may vary due to sorting)
        assert!(token.contains("42:1#123#4=500"));
        assert!(token.contains("43:1#124#4=501"));
        assert!(token.contains(","));
    }

    #[test]
    fn get_partition_session_token() {
        let session = ContainerSession::new();
        session
            .set_session_token("42:1#123#4=500,43:1#124#4=501")
            .unwrap();

        let pk_range_id = PartitionKeyRangeId::new("42".to_string());
        let token = session.get_partition_session_token(&pk_range_id).unwrap();
        assert_eq!(token, "42:1#123#4=500");

        let missing_id = PartitionKeyRangeId::new("99".to_string());
        assert!(session.get_partition_session_token(&missing_id).is_none());
    }

    #[test]
    fn clear_session() {
        let session = ContainerSession::new();
        session.set_session_token("42:1#123#4=500").unwrap();
        assert!(session.get_session_token().is_some());

        session.clear_session();
        assert!(session.get_session_token().is_none());
    }

    #[test]
    fn set_empty_token_fails() {
        let session = ContainerSession::new();
        let result = session.set_session_token("");
        assert_eq!(result.unwrap_err(), Error::EmptyInput);
    }

    #[test]
    fn set_invalid_partition_token_fails() {
        let session = ContainerSession::new();
        let result = session.set_session_token("invalid_token");
        assert!(result.is_err());
    }

    #[test]
    fn partition_tokens_are_replaced_on_update() {
        let session = ContainerSession::new();

        // Set initial token
        session.set_session_token("42:1#123#4=500").unwrap();
        let pk_range_id = PartitionKeyRangeId::new("42".to_string());
        assert_eq!(
            session.get_partition_session_token(&pk_range_id).unwrap(),
            "42:1#123#4=500"
        );

        // Update with new token for same partition
        session.set_session_token("42:2#456#4=600").unwrap();
        assert_eq!(
            session.get_partition_session_token(&pk_range_id).unwrap(),
            "42:2#456#4=600"
        );
    }
}
