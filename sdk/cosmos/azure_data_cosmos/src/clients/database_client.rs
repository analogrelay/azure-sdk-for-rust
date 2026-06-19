// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use crate::{
    clients::{offers_client, ClientContext, ContainerClient},
    feed::QueryItemIterator,
    models::ResourceResponse,
    models::{ContainerProperties, DatabaseProperties, ThroughputProperties},
    options::{
        CreateContainerOptions, DeleteDatabaseOptions, QueryContainersOptions, ReadDatabaseOptions,
        ThroughputOptions,
    },
    Query,
};
use azure_data_cosmos_driver::models::{CosmosOperation, DatabaseReference};

use super::ThroughputPoller;

/// Client for a specific database in an Azure Cosmos DB account.
///
/// Get a [`DatabaseClient`] by calling [`CosmosClient::database_client`](crate::CosmosClient::database_client).
pub struct DatabaseClient {
    database_id: String,
    context: ClientContext,
    database_ref: DatabaseReference,
}

impl DatabaseClient {
    pub(crate) fn new(context: ClientContext, database_id: &str) -> Self {
        let database_id = database_id.to_string();
        let database_ref =
            DatabaseReference::from_name(context.driver.account().clone(), database_id.clone());

        Self {
            database_id,
            context,
            database_ref,
        }
    }

    /// Returns a [`ContainerClient`] for the container with the given name.
    ///
    /// This method resolves container metadata up front so the returned client
    /// is ready to use without an extra lookup on each request.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
    /// # let db_client: azure_data_cosmos::clients::DatabaseClient = panic!("non-running example");
    /// let container_client = db_client.container_client("products").await?;
    /// # let _ = container_client;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the container does not exist or its metadata cannot
    /// be resolved.
    pub async fn container_client(&self, name: &str) -> crate::Result<ContainerClient> {
        ContainerClient::new(self.context.clone(), name, &self.database_id).await
    }

    /// Returns the database ID.
    pub fn id(&self) -> &str {
        &self.database_id
    }

