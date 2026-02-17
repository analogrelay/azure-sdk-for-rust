// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Generic resource reference type for Cosmos DB resources.

use crate::models::{
    AccountReference, ContainerReference, DatabaseReference, ItemReference, ResourceType,
};
use std::borrow::Cow;

/// A generic reference to any Cosmos DB resource.
///
/// Contains the resource type, optional parent references (account, database, container),
/// and either a name or resource identifier (RID) for the resource itself.
///
/// Use the factory methods to create references for specific resource types:
/// - [`CosmosResourceReference::account_resource`] - Account-level resources
/// - [`CosmosResourceReference::database_by_name`] / [`CosmosResourceReference::database_by_rid`] - Database resources
/// - [`CosmosResourceReference::document_collection_by_name`] / [`CosmosResourceReference::document_collection_by_rid`] - Container/collection resources
/// - [`CosmosResourceReference::document_by_name`] / [`CosmosResourceReference::document_by_rid`] - Document/item resources
/// - [`CosmosResourceReference::stored_procedure_by_name`] / [`CosmosResourceReference::stored_procedure_by_rid`] - Stored procedure resources
/// - [`CosmosResourceReference::trigger_by_name`] / [`CosmosResourceReference::trigger_by_rid`] - Trigger resources
/// - [`CosmosResourceReference::user_defined_function_by_name`] / [`CosmosResourceReference::user_defined_function_by_rid`] - UDF resources
/// - [`CosmosResourceReference::partition_key_range`] - Partition key range resources
/// - [`CosmosResourceReference::offer_by_rid`] - Offer resources
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CosmosResourceReference {
    /// The type of resource being referenced.
    resource_type: ResourceType,
    /// Reference to the parent account (always required).
    account: AccountReference,
    /// Reference to the parent database (optional, depends on resource type).
    database: Option<DatabaseReference>,
    /// Reference to the parent container (optional, depends on resource type).
    container: Option<ContainerReference>,
    /// The resource name (mutually exclusive with RID for identification).
    name: Option<Cow<'static, str>>,
    /// The resource identifier (RID) (mutually exclusive with name for identification).
    rid: Option<Cow<'static, str>>,
}

impl CosmosResourceReference {
    /// Returns the resource type.
    pub fn resource_type(&self) -> ResourceType {
        self.resource_type
    }

    /// Returns a reference to the account.
    pub fn account(&self) -> &AccountReference {
        &self.account
    }

    /// Returns a reference to the database, if applicable.
    pub fn database(&self) -> Option<&DatabaseReference> {
        self.database.as_ref()
    }

    /// Returns a reference to the container, if applicable.
    pub fn container(&self) -> Option<&ContainerReference> {
        self.container.as_ref()
    }

    /// Returns the resource name, if set.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the resource identifier (RID), if set.
    pub fn rid(&self) -> Option<&str> {
        self.rid.as_deref()
    }

    /// Sets the resource name.
    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the resource identifier (RID).
    pub fn with_rid(mut self, rid: impl Into<Cow<'static, str>>) -> Self {
        self.rid = Some(rid.into());
        self
    }

    // ===== Factory Methods =====

    /// Creates a reference to the database account.
    ///
    /// Account-level operations don't require database or container references.
    pub fn account_resource(account: AccountReference) -> Self {
        Self {
            resource_type: ResourceType::DatabaseAccount,
            account,
            database: None,
            container: None,
            name: None,
            rid: None,
        }
    }

    /// Creates a reference to a database by name.
    ///
    /// # Panics
    ///
    /// Panics if the `DatabaseReference` does not have a name set.
    pub fn database_by_name(database: DatabaseReference) -> Self {
        let name = database
            .name()
            .expect("DatabaseReference must have a name for database_by_name")
            .to_owned();
        let account = database.account().clone();
        Self {
            resource_type: ResourceType::Database,
            account,
            database: Some(database),
            container: None,
            name: Some(Cow::Owned(name)),
            rid: None,
        }
    }

    /// Creates a reference to a database by RID.
    ///
    /// # Panics
    ///
    /// Panics if the `DatabaseReference` does not have a RID set.
    pub fn database_by_rid(database: DatabaseReference) -> Self {
        let rid = database
            .rid()
            .expect("DatabaseReference must have a RID for database_by_rid")
            .to_owned();
        let account = database.account().clone();
        Self {
            resource_type: ResourceType::Database,
            account,
            database: Some(database),
            container: None,
            name: None,
            rid: Some(Cow::Owned(rid)),
        }
    }

