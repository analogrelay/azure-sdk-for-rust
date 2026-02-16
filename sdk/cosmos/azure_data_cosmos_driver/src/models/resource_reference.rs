// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Resource reference types for Cosmos DB resources.
//!
//! These types provide compile-time safe references to Cosmos DB resources.
//! Each reference enforces either all-names or all-RIDs addressing through
//! internal enums, preventing mixed addressing modes.

use crate::models::{
    resource_id::{
        ContainerId, DatabaseId, ItemIdentifier, PartitionKeyRangeId, ResourceName, ResourceRid,
        StoredProcedureId, TriggerId, UdfId,
    },
    AccountReference, PartitionKey,
};

// =============================================================================
// DatabaseReference
// =============================================================================

/// A reference to a Cosmos DB database.
///
/// Contains either the name or resource identifier (RID) of the database,
/// along with a reference to its parent account. The addressing mode (name vs RID)
/// is enforced at compile time through internal enums.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct DatabaseReference {
    /// Reference to the parent account.
    account: AccountReference,
    /// The database identifier (by name or by RID).
    id: DatabaseId,
}

impl DatabaseReference {
    /// Creates a new database reference by name.
    pub fn from_name(account: AccountReference, name: impl Into<ResourceName>) -> Self {
        Self {
            account,
            id: DatabaseId::ByName(name.into()),
        }
    }

    /// Creates a new database reference by RID.
    pub fn from_rid(account: AccountReference, rid: impl Into<ResourceRid>) -> Self {
        Self {
            account,
            id: DatabaseId::ByRid(rid.into()),
        }
    }

    /// Returns a reference to the parent account.
    pub fn account(&self) -> &AccountReference {
        &self.account
    }

    /// Returns the database name, if this is a name-based reference.
    pub fn name(&self) -> Option<&str> {
        self.id.name()
    }

    /// Returns the database RID, if this is a RID-based reference.
    pub fn rid(&self) -> Option<&str> {
        self.id.rid()
    }

    /// Returns the internal database ID.
    pub(crate) fn id(&self) -> &DatabaseId {
        &self.id
    }

    /// Returns `true` if this is a name-based reference.
    pub fn is_by_name(&self) -> bool {
        matches!(self.id, DatabaseId::ByName(_))
    }

    /// Returns `true` if this is a RID-based reference.
    pub fn is_by_rid(&self) -> bool {
        matches!(self.id, DatabaseId::ByRid(_))
    }

    /// Returns the name-based relative path: `/dbs/{name}`
    ///
    /// Returns `None` if this is a RID-based reference.
    pub fn name_based_path(&self) -> Option<String> {
        self.id.name().map(|n| format!("/dbs/{}", n))
    }

    /// Returns the RID-based relative path: `/dbs/{rid}`
    ///
    /// Returns `None` if this is a name-based reference.
    pub fn rid_based_path(&self) -> Option<String> {
        self.id.rid().map(|r| format!("/dbs/{}", r))
    }
}

// =============================================================================
// ContainerReference
// =============================================================================

/// A reference to a Cosmos DB container.
///
/// Contains either the name or resource identifier (RID) of the container,
/// along with references to its parent database and account. The addressing mode
/// (name vs RID) is enforced at compile time - if the container is by name,
/// the database must also be by name.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ContainerReference {
    /// Reference to the parent account.
    account: AccountReference,
    /// The container identifier (includes database identifier).
    id: ContainerId,
}

impl ContainerReference {
    /// Creates a new container reference by name.
    ///
    /// Both database and container are identified by their user-provided names.
    pub fn from_name(
        account: AccountReference,
        db_name: impl Into<ResourceName>,
        container_name: impl Into<ResourceName>,
    ) -> Self {
        Self {
            account,
            id: ContainerId::ByName {
                db_name: db_name.into(),
                name: container_name.into(),
            },
        }
    }

    /// Creates a new container reference by name from a parent database reference.
    ///
    /// This is a convenience method that extracts the account and database name
    /// from the parent `DatabaseReference`.
    ///
    /// # Panics
    ///
    /// Panics if the database reference is RID-based (not name-based).
    pub fn from_database(
        database: &DatabaseReference,
        container_name: impl Into<ResourceName>,
    ) -> Self {
        let db_name = database
            .name()
            .expect("DatabaseReference must be name-based to create ContainerReference by name");
        Self {
            account: database.account().clone(),
            id: ContainerId::ByName {
                db_name: ResourceName::new(db_name.to_owned()),
                name: container_name.into(),
            },
        }
    }

