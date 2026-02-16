// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Resource identification types for Cosmos DB resources.
//!
//! This module provides newtypes for resource names and RIDs (resource identifiers),
//! as well as internal ID enums that enforce either all-names or all-RIDs addressing.

use std::borrow::Cow;

/// A resource name (user-provided identifier).
///
/// Used for human-readable identifiers like database names, container names, etc.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ResourceName(Cow<'static, str>);

impl ResourceName {
    /// Creates a new resource name.
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self(name.into())
    }

    /// Returns the name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the `ResourceName` and returns the inner `Cow<'static, str>`.
    pub fn into_inner(self) -> Cow<'static, str> {
        self.0
    }
}

impl From<&'static str> for ResourceName {
    fn from(s: &'static str) -> Self {
        Self::new(s)
    }
}

impl From<String> for ResourceName {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl AsRef<str> for ResourceName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for ResourceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A resource identifier (RID) - internal Cosmos DB identifier.
///
/// RIDs are base64-encoded internal identifiers assigned by Cosmos DB.
/// They encode the resource hierarchy (account → database → container → document).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ResourceRid(Cow<'static, str>);

impl ResourceRid {
    /// Creates a new resource RID.
    pub fn new(rid: impl Into<Cow<'static, str>>) -> Self {
        Self(rid.into())
    }

    /// Returns the RID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the `ResourceRid` and returns the inner `Cow<'static, str>`.
    pub fn into_inner(self) -> Cow<'static, str> {
        self.0
    }
}

impl From<&'static str> for ResourceRid {
    fn from(s: &'static str) -> Self {
        Self::new(s)
    }
}

impl From<String> for ResourceRid {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl AsRef<str> for ResourceRid {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for ResourceRid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// =============================================================================
// Internal ID Enums (pub(crate))
// =============================================================================
// These enums enforce either all-names or all-RIDs addressing at compile time.

/// Database identifier - either by name or by RID.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum DatabaseId {
    /// Reference by user-provided name.
    ByName(ResourceName),
    /// Reference by internal RID.
    ByRid(ResourceRid),
}

impl DatabaseId {
    /// Returns the name if this is a name-based identifier.
    pub(crate) fn name(&self) -> Option<&str> {
        match self {
            Self::ByName(name) => Some(name.as_str()),
            Self::ByRid(_) => None,
        }
    }

    /// Returns the RID if this is a RID-based identifier.
    pub(crate) fn rid(&self) -> Option<&str> {
        match self {
            Self::ByName(_) => None,
            Self::ByRid(rid) => Some(rid.as_str()),
        }
    }

    /// Returns the name-based path segment: `{name}` or `None` if RID-based.
    pub(crate) fn name_segment(&self) -> Option<&str> {
        self.name()
    }

    /// Returns the RID-based path segment: `{rid}` or `None` if name-based.
    pub(crate) fn rid_segment(&self) -> Option<&str> {
        self.rid()
    }
}

/// Container identifier - either by name (with database name) or by RID (with database RID).
///
/// Enforces consistency: if container is by name, database must also be by name.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ContainerId {
    /// Reference by user-provided names (database name + container name).
    ByName {
        /// Parent database name.
        db_name: ResourceName,
        /// Container name.
        name: ResourceName,
    },
    /// Reference by internal RIDs (database RID + container RID).
    ByRid {
        /// Parent database RID.
        db_rid: ResourceRid,
        /// Container RID.
        rid: ResourceRid,
    },
}

impl ContainerId {
    /// Returns the container name if this is a name-based identifier.
    pub(crate) fn name(&self) -> Option<&str> {
        match self {
            Self::ByName { name, .. } => Some(name.as_str()),
            Self::ByRid { .. } => None,
        }
    }

    /// Returns the container RID if this is a RID-based identifier.
    pub(crate) fn rid(&self) -> Option<&str> {
        match self {
            Self::ByName { .. } => None,
            Self::ByRid { rid, .. } => Some(rid.as_str()),
        }
    }

