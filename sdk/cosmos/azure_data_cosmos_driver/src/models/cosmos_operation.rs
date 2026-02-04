// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Cosmos DB operation representation.

use crate::models::{
    ContainerReference, CosmosResourceReference, OperationType, PartitionKey, ResourceType,
};
use azure_core::http::headers::Headers;

/// Represents a Cosmos DB operation with its routing and execution context.
///
/// This is the driver's internal representation of an operation before it is
/// converted into a wire-level HTTP request. It captures the operation intent
/// (create/read/query/etc.), resource routing information, and optional
/// operation-specific settings.
///
/// # Immutable Fields
///
/// The `operation_type` and `resource_type` fields are set at construction time
/// and cannot be changed. Use the factory methods to create operations with the
/// correct types.
///
/// # Examples
///
/// ```
/// use azure_data_cosmos_driver::models::{
///     AccountReference, ContainerReference, CosmosOperation, CosmosResourceReference,
///     DatabaseReference, ItemReference, OperationType, PartitionKey,
/// };
/// use url::Url;
///
/// let account = AccountReference::with_master_key(
///     Url::parse("https://myaccount.documents.azure.com:443/").unwrap(),
///     "my-key",
/// );
///
/// // Using typed ItemReference (recommended)
/// let item_ref = ItemReference::from_name(account.clone(), "mydb", "mycontainer", "doc1");
/// let operation = CosmosOperation::read(item_ref)
///     .with_partition_key(PartitionKey::from("partition1"));
///
/// // Or using CosmosResourceReference directly
/// let container = ContainerReference::from_name(account, "mydb", "mycontainer");
/// let operation = CosmosOperation::read(
///     CosmosResourceReference::document_by_name(container, "doc1"),
/// )
/// .with_partition_key(PartitionKey::from("partition1"));
/// ```
#[derive(Clone, Debug)]
pub struct CosmosOperation {
    /// The type of operation (immutable after construction).
    operation_type: OperationType,
    /// The type of resource (derived from resource reference, immutable).
    resource_type: ResourceType,
    /// Reference to the resource being operated on.
    resource_reference: CosmosResourceReference,
    /// Optional partition key for data plane operations.
    partition_key: Option<PartitionKey>,
    /// Additional headers to include in the request.
    headers: Headers,
    /// Optional request body (raw bytes, schema-agnostic).
    body: Option<Vec<u8>>,
}

impl CosmosOperation {
    /// Returns the operation type.
    pub fn operation_type(&self) -> OperationType {
        self.operation_type
    }

    /// Returns the resource type.
    pub fn resource_type(&self) -> ResourceType {
        self.resource_type
    }

    /// Returns a reference to the resource being operated on.
    pub fn resource_reference(&self) -> &CosmosResourceReference {
        &self.resource_reference
    }

    /// Returns the container for this operation, if applicable.
    ///
    /// Returns `None` for account-level and database-level operations.
    pub fn container(&self) -> Option<&ContainerReference> {
        self.resource_reference.container()
    }

    /// Returns the partition key, if set.
    pub fn partition_key(&self) -> Option<&PartitionKey> {
        self.partition_key.as_ref()
    }

    /// Returns the additional headers.
    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Returns the request body, if set.
    pub fn body(&self) -> Option<&[u8]> {
        self.body.as_deref()
    }

    /// Sets the partition key for the operation.
    #[must_use]
    pub fn with_partition_key(mut self, partition_key: impl Into<PartitionKey>) -> Self {
        self.partition_key = Some(partition_key.into());
        self
    }

    /// Adds a header to the operation.
    #[must_use]
    pub fn with_header(
        mut self,
        name: impl Into<azure_core::http::headers::HeaderName>,
        value: impl Into<azure_core::http::headers::HeaderValue>,
    ) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Sets the request body.
    #[must_use]
    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    // ===== Factory Methods =====

    /// Creates a new operation with the specified type and resource reference.
    fn new(
        operation_type: OperationType,
        resource_reference: impl Into<CosmosResourceReference>,
    ) -> Self {
        let resource_reference = resource_reference.into();
        let resource_type = resource_reference.resource_type();
        Self {
            operation_type,
            resource_type,
            resource_reference,
            partition_key: None,
            headers: Headers::new(),
            body: None,
        }
    }

    /// Creates a Create operation.
    ///
    /// Accepts any type that can be converted into a `CosmosResourceReference`,
    /// including typed references like `ItemReference`, `ContainerReference`, etc.
    pub fn create(resource_reference: impl Into<CosmosResourceReference>) -> Self {
        Self::new(OperationType::Create, resource_reference)
    }

    /// Creates a Read operation.
    ///
    /// Accepts any type that can be converted into a `CosmosResourceReference`,
    /// including typed references like `ItemReference`, `ContainerReference`, etc.
    pub fn read(resource_reference: impl Into<CosmosResourceReference>) -> Self {
        Self::new(OperationType::Read, resource_reference)
    }

    /// Creates a ReadFeed operation.
    ///
    /// Accepts any type that can be converted into a `CosmosResourceReference`,
    /// including typed references like `ContainerReference`, `DatabaseReference`, etc.
    pub fn read_feed(resource_reference: impl Into<CosmosResourceReference>) -> Self {
        Self::new(OperationType::ReadFeed, resource_reference)
    }