    /// Creates a new container reference by RID.
    ///
    /// Both database and container are identified by their internal RIDs.
    pub fn from_rid(
        account: AccountReference,
        db_rid: impl Into<ResourceRid>,
        container_rid: impl Into<ResourceRid>,
    ) -> Self {
        Self {
            account,
            id: ContainerId::ByRid {
                db_rid: db_rid.into(),
                rid: container_rid.into(),
            },
        }
    }

    /// Creates a new container reference by RID from a parent database reference.
    ///
    /// This is a convenience method that extracts the account and database RID
    /// from the parent `DatabaseReference`.
    ///
    /// # Panics
    ///
    /// Panics if the database reference is name-based (not RID-based).
    pub fn from_database_rid(
        database: &DatabaseReference,
        container_rid: impl Into<ResourceRid>,
    ) -> Self {
        let db_rid = database
            .rid()
            .expect("DatabaseReference must be RID-based to create ContainerReference by RID");
        Self {
            account: database.account().clone(),
            id: ContainerId::ByRid {
                db_rid: ResourceRid::new(db_rid.to_owned()),
                rid: container_rid.into(),
            },
        }
    }

    /// Returns a reference to the parent account.
    pub fn account(&self) -> &AccountReference {
        &self.account
    }

    /// Returns the container name, if this is a name-based reference.
    pub fn name(&self) -> Option<&str> {
        self.id.name()
    }

    /// Returns the container RID, if this is a RID-based reference.
    pub fn rid(&self) -> Option<&str> {
        self.id.rid()
    }

    /// Returns the database name, if this is a name-based reference.
    pub fn database_name(&self) -> Option<&str> {
        self.id.database_name()
    }

    /// Returns the database RID, if this is a RID-based reference.
    pub fn database_rid(&self) -> Option<&str> {
        self.id.database_rid()
    }

    /// Returns the internal container ID.
    pub(crate) fn id(&self) -> &ContainerId {
        &self.id
    }

    /// Returns a `DatabaseReference` for the parent database.
    pub fn database(&self) -> DatabaseReference {
        DatabaseReference {
            account: self.account.clone(),
            id: self.id.database_id(),
        }
    }

    /// Returns `true` if this is a name-based reference.
    pub fn is_by_name(&self) -> bool {
        matches!(self.id, ContainerId::ByName { .. })
    }

    /// Returns `true` if this is a RID-based reference.
    pub fn is_by_rid(&self) -> bool {
        matches!(self.id, ContainerId::ByRid { .. })
    }

    /// Returns the name-based relative path: `/dbs/{db_name}/colls/{container_name}`
    ///
    /// Returns `None` if this is a RID-based reference.
    pub fn name_based_path(&self) -> Option<String> {
        match &self.id {
            ContainerId::ByName { db_name, name } => {
                Some(format!("/dbs/{}/colls/{}", db_name, name))
            }
            ContainerId::ByRid { .. } => None,
        }
    }

    /// Returns the RID-based relative path: `/dbs/{db_rid}/colls/{container_rid}`
    ///
    /// Returns `None` if this is a name-based reference.
    pub fn rid_based_path(&self) -> Option<String> {
        match &self.id {
            ContainerId::ByName { .. } => None,
            ContainerId::ByRid { db_rid, rid } => Some(format!("/dbs/{}/colls/{}", db_rid, rid)),
        }
    }
}

// =============================================================================
// ItemReference
// =============================================================================

/// A reference to a Cosmos DB item (document).
///
/// Contains the container reference, partition key, and item identifier (name or RID).
/// The partition key is required because all item operations in Cosmos DB require it.
///
/// The resource link is pre-computed for efficiency.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ItemReference {
    /// Reference to the parent container.
    container: ContainerReference,
    /// The partition key for the item.
    partition_key: PartitionKey,
    /// The item identifier (name or RID).
    item_identifier: ItemIdentifier,
    /// Pre-computed resource link.
    resource_link: String,
}

