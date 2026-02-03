// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Resource and operation type enumerations.

/// The type of resource being operated on.
///
/// Used to identify the Cosmos DB resource category for routing and authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ResourceType {
    /// Database account (root level).
    DatabaseAccount,
    /// A database within an account.
    Database,
    /// A container (collection) within a database.
    DocumentCollection,
    /// A document (item) within a container.
    Document,
    /// A stored procedure within a container.
    StoredProcedure,
    /// A trigger within a container.
    Trigger,
    /// A user-defined function within a container.
    UserDefinedFunction,
    /// A partition key range within a container.
    PartitionKeyRange,
    /// An offer (throughput configuration).
    Offer,
}

impl ResourceType {
    /// Returns the URL path segment for this resource type.
    pub fn path_segment(self) -> &'static str {
        match self {
            ResourceType::DatabaseAccount => "",
            ResourceType::Database => "dbs",
            ResourceType::DocumentCollection => "colls",
            ResourceType::Document => "docs",
            ResourceType::StoredProcedure => "sprocs",
            ResourceType::Trigger => "triggers",
            ResourceType::UserDefinedFunction => "udfs",
            ResourceType::PartitionKeyRange => "pkranges",
            ResourceType::Offer => "offers",
        }
    }

    /// Returns true if this resource type is metadata (not data plane items).
    pub fn is_metadata(self) -> bool {
        matches!(
            self,
            ResourceType::DatabaseAccount
                | ResourceType::Database
                | ResourceType::DocumentCollection
                | ResourceType::PartitionKeyRange
                | ResourceType::Offer
        )
    }

    /// Returns true if this resource type requires a container reference.
    pub fn requires_container(self) -> bool {
        matches!(
            self,
            ResourceType::Document
                | ResourceType::DocumentCollection
                | ResourceType::StoredProcedure
                | ResourceType::Trigger
                | ResourceType::UserDefinedFunction
                | ResourceType::PartitionKeyRange
        )
    }

    /// Returns true if this resource type requires a database reference.
    pub fn requires_database(self) -> bool {
        matches!(
            self,
            ResourceType::Database
                | ResourceType::DocumentCollection
                | ResourceType::Document
                | ResourceType::StoredProcedure
                | ResourceType::Trigger
                | ResourceType::UserDefinedFunction
                | ResourceType::PartitionKeyRange
        )
    }
}

/// The type of operation being performed.
///
/// Used to determine HTTP method, retry behavior, and authorization requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OperationType {
    /// Create a new resource.
    Create,
    /// Read an existing resource.
    Read,
    /// Read a feed (list) of resources.
    ReadFeed,
    /// Replace an existing resource.
    Replace,
    /// Delete a resource.
    Delete,
    /// Create or replace a resource.
    Upsert,
    /// Execute a query.
    Query,
    /// Execute a SQL query.
    SqlQuery,
    /// Get a query plan.
    QueryPlan,
    /// Execute a batch operation.
    Batch,
    /// Partially update a resource.
    Patch,
    /// Check resource existence (HEAD).
    Head,
    /// Check feed existence (HEAD).
    HeadFeed,
    /// Execute a stored procedure.
    Execute,
}

impl OperationType {
    /// Returns true if the operation does not modify server state.
    pub fn is_read_only(self) -> bool {
        matches!(
            self,
            OperationType::Read
                | OperationType::ReadFeed
                | OperationType::Query
                | OperationType::SqlQuery
                | OperationType::QueryPlan
                | OperationType::Head
                | OperationType::HeadFeed
        )
    }

    /// Returns true if the operation is idempotent (safe to retry).
    pub fn is_idempotent(self) -> bool {
        matches!(
            self,
            OperationType::Read
                | OperationType::ReadFeed
                | OperationType::Query
                | OperationType::SqlQuery
                | OperationType::QueryPlan
                | OperationType::Head
                | OperationType::HeadFeed
                | OperationType::Replace
                | OperationType::Delete
        )
    }
}