    /// Creates a reference to a container (document collection) by name.
    pub fn document_collection_by_name(container: ContainerReference) -> Self {
        let name = container.name().to_owned();
        let account = container.account().clone();
        let database = Some(container.database().clone());
        Self {
            resource_type: ResourceType::DocumentCollection,
            account,
            database,
            container: Some(container),
            name: Some(Cow::Owned(name)),
            rid: None,
        }
    }

    /// Creates a name-based reference to a container within a database.
    ///
    /// Unlike [`document_collection_by_name`](Self::document_collection_by_name), this
    /// variant does not require an already-resolved `ContainerReference`. Use this
    /// for operations that read or resolve a container by name, such as the
    /// initial read that populates the container cache.
    pub fn document_collection_in_database(
        database: DatabaseReference,
        container_name: impl Into<Cow<'static, str>>,
    ) -> Self {
        let account = database.account().clone();
        Self {
            resource_type: ResourceType::DocumentCollection,
            account,
            database: Some(database),
            container: None,
            name: Some(container_name.into()),
            rid: None,
        }
    }

    /// Creates a reference to a container (document collection) by RID.
    pub fn document_collection_by_rid(container: ContainerReference) -> Self {
        let rid = container.rid().to_owned();
        let account = container.account().clone();
        let database = Some(container.database().clone());
        Self {
            resource_type: ResourceType::DocumentCollection,
            account,
            database,
            container: Some(container),
            name: None,
            rid: Some(Cow::Owned(rid)),
        }
    }

    /// Creates a reference to a document by name.
    pub fn document_by_name(
        container: ContainerReference,
        document_name: impl Into<Cow<'static, str>>,
    ) -> Self {
        let account = container.account().clone();
        let database = Some(container.database());
        Self {
            resource_type: ResourceType::Document,
            account,
            database,
            container: Some(container),
            name: Some(document_name.into()),
            rid: None,
        }
    }

    /// Creates a reference to a document by RID.
    pub fn document_by_rid(
        container: ContainerReference,
        document_rid: impl Into<Cow<'static, str>>,
    ) -> Self {
        let account = container.account().clone();
        let database = Some(container.database());
        Self {
            resource_type: ResourceType::Document,
            account,
            database,
            container: Some(container),
            name: None,
            rid: Some(document_rid.into()),
        }
    }

    /// Creates a reference to a stored procedure by name.
    pub fn stored_procedure_by_name(
        container: ContainerReference,
        sproc_name: impl Into<Cow<'static, str>>,
    ) -> Self {
        let account = container.account().clone();
        let database = Some(container.database());
        Self {
            resource_type: ResourceType::StoredProcedure,
            account,
            database,
            container: Some(container),
            name: Some(sproc_name.into()),
            rid: None,
        }
    }

    /// Creates a reference to a stored procedure by RID.
    pub fn stored_procedure_by_rid(
        container: ContainerReference,
        sproc_rid: impl Into<Cow<'static, str>>,
    ) -> Self {
        let account = container.account().clone();
        let database = Some(container.database());
        Self {
            resource_type: ResourceType::StoredProcedure,
            account,
            database,
            container: Some(container),
            name: None,
            rid: Some(sproc_rid.into()),
        }
    }

    /// Creates a reference to a trigger by name.
    pub fn trigger_by_name(
        container: ContainerReference,
        trigger_name: impl Into<Cow<'static, str>>,
    ) -> Self {
        let account = container.account().clone();
        let database = Some(container.database());
        Self {
            resource_type: ResourceType::Trigger,
            account,
            database,
            container: Some(container),
            name: Some(trigger_name.into()),
            rid: None,
        }
    }

    /// Creates a reference to a trigger by RID.
    pub fn trigger_by_rid(
        container: ContainerReference,
        trigger_rid: impl Into<Cow<'static, str>>,
    ) -> Self {
        let account = container.account().clone();
        let database = Some(container.database());
        Self {
            resource_type: ResourceType::Trigger,
            account,
            database,
            container: Some(container),
            name: None,
            rid: Some(trigger_rid.into()),
        }
    }

    /// Creates a reference to a user-defined function by name.
    pub fn user_defined_function_by_name(
        container: ContainerReference,
        udf_name: impl Into<Cow<'static, str>>,
    ) -> Self {
        let account = container.account().clone();
        let database = Some(container.database());
        Self {
            resource_type: ResourceType::UserDefinedFunction,
            account,
            database,
            container: Some(container),
            name: Some(udf_name.into()),
            rid: None,
        }
    }