impl ItemReference {
    /// Creates a new item reference by name.
    ///
    /// # Arguments
    ///
    /// * `container` - Reference to the parent container.
    /// * `partition_key` - The partition key for the item.
    /// * `item_name` - The document ID (name) of the item.
    ///
    /// # Panics
    ///
    /// Panics if the container reference is RID-based (not name-based).
    pub fn from_name(
        container: &ContainerReference,
        partition_key: PartitionKey,
        item_name: impl Into<ResourceName>,
    ) -> Self {
        let name = item_name.into();
        let resource_link = container
            .name_based_path()
            .map(|path| format!("{}/docs/{}", path, name))
            .expect("ContainerReference must be name-based to create ItemReference by name");
        Self {
            container: container.clone(),
            partition_key,
            item_identifier: ItemIdentifier::ByName(name),
            resource_link,
        }
    }

    /// Creates a new item reference by RID.
    ///
    /// # Arguments
    ///
    /// * `container` - Reference to the parent container.
    /// * `partition_key` - The partition key for the item.
    /// * `item_rid` - The internal RID of the item.
    ///
    /// # Panics
    ///
    /// Panics if the container reference is name-based (not RID-based).
    pub fn from_rid(
        container: &ContainerReference,
        partition_key: PartitionKey,
        item_rid: impl Into<ResourceRid>,
    ) -> Self {
        let rid = item_rid.into();
        let resource_link = container
            .rid_based_path()
            .map(|path| format!("{}/docs/{}", path, rid))
            .expect("ContainerReference must be RID-based to create ItemReference by RID");
        Self {
            container: container.clone(),
            partition_key,
            item_identifier: ItemIdentifier::ByRid(rid),
            resource_link,
        }
    }

    /// Returns a reference to the parent container.
    pub fn container(&self) -> &ContainerReference {
        &self.container
    }

    /// Returns a reference to the parent account.
    pub fn account(&self) -> &AccountReference {
        self.container.account()
    }

    /// Returns a reference to the partition key.
    pub fn partition_key(&self) -> &PartitionKey {
        &self.partition_key
    }

    /// Returns a reference to the item identifier.
    pub(crate) fn item_identifier(&self) -> &ItemIdentifier {
        &self.item_identifier
    }

    /// Returns the item name (document ID), if this is a name-based reference.
    pub fn name(&self) -> Option<&str> {
        self.item_identifier.name()
    }

    /// Returns the item RID, if this is a RID-based reference.
    pub fn rid(&self) -> Option<&str> {
        self.item_identifier.rid()
    }

    /// Returns `true` if this is a name-based reference.
    pub fn is_by_name(&self) -> bool {
        self.item_identifier.is_by_name()
    }

    /// Returns `true` if this is a RID-based reference.
    pub fn is_by_rid(&self) -> bool {
        self.item_identifier.is_by_rid()
    }

    /// Returns the pre-computed resource link for this item.
    ///
    /// For name-based references: `/dbs/{db}/colls/{coll}/docs/{item}`
    /// For RID-based references: `/dbs/{db_rid}/colls/{coll_rid}/docs/{item_rid}`
    pub fn resource_link(&self) -> &str {
        &self.resource_link
    }
}

// =============================================================================
// StoredProcedureReference
// =============================================================================

/// A reference to a Cosmos DB stored procedure.
///
/// Contains either the name or resource identifier (RID) of the stored procedure,
/// along with references to its parent container, database, and account.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct StoredProcedureReference {
    /// Reference to the parent account.
    account: AccountReference,
    /// The stored procedure identifier.
    id: StoredProcedureId,
}

impl StoredProcedureReference {
    /// Creates a new stored procedure reference by name.
    pub fn from_name(
        account: AccountReference,
        db_name: impl Into<ResourceName>,
        container_name: impl Into<ResourceName>,
        sproc_name: impl Into<ResourceName>,
    ) -> Self {
        Self {
            account,
            id: StoredProcedureId::ByName {
                container: ContainerId::ByName {
                    db_name: db_name.into(),
                    name: container_name.into(),
                },
                name: sproc_name.into(),
            },
        }
    }

    /// Creates a new stored procedure reference by name from a parent container reference.
    ///
    /// # Panics
    ///
    /// Panics if the container reference is RID-based (not name-based).
    pub fn from_container(
        container: &ContainerReference,
        sproc_name: impl Into<ResourceName>,
    ) -> Self {
        let container_id = container.id().clone();
        if !container.is_by_name() {
            panic!(
                "ContainerReference must be name-based to create StoredProcedureReference by name"
            );
        }
        Self {
            account: container.account().clone(),
            id: StoredProcedureId::ByName {
                container: container_id,
                name: sproc_name.into(),
            },
        }
    }