    /// Reads the database properties.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
    /// # let db_client: azure_data_cosmos::clients::DatabaseClient = panic!("non-running example");
    /// let database = db_client.read(None).await?.into_model()?;
    /// # let _ = database;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn read(
        &self,
        options: Option<ReadDatabaseOptions>,
    ) -> crate::Result<ResourceResponse<DatabaseProperties>> {
        let options = options.unwrap_or_default();
        let operation = CosmosOperation::read_database(self.database_ref.clone());

        let driver_response = self
            .context
            .driver
            .execute_singleton_operation(operation, options.operation)
            .await?;

        Ok(ResourceResponse::new(
            crate::driver_bridge::driver_response_to_cosmos_response(driver_response),
        ))
    }

    /// Executes a query against containers in the database.
    ///
    /// # Examples
    ///
    /// The `query` parameter accepts anything that can be transformed [`Into`] a [`Query`].
    /// This allows simple queries without parameters to be expressed easily:
    ///
    /// ```rust,no_run
    /// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
    /// # use azure_data_cosmos::clients::DatabaseClient;
    /// # let db_client: DatabaseClient = panic!("this is a non-running example");
    /// let containers = db_client
    ///     .query_containers("SELECT * FROM dbs", None)
    ///     .await?;
    /// # }
    /// ```
    ///
    /// See [`Query`] for more information about building queries.
    ///
    /// # Errors
    ///
    /// Returns an error if the query cannot be serialized or the request fails.
    pub async fn query_containers(
        &self,
        query: impl Into<Query>,
        options: Option<QueryContainersOptions>,
    ) -> crate::Result<QueryItemIterator<ContainerProperties>> {
        let options = options.unwrap_or_default();
        let query = query.into();
        let initial_operation = CosmosOperation::query_containers(self.database_ref.clone())
            .with_body(serde_json::to_vec(&query)?);
        let operation_options = options.operation;

        let plan = self
            .context
            .driver
            .plan_operation(initial_operation, &operation_options, None)
            .await?;

        Ok(QueryItemIterator::new(
            self.context.driver.clone(),
            None,
            plan,
            operation_options,
        ))
    }

    /// Creates a new container.
    ///
    #[doc = include_str!("../../docs/control-plane-warning.md")]
    ///
    #[doc = include_str!("../../docs/control-plane-always-returns-body.md")]
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
    /// use azure_data_cosmos::models::ContainerProperties;
    /// # let db_client: azure_data_cosmos::clients::DatabaseClient = panic!("non-running example");
    /// let properties = ContainerProperties::new("products", "/category_id".into());
    /// let container = db_client.create_container(properties, None).await?.into_model()?;
    /// # let _ = container;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn create_container(
        &self,
        properties: ContainerProperties,
        options: Option<CreateContainerOptions>,
    ) -> crate::Result<ResourceResponse<ContainerProperties>> {
        let options = options.unwrap_or_default();
        let body = serde_json::to_vec(&properties)?;
        let mut operation =
            CosmosOperation::create_container(self.database_ref.clone()).with_body(body);

        if let Some(throughput) = &options.throughput {
            let mut headers = azure_data_cosmos_driver::models::CosmosRequestHeaders::new();
            throughput.apply_headers(&mut headers);
            operation = operation.with_request_headers(headers);
        }

        // Control-plane creates always need the full response body so the
        // caller can inspect the created resource properties.
        let mut operation_options = options.operation;
        operation_options.content_response_on_write =
            Some(azure_data_cosmos_driver::options::ContentResponseOnWrite::Enabled);

        let driver_response = self
            .context
            .driver
            .execute_singleton_operation(operation, operation_options)
            .await?;

        Ok(ResourceResponse::new(
            crate::driver_bridge::driver_response_to_cosmos_response(driver_response),
        ))
    }

    /// Deletes this database.
    ///
    #[doc = include_str!("../../docs/control-plane-warning.md")]
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
    /// # let db_client: azure_data_cosmos::clients::DatabaseClient = panic!("non-running example");
    /// db_client.delete(None).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub async fn delete(
        &self,
        options: Option<DeleteDatabaseOptions>,
    ) -> crate::Result<ResourceResponse<()>> {
        let options = options.unwrap_or_default();
        let operation = CosmosOperation::delete_database(self.database_ref.clone());

        let driver_response = self
            .context
            .driver
            .execute_singleton_operation(operation, options.operation)
            .await?;

        Ok(ResourceResponse::new(
            crate::driver_bridge::driver_response_to_cosmos_response(driver_response),
        ))
    }

    /// Reads the database throughput settings, if any.
    ///
    /// Returns `None` if the database does not have dedicated throughput.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
    /// # let db_client: azure_data_cosmos::clients::DatabaseClient = panic!("non-running example");
    /// if let Some(throughput) = db_client.read_throughput(None).await? {
    /// # let _ = throughput;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the database metadata or throughput offer cannot be read.
    pub async fn read_throughput(
        &self,
        options: Option<ThroughputOptions>,
    ) -> crate::Result<Option<ThroughputProperties>> {
        let options = options.unwrap_or_default();
        // We need to get the RID for the database.
        let db = self.read(None).await?.into_model()?;
        let resource_id = resource_id_or_error(db.system_properties.resource_id, "database")?;

        offers_client::find_offer(
            &self.context.driver,
            self.context.driver.account(),
            &resource_id,
            options.operation,
        )
        .await
    }

    /// Starts replacing the database throughput settings.
    ///
    /// The Cosmos DB service may process throughput changes asynchronously. The returned
    /// [`ThroughputPoller`] can be awaited for the final result or polled as a stream
    /// to observe progress.
    ///
    #[doc = include_str!("../../docs/control-plane-always-returns-body.md")]
    ///
    /// # Errors
    ///
    /// Returns an error if the database metadata cannot be read or the replace request fails.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use azure_data_cosmos::models::ThroughputProperties;
    /// # async fn example(db_client: azure_data_cosmos::clients::DatabaseClient) -> azure_data_cosmos::Result<()> {
    /// let throughput = db_client
    ///     .begin_replace_throughput(ThroughputProperties::manual(500), None)
    ///     .await? // start the replace operation
    ///     .await? // wait for completion (polls if async)
    ///     .into_model()?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn begin_replace_throughput(
        &self,
        throughput: ThroughputProperties,
        options: Option<ThroughputOptions>,
    ) -> crate::Result<ThroughputPoller> {
        let options = options.unwrap_or_default();
        // We need to get the RID for the database.
        let db = self.read(None).await?.into_model()?;
        let resource_id = resource_id_or_error(db.system_properties.resource_id, "database")?;

        offers_client::begin_replace(
            self.context.driver.clone(),
            self.context.driver.account().clone(),
            &resource_id,
            throughput,
            options.operation,
        )
        .await
    }
}

/// Unwraps the `_rid` from a system-properties response. The Cosmos service
/// is contractually required to populate `_rid` on every resource read; if it
/// is missing we surface a synthetic 500 [`CosmosError`](crate::CosmosError)
/// rather than panicking, since panics in public methods would crash callers'
/// applications. The `debug_assert!` keeps tests honest while still letting
/// release builds recover.
fn resource_id_or_error(rid: Option<String>, resource_kind: &str) -> crate::Result<String> {
    debug_assert!(
        rid.is_some(),
        "service should always return a '_rid' for a {resource_kind}"
    );
    rid.ok_or_else(|| {
        crate::DriverCosmosError::builder()
            .with_status(crate::error::CosmosStatus::SERVICE_RETURNED_OBJECT_WITHOUT_RID)
            .with_message(format!(
                "service did not return a '_rid' for a {resource_kind}; cannot resolve the throughput offer"
            ))
            .build()
            .into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile-time assertion that `DatabaseClient` async method futures are `Send`.
    ///
    /// This function is never called; it only needs to compile.
    /// If any future is not `Send`, compilation will fail.
    #[allow(dead_code, unreachable_code, unused_variables)]
    fn _assert_futures_are_send() {
        fn assert_send<T: Send>(_: T) {}
        let client: &DatabaseClient = todo!();
        assert_send(client.container_client(todo!()));
        assert_send(client.read(todo!()));
        assert_send(client.query_containers(Query::from("SELECT * FROM c"), todo!()));
        assert_send(client.create_container(todo!(), todo!()));
        assert_send(client.delete(todo!()));
        assert_send(client.read_throughput(todo!()));
        assert_send(client.begin_replace_throughput(todo!(), todo!()));
    }
}