    /// Creates a reference to a user-defined function by RID.
    pub fn user_defined_function_by_rid(
        container: ContainerReference,
        udf_rid: impl Into<Cow<'static, str>>,
    ) -> Self {
        let account = container.account().clone();
        let database = Some(container.database());
        Self {
            resource_type: ResourceType::UserDefinedFunction,
            account,
            database,
            container: Some(container),
            name: None,
            rid: Some(udf_rid.into()),
        }
    }

    /// Creates a reference to a partition key range.
    ///
    /// Partition key ranges are identified by their ID (not name or RID in the traditional sense).
    pub fn partition_key_range(
        container: ContainerReference,
        range_id: impl Into<Cow<'static, str>>,
    ) -> Self {
        let account = container.database().account().clone();
        let database = Some(container.database().clone());
        Self {
            resource_type: ResourceType::PartitionKeyRange,
            account,
            database,
            container: Some(container),
            name: Some(range_id.into()),
            rid: None,
        }
    }

    /// Creates a reference to an offer by RID.
    ///
    /// Offers are typically referenced by their RID.
    pub fn offer_by_rid(
        account: AccountReference,
        offer_rid: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            resource_type: ResourceType::Offer,
            account,
            database: None,
            container: None,
            name: None,
            rid: Some(offer_rid.into()),
        }
    }

    // ===== Collection-Level Factory Methods (for Create/List operations) =====

    /// Creates a reference to the databases collection for an account.
    ///
    /// Used for operations that target the collection of databases (create, list, query).
    /// The resulting resource type is `Database` with no specific name, which signals
    /// that the operation targets the collection rather than a specific database.
    pub fn databases_collection(account: AccountReference) -> Self {
        Self {
            resource_type: ResourceType::Database,
            account,
            database: None,
            container: None,
            name: None,
            rid: None,
        }
    }

    /// Creates a reference to the containers collection for a database.
    ///
    /// Used for operations that target the collection of containers (create, list, query).
    /// The resulting resource type is `DocumentCollection` with no specific name, which signals
    /// that the operation targets the collection rather than a specific container.
    pub fn containers_collection(database: DatabaseReference) -> Self {
        let account = database.account().clone();
        Self {
            resource_type: ResourceType::DocumentCollection,
            account,
            database: Some(database),
            container: None,
            name: None,
            rid: None,
        }
    }

    /// Creates a reference to the documents collection for a container.
    ///
    /// Used for operations that target the collection of documents (create, list, query).
    /// The resulting resource type is `Document` with no specific name, which signals
    /// that the operation targets the collection rather than a specific document.
    pub fn documents_collection(container: ContainerReference) -> Self {
        let account = container.database().account().clone();
        let database = Some(container.database().clone());
        Self {
            resource_type: ResourceType::Document,
            account,
            database,
            container: Some(container),
            name: None,
            rid: None,
        }
    }

    /// Returns the name-based relative path for this resource.
    ///
    /// Returns `None` if the required names are not set for this resource type.
    pub fn name_based_path(&self) -> Option<String> {
        match self.resource_type {
            ResourceType::DatabaseAccount => Some(String::new()),
            ResourceType::Database => {
                // If we have a database reference, return its path
                // Otherwise, return the databases collection path
                if let Some(db) = self.database.as_ref() {
                    db.name_based_path()
                } else {
                    Some("/dbs".to_string())
                }
            }
            ResourceType::DocumentCollection => {
                // If we have a resolved container reference, use its pre-computed path.
                // Otherwise build the path from the database + optional container name.
                if let Some(container) = self.container.as_ref() {
                    Some(container.name_based_path())
                } else {
                    let db_path = self.database.as_ref()?.name_based_path()?;
                    if let Some(name) = self.name.as_ref() {
                        // Specific container by name (e.g., read_container_by_name)
                        Some(format!("{}/colls/{}", db_path, name))
                    } else {
                        // Container collection (create, list, query)
                        Some(format!("{}/colls", db_path))
                    }
                }
            }
            ResourceType::Document => {
                // Return the name-based path only if a document name is present.
                // Feed references are handled separately by parent_signing_link.
                let container_path = self.container.as_ref()?.name_based_path();
                let name = self.name.as_ref()?;
                Some(format!("{}/docs/{}", container_path, name))
            }
            ResourceType::StoredProcedure
            | ResourceType::Trigger
            | ResourceType::UserDefinedFunction
            | ResourceType::PartitionKeyRange => {
                let container_path = self.container.as_ref()?.name_based_path();
                let name = self.name.as_ref()?;
                let segment = self.resource_type.path_segment();
                Some(format!("{}/{}/{}", container_path, segment, name))
            }
            ResourceType::Offer => {
                let rid = self.rid.as_ref()?;
                Some(format!("/offers/{}", rid))
            }
        }
    }

    /// Returns the RID-based relative path for this resource.
    ///
    /// Returns `None` if the required RIDs are not set for this resource type.
    pub fn rid_based_path(&self) -> Option<String> {
        match self.resource_type {
            ResourceType::DatabaseAccount => Some(String::new()),
            ResourceType::Database => {
                // If we have a database reference, return its path
                // Otherwise, return the databases collection path
                if let Some(db) = self.database.as_ref() {
                    db.rid_based_path()
                } else {
                    Some("/dbs".to_string())
                }
            }
            ResourceType::DocumentCollection => {
                // If we have a container reference, return its path
                // Otherwise, return the containers collection path within the database
                if let Some(container) = self.container.as_ref() {
                    Some(container.rid_based_path())
                } else {
                    let db_path = self.database.as_ref()?.rid_based_path()?;
                    Some(format!("{}/colls", db_path))
                }
            }
            ResourceType::Document => {
                // Return the RID-based path only if a document RID is present.
                // Feed references are handled separately by parent_signing_link.
                let container_path = self.container.as_ref()?.rid_based_path();
                let rid = self.rid.as_ref()?;
                Some(format!("{}/docs/{}", container_path, rid))
            }
            ResourceType::StoredProcedure
            | ResourceType::Trigger
            | ResourceType::UserDefinedFunction
            | ResourceType::PartitionKeyRange => {
                let container_path = self.container.as_ref()?.rid_based_path();
                let rid = self.rid.as_ref()?;
                let segment = self.resource_type.path_segment();
                Some(format!("{}/{}/{}", container_path, segment, rid))
            }
            ResourceType::Offer => {
                let rid = self.rid.as_ref()?;
                Some(format!("/offers/{}", rid))
            }
        }
    }

    /// Returns the resource link for authorization signing.
    ///
    /// The resource link is an unencoded path used for generating the
    /// authorization signature. Prefers name-based paths over RID-based.
    ///
    /// **Important**: For feed operations (create, list, query) where no specific
    /// item is targeted, this returns the **parent's** path, not the collection path.
    /// This matches the Cosmos DB signature algorithm requirements.
    ///
    /// Examples:
    /// - Creating a database: signing link = "" (empty, account has no parent)
    /// - Creating a container in "mydb": signing link = "dbs/mydb"
    /// - Creating a document: signing link = "dbs/mydb/colls/mycoll"
    /// - Reading a specific database "mydb": signing link = "dbs/mydb"
    /// - Reading a specific document: signing link = "dbs/mydb/colls/mycoll/docs/mydoc"
    ///
    /// This method always returns a valid path because `CosmosResourceReference`
    /// validates that the required identifiers are present at construction time.
    pub fn link_for_signing(&self) -> String {
        // Check if this is a feed operation (no specific item targeted)
        let is_feed = self.is_feed_reference();

        if is_feed {
            // For feed operations, return parent's path
            self.parent_signing_link()
        } else {
            // For item operations, return the full path
            self.name_based_path()
                .or_else(|| self.rid_based_path())
                .expect("CosmosResourceReference is guaranteed to have a valid path")
        }
    }

    /// Returns true if this reference targets a feed (collection) rather than a specific item.
    fn is_feed_reference(&self) -> bool {
        match self.resource_type {
            ResourceType::DatabaseAccount => false,
            ResourceType::Database => self.database.is_none(),
            ResourceType::DocumentCollection => self.container.is_none() && self.name.is_none(),
            ResourceType::Document => self.name.is_none() && self.rid.is_none(),
            ResourceType::StoredProcedure
            | ResourceType::Trigger
            | ResourceType::UserDefinedFunction
            | ResourceType::PartitionKeyRange => self.name.is_none() && self.rid.is_none(),
            ResourceType::Offer => self.rid.is_none(),
        }
    }

    /// Returns the parent's path for signing feed operations.
    fn parent_signing_link(&self) -> String {
        match self.resource_type {
            ResourceType::DatabaseAccount => String::new(),
            ResourceType::Database => {
                // Parent is account, which has no path
                String::new()
            }
            ResourceType::DocumentCollection => {
                // Parent is database
                self.database
                    .as_ref()
                    .and_then(|db| db.name_based_path().or_else(|| db.rid_based_path()))
                    .map(|p| p.trim_start_matches('/').to_string())
                    .unwrap_or_default()
            }
            ResourceType::Document
            | ResourceType::StoredProcedure
            | ResourceType::Trigger
            | ResourceType::UserDefinedFunction
            | ResourceType::PartitionKeyRange => {
                // Parent is container — both paths are always available
                self.container
                    .as_ref()
                    .map(|c| c.name_based_path())
                    .map(|p| p.trim_start_matches('/').to_string())
                    .unwrap_or_default()
            }
            ResourceType::Offer => String::new(),
        }
    }

    /// Returns the URL path for this resource.
    ///
    /// This path can be appended to the account endpoint to form the
    /// full request URL. Prefers name-based paths over RID-based.
    ///
    /// This method always returns a valid path because `CosmosResourceReference`
    /// validates that the required identifiers are present at construction time.
    pub fn request_path(&self) -> String {
        self.name_based_path()
            .or_else(|| self.rid_based_path())
            .expect("CosmosResourceReference is guaranteed to have a valid path")
    }
}

