// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Generic resource reference type for Cosmos DB resources.

use crate::models::{AccountReference, ContainerReference, DatabaseReference, ResourceType};
use std::borrow::Cow;

/// A generic reference to any Cosmos DB resource.
///
/// Contains the resource type, optional parent references (account, database, container),
/// and either a name or resource identifier (RID) for the resource itself.
///
/// Use the factory methods to create references for specific resource types:
/// - [`CosmosResourceReference::account`] - Account-level resources
/// - [`CosmosResourceReference::database`] - Database resources
/// - [`CosmosResourceReference::document_collection`] - Container/collection resources
/// - [`CosmosResourceReference::document`] - Document/item resources
/// - [`CosmosResourceReference::stored_procedure`] - Stored procedure resources
/// - [`CosmosResourceReference::trigger`] - Trigger resources
/// - [`CosmosResourceReference::user_defined_function`] - UDF resources
/// - [`CosmosResourceReference::partition_key_range`] - Partition key range resources
/// - [`CosmosResourceReference::offer`] - Offer resources
#[derive(Clone, Debug, PartialEq)]
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
    #[must_use]
    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the resource identifier (RID).
    #[must_use]
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
    pub fn database_by_name(database: DatabaseReference) -> Self {
        let account = database.account().clone();
        let name = database.name().map(|n| Cow::Owned(n.to_owned()));
        Self {
            resource_type: ResourceType::Database,
            account,
            database: Some(database),
            container: None,
            name,
            rid: None,
        }
    }

    /// Creates a reference to a database by RID.
    pub fn database_by_rid(database: DatabaseReference) -> Self {
        let account = database.account().clone();
        let rid = database.rid().map(|r| Cow::Owned(r.to_owned()));
        Self {
            resource_type: ResourceType::Database,
            account,
            database: Some(database),
            container: None,
            name: None,
            rid,
        }
    }

    /// Creates a reference to a container (document collection) by name.
    pub fn document_collection_by_name(container: ContainerReference) -> Self {
        let account = container.database().account().clone();
        let database = Some(container.database().clone());
        let name = container.name().map(|n| Cow::Owned(n.to_owned()));
        Self {
            resource_type: ResourceType::DocumentCollection,
            account,
            database,
            container: Some(container),
            name,
            rid: None,
        }
    }

    /// Creates a reference to a container (document collection) by RID.
    pub fn document_collection_by_rid(container: ContainerReference) -> Self {
        let account = container.database().account().clone();
        let database = Some(container.database().clone());
        let rid = container.rid().map(|r| Cow::Owned(r.to_owned()));
        Self {
            resource_type: ResourceType::DocumentCollection,
            account,
            database,
            container: Some(container),
            name: None,
            rid,
        }
    }

    /// Creates a reference to a document by name.
    pub fn document_by_name(
        container: ContainerReference,
        document_name: impl Into<Cow<'static, str>>,
    ) -> Self {
        let account = container.database().account().clone();
        let database = Some(container.database().clone());
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
        let account = container.database().account().clone();
        let database = Some(container.database().clone());
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
        let account = container.database().account().clone();
        let database = Some(container.database().clone());
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
        let account = container.database().account().clone();
        let database = Some(container.database().clone());
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
        let account = container.database().account().clone();
        let database = Some(container.database().clone());
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
        let account = container.database().account().clone();
        let database = Some(container.database().clone());
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
        let account = container.database().account().clone();
        let database = Some(container.database().clone());
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
        let account = container.database().account().clone();
        let database = Some(container.database().clone());
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

    /// Returns the name-based relative path for this resource.
    ///
    /// Returns `None` if the required names are not set for this resource type.
    pub fn name_based_path(&self) -> Option<String> {
        match self.resource_type {
            ResourceType::DatabaseAccount => Some(String::new()),
            ResourceType::Database => self.database.as_ref()?.name_based_path(),
            ResourceType::DocumentCollection => self.container.as_ref()?.name_based_path(),
            ResourceType::Document
            | ResourceType::StoredProcedure
            | ResourceType::Trigger
            | ResourceType::UserDefinedFunction
            | ResourceType::PartitionKeyRange => {
                let container_path = self.container.as_ref()?.name_based_path()?;
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
            ResourceType::Database => self.database.as_ref()?.rid_based_path(),
            ResourceType::DocumentCollection => self.container.as_ref()?.rid_based_path(),
            ResourceType::Document
            | ResourceType::StoredProcedure
            | ResourceType::Trigger
            | ResourceType::UserDefinedFunction
            | ResourceType::PartitionKeyRange => {
                let container_path = self.container.as_ref()?.rid_based_path()?;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    fn test_account() -> AccountReference {
        AccountReference::new(Url::parse("https://test.documents.azure.com:443/").unwrap())
    }

    fn test_database() -> DatabaseReference {
        DatabaseReference::from_name(test_account(), "testdb")
    }

    fn test_container() -> ContainerReference {
        ContainerReference::from_name(test_database(), "testcontainer")
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
}