    /// Returns the database name if this is a name-based identifier.
    pub(crate) fn database_name(&self) -> Option<&str> {
        match self {
            Self::ByName { db_name, .. } => Some(db_name.as_str()),
            Self::ByRid { .. } => None,
        }
    }

    /// Returns the database RID if this is a RID-based identifier.
    pub(crate) fn database_rid(&self) -> Option<&str> {
        match self {
            Self::ByName { .. } => None,
            Self::ByRid { db_rid, .. } => Some(db_rid.as_str()),
        }
    }

    /// Extracts the database ID from this container ID.
    pub(crate) fn database_id(&self) -> DatabaseId {
        match self {
            Self::ByName { db_name, .. } => DatabaseId::ByName(db_name.clone()),
            Self::ByRid { db_rid, .. } => DatabaseId::ByRid(db_rid.clone()),
        }
    }
}

/// Simple item (document) identifier - just name or RID.
///
/// Used in [`crate::models::ItemReference`] where the container is stored separately.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ItemIdentifier {
    /// Reference by user-provided document ID (name).
    ByName(ResourceName),
    /// Reference by internal RID.
    ByRid(ResourceRid),
}

impl ItemIdentifier {
    /// Creates an item identifier by name.
    pub(crate) fn by_name(name: impl Into<ResourceName>) -> Self {
        Self::ByName(name.into())
    }

    /// Creates an item identifier by RID.
    pub(crate) fn by_rid(rid: impl Into<ResourceRid>) -> Self {
        Self::ByRid(rid.into())
    }

    /// Returns the item name if this is a name-based identifier.
    pub(crate) fn name(&self) -> Option<&str> {
        match self {
            Self::ByName(name) => Some(name.as_str()),
            Self::ByRid(_) => None,
        }
    }

    /// Returns the item RID if this is a RID-based identifier.
    pub(crate) fn rid(&self) -> Option<&str> {
        match self {
            Self::ByName(_) => None,
            Self::ByRid(rid) => Some(rid.as_str()),
        }
    }

    /// Returns `true` if this is a name-based identifier.
    pub(crate) fn is_by_name(&self) -> bool {
        matches!(self, Self::ByName(_))
    }

    /// Returns `true` if this is a RID-based identifier.
    pub(crate) fn is_by_rid(&self) -> bool {
        matches!(self, Self::ByRid(_))
    }
}

/// Item (document) identifier - either by name or by RID.
///
/// Enforces consistency: if item is by name, container must also be by name.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ItemId {
    /// Reference by user-provided names.
    ByName {
        /// Parent container identifier (by name).
        container: ContainerId,
        /// Item name (document ID).
        name: ResourceName,
    },
    /// Reference by internal RIDs.
    ByRid {
        /// Parent container RID.
        container_rid: ResourceRid,
        /// Item RID.
        rid: ResourceRid,
    },
}

impl ItemId {
    /// Returns the item name if this is a name-based identifier.
    pub(crate) fn name(&self) -> Option<&str> {
        match self {
            Self::ByName { name, .. } => Some(name.as_str()),
            Self::ByRid { .. } => None,
        }
    }

    /// Returns the item RID if this is a RID-based identifier.
    pub(crate) fn rid(&self) -> Option<&str> {
        match self {
            Self::ByName { .. } => None,
            Self::ByRid { rid, .. } => Some(rid.as_str()),
        }
    }

    /// Returns the container ID if this is a name-based identifier.
    pub(crate) fn container_id(&self) -> Option<&ContainerId> {
        match self {
            Self::ByName { container, .. } => Some(container),
            Self::ByRid { .. } => None,
        }
    }

    /// Returns the container RID if this is a RID-based identifier.
    pub(crate) fn container_rid(&self) -> Option<&str> {
        match self {
            Self::ByName { .. } => None,
            Self::ByRid { container_rid, .. } => Some(container_rid.as_str()),
        }
    }
}

/// Stored procedure identifier - either by name or by RID.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum StoredProcedureId {
    /// Reference by user-provided names.
    ByName {
        /// Parent container identifier (by name).
        container: ContainerId,
        /// Stored procedure name.
        name: ResourceName,
    },
    /// Reference by internal RIDs.
    ByRid {
        /// Parent container RID.
        container_rid: ResourceRid,
        /// Stored procedure RID.
        rid: ResourceRid,
    },
}