    /// Creates a new stored procedure reference by RID.
    pub fn from_rid(
        account: AccountReference,
        container_rid: impl Into<ResourceRid>,
        sproc_rid: impl Into<ResourceRid>,
    ) -> Self {
        Self {
            account,
            id: StoredProcedureId::ByRid {
                container_rid: container_rid.into(),
                rid: sproc_rid.into(),
            },
        }
    }

    /// Creates a new stored procedure reference by RID from a parent container reference.
    ///
    /// # Panics
    ///
    /// Panics if the container reference is name-based (not RID-based).
    pub fn from_container_rid(
        container: &ContainerReference,
        sproc_rid: impl Into<ResourceRid>,
    ) -> Self {
        let container_rid = container.rid().expect(
            "ContainerReference must be RID-based to create StoredProcedureReference by RID",
        );
        Self {
            account: container.account().clone(),
            id: StoredProcedureId::ByRid {
                container_rid: ResourceRid::new(container_rid.to_owned()),
                rid: sproc_rid.into(),
            },
        }
    }

    /// Returns a reference to the parent account.
    pub fn account(&self) -> &AccountReference {
        &self.account
    }

    /// Returns the stored procedure name, if this is a name-based reference.
    pub fn name(&self) -> Option<&str> {
        self.id.name()
    }

    /// Returns the stored procedure RID, if this is a RID-based reference.
    pub fn rid(&self) -> Option<&str> {
        self.id.rid()
    }

    /// Returns the internal stored procedure ID.
    pub(crate) fn id(&self) -> &StoredProcedureId {
        &self.id
    }

    /// Returns `true` if this is a name-based reference.
    pub fn is_by_name(&self) -> bool {
        matches!(self.id, StoredProcedureId::ByName { .. })
    }

    /// Returns `true` if this is a RID-based reference.
    pub fn is_by_rid(&self) -> bool {
        matches!(self.id, StoredProcedureId::ByRid { .. })
    }

    /// Returns the name-based relative path.
    pub fn name_based_path(&self) -> Option<String> {
        match &self.id {
            StoredProcedureId::ByName { container, name } => match container {
                ContainerId::ByName {
                    db_name,
                    name: container_name,
                } => Some(format!(
                    "/dbs/{}/colls/{}/sprocs/{}",
                    db_name, container_name, name
                )),
                ContainerId::ByRid { .. } => None,
            },
            StoredProcedureId::ByRid { .. } => None,
        }
    }

    /// Returns the RID-based relative path.
    pub fn rid_based_path(&self) -> Option<String> {
        match &self.id {
            StoredProcedureId::ByName { .. } => None,
            StoredProcedureId::ByRid { container_rid, rid } => {
                Some(format!("/colls/{}/sprocs/{}", container_rid, rid))
            }
        }
    }
}

// =============================================================================
// TriggerReference
// =============================================================================

/// A reference to a Cosmos DB trigger resource.
///
/// Contains either the name or resource identifier (RID) of the trigger,
/// along with references to its parent container, database, and account.
///
/// Note: This is different from `TriggerInvocation` which specifies which trigger
/// to invoke during an operation. This type is for referencing trigger definitions.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct TriggerReference {
    /// Reference to the parent account.
    account: AccountReference,
    /// The trigger identifier.
    id: TriggerId,
}

impl TriggerReference {
    /// Creates a new trigger reference by name.
    pub fn from_name(
        account: AccountReference,
        db_name: impl Into<ResourceName>,
        container_name: impl Into<ResourceName>,
        trigger_name: impl Into<ResourceName>,
    ) -> Self {
        Self {
            account,
            id: TriggerId::ByName {
                container: ContainerId::ByName {
                    db_name: db_name.into(),
                    name: container_name.into(),
                },
                name: trigger_name.into(),
            },
        }
    }

    /// Creates a new trigger reference by name from a parent container reference.
    ///
    /// # Panics
    ///
    /// Panics if the container reference is RID-based (not name-based).
    pub fn from_container(
        container: &ContainerReference,
        trigger_name: impl Into<ResourceName>,
    ) -> Self {
        let container_id = container.id().clone();
        if !container.is_by_name() {
            panic!("ContainerReference must be name-based to create TriggerReference by name");
        }
        Self {
            account: container.account().clone(),
            id: TriggerId::ByName {
                container: container_id,
                name: trigger_name.into(),
            },
        }
    }

