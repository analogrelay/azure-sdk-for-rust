// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Container cache for Cosmos DB driver.

use super::AsyncCache;
use crate::models::{ContainerProperties, ContainerReference};
use std::sync::Arc;

/// Cache for Cosmos DB container metadata.
///
/// Stores container properties (partition key definition, indexing policy)
/// keyed by container reference. Uses single-pending-I/O semantics -
/// concurrent requests for the same container share one initialization future.
#[derive(Debug)]
pub(crate) struct ContainerCache {
    cache: AsyncCache<ContainerReference, ContainerProperties>,
}

impl ContainerCache {
    /// Creates a new empty container cache.
    pub(crate) fn new() -> Self {
        Self {
            cache: AsyncCache::new(),
        }
    }

    /// Gets container properties, fetching them if not cached.
    ///
    /// If the container is not in the cache, calls `fetch_fn` to retrieve
    /// the properties. Concurrent requests for the same container share
    /// the same fetch operation.
    pub(crate) async fn get_or_fetch<F, Fut>(
        &self,
        container: ContainerReference,
        fetch_fn: F,
    ) -> Arc<ContainerProperties>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ContainerProperties>,
    {
        self.cache.get_or_insert_with(container, fetch_fn).await
    }

    /// Gets cached container properties if available.
    ///
    /// Returns `None` if the container is not in the cache.
    pub(crate) async fn get(
        &self,
        container: &ContainerReference,
    ) -> Option<Arc<ContainerProperties>> {
        self.cache.get(container).await
    }

    /// Invalidates the cached properties for a container.
    ///
    /// Returns the previously cached value if it existed.
    pub(crate) async fn invalidate(
        &self,
        container: &ContainerReference,
    ) -> Option<Arc<ContainerProperties>> {
        self.cache.invalidate(container).await
    }

    /// Clears all cached container metadata.
    pub(crate) async fn clear(&self) {
        self.cache.clear().await;
    }
}

impl Default for ContainerCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        AccountReference, ContainerReference, PartitionKeyDefinition, SystemProperties,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use url::Url;

    fn test_account() -> AccountReference {
        AccountReference::new(Url::parse("https://myaccount.documents.azure.com:443/").unwrap())
    }

    fn test_container(db: &str, container: &str) -> ContainerReference {
        ContainerReference::from_name(test_account(), db.to_owned(), container.to_owned())
    }

    fn test_properties(id: &str) -> ContainerProperties {
        ContainerProperties {
            id: id.to_owned().into(),
            partition_key: PartitionKeyDefinition {
                paths: vec!["/pk".into()],
                ..Default::default()
            },
            indexing_policy: None,
            system_properties: SystemProperties::default(),
        }
    }

    #[tokio::test]
    async fn caches_container_properties() {
        let cache = ContainerCache::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let container = test_container("mydb", "mycoll");

        let counter_clone = counter.clone();
        let props = cache
            .get_or_fetch(container.clone(), || async move {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                test_properties("mycoll")
            })
            .await;

        assert_eq!(props.id.as_ref(), "mycoll");
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Second access uses cached value
        let counter_clone = counter.clone();
        let props2 = cache
            .get_or_fetch(container, || async move {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                test_properties("othercoll")
            })
            .await;

        assert_eq!(props2.id.as_ref(), "mycoll");
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn different_containers_cached_separately() {
        let cache = ContainerCache::new();

        let props1 = cache
            .get_or_fetch(test_container("db1", "coll1"), || async {
                test_properties("coll1")
            })
            .await;

        let props2 = cache
            .get_or_fetch(test_container("db1", "coll2"), || async {
                test_properties("coll2")
            })
            .await;

        assert_eq!(props1.id.as_ref(), "coll1");
        assert_eq!(props2.id.as_ref(), "coll2");
    }

    #[tokio::test]
    async fn same_container_different_databases() {
        let cache = ContainerCache::new();

        let props1 = cache
            .get_or_fetch(test_container("db1", "coll"), || async {
                test_properties("db1-coll")
            })
            .await;

        let props2 = cache
            .get_or_fetch(test_container("db2", "coll"), || async {
                test_properties("db2-coll")
            })
            .await;

        assert_eq!(props1.id.as_ref(), "db1-coll");
        assert_eq!(props2.id.as_ref(), "db2-coll");
    }

    #[tokio::test]
    async fn get_returns_none_before_fetch() {
        let cache = ContainerCache::new();
        assert!(cache.get(&test_container("db", "unknown")).await.is_none());
    }

    #[tokio::test]
    async fn invalidate_removes_entry() {
        let cache = ContainerCache::new();
        let container = test_container("mydb", "mycoll");

        cache
            .get_or_fetch(container.clone(), || async { test_properties("mycoll") })
            .await;

        let removed = cache.invalidate(&container).await;
        assert!(removed.is_some());
        assert!(cache.get(&container).await.is_none());
    }

    #[tokio::test]
    async fn clear_removes_all() {
        let cache = ContainerCache::new();

        cache
            .get_or_fetch(test_container("db", "coll1"), || async {
                test_properties("coll1")
            })
            .await;
        cache
            .get_or_fetch(test_container("db", "coll2"), || async {
                test_properties("coll2")
            })
            .await;

        cache.clear().await;

        assert!(cache.get(&test_container("db", "coll1")).await.is_none());
        assert!(cache.get(&test_container("db", "coll2")).await.is_none());
    }
}