impl StoredProcedureId {
    /// Returns the stored procedure name if this is a name-based identifier.
    pub(crate) fn name(&self) -> Option<&str> {
        match self {
            Self::ByName { name, .. } => Some(name.as_str()),
            Self::ByRid { .. } => None,
        }
    }

    /// Returns the stored procedure RID if this is a RID-based identifier.
    pub(crate) fn rid(&self) -> Option<&str> {
        match self {
            Self::ByName { .. } => None,
            Self::ByRid { rid, .. } => Some(rid.as_str()),
        }
    }

    /// Returns the container ID if this is a name-based identifier.
    pub(crate) fn container_id(&self) -> Option<&ContainerId> {
        match self {
            Self::ByName { container, .. } => Some(container),
            Self::ByRid { .. } => None,
        }
    }

    /// Returns the container RID if this is a RID-based identifier.
    pub(crate) fn container_rid(&self) -> Option<&str> {
        match self {
            Self::ByName { .. } => None,
            Self::ByRid { container_rid, .. } => Some(container_rid.as_str()),
        }
    }
}

/// Trigger identifier - either by name or by RID.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum TriggerId {
    /// Reference by user-provided names.
    ByName {
        /// Parent container identifier (by name).
        container: ContainerId,
        /// Trigger name.
        name: ResourceName,
    },
    /// Reference by internal RIDs.
    ByRid {
        /// Parent container RID.
        container_rid: ResourceRid,
        /// Trigger RID.
        rid: ResourceRid,
    },
}

impl TriggerId {
    /// Returns the trigger name if this is a name-based identifier.
    pub(crate) fn name(&self) -> Option<&str> {
        match self {
            Self::ByName { name, .. } => Some(name.as_str()),
            Self::ByRid { .. } => None,
        }
    }

    /// Returns the trigger RID if this is a RID-based identifier.
    pub(crate) fn rid(&self) -> Option<&str> {
        match self {
            Self::ByName { .. } => None,
            Self::ByRid { rid, .. } => Some(rid.as_str()),
        }
    }

    /// Returns the container ID if this is a name-based identifier.
    pub(crate) fn container_id(&self) -> Option<&ContainerId> {
        match self {
            Self::ByName { container, .. } => Some(container),
            Self::ByRid { .. } => None,
        }
    }

    /// Returns the container RID if this is a RID-based identifier.
    pub(crate) fn container_rid(&self) -> Option<&str> {
        match self {
            Self::ByName { .. } => None,
            Self::ByRid { container_rid, .. } => Some(container_rid.as_str()),
        }
    }
}

/// User-defined function identifier - either by name or by RID.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum UdfId {
    /// Reference by user-provided names.
    ByName {
        /// Parent container identifier (by name).
        container: ContainerId,
        /// UDF name.
        name: ResourceName,
    },
    /// Reference by internal RIDs.
    ByRid {
        /// Parent container RID.
        container_rid: ResourceRid,
        /// UDF RID.
        rid: ResourceRid,
    },
}

impl UdfId {
    /// Returns the UDF name if this is a name-based identifier.
    pub(crate) fn name(&self) -> Option<&str> {
        match self {
            Self::ByName { name, .. } => Some(name.as_str()),
            Self::ByRid { .. } => None,
        }
    }

    /// Returns the UDF RID if this is a RID-based identifier.
    pub(crate) fn rid(&self) -> Option<&str> {
        match self {
            Self::ByName { .. } => None,
            Self::ByRid { rid, .. } => Some(rid.as_str()),
        }
    }

    /// Returns the container ID if this is a name-based identifier.
    pub(crate) fn container_id(&self) -> Option<&ContainerId> {
        match self {
            Self::ByName { container, .. } => Some(container),
            Self::ByRid { .. } => None,
        }
    }