    /// Creates a new trigger reference by RID.
    pub fn from_rid(
        account: AccountReference,
        container_rid: impl Into<ResourceRid>,
        trigger_rid: impl Into<ResourceRid>,
    ) -> Self {
        Self {
            account,
            id: TriggerId::ByRid {
                container_rid: container_rid.into(),
                rid: trigger_rid.into(),
            },
        }
    }

    /// Creates a new trigger reference by RID from a parent container reference.
    ///
    /// # Panics
    ///
    /// Panics if the container reference is name-based (not RID-based).
    pub fn from_container_rid(
        container: &ContainerReference,
        trigger_rid: impl Into<ResourceRid>,
    ) -> Self {
        let container_rid = container
            .rid()
            .expect("ContainerReference must be RID-based to create TriggerReference by RID");
        Self {
            account: container.account().clone(),
            id: TriggerId::ByRid {
                container_rid: ResourceRid::new(container_rid.to_owned()),
                rid: trigger_rid.into(),
            },
        }
    }

    /// Returns a reference to the parent account.
    pub fn account(&self) -> &AccountReference {
        &self.account
    }

    /// Returns the trigger name, if this is a name-based reference.
    pub fn name(&self) -> Option<&str> {
        self.id.name()
    }

    /// Returns the trigger RID, if this is a RID-based reference.
    pub fn rid(&self) -> Option<&str> {
        self.id.rid()
    }

    /// Returns the internal trigger ID.
    pub(crate) fn id(&self) -> &TriggerId {
        &self.id
    }

    /// Returns `true` if this is a name-based reference.
    pub fn is_by_name(&self) -> bool {
        matches!(self.id, TriggerId::ByName { .. })
    }

    /// Returns `true` if this is a RID-based reference.
    pub fn is_by_rid(&self) -> bool {
        matches!(self.id, TriggerId::ByRid { .. })
    }

    /// Returns the name-based relative path.
    pub fn name_based_path(&self) -> Option<String> {
        match &self.id {
            TriggerId::ByName { container, name } => match container {
                ContainerId::ByName {
                    db_name,
                    name: container_name,
                } => Some(format!(
                    "/dbs/{}/colls/{}/triggers/{}",
                    db_name, container_name, name
                )),
                ContainerId::ByRid { .. } => None,
            },
            TriggerId::ByRid { .. } => None,
        }
    }

    /// Returns the RID-based relative path.
    pub fn rid_based_path(&self) -> Option<String> {
        match &self.id {
            TriggerId::ByName { .. } => None,
            TriggerId::ByRid { container_rid, rid } => {
                Some(format!("/colls/{}/triggers/{}", container_rid, rid))
            }
        }
    }
}

// =============================================================================
// UdfReference (User-Defined Function)
// =============================================================================

/// A reference to a Cosmos DB user-defined function (UDF).
///
/// Contains either the name or resource identifier (RID) of the UDF,
/// along with references to its parent container, database, and account.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct UdfReference {
    /// Reference to the parent account.
    account: AccountReference,
    /// The UDF identifier.
    id: UdfId,
}

impl UdfReference {
    /// Creates a new UDF reference by name.
    pub fn from_name(
        account: AccountReference,
        db_name: impl Into<ResourceName>,
        container_name: impl Into<ResourceName>,
        udf_name: impl Into<ResourceName>,
    ) -> Self {
        Self {
            account,
            id: UdfId::ByName {
                container: ContainerId::ByName {
                    db_name: db_name.into(),
                    name: container_name.into(),
                },
                name: udf_name.into(),
            },
        }
    }

    /// Creates a new UDF reference by name from a parent container reference.
    ///
    /// # Panics
    ///
    /// Panics if the container reference is RID-based (not name-based).
    pub fn from_container(
        container: &ContainerReference,
        udf_name: impl Into<ResourceName>,
    ) -> Self {
        let container_id = container.id().clone();
        if !container.is_by_name() {
            panic!("ContainerReference must be name-based to create UdfReference by name");
        }
        Self {
            account: container.account().clone(),
            id: UdfId::ByName {
                container: container_id,
                name: udf_name.into(),
            },
        }
    }

