// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Data models for Cosmos DB management and metadata operations.
//!
//! This module contains types representing Cosmos DB resources (accounts, databases, containers)
//! and their supporting structures. These are for **metadata/management operations only**.
//!
//! **Important**: This module does NOT contain data plane item/document types.
//! The driver is schema-agnostic - data plane operations work with raw bytes (`&[u8]`).

mod account_reference;
mod activity_id;
mod cosmos_operation;
mod cosmos_resource_reference;
mod cosmos_result;
mod etag;
mod partition_key;
mod resource_id;
mod resource_reference;
mod resource_types;
mod session;
mod sub_status_code;
mod throughput_control;
mod triggers;
mod user_agent;

pub use account_reference::{AccountReference, AccountReferenceBuilder, AuthOptions, MasterKey};
pub use activity_id::ActivityId;
pub use cosmos_operation::CosmosOperation;
pub use cosmos_resource_reference::CosmosResourceReference;
pub use cosmos_result::{CosmosHeaders, CosmosResult};
pub use etag::{ETag, ETagCondition};
pub use partition_key::{PartitionKey, PartitionKeyValue};
pub use resource_id::{ResourceName, ResourceRid};
pub use resource_reference::{
    ContainerReference, DatabaseReference, ItemReference, StoredProcedureReference,
    TriggerReference, UdfReference,
};
pub use resource_types::{OperationType, ResourceType};
pub use session::SessionToken;
pub use sub_status_code::SubStatusCode;
pub use throughput_control::ThroughputControlGroupName;
pub use triggers::TriggerInvocation;
pub use user_agent::UserAgent;

pub(crate) use account_reference::AccountEndpoint;

use crate::options::Region;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Properties of a Cosmos DB account.
///
/// Contains metadata about a Cosmos DB account including its regions and capabilities.
/// Used internally by the driver for routing and caching.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) struct AccountProperties {
    /// The account's primary (write) region.
    pub write_region: Region,
    /// All readable regions for this account (ordered by preference).
    pub read_regions: Vec<Region>,
    /// The system-assigned resource ID for the account.
    pub rid: Option<String>,
}

impl AccountProperties {
    /// Creates new account properties.
    pub fn new(write_region: Region, read_regions: Vec<Region>) -> Self {
        Self {
            write_region,
            read_regions,
            rid: None,
        }
    }

    /// Sets the account's resource ID.
    #[must_use]
    pub fn with_rid(mut self, rid: impl Into<String>) -> Self {
        self.rid = Some(rid.into());
        self
    }
}

/// Properties of a Cosmos DB database.
///
/// Returned by database read/query operations and used when creating databases.
#[derive(Clone, Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct DatabaseProperties {
    /// Unique identifier for the database within the account.
    pub id: Cow<'static, str>,

    /// System-managed properties (e.g., _rid, _ts, _etag).
    #[serde(flatten)]
    pub system_properties: SystemProperties,
}

/// Properties of a Cosmos DB container.
///
/// Returned by container read/query operations and used when creating/updating containers.
#[derive(Clone, Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct ContainerProperties {
    /// Unique identifier for the container within the database.
    pub id: Cow<'static, str>,

    /// Partition key definition specifying the partition key path(s).
    pub partition_key: PartitionKeyDefinition,

    /// Optional indexing policy controlling how items are indexed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexing_policy: Option<IndexingPolicy>,

    /// System-managed properties (e.g., _rid, _ts, _etag).
    #[serde(flatten)]
    pub system_properties: SystemProperties,
}

/// Partition key definition for a container.
///
/// Specifies the JSON path(s) used for partitioning data across physical partitions.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct PartitionKeyDefinition {
    /// List of partition key paths (e.g., `["/tenantId"]` for single partition key).
    pub paths: Vec<Cow<'static, str>>,

    /// Partition key version (1 for single, 2 for hierarchical).
    #[serde(default = "default_pk_version")]
    pub version: u32,

    /// Partition key kind (Hash is the standard).
    #[serde(default)]
    pub kind: PartitionKeyKind,
}

impl Default for PartitionKeyDefinition {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            version: 2,
            kind: PartitionKeyKind::Hash,
        }
    }
}

fn default_pk_version() -> u32 {
    2
}

/// Partition key kind.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum PartitionKeyKind {
    /// Hash partitioning (standard).
    #[default]
    Hash,
    /// Range partitioning (legacy).
    Range,
}

/// Indexing policy for a container.
///
/// Controls how items are indexed for query performance.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct IndexingPolicy {
    /// Indexing mode.
    #[serde(default)]
    pub indexing_mode: IndexingMode,

    /// Whether indexing is automatic.
    #[serde(default = "default_true")]
    pub automatic: bool,
}

fn default_true() -> bool {
    true
}

/// Indexing mode.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum IndexingMode {
    /// Items are indexed synchronously.
    #[default]
    Consistent,
    /// Items are indexed asynchronously.
    Lazy,
    /// Indexing is disabled.
    None,
}

/// System-managed properties present on all Cosmos DB resources.
#[derive(Clone, Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SystemProperties {
    /// Resource ID (internal identifier).
    #[serde(rename = "_rid", skip_serializing_if = "Option::is_none")]
    pub rid: Option<String>,

    /// Resource timestamp (last modified time in Unix epoch seconds).
    #[serde(rename = "_ts", skip_serializing_if = "Option::is_none")]
    pub ts: Option<u64>,

    /// ETag for optimistic concurrency control.
    #[serde(rename = "_etag", skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
}