    /// Returns the container RID if this is a RID-based identifier.
    pub(crate) fn container_rid(&self) -> Option<&str> {
        match self {
            Self::ByName { .. } => None,
            Self::ByRid { container_rid, .. } => Some(container_rid.as_str()),
        }
    }
}

/// Partition key range identifier.
///
/// Partition key ranges are internal resources identified by their ID string.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum PartitionKeyRangeId {
    /// Reference by range ID within a container (by name).
    ByName {
        /// Parent container identifier (by name).
        container: ContainerId,
        /// Partition key range ID.
        range_id: ResourceName,
    },
    /// Reference by range ID within a container (by RID).
    ByRid {
        /// Parent container RID.
        container_rid: ResourceRid,
        /// Partition key range ID.
        range_id: ResourceName,
    },
}

impl PartitionKeyRangeId {
    /// Returns the partition key range ID.
    pub(crate) fn range_id(&self) -> &str {
        match self {
            Self::ByName { range_id, .. } | Self::ByRid { range_id, .. } => range_id.as_str(),
        }
    }

    /// Returns the container ID if this is a name-based identifier.
    pub(crate) fn container_id(&self) -> Option<&ContainerId> {
        match self {
            Self::ByName { container, .. } => Some(container),
            Self::ByRid { .. } => None,
        }
    }

    /// Returns the container RID if this is a RID-based identifier.
    pub(crate) fn container_rid(&self) -> Option<&str> {
        match self {
            Self::ByName { .. } => None,
            Self::ByRid { container_rid, .. } => Some(container_rid.as_str()),
        }
    }
}

// =============================================================================
// ParsedResourceId (pub(crate))
// =============================================================================

/// Parsed components of a Cosmos DB RID.
///
/// RIDs encode the resource hierarchy. This struct extracts the individual
/// components for validation and path construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParsedResourceId {
    /// The database RID component (if present).
    database_rid: Option<ResourceRid>,
    /// The container/collection RID component (if present).
    container_rid: Option<ResourceRid>,
    /// The document/item RID component (if present).
    document_rid: Option<ResourceRid>,
}

impl ParsedResourceId {
    /// Creates an empty parsed resource ID.
    pub(crate) fn empty() -> Self {
        Self {
            database_rid: None,
            container_rid: None,
            document_rid: None,
        }
    }

    /// Creates a parsed resource ID for a database.
    pub(crate) fn database(database_rid: ResourceRid) -> Self {
        Self {
            database_rid: Some(database_rid),
            container_rid: None,
            document_rid: None,
        }
    }

    /// Creates a parsed resource ID for a container.
    pub(crate) fn container(database_rid: ResourceRid, container_rid: ResourceRid) -> Self {
        Self {
            database_rid: Some(database_rid),
            container_rid: Some(container_rid),
            document_rid: None,
        }
    }

    /// Creates a parsed resource ID for a document.
    pub(crate) fn document(
        database_rid: ResourceRid,
        container_rid: ResourceRid,
        document_rid: ResourceRid,
    ) -> Self {
        Self {
            database_rid: Some(database_rid),
            container_rid: Some(container_rid),
            document_rid: Some(document_rid),
        }
    }

    /// Returns the database RID component.
    pub(crate) fn database_rid(&self) -> Option<&ResourceRid> {
        self.database_rid.as_ref()
    }

    /// Returns the container RID component.
    pub(crate) fn container_rid(&self) -> Option<&ResourceRid> {
        self.container_rid.as_ref()
    }