// =============================================================================
// From implementations for typed references
// =============================================================================

impl From<DatabaseReference> for CosmosResourceReference {
    /// Converts a `DatabaseReference` into a `CosmosResourceReference`.
    ///
    /// The resulting reference has `ResourceType::Database` and preserves
    /// the name-based or RID-based addressing mode.
    fn from(database: DatabaseReference) -> Self {
        if database.is_by_name() {
            Self::database_by_name(database)
        } else {
            Self::database_by_rid(database)
        }
    }
}

impl From<ContainerReference> for CosmosResourceReference {
    /// Converts a `ContainerReference` into a `CosmosResourceReference`.
    ///
    /// The resulting reference has `ResourceType::DocumentCollection` and uses
    /// name-based addressing (both name and RID are always available on
    /// a resolved `ContainerReference`).
    fn from(container: ContainerReference) -> Self {
        Self::document_collection_by_name(container)
    }
}

impl From<ItemReference> for CosmosResourceReference {
    /// Converts an `ItemReference` into a `CosmosResourceReference`.
    ///
    /// The resulting reference has `ResourceType::Document` and preserves
    /// the name-based or RID-based addressing mode.
    fn from(item: ItemReference) -> Self {
        let container = item.container().clone();

        if item.is_by_name() {
            let item_name = item.name().expect("name-based item must have name");
            Self::document_by_name(container, item_name.to_owned())
        } else {
            let item_rid = item.rid().expect("RID-based item must have RID");
            Self::document_by_rid(container, item_rid.to_owned())
        }
    }
}

