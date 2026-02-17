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
    AccountReference, ImmutableContainerProperties, PartitionKey,
};

use std::hash::{Hash, Hasher};
use std::sync::Arc;

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

/// A resolved reference to a Cosmos DB container.
///
/// Always carries both the name-based and RID-based identifiers for the container
/// and its parent database, along with immutable container properties (partition key
/// definition and unique key policy). This guarantees that both addressing modes
/// are available without additional I/O.
///
/// Instances are created via async factory methods that resolve the container
/// metadata from the Cosmos DB service or cache.
///
/// ## Equality and Hashing
///
/// Two `ContainerReference` values are considered equal if they refer to the same
/// account, container RID, and container name. This detects both delete + recreate
/// (same name, different RID) and rename scenarios (same RID, different name).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ContainerReference {
    /// Reference to the parent account.
    account: AccountReference,
    /// The database user-provided name.
    db_name: ResourceName,
    /// The database internal RID.
    db_rid: ResourceRid,
    /// The container user-provided name.
    container_name: ResourceName,
    /// The container internal RID.
    container_rid: ResourceRid,
    /// Immutable container properties (partition key, unique key policy).
    immutable_properties: Arc<ImmutableContainerProperties>,
}

impl PartialEq for ContainerReference {
    fn eq(&self, other: &Self) -> bool {
        self.account == other.account
            && self.container_rid == other.container_rid
            && self.container_name == other.container_name
    }
}

impl Eq for ContainerReference {}

impl Hash for ContainerReference {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.account.hash(state);
        self.container_rid.hash(state);
        self.container_name.hash(state);
    }
}

impl ContainerReference {
    /// Creates a fully resolved container reference.
    ///
    /// All fields are required — the caller must have already resolved both
    /// name-based and RID-based identifiers (typically by reading the container
    /// from the Cosmos DB service).
    ///
    /// The immutable properties (partition key definition, unique key policy)
    /// are extracted from `container_properties` and stored internally.
    ///
    /// Not exposed publicly — use [`CosmosDriver::resolve_container()`](crate::driver::CosmosDriver::resolve_container)
    /// to obtain a resolved container reference.
    pub(crate) fn new(
        account: AccountReference,
        db_name: impl Into<ResourceName>,
        db_rid: impl Into<ResourceRid>,
        container_name: impl Into<ResourceName>,
        container_rid: impl Into<ResourceRid>,
        container_properties: &crate::models::ContainerProperties,
    ) -> Self {
        Self {
            account,
            db_name: db_name.into(),
            db_rid: db_rid.into(),
            container_name: container_name.into(),
            container_rid: container_rid.into(),
            immutable_properties: Arc::new(
                ImmutableContainerProperties::from_container_properties(container_properties),
            ),
        }
    }

    /// Returns a reference to the parent account.
    pub fn account(&self) -> &AccountReference {
        &self.account
    }

    /// Returns the container name.
    pub fn name(&self) -> &str {
        self.container_name.as_str()
    }

    /// Returns the container RID.
    pub fn rid(&self) -> &str {
        self.container_rid.as_str()
    }

    /// Returns the database name.
    pub fn database_name(&self) -> &str {
        self.db_name.as_str()
    }

    /// Returns the database RID.
    pub fn database_rid(&self) -> &str {
        self.db_rid.as_str()
    }

    /// Returns the partition key definition for this container.
    pub fn partition_key(&self) -> &crate::models::PartitionKeyDefinition {
        self.immutable_properties.partition_key()
    }

    /// Returns the unique key policy for this container, if any.
    pub fn unique_key_policy(&self) -> Option<&crate::models::UniqueKeyPolicy> {
        self.immutable_properties.unique_key_policy()
    }

    /// Returns the immutable container properties.
    pub(crate) fn immutable_properties(&self) -> &Arc<ImmutableContainerProperties> {
        &self.immutable_properties
    }

    /// Returns a `DatabaseReference` for the parent database (name-based).
    pub fn database(&self) -> DatabaseReference {
        DatabaseReference {
            account: self.account.clone(),
            id: DatabaseId::ByName(self.db_name.clone()),
        }
    }

    /// Returns the name-based relative path: `/dbs/{db_name}/colls/{container_name}`
    pub fn name_based_path(&self) -> String {
        format!(
            "/dbs/{}/colls/{}",
            self.db_name, self.container_name
        )
    }

    /// Returns the RID-based relative path: `/dbs/{db_rid}/colls/{container_rid}`
    pub fn rid_based_path(&self) -> String {
        format!(
            "/dbs/{}/colls/{}",
            self.db_rid, self.container_rid
        )
    }

    /// Returns the internal container name as a `ResourceName`.
    pub(crate) fn container_name_ref(&self) -> &ResourceName {
        &self.container_name
    }

    /// Returns the internal container RID as a `ResourceRid`.
    pub(crate) fn container_rid_ref(&self) -> &ResourceRid {
        &self.container_rid
    }

    /// Returns the internal database name as a `ResourceName`.
    pub(crate) fn db_name_ref(&self) -> &ResourceName {
        &self.db_name
    }

    /// Returns the internal database RID as a `ResourceRid`.
    pub(crate) fn db_rid_ref(&self) -> &ResourceRid {
        &self.db_rid
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
    pub fn from_name(
        container: &ContainerReference,
        partition_key: PartitionKey,
        item_name: impl Into<ResourceName>,
    ) -> Self {
        let name = item_name.into();
        let resource_link = format!("{}/docs/{}", container.name_based_path(), name);
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
    pub fn from_rid(
        container: &ContainerReference,
        partition_key: PartitionKey,
        item_rid: impl Into<ResourceRid>,
    ) -> Self {
        let rid = item_rid.into();
        let resource_link = format!("{}/docs/{}", container.rid_based_path(), rid);
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
    pub fn from_container(
        container: &ContainerReference,
        sproc_name: impl Into<ResourceName>,
    ) -> Self {
        Self {
            account: container.account().clone(),
            id: StoredProcedureId::ByName {
                container: ContainerId::ByName {
                    db_name: container.db_name_ref().clone(),
                    name: container.container_name_ref().clone(),
                },
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
    pub fn from_container_rid(
        container: &ContainerReference,
        sproc_rid: impl Into<ResourceRid>,
    ) -> Self {
        Self {
            account: container.account().clone(),
            id: StoredProcedureId::ByRid {
                container_rid: container.container_rid_ref().clone(),
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
    pub fn from_container(
        container: &ContainerReference,
        trigger_name: impl Into<ResourceName>,
    ) -> Self {
        Self {
            account: container.account().clone(),
            id: TriggerId::ByName {
                container: ContainerId::ByName {
                    db_name: container.db_name_ref().clone(),
                    name: container.container_name_ref().clone(),
                },
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
    pub fn from_container_rid(
        container: &ContainerReference,
        trigger_rid: impl Into<ResourceRid>,
    ) -> Self {
        Self {
            account: container.account().clone(),
            id: TriggerId::ByRid {
                container_rid: container.container_rid_ref().clone(),
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
    pub fn from_container(
        container: &ContainerReference,
        udf_name: impl Into<ResourceName>,
    ) -> Self {
        Self {
            account: container.account().clone(),
            id: UdfId::ByName {
                container: ContainerId::ByName {
                    db_name: container.db_name_ref().clone(),
                    name: container.container_name_ref().clone(),
                },
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
    pub fn from_container_rid(
        container: &ContainerReference,
        udf_rid: impl Into<ResourceRid>,
    ) -> Self {
        Self {
            account: container.account().clone(),
            id: UdfId::ByRid {
                container_rid: container.container_rid_ref().clone(),
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