    /// Returns the document RID component.
    pub(crate) fn document_rid(&self) -> Option<&ResourceRid> {
        self.document_rid.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_name_from_str() {
        let name = ResourceName::from("mydb");
        assert_eq!(name.as_str(), "mydb");
    }

    #[test]
    fn resource_name_from_string() {
        let name = ResourceName::from(String::from("mydb"));
        assert_eq!(name.as_str(), "mydb");
    }

    #[test]
    fn resource_rid_from_str() {
        let rid = ResourceRid::from("abc123");
        assert_eq!(rid.as_str(), "abc123");
    }

    #[test]
    fn database_id_by_name() {
        let id = DatabaseId::ByName(ResourceName::from("testdb"));
        assert_eq!(id.name(), Some("testdb"));
        assert_eq!(id.rid(), None);
    }

    #[test]
    fn database_id_by_rid() {
        let id = DatabaseId::ByRid(ResourceRid::from("abc123"));
        assert_eq!(id.name(), None);
        assert_eq!(id.rid(), Some("abc123"));
    }

    #[test]
    fn container_id_by_name() {
        let id = ContainerId::ByName {
            db_name: ResourceName::from("testdb"),
            name: ResourceName::from("testcontainer"),
        };
        assert_eq!(id.database_name(), Some("testdb"));
        assert_eq!(id.name(), Some("testcontainer"));
        assert_eq!(id.database_rid(), None);
        assert_eq!(id.rid(), None);
    }

    #[test]
    fn container_id_by_rid() {
        let id = ContainerId::ByRid {
            db_rid: ResourceRid::from("db123"),
            rid: ResourceRid::from("coll456"),
        };
        assert_eq!(id.database_name(), None);
        assert_eq!(id.name(), None);
        assert_eq!(id.database_rid(), Some("db123"));
        assert_eq!(id.rid(), Some("coll456"));
    }

    #[test]
    fn container_id_extracts_database_id() {
        let container_by_name = ContainerId::ByName {
            db_name: ResourceName::from("testdb"),
            name: ResourceName::from("testcontainer"),
        };
        let db_id = container_by_name.database_id();
        assert!(matches!(db_id, DatabaseId::ByName(_)));
        assert_eq!(db_id.name(), Some("testdb"));

        let container_by_rid = ContainerId::ByRid {
            db_rid: ResourceRid::from("db123"),
            rid: ResourceRid::from("coll456"),
        };
        let db_id = container_by_rid.database_id();
        assert!(matches!(db_id, DatabaseId::ByRid(_)));
        assert_eq!(db_id.rid(), Some("db123"));
    }

    #[test]
    fn item_id_by_name() {
        let container = ContainerId::ByName {
            db_name: ResourceName::from("testdb"),
            name: ResourceName::from("testcontainer"),
        };
        let id = ItemId::ByName {
            container,
            name: ResourceName::from("doc1"),
        };
        assert_eq!(id.name(), Some("doc1"));
        assert_eq!(id.rid(), None);
        assert!(id.container_id().is_some());
        assert_eq!(id.container_rid(), None);
    }

    #[test]
    fn item_id_by_rid() {
        let id = ItemId::ByRid {
            container_rid: ResourceRid::from("coll456"),
            rid: ResourceRid::from("doc789"),
        };
        assert_eq!(id.name(), None);
        assert_eq!(id.rid(), Some("doc789"));
        assert!(id.container_id().is_none());
        assert_eq!(id.container_rid(), Some("coll456"));
    }

    #[test]
    fn parsed_resource_id_database() {
        let parsed = ParsedResourceId::database(ResourceRid::from("db123"));
        assert_eq!(parsed.database_rid().map(|r| r.as_str()), Some("db123"));
        assert!(parsed.container_rid().is_none());
        assert!(parsed.document_rid().is_none());
    }

    #[test]
    fn parsed_resource_id_container() {
        let parsed =
            ParsedResourceId::container(ResourceRid::from("db123"), ResourceRid::from("coll456"));
        assert_eq!(parsed.database_rid().map(|r| r.as_str()), Some("db123"));
        assert_eq!(parsed.container_rid().map(|r| r.as_str()), Some("coll456"));
        assert!(parsed.document_rid().is_none());
    }

    #[test]
    fn parsed_resource_id_document() {
        let parsed = ParsedResourceId::document(
            ResourceRid::from("db123"),
            ResourceRid::from("coll456"),
            ResourceRid::from("doc789"),
        );
        assert_eq!(parsed.database_rid().map(|r| r.as_str()), Some("db123"));
        assert_eq!(parsed.container_rid().map(|r| r.as_str()), Some("coll456"));
        assert_eq!(parsed.document_rid().map(|r| r.as_str()), Some("doc789"));
    }
}