    /// Creates a new UDF reference by RID.
    pub fn from_rid(
        account: AccountReference,
        container_rid: impl Into<ResourceRid>,
        udf_rid: impl Into<ResourceRid>,
    ) -> Self {
        Self {
            account,
            id: UdfId::ByRid {
                container_rid: container_rid.into(),
                rid: udf_rid.into(),
            },
        }
    }

    /// Creates a new UDF reference by RID from a parent container reference.
    ///
    /// # Panics
    ///
    /// Panics if the container reference is name-based (not RID-based).
    pub fn from_container_rid(
        container: &ContainerReference,
        udf_rid: impl Into<ResourceRid>,
    ) -> Self {
        let container_rid = container
            .rid()
            .expect("ContainerReference must be RID-based to create UdfReference by RID");
        Self {
            account: container.account().clone(),
            id: UdfId::ByRid {
                container_rid: ResourceRid::new(container_rid.to_owned()),
                rid: udf_rid.into(),
            },
        }
    }

    /// Returns a reference to the parent account.
    pub fn account(&self) -> &AccountReference {
        &self.account
    }

    /// Returns the UDF name, if this is a name-based reference.
    pub fn name(&self) -> Option<&str> {
        self.id.name()
    }

    /// Returns the UDF RID, if this is a RID-based reference.
    pub fn rid(&self) -> Option<&str> {
        self.id.rid()
    }

    /// Returns the internal UDF ID.
    pub(crate) fn id(&self) -> &UdfId {
        &self.id
    }

    /// Returns `true` if this is a name-based reference.
    pub fn is_by_name(&self) -> bool {
        matches!(self.id, UdfId::ByName { .. })
    }

    /// Returns `true` if this is a RID-based reference.
    pub fn is_by_rid(&self) -> bool {
        matches!(self.id, UdfId::ByRid { .. })
    }

    /// Returns the name-based relative path.
    pub fn name_based_path(&self) -> Option<String> {
        match &self.id {
            UdfId::ByName { container, name } => match container {
                ContainerId::ByName {
                    db_name,
                    name: container_name,
                } => Some(format!(
                    "/dbs/{}/colls/{}/udfs/{}",
                    db_name, container_name, name
                )),
                ContainerId::ByRid { .. } => None,
            },
            UdfId::ByRid { .. } => None,
        }
    }

    /// Returns the RID-based relative path.
    pub fn rid_based_path(&self) -> Option<String> {
        match &self.id {
            UdfId::ByName { .. } => None,
            UdfId::ByRid { container_rid, rid } => {
                Some(format!("/colls/{}/udfs/{}", container_rid, rid))
            }
        }
    }
}

// =============================================================================
// PartitionKeyRangeReference (pub(crate))
// =============================================================================

/// A reference to a Cosmos DB partition key range.
///
/// This is an internal type used for partition key range operations.
/// Partition key ranges are internal resources that should not be exposed
/// in the public API.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PartitionKeyRangeReference {
    /// Reference to the parent account.
    account: AccountReference,
    /// The partition key range identifier.
    id: PartitionKeyRangeId,
}

impl PartitionKeyRangeReference {
    /// Creates a new partition key range reference by name.
    pub(crate) fn from_name(
        account: AccountReference,
        db_name: impl Into<ResourceName>,
        container_name: impl Into<ResourceName>,
        range_id: impl Into<ResourceName>,
    ) -> Self {
        Self {
            account,
            id: PartitionKeyRangeId::ByName {
                container: ContainerId::ByName {
                    db_name: db_name.into(),
                    name: container_name.into(),
                },
                range_id: range_id.into(),
            },
        }
    }

    /// Creates a new partition key range reference by RID.
    pub(crate) fn from_rid(
        account: AccountReference,
        container_rid: impl Into<ResourceRid>,
        range_id: impl Into<ResourceName>,
    ) -> Self {
        Self {
            account,
            id: PartitionKeyRangeId::ByRid {
                container_rid: container_rid.into(),
                range_id: range_id.into(),
            },
        }
    }

    /// Returns a reference to the parent account.
    pub(crate) fn account(&self) -> &AccountReference {
        &self.account
    }

    /// Returns the partition key range ID.
    pub(crate) fn range_id(&self) -> &str {
        self.id.range_id()
    }

    /// Returns the internal partition key range ID.
    pub(crate) fn id(&self) -> &PartitionKeyRangeId {
        &self.id
    }
}
