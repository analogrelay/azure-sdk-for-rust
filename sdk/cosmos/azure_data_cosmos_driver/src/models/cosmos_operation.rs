// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Cosmos DB operation representation.

use crate::models::{
    AccountReference, ContainerReference, CosmosResourceReference, DatabaseReference,
    OperationType, PartitionKey, ResourceType,
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

    // ===== Control Plane Factory Methods =====

    /// Creates a database in the account.
    ///
    /// Use `with_body()` to provide the database properties JSON:
    /// ```json
    /// {"id": "my-database"}
    /// ```
    ///
    /// # Example
    ///
    /// ```
    /// use azure_data_cosmos_driver::models::{AccountReference, CosmosOperation};
    /// use url::Url;
    ///
    /// let account = AccountReference::with_master_key(
    ///     Url::parse("https://myaccount.documents.azure.com:443/").unwrap(),
    ///     "my-key",
    /// );
    ///
    /// let operation = CosmosOperation::create_database(account)
    ///     .with_body(br#"{"id": "my-database"}"#.to_vec());
    /// ```
    pub fn create_database(account: AccountReference) -> Self {
        let resource_ref = CosmosResourceReference::databases_collection(account);
        Self::new(OperationType::Create, resource_ref)
    }

    /// Reads (lists) all databases in the account.
    ///
    /// Returns a feed of database resources.
    pub fn read_all_databases(account: AccountReference) -> Self {
        let resource_ref = CosmosResourceReference::databases_collection(account);
        Self::new(OperationType::ReadFeed, resource_ref)
    }

    /// Queries databases in the account.
    ///
    /// Use `with_body()` to provide the query JSON.
    pub fn query_databases(account: AccountReference) -> Self {
        let resource_ref = CosmosResourceReference::databases_collection(account);
        Self::new(OperationType::Query, resource_ref)
    }

    /// Deletes a database.
    ///
    /// # Example
    ///
    /// ```
    /// use azure_data_cosmos_driver::models::{
    ///     AccountReference, CosmosOperation, DatabaseReference,
    /// };
    /// use url::Url;
    ///
    /// let account = AccountReference::with_master_key(
    ///     Url::parse("https://myaccount.documents.azure.com:443/").unwrap(),
    ///     "my-key",
    /// );
    ///
    /// let database = DatabaseReference::from_name(account, "my-database");
    /// let operation = CosmosOperation::delete_database(database);
    /// ```
    pub fn delete_database(database: DatabaseReference) -> Self {
        let resource_ref = CosmosResourceReference::database_by_name(database);
        Self::new(OperationType::Delete, resource_ref)
    }

    /// Creates a container in a database.
    ///
    /// Use `with_body()` to provide the container properties JSON:
    /// ```json
    /// {"id": "my-container", "partitionKey": {"paths": ["/pk"], "kind": "Hash"}}
    /// ```
    ///
    /// # Example
    ///
    /// ```
    /// use azure_data_cosmos_driver::models::{
    ///     AccountReference, CosmosOperation, DatabaseReference,
    /// };
    /// use url::Url;
    ///
    /// let account = AccountReference::with_master_key(
    ///     Url::parse("https://myaccount.documents.azure.com:443/").unwrap(),
    ///     "my-key",
    /// );
    ///
    /// let database = DatabaseReference::from_name(account, "my-database");
    /// let operation = CosmosOperation::create_container(database)
    ///     .with_body(br#"{"id": "my-container", "partitionKey": {"paths": ["/pk"], "kind": "Hash"}}"#.to_vec());
    /// ```
    pub fn create_container(database: DatabaseReference) -> Self {
        let resource_ref = CosmosResourceReference::containers_collection(database);
        Self::new(OperationType::Create, resource_ref)
    }

    /// Reads (lists) all containers in a database.
    ///
    /// Returns a feed of container resources.
    pub fn read_all_containers(database: DatabaseReference) -> Self {
        let resource_ref = CosmosResourceReference::containers_collection(database);
        Self::new(OperationType::ReadFeed, resource_ref)
    }

    /// Queries containers in a database.
    ///
    /// Use `with_body()` to provide the query JSON.
    pub fn query_containers(database: DatabaseReference) -> Self {
        let resource_ref = CosmosResourceReference::containers_collection(database);
        Self::new(OperationType::Query, resource_ref)
    }

    /// Deletes a container.
    ///
    /// # Example
    ///
    /// ```
    /// use azure_data_cosmos_driver::models::{
    ///     AccountReference, ContainerReference, CosmosOperation,
    /// };
    /// use url::Url;
    ///
    /// let account = AccountReference::with_master_key(
    ///     Url::parse("https://myaccount.documents.azure.com:443/").unwrap(),
    ///     "my-key",
    /// );
    ///
    /// let container = ContainerReference::from_name(account, "my-database", "my-container");
    /// let operation = CosmosOperation::delete_container(container);
    /// ```
    pub fn delete_container(container: ContainerReference) -> Self {
        let resource_ref = CosmosResourceReference::document_collection_by_name(container);
        Self::new(OperationType::Delete, resource_ref)
    }

    // ===== Data Plane Factory Methods =====

    /// Creates an item (document) in a container.
    ///
    /// Use `with_partition_key()` to set the partition key and `with_body()` to provide
    /// the document JSON.
    ///
    /// # Example
    ///
    /// ```
    /// use azure_data_cosmos_driver::models::{
    ///     AccountReference, ContainerReference, CosmosOperation, PartitionKey,
    /// };
    /// use url::Url;
    ///
    /// let account = AccountReference::with_master_key(
    ///     Url::parse("https://myaccount.documents.azure.com:443/").unwrap(),
    ///     "my-key",
    /// );
    ///
    /// let container = ContainerReference::from_name(account, "my-database", "my-container");
    /// let operation = CosmosOperation::create_item(container)
    ///     .with_partition_key(PartitionKey::from("pk-value"))
    ///     .with_body(br#"{"id": "doc1", "pk": "pk-value", "data": "hello"}"#.to_vec());
    /// ```
    pub fn create_item(container: ContainerReference) -> Self {
        let resource_ref = CosmosResourceReference::documents_collection(container);
        Self::new(OperationType::Create, resource_ref)
    }

    /// Reads an item (document) from a container.
    ///
    /// Use `with_partition_key()` to set the partition key.
    ///
    /// # Example
    ///
    /// ```
    /// use azure_data_cosmos_driver::models::{
    ///     AccountReference, ContainerReference, CosmosOperation, CosmosResourceReference, PartitionKey,
    /// };
    /// use url::Url;
    ///
    /// let account = AccountReference::with_master_key(
    ///     Url::parse("https://myaccount.documents.azure.com:443/").unwrap(),
    ///     "my-key",
    /// );
    ///
    /// let container = ContainerReference::from_name(account, "my-database", "my-container");
    /// let item_ref = CosmosResourceReference::document_by_name(container, "doc1");
    /// let operation = CosmosOperation::read_item(item_ref)
    ///     .with_partition_key(PartitionKey::from("pk-value"));
    /// ```
    pub fn read_item(item_reference: impl Into<CosmosResourceReference>) -> Self {
        Self::new(OperationType::Read, item_reference)
    }

    /// Deletes an item (document) from a container.
    ///
    /// Use `with_partition_key()` to set the partition key.
    pub fn delete_item(item_reference: impl Into<CosmosResourceReference>) -> Self {
        Self::new(OperationType::Delete, item_reference)
    }

    /// Reads (lists) all items in a container.
    ///
    /// Returns a feed of document resources.
    pub fn read_all_items(container: ContainerReference) -> Self {
        let resource_ref = CosmosResourceReference::documents_collection(container);
        Self::new(OperationType::ReadFeed, resource_ref)
    }

    /// Queries items in a container.
    ///
    /// Use `with_partition_key()` to scope the query to a partition and
    /// `with_body()` to provide the query JSON.
    pub fn query_items(container: ContainerReference) -> Self {
        let resource_ref = CosmosResourceReference::documents_collection(container);
        Self::new(OperationType::Query, resource_ref)
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
