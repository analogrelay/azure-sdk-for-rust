// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Top-level session management for Cosmos DB operations.

use crate::session::{ContainerSession, Error};
use crate::{PartitionKeyRangeId, ResourceId};
use std::collections::HashMap;
use std::sync::RwLock;

/// Represents the session state for all Cosmos DB containers in a client.
///
/// This type maintains a mapping from resource IDs (containers) to their corresponding
/// `ContainerSession` instances, allowing for proper session consistency tracking across
/// all containers in the client.
#[derive(Debug)]
pub struct Session {
    /// Container sessions indexed by resource ID.
    container_sessions: RwLock<HashMap<ResourceId, ContainerSession>>,
}

impl Session {
    /// Creates a new empty session.
    pub fn new() -> Self {
        Self {
            container_sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Updates the session token for the specified container.
    ///
    /// A Container Session Token is a comma-separated list of Partition Session Tokens.
    /// For example: `"42:1#123#4=500,43:1#124#4=501"`
    pub fn set_session_token(&self, container: &ResourceId, token: &str) -> Result<(), Error> {
        {
            let container_sessions = self.container_sessions.read().unwrap();
            if let Some(container_session) = container_sessions.get(container) {
                return container_session.set_session_token(token);
            }
        }

        let mut container_sessions = self.container_sessions.write().unwrap();

        // Validate token before creating container entry to avoid orphaned containers on error
        use std::collections::hash_map::Entry;
        match container_sessions.entry(container.clone()) {
            Entry::Occupied(entry) => entry.get().set_session_token(token),
            Entry::Vacant(entry) => {
                let container_session = ContainerSession::new();
                container_session.set_session_token(token)?;
                entry.insert(container_session);
                Ok(())
            }
        }
    }

    /// Retrieves the session token for the specified container.
    ///
    /// Returns `None` if the container has no session tokens.
    pub fn get_session_token(&self, container: &ResourceId) -> Option<String> {
        let container_sessions = self.container_sessions.read().unwrap();
        container_sessions
            .get(container)
            .and_then(|session| session.get_session_token())
    }

    /// Retrieves the session token for the specified container and partition.
    ///
    /// Returns `None` if the container or partition doesn't exist.
    pub fn get_partition_session_token(
        &self,
        container: &ResourceId,
        pk_range_id: &PartitionKeyRangeId,
    ) -> Option<String> {
        let container_sessions = self.container_sessions.read().unwrap();
        container_sessions
            .get(container)
            .and_then(|session| session.get_partition_session_token(pk_range_id))
    }

    /// Clears all session tokens for the specified container.
    ///
    /// This method does nothing if the container doesn't exist.
    pub fn clear_session(&self, container: &ResourceId) {
        let container_sessions = self.container_sessions.read().unwrap();
        if let Some(container_session) = container_sessions.get(container) {
            container_session.clear_session();
        }
    }

    /// Clears all session tokens for all containers.
    pub fn clear_all_sessions(&self) {
        let mut container_sessions = self.container_sessions.write().unwrap();
        container_sessions.clear();
    }

    /// Returns the number of containers being tracked.
    pub fn container_count(&self) -> usize {
        let container_sessions = self.container_sessions.read().unwrap();
        container_sessions.len()
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_is_empty() {
        let session = Session::new();
        let container = ResourceId::new("container1".to_string());
        assert!(session.get_session_token(&container).is_none());
        assert_eq!(session.container_count(), 0);
    }

    #[test]
    fn set_and_get_session_token() {
        let session = Session::new();
        let container = ResourceId::new("container1".to_string());

        session
            .set_session_token(&container, "42:1#123#4=500")
            .unwrap();

        let token = session.get_session_token(&container).unwrap();
        assert_eq!(token, "42:1#123#4=500");
        assert_eq!(session.container_count(), 1);
    }

    #[test]
    fn multiple_containers() {
        let session = Session::new();
        let container1 = ResourceId::new("container1".to_string());
        let container2 = ResourceId::new("container2".to_string());

        session
            .set_session_token(&container1, "42:1#123#4=500")
            .unwrap();
        session
            .set_session_token(&container2, "43:1#124#4=501")
            .unwrap();

        assert_eq!(
            session.get_session_token(&container1).unwrap(),
            "42:1#123#4=500"
        );
        assert_eq!(
            session.get_session_token(&container2).unwrap(),
            "43:1#124#4=501"
        );
        assert_eq!(session.container_count(), 2);
    }

    #[test]
    fn get_partition_session_token() {
        let session = Session::new();
        let container = ResourceId::new("container1".to_string());
        let pk_range_id = PartitionKeyRangeId::new("42".to_string());

        session
            .set_session_token(&container, "42:1#123#4=500,43:1#124#4=501")
            .unwrap();

        let token = session
            .get_partition_session_token(&container, &pk_range_id)
            .unwrap();
        assert_eq!(token, "42:1#123#4=500");

        // Test non-existent partition
        let missing_pk = PartitionKeyRangeId::new("99".to_string());
        assert!(session
            .get_partition_session_token(&container, &missing_pk)
            .is_none());

        // Test non-existent container
        let missing_container = ResourceId::new("missing".to_string());
        assert!(session
            .get_partition_session_token(&missing_container, &pk_range_id)
            .is_none());
    }

    #[test]
    fn clear_session() {
        let session = Session::new();
        let container = ResourceId::new("container1".to_string());

        session
            .set_session_token(&container, "42:1#123#4=500")
            .unwrap();
        assert!(session.get_session_token(&container).is_some());

        session.clear_session(&container);
        assert!(session.get_session_token(&container).is_none());
        // Container still exists but has no tokens
        assert_eq!(session.container_count(), 1);
    }

    #[test]
    fn clear_all_sessions() {
        let session = Session::new();
        let container1 = ResourceId::new("container1".to_string());
        let container2 = ResourceId::new("container2".to_string());

        session
            .set_session_token(&container1, "42:1#123#4=500")
            .unwrap();
        session
            .set_session_token(&container2, "43:1#124#4=501")
            .unwrap();
        assert_eq!(session.container_count(), 2);

        session.clear_all_sessions();
        assert_eq!(session.container_count(), 0);
        assert!(session.get_session_token(&container1).is_none());
        assert!(session.get_session_token(&container2).is_none());
    }

    #[test]
    fn clear_nonexistent_container_does_nothing() {
        let session = Session::new();
        let container = ResourceId::new("nonexistent".to_string());

        // Should not panic or error
        session.clear_session(&container);
        assert_eq!(session.container_count(), 0);
    }

    #[test]
    fn set_empty_token_fails() {
        let session = Session::new();
        let container = ResourceId::new("container1".to_string());

        let result = session.set_session_token(&container, "");
        assert_eq!(result.unwrap_err(), Error::EmptyInput);
        assert_eq!(session.container_count(), 0);
    }

    #[test]
    fn default_session_is_empty() {
        let session = Session::default();
        assert_eq!(session.container_count(), 0);
    }

    #[test]
    fn concurrent_access_same_container() {
        use std::sync::Arc;
        use std::thread;

        let session = Arc::new(Session::new());
        let container = ResourceId::new("shared_container".to_string());

        // Set initial token
        session
            .set_session_token(&container, "42:1#123#4=500")
            .unwrap();

        let session_clone = Arc::clone(&session);
        let container_clone = container.clone();

        // Spawn a thread that tries to read the token
        let handle = thread::spawn(move || session_clone.get_session_token(&container_clone));

        // Read from main thread as well
        let main_result = session.get_session_token(&container);

        let thread_result = handle.join().unwrap();

        // Both should succeed
        assert!(main_result.is_some());
        assert!(thread_result.is_some());
        assert_eq!(main_result, thread_result);
    }
}