// TODO: Re-implement From<StoredProcedureReference>, From<TriggerReference>,
// and From<UdfReference> once these types are updated to carry a fully resolved
// ContainerReference instead of a partial ContainerId.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PartitionKeyDefinition;
    use std::sync::Arc;
    use url::Url;

    fn test_account() -> AccountReference {
        AccountReference::with_master_key(
            Url::parse("https://test.documents.azure.com:443/").unwrap(),
            "test-key",
        )
    }

    fn test_database() -> DatabaseReference {
        DatabaseReference::from_name(test_account(), "testdb")
    }

    fn test_container_props() -> crate::models::ContainerProperties {
        crate::models::ContainerProperties {
            id: "testcontainer".into(),
            partition_key: PartitionKeyDefinition {
                paths: vec!["/pk".into()],
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn test_container() -> ContainerReference {
        ContainerReference::new(
            test_account(),
            "testdb",
            "dbRid123",
            "testcontainer",
            "collRid456",
            &test_container_props(),
        )
    }

    #[test]
    fn account_resource() {
        let ref_ = CosmosResourceReference::account_resource(test_account());
        assert_eq!(ref_.resource_type(), ResourceType::DatabaseAccount);
        assert!(ref_.database().is_none());
        assert!(ref_.container().is_none());
        assert_eq!(ref_.name_based_path(), Some(String::new()));
    }

    #[test]
    fn database_by_name() {
        let ref_ = CosmosResourceReference::database_by_name(test_database());
        assert_eq!(ref_.resource_type(), ResourceType::Database);
        assert!(ref_.database().is_some());
        assert!(ref_.container().is_none());
        assert_eq!(ref_.name(), Some("testdb"));
        assert_eq!(ref_.name_based_path(), Some("/dbs/testdb".to_string()));
    }

    #[test]
    fn document_collection_by_name() {
        let ref_ = CosmosResourceReference::document_collection_by_name(test_container());
        assert_eq!(ref_.resource_type(), ResourceType::DocumentCollection);
        assert!(ref_.database().is_some());
        assert!(ref_.container().is_some());
        assert_eq!(ref_.name(), Some("testcontainer"));
        assert_eq!(
            ref_.name_based_path(),
            Some("/dbs/testdb/colls/testcontainer".to_string())
        );
    }

    #[test]
    fn document_by_name() {
        let ref_ = CosmosResourceReference::document_by_name(test_container(), "doc1");
        assert_eq!(ref_.resource_type(), ResourceType::Document);
        assert!(ref_.database().is_some());
        assert!(ref_.container().is_some());
        assert_eq!(ref_.name(), Some("doc1"));
        assert_eq!(
            ref_.name_based_path(),
            Some("/dbs/testdb/colls/testcontainer/docs/doc1".to_string())
        );
    }

    #[test]
    fn stored_procedure_by_name() {
        let ref_ = CosmosResourceReference::stored_procedure_by_name(test_container(), "mysproc");
        assert_eq!(ref_.resource_type(), ResourceType::StoredProcedure);
        assert_eq!(ref_.name(), Some("mysproc"));
        assert_eq!(
            ref_.name_based_path(),
            Some("/dbs/testdb/colls/testcontainer/sprocs/mysproc".to_string())
        );
    }

    #[test]
    fn trigger_by_name() {
        let ref_ = CosmosResourceReference::trigger_by_name(test_container(), "mytrigger");
        assert_eq!(ref_.resource_type(), ResourceType::Trigger);
        assert_eq!(ref_.name(), Some("mytrigger"));
        assert_eq!(
            ref_.name_based_path(),
            Some("/dbs/testdb/colls/testcontainer/triggers/mytrigger".to_string())
        );
    }

    #[test]
    fn user_defined_function_by_name() {
        let ref_ =
            CosmosResourceReference::user_defined_function_by_name(test_container(), "myudf");
        assert_eq!(ref_.resource_type(), ResourceType::UserDefinedFunction);
        assert_eq!(ref_.name(), Some("myudf"));
        assert_eq!(
            ref_.name_based_path(),
            Some("/dbs/testdb/colls/testcontainer/udfs/myudf".to_string())
        );
    }

    #[test]
    fn offer_by_rid() {
        let ref_ = CosmosResourceReference::offer_by_rid(test_account(), "offer123");
        assert_eq!(ref_.resource_type(), ResourceType::Offer);
        assert_eq!(ref_.rid(), Some("offer123"));
        assert_eq!(ref_.name_based_path(), Some("/offers/offer123".to_string()));
    }

    #[test]
    fn link_for_signing_prefers_name_based() {
        // Document with name
        let ref_ = CosmosResourceReference::document_by_name(test_container(), "doc1");
        assert_eq!(
            ref_.link_for_signing(),
            "/dbs/testdb/colls/testcontainer/docs/doc1"
        );

        // Database with name
        let ref_ = CosmosResourceReference::database_by_name(test_database());
        assert_eq!(ref_.link_for_signing(), "/dbs/testdb");

        // Account resource (empty path)
        let ref_ = CosmosResourceReference::account_resource(test_account());
        assert_eq!(ref_.link_for_signing(), "");
    }

    #[test]
    fn link_for_signing_falls_back_to_rid() {
        // Document with RID only - uses the same resolved container
        let container = test_container();
        let ref_ = CosmosResourceReference::document_by_rid(container, "docRid789");
        assert_eq!(
            ref_.link_for_signing(),
            "/dbs/dbRid123/colls/collRid456/docs/docRid789"
        );
    }

    #[test]
    fn request_path_matches_link_for_signing() {
        let ref_ = CosmosResourceReference::document_by_name(test_container(), "doc1");
        assert_eq!(ref_.request_path(), ref_.link_for_signing());

        let ref_ = CosmosResourceReference::database_by_name(test_database());
        assert_eq!(ref_.request_path(), ref_.link_for_signing());
    }
}