    /// Creates a Replace operation.
    ///
    /// Accepts any type that can be converted into a `CosmosResourceReference`,
    /// including typed references like `ItemReference`, `ContainerReference`, etc.
    pub fn replace(resource_reference: impl Into<CosmosResourceReference>) -> Self {
        Self::new(OperationType::Replace, resource_reference)
    }

    /// Creates a Delete operation.
    ///
    /// Accepts any type that can be converted into a `CosmosResourceReference`,
    /// including typed references like `ItemReference`, `ContainerReference`, etc.
    pub fn delete(resource_reference: impl Into<CosmosResourceReference>) -> Self {
        Self::new(OperationType::Delete, resource_reference)
    }

    /// Creates an Upsert operation.
    ///
    /// Accepts any type that can be converted into a `CosmosResourceReference`,
    /// including typed references like `ItemReference`.
    pub fn upsert(resource_reference: impl Into<CosmosResourceReference>) -> Self {
        Self::new(OperationType::Upsert, resource_reference)
    }

    /// Creates a Query operation.
    ///
    /// Accepts any type that can be converted into a `CosmosResourceReference`,
    /// including typed references like `ContainerReference`.
    pub fn query(resource_reference: impl Into<CosmosResourceReference>) -> Self {
        Self::new(OperationType::Query, resource_reference)
    }

    /// Creates an Execute operation (for stored procedures).
    ///
    /// Accepts any type that can be converted into a `CosmosResourceReference`,
    /// including `StoredProcedureReference`.
    pub fn execute(resource_reference: impl Into<CosmosResourceReference>) -> Self {
        Self::new(OperationType::Execute, resource_reference)
    }

    /// Creates a Patch operation.
    ///
    /// Accepts any type that can be converted into a `CosmosResourceReference`,
    /// including typed references like `ItemReference`.
    pub fn patch(resource_reference: impl Into<CosmosResourceReference>) -> Self {
        Self::new(OperationType::Patch, resource_reference)
    }

    /// Creates a Batch operation.
    ///
    /// Accepts any type that can be converted into a `CosmosResourceReference`,
    /// including typed references like `ContainerReference`.
    pub fn batch(resource_reference: impl Into<CosmosResourceReference>) -> Self {
        Self::new(OperationType::Batch, resource_reference)
    }

    /// Creates a Head operation.
    ///
    /// Accepts any type that can be converted into a `CosmosResourceReference`,
    /// including typed references like `ItemReference`, `ContainerReference`, etc.
    pub fn head(resource_reference: impl Into<CosmosResourceReference>) -> Self {
        Self::new(OperationType::Head, resource_reference)
    }

    /// Returns true if this is a read-only operation.
    pub fn is_read_only(&self) -> bool {
        self.operation_type.is_read_only()
    }

    /// Returns true if this operation is idempotent.
    pub fn is_idempotent(&self) -> bool {
        self.operation_type.is_idempotent()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AccountReference, ContainerReference, DatabaseReference};
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

    fn test_container() -> ContainerReference {
        ContainerReference::from_database(&test_database(), "testcontainer")
    }

    #[test]
    fn create_operation() {
        let resource_ref = CosmosResourceReference::document_by_name(test_container(), "doc1");
        let op = CosmosOperation::create(resource_ref);

        assert_eq!(op.operation_type(), OperationType::Create);
        assert_eq!(op.resource_type(), ResourceType::Document);
        assert!(!op.is_read_only());
        assert!(!op.is_idempotent());
    }

    #[test]
    fn read_operation() {
        let resource_ref = CosmosResourceReference::document_by_name(test_container(), "doc1");
        let op = CosmosOperation::read(resource_ref);

        assert_eq!(op.operation_type(), OperationType::Read);
        assert_eq!(op.resource_type(), ResourceType::Document);
        assert!(op.is_read_only());
        assert!(op.is_idempotent());
    }

    #[test]
    fn operation_with_partition_key() {
        let resource_ref = CosmosResourceReference::document_by_name(test_container(), "doc1");
        let op = CosmosOperation::read(resource_ref).with_partition_key(PartitionKey::from("pk1"));

        assert!(op.partition_key().is_some());
    }

    #[test]
    fn operation_with_body() {
        let resource_ref = CosmosResourceReference::document_by_name(test_container(), "doc1");
        let body = b"{\"id\":\"doc1\"}".to_vec();
        let op = CosmosOperation::create(resource_ref).with_body(body.clone());

        assert_eq!(op.body(), Some(body.as_slice()));
    }

    #[test]
    fn replace_is_idempotent() {
        let resource_ref = CosmosResourceReference::document_by_name(test_container(), "doc1");
        let op = CosmosOperation::replace(resource_ref);

        assert!(!op.is_read_only());
        assert!(op.is_idempotent());
    }

    #[test]
    fn upsert_is_not_idempotent() {
        let resource_ref = CosmosResourceReference::document_by_name(test_container(), "doc1");
        let op = CosmosOperation::upsert(resource_ref);

        assert!(!op.is_read_only());
        assert!(!op.is_idempotent());
    }
}
