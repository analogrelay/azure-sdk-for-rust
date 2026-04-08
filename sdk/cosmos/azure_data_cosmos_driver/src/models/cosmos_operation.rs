// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Cosmos DB operation representation.

use crate::models::{
    AccountReference, ContainerReference, CosmosRequestHeaders, CosmosResourceReference,
    DatabaseReference, ItemReference, OperationType, PartitionKey, Precondition, ResourceType,
};

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
/// ```no_run
/// use azure_data_cosmos_driver::driver::CosmosDriverRuntime;
/// use azure_data_cosmos_driver::models::{
///     AccountReference, CosmosOperation,
///     ItemReference, PartitionKey,
/// };
/// use azure_data_cosmos_driver::options::OperationOptions;
/// use url::Url;
///
/// # async fn example() -> azure_core::Result<()> {
/// // 1. Set up runtime and driver
/// let runtime = CosmosDriverRuntime::builder().build().await?;
/// let account = AccountReference::with_master_key(
///     Url::parse("https://myaccount.documents.azure.com:443/").unwrap(),
///     "my-key",
/// );
/// let driver = runtime.get_or_create_driver(account, None).await?;
///
/// // 2. Resolve the container (reads database + container from service, caches result)
/// let container = driver.resolve_container("mydb", "mycontainer").await?;
///
/// // 3. Build and execute item operations
/// let item = ItemReference::from_name(&container, PartitionKey::from("pk1"), "doc1");
/// let result = driver
///     .execute_operation(CosmosOperation::read_item(item), OperationOptions::default())
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CosmosOperation {
    /// The type of operation (immutable after construction).
    operation_type: OperationType,
    /// The type of resource (derived from resource reference, immutable).
    resource_type: ResourceType,
    /// Reference to the resource being operated on.
    resource_reference: CosmosResourceReference,
    /// Optional partition key for data plane operations.
    partition_key: Option<PartitionKey>,
    /// Additional request headers to include in the request.
    request_headers: CosmosRequestHeaders,
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
    pub(crate) fn resource_reference(&self) -> &CosmosResourceReference {
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

    /// Returns the request headers.
    pub fn request_headers(&self) -> &CosmosRequestHeaders {
        &self.request_headers
    }

    /// Returns the request body, if set.
    pub fn body(&self) -> Option<&[u8]> {
        self.body.as_deref()
    }

    /// Sets the partition key for the operation.
    pub fn with_partition_key(mut self, partition_key: impl Into<PartitionKey>) -> Self {
        self.partition_key = Some(partition_key.into());
        self
    }

    /// Sets request headers for the operation.
    pub fn with_request_headers(mut self, headers: CosmosRequestHeaders) -> Self {
        self.request_headers = headers;
        self
    }

    /// Sets the session token request header for the operation.
    pub fn with_session_token(
        mut self,
        session_token: impl Into<crate::models::SessionToken>,
    ) -> Self {
        self.request_headers.session_token = Some(session_token.into());
        self
    }

    /// Sets the activity ID request header for the operation.
    pub fn with_activity_id(mut self, activity_id: crate::models::ActivityId) -> Self {
        self.request_headers.activity_id = Some(activity_id);
        self
    }

    /// Sets the precondition for optimistic concurrency control.
    pub fn with_precondition(mut self, precondition: Precondition) -> Self {
        self.request_headers.precondition = Some(precondition);
        self
    }

    /// Returns the precondition, if set.
    pub fn precondition(&self) -> Option<&Precondition> {
        self.request_headers.precondition.as_ref()
    }

    /// Sets the request body.
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
            request_headers: CosmosRequestHeaders::new(),
            body: None,
        }
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
        let resource_ref: CosmosResourceReference = CosmosResourceReference::from(account)
            .with_resource_type(ResourceType::Database)
            .into_feed_reference();
        Self::new(OperationType::Create, resource_ref)
    }

    /// Reads (lists) all databases in the account.
    ///
    /// Returns a feed of database resources.
    pub fn read_all_databases(account: AccountReference) -> Self {
        let resource_ref = Into::<CosmosResourceReference>::into(account)
            .with_resource_type(ResourceType::Database)
            .into_feed_reference();
        Self::new(OperationType::ReadFeed, resource_ref)
    }

    /// Queries databases in the account.
    ///
    /// Use `with_body()` to provide the query JSON.
    pub fn query_databases(account: AccountReference) -> Self {
        let resource_ref: CosmosResourceReference = CosmosResourceReference::from(account)
            .with_resource_type(ResourceType::Database)
            .into_feed_reference();
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
        let resource_ref: CosmosResourceReference = database.into();
        Self::new(OperationType::Delete, resource_ref)
    }

    /// Reads a database's properties from the service.
    ///
    /// Returns the database properties payload, including
    /// the system-managed `_rid`, `_ts`, and `_etag`.
    pub fn read_database(database: DatabaseReference) -> Self {
        let resource_ref: CosmosResourceReference = database.into();
        Self::new(OperationType::Read, resource_ref)
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
        let resource_ref: CosmosResourceReference = CosmosResourceReference::from(database)
            .with_resource_type(ResourceType::DocumentCollection)
            .into_feed_reference();
        Self::new(OperationType::Create, resource_ref)
    }

    /// Reads (lists) all containers in a database.
    ///
    /// Returns a feed of container resources.
    pub fn read_all_containers(database: DatabaseReference) -> Self {
        let resource_ref: CosmosResourceReference = CosmosResourceReference::from(database)
            .with_resource_type(ResourceType::DocumentCollection)
            .into_feed_reference();
        Self::new(OperationType::ReadFeed, resource_ref)
    }

    /// Queries containers in a database.
    ///
    /// Use `with_body()` to provide the query JSON.
    pub fn query_containers(database: DatabaseReference) -> Self {
        let resource_ref: CosmosResourceReference = CosmosResourceReference::from(database)
            .with_resource_type(ResourceType::DocumentCollection)
            .into_feed_reference();
        Self::new(OperationType::Query, resource_ref)
    }

    /// Deletes a container.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use azure_data_cosmos_driver::driver::CosmosDriverRuntime;
    /// use azure_data_cosmos_driver::models::{
    ///     AccountReference, CosmosOperation,
    /// };
    /// use azure_data_cosmos_driver::options::OperationOptions;
    /// use url::Url;
    ///
    /// # async fn example() -> azure_core::Result<()> {
    /// let runtime = CosmosDriverRuntime::builder().build().await?;
    /// let account = AccountReference::with_master_key(
    ///     Url::parse("https://myaccount.documents.azure.com:443/").unwrap(),
    ///     "my-key",
    /// );
    /// let driver = runtime.get_or_create_driver(account, None).await?;
    /// let container = driver.resolve_container("my-database", "my-container").await?;
    ///
    /// let result = driver
    ///     .execute_operation(
    ///         CosmosOperation::delete_container(container),
    ///         OperationOptions::default(),
    ///     )
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn delete_container(container: ContainerReference) -> Self {
        let resource_ref: CosmosResourceReference = container.into();
        Self::new(OperationType::Delete, resource_ref)
    }

    /// Reads a container's properties from the service.
    ///
    /// Returns the full container properties payload for the container,
    /// including system-managed properties like `_rid`, `_ts`, and `_etag`.
    pub fn read_container(container: ContainerReference) -> Self {
        let resource_ref: CosmosResourceReference = container.into();
        Self::new(OperationType::Read, resource_ref)
    }

    /// Reads a container's properties by database and container name.
    ///
    /// Unlike [`read_container`](Self::read_container), this does not require an
    /// already-resolved `ContainerReference`. Use this for initial container
    /// resolution when only the names are known.
    pub fn read_container_by_name(
        database: DatabaseReference,
        container_name: impl Into<std::borrow::Cow<'static, str>>,
    ) -> Self {
        let resource_ref: CosmosResourceReference = CosmosResourceReference::from(database)
            .with_resource_type(ResourceType::DocumentCollection)
            .with_name(container_name.into());
        Self::new(OperationType::Read, resource_ref)
    }

    /// Reads a container's properties by database RID and container RID.
    pub fn read_container_by_rid(
        database: DatabaseReference,
        container_rid: impl Into<std::borrow::Cow<'static, str>>,
    ) -> Self {
        let resource_ref: CosmosResourceReference = CosmosResourceReference::from(database)
            .with_resource_type(ResourceType::DocumentCollection)
            .with_rid(container_rid.into());
        Self::new(OperationType::Read, resource_ref)
    }

    // ===== Data Plane Factory Methods =====

    /// Creates an item (document) in a container.
    ///
    /// The `container` and `partition_key` identify where to create the document.
    /// Use `with_body()` to provide the document JSON.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use azure_data_cosmos_driver::driver::CosmosDriverRuntime;
    /// use azure_data_cosmos_driver::models::{
    ///     AccountReference, CosmosOperation, ContainerReference, PartitionKey,
    /// };
    /// use azure_data_cosmos_driver::options::OperationOptions;
    /// use url::Url;
    ///
    /// # async fn example() -> azure_core::Result<()> {
    /// let runtime = CosmosDriverRuntime::builder().build().await?;
    /// let account = AccountReference::with_master_key(
    ///     Url::parse("https://myaccount.documents.azure.com:443/").unwrap(),
    ///     "my-key",
    /// );
    /// let driver = runtime.get_or_create_driver(account, None).await?;
    /// let container = driver.resolve_container("my-database", "my-container").await?;
    ///
    /// let pk = PartitionKey::from("pk-value");
    /// let result = driver
    ///     .execute_operation(
    ///         CosmosOperation::create_item(container, pk)
    ///             .with_body(br#"{"id": "doc1", "pk": "pk-value", "data": "hello"}"#.to_vec()),
    ///         OperationOptions::default(),
    ///     )
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_item(container: ContainerReference, partition_key: PartitionKey) -> Self {
        let resource_ref: CosmosResourceReference = CosmosResourceReference::from(container)
            .with_resource_type(ResourceType::Document)
            .into_feed_reference();
        Self::new(OperationType::Create, resource_ref).with_partition_key(partition_key)
    }

    /// Reads an item (document) from a container.
    ///
    /// The `ItemReference` contains the container, partition key, and item identifier,
    /// providing all the information needed for the operation.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use azure_data_cosmos_driver::driver::CosmosDriverRuntime;
    /// use azure_data_cosmos_driver::models::{
    ///     AccountReference, CosmosOperation, ItemReference,
    ///     PartitionKey,
    /// };
    /// use azure_data_cosmos_driver::options::OperationOptions;
    /// use url::Url;
    ///
    /// # async fn example() -> azure_core::Result<()> {
    /// let runtime = CosmosDriverRuntime::builder().build().await?;
    /// let account = AccountReference::with_master_key(
    ///     Url::parse("https://myaccount.documents.azure.com:443/").unwrap(),
    ///     "my-key",
    /// );
    /// let driver = runtime.get_or_create_driver(account, None).await?;
    /// let container = driver.resolve_container("my-database", "my-container").await?;
    ///
    /// let item = ItemReference::from_name(&container, PartitionKey::from("pk-value"), "doc1");
    /// let result = driver
    ///     .execute_operation(CosmosOperation::read_item(item), OperationOptions::default())
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn read_item(item: ItemReference) -> Self {
        let partition_key = item.partition_key().clone();
        Self::new(OperationType::Read, item).with_partition_key(partition_key)
    }

    /// Deletes an item (document) from a container.
    ///
    /// The `ItemReference` contains the container, partition key, and item identifier,
    /// providing all the information needed for the operation.
    pub fn delete_item(item: ItemReference) -> Self {
        let partition_key = item.partition_key().clone();
        Self::new(OperationType::Delete, item).with_partition_key(partition_key)
    }

    /// Upserts (creates or replaces) an item (document) in a container.
    ///
    /// The `ItemReference` contains the container, partition key, and item identifier,
    /// providing all the information needed for the operation.
    /// Use `with_body()` to provide the document JSON.
    /// If an item with the same ID exists, it will be replaced; otherwise, a new item is created.
    pub fn upsert_item(item: ItemReference) -> Self {
        let partition_key = item.partition_key().clone();
        Self::new(OperationType::Upsert, item).with_partition_key(partition_key)
    }

    /// Replaces an existing item (document) in a container.
    ///
    /// The `ItemReference` contains the container, partition key, and item identifier,
    /// providing all the information needed for the operation.
    /// Use `with_body()` to provide the new document JSON.
    pub fn replace_item(item: ItemReference) -> Self {
        let partition_key = item.partition_key().clone();
        Self::new(OperationType::Replace, item).with_partition_key(partition_key)
    }

    /// Reads (lists) all items within a single partition.
    ///
    /// Returns a feed of document resources from the specified partition.
    /// This is more efficient than cross-partition reads.
    pub fn read_all_items(container: ContainerReference, partition_key: PartitionKey) -> Self {
        let resource_ref: CosmosResourceReference = CosmosResourceReference::from(container)
            .with_resource_type(ResourceType::Document)
            .into_feed_reference();
        Self::new(OperationType::ReadFeed, resource_ref).with_partition_key(partition_key)
    }

    /// Reads (lists) all items across all partitions.
    ///
    /// Returns a feed of document resources from all partitions.
    ///
    /// **Warning:** Cross-partition reads are inherently less efficient than
    /// single-partition reads. Use `read_all_items()` with a partition key
    /// when possible.
    pub fn read_all_items_cross_partition(container: ContainerReference) -> Self {
        let resource_ref: CosmosResourceReference = CosmosResourceReference::from(container)
            .with_resource_type(ResourceType::Document)
            .into_feed_reference();
        Self::new(OperationType::ReadFeed, resource_ref)
    }

    /// Queries items within a single partition.
    ///
    /// Use `with_body()` to provide the query JSON.
    /// This is more efficient than cross-partition queries.
    pub fn query_items(container: ContainerReference, partition_key: PartitionKey) -> Self {
        let resource_ref: CosmosResourceReference = CosmosResourceReference::from(container)
            .with_resource_type(ResourceType::Document)
            .into_feed_reference();
        Self::new(OperationType::Query, resource_ref).with_partition_key(partition_key)
    }

    /// Queries items across all partitions.
    ///
    /// Use `with_body()` to provide the query JSON.
    ///
    /// **Warning:** Cross-partition queries are inherently less efficient than
    /// single-partition queries. Use `query_items()` with a partition key
    /// when possible.
    pub fn query_items_cross_partition(container: ContainerReference) -> Self {
        let resource_ref: CosmosResourceReference = CosmosResourceReference::from(container)
            .with_resource_type(ResourceType::Document)
            .into_feed_reference();
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

    /// Returns the OpenTelemetry-compliant operation name for this operation.
    ///
    /// Maps `(OperationType, ResourceType)` to the well-known names defined in the
    /// [Cosmos DB OTEL Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/db/cosmosdb/#spans).
    pub fn otel_operation_name(&self) -> &'static str {
        use OperationType::*;
        use ResourceType::*;
        match (self.operation_type, self.resource_type) {
            // Database operations
            (Create, Database) => "create_database",
            (Read, Database) => "read_database",
            (ReadFeed, Database) => "read_all_databases",
            (Query, Database) => "query_databases",
            (Delete, Database) => "delete_database",
            (Replace, Database) => "replace_database",

            // Container operations
            (Create, DocumentCollection) => "create_container",
            (Read, DocumentCollection) => "read_container",
            (ReadFeed, DocumentCollection) => "read_all_containers",
            (Query, DocumentCollection) => "query_containers",
            (Delete, DocumentCollection) => "delete_container",
            (Replace, DocumentCollection) => "replace_container",

            // Item operations
            (Create, Document) => "create_item",
            (Read, Document) => "read_item",
            (ReadFeed, Document) => "read_all_items",
            (Query, Document) => "query_items",
            (SqlQuery, Document) => "query_items",
            (Replace, Document) => "replace_item",
            (Delete, Document) => "delete_item",
            (Upsert, Document) => "upsert_item",
            (Batch, Document) => "execute_batch",

            // Stored procedure operations
            (Create, StoredProcedure) => "create_stored_procedure",
            (Read, StoredProcedure) => "read_stored_procedure",
            (ReadFeed, StoredProcedure) => "read_all_stored_procedures",
            (Query, StoredProcedure) => "query_stored_procedures",
            (Delete, StoredProcedure) => "delete_stored_procedure",
            (Replace, StoredProcedure) => "replace_stored_procedure",
            (Execute, StoredProcedure) => "execute_stored_procedure",

            // Trigger operations
            (Create, Trigger) => "create_trigger",
            (Read, Trigger) => "read_trigger",
            (ReadFeed, Trigger) => "read_all_triggers",
            (Query, Trigger) => "query_triggers",
            (Delete, Trigger) => "delete_trigger",
            (Replace, Trigger) => "replace_trigger",

            // UDF operations
            (Create, UserDefinedFunction) => "create_user_defined_function",
            (Read, UserDefinedFunction) => "read_user_defined_function",
            (ReadFeed, UserDefinedFunction) => "read_all_user_defined_functions",
            (Query, UserDefinedFunction) => "query_user_defined_functions",
            (Delete, UserDefinedFunction) => "delete_user_defined_function",

            // Fallback: combine operation_type and resource_type
            _ => self.operation_type.as_str(),
        }
    }

    /// Returns the database name for this operation, if available.
    ///
    /// Used for the `db.namespace` span attribute.
    pub fn database_name(&self) -> Option<&str> {
        self.resource_reference
            .container()
            .map(|c| c.database_name())
            .or_else(|| self.resource_reference.database().and_then(|d| d.name()))
    }

    /// Returns the container name for this operation, if available.
    ///
    /// Used for the `db.collection.name` span attribute.
    pub fn container_name(&self) -> Option<&str> {
        self.resource_reference.container().map(|c| c.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        AccountReference, ContainerProperties, ContainerReference, PartitionKeyDefinition,
        SystemProperties,
    };

    use url::Url;

    fn test_account() -> AccountReference {
        AccountReference::with_master_key(
            Url::parse("https://test.documents.azure.com:443/").unwrap(),
            "test-key",
        )
    }

    fn test_partition_key_definition(path: &str) -> PartitionKeyDefinition {
        serde_json::from_str(&format!(r#"{{"paths":["{path}"]}}"#)).unwrap()
    }

    fn test_container_props() -> ContainerProperties {
        ContainerProperties {
            id: "testcontainer".into(),
            partition_key: test_partition_key_definition("/pk"),
            system_properties: SystemProperties::default(),
        }
    }

    fn test_container() -> ContainerReference {
        ContainerReference::new(
            test_account(),
            "testdb",
            "testdb_rid",
            "testcontainer",
            "testcontainer_rid",
            &test_container_props(),
        )
    }

    #[test]
    fn create_operation() {
        let item_ref =
            ItemReference::from_name(&test_container(), PartitionKey::from("pk1"), "doc1");
        let resource_ref: CosmosResourceReference = item_ref.into();
        let op = CosmosOperation::new(OperationType::Create, resource_ref);

        assert_eq!(op.operation_type(), OperationType::Create);
        assert_eq!(op.resource_type(), ResourceType::Document);
        assert!(!op.is_read_only());
        assert!(!op.is_idempotent());
    }

    #[test]
    fn read_operation() {
        let item_ref =
            ItemReference::from_name(&test_container(), PartitionKey::from("pk1"), "doc1");
        let resource_ref: CosmosResourceReference = item_ref.into();
        let op = CosmosOperation::new(OperationType::Read, resource_ref);

        assert_eq!(op.operation_type(), OperationType::Read);
        assert_eq!(op.resource_type(), ResourceType::Document);
        assert!(op.is_read_only());
        assert!(op.is_idempotent());
    }

    #[test]
    fn operation_with_partition_key() {
        let item_ref =
            ItemReference::from_name(&test_container(), PartitionKey::from("pk1"), "doc1");
        let resource_ref: CosmosResourceReference = item_ref.into();
        let op = CosmosOperation::new(OperationType::Read, resource_ref)
            .with_partition_key(PartitionKey::from("pk1"));

        assert!(op.partition_key().is_some());
    }

    #[test]
    fn operation_with_body() {
        let item_ref =
            ItemReference::from_name(&test_container(), PartitionKey::from("pk1"), "doc1");
        let resource_ref: CosmosResourceReference = item_ref.into();
        let body = b"{\"id\":\"doc1\"}".to_vec();
        let op = CosmosOperation::new(OperationType::Create, resource_ref).with_body(body.clone());

        assert_eq!(op.body(), Some(body.as_slice()));
    }

    #[test]
    fn replace_is_idempotent() {
        let item_ref =
            ItemReference::from_name(&test_container(), PartitionKey::from("pk1"), "doc1");
        let resource_ref: CosmosResourceReference = item_ref.into();
        let op = CosmosOperation::new(OperationType::Replace, resource_ref);

        assert!(!op.is_read_only());
        assert!(op.is_idempotent());
    }

    #[test]
    fn upsert_is_not_idempotent() {
        let item_ref =
            ItemReference::from_name(&test_container(), PartitionKey::from("pk1"), "doc1");
        let resource_ref: CosmosResourceReference = item_ref.into();
        let op = CosmosOperation::new(OperationType::Upsert, resource_ref);

        assert!(!op.is_read_only());
        assert!(!op.is_idempotent());
    }

    // ===== OTEL Operation Name Tests =====

    #[test]
    fn otel_operation_name_database_operations() {
        let account = test_account();
        assert_eq!(
            CosmosOperation::create_database(account.clone()).otel_operation_name(),
            "create_database"
        );
        assert_eq!(
            CosmosOperation::read_all_databases(account.clone()).otel_operation_name(),
            "read_all_databases"
        );
        assert_eq!(
            CosmosOperation::query_databases(account.clone()).otel_operation_name(),
            "query_databases"
        );
        let db = DatabaseReference::from_name(account.clone(), "mydb");
        assert_eq!(
            CosmosOperation::read_database(db.clone()).otel_operation_name(),
            "read_database"
        );
        assert_eq!(
            CosmosOperation::delete_database(db).otel_operation_name(),
            "delete_database"
        );
    }

    #[test]
    fn otel_operation_name_container_operations() {
        let db = DatabaseReference::from_name(test_account(), "mydb");
        assert_eq!(
            CosmosOperation::create_container(db.clone()).otel_operation_name(),
            "create_container"
        );
        assert_eq!(
            CosmosOperation::read_all_containers(db.clone()).otel_operation_name(),
            "read_all_containers"
        );
        assert_eq!(
            CosmosOperation::query_containers(db.clone()).otel_operation_name(),
            "query_containers"
        );
        let container = test_container();
        assert_eq!(
            CosmosOperation::read_container(container.clone()).otel_operation_name(),
            "read_container"
        );
        assert_eq!(
            CosmosOperation::delete_container(container).otel_operation_name(),
            "delete_container"
        );
    }

    #[test]
    fn otel_operation_name_item_operations() {
        let container = test_container();
        let pk = PartitionKey::from("pk1");
        assert_eq!(
            CosmosOperation::create_item(container.clone(), pk.clone()).otel_operation_name(),
            "create_item"
        );
        assert_eq!(
            CosmosOperation::read_all_items(container.clone(), pk.clone()).otel_operation_name(),
            "read_all_items"
        );
        assert_eq!(
            CosmosOperation::query_items(container.clone(), pk.clone()).otel_operation_name(),
            "query_items"
        );
        assert_eq!(
            CosmosOperation::query_items_cross_partition(container.clone()).otel_operation_name(),
            "query_items"
        );

        let item = ItemReference::from_name(&container, pk.clone(), "doc1");
        assert_eq!(
            CosmosOperation::read_item(item.clone()).otel_operation_name(),
            "read_item"
        );
        assert_eq!(
            CosmosOperation::delete_item(item.clone()).otel_operation_name(),
            "delete_item"
        );
        assert_eq!(
            CosmosOperation::replace_item(item.clone()).otel_operation_name(),
            "replace_item"
        );
        assert_eq!(
            CosmosOperation::upsert_item(item).otel_operation_name(),
            "upsert_item"
        );
    }

    #[test]
    fn otel_database_name_and_container_name() {
        // Database-level operations have database_name but no container_name
        let db = DatabaseReference::from_name(test_account(), "mydb");
        let op = CosmosOperation::read_database(db);
        assert_eq!(op.database_name(), Some("mydb"));
        assert_eq!(op.container_name(), None);

        // Container-level operations have both
        let container = test_container();
        let pk = PartitionKey::from("pk1");
        let op = CosmosOperation::create_item(container, pk);
        assert_eq!(op.database_name(), Some("testdb"));
        assert_eq!(op.container_name(), Some("testcontainer"));

        // Account-level operations have neither
        let op = CosmosOperation::read_all_databases(test_account());
        assert_eq!(op.database_name(), None);
        assert_eq!(op.container_name(), None);
    }
}
