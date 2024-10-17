// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use crate::models::{ContainerQueryResults, DatabaseProperties, ThroughputProperties};
use crate::options::ReadDatabaseOptions;
use crate::pipeline::ResourceType;
use crate::utils::AppendPathSegments;
use crate::ThroughputOptions;
use crate::{clients::ContainerClient, pipeline::CosmosPipeline};
use crate::{Query, QueryContainersOptions};

use azure_core::{Context, Method, Model, Pager, Request, Response};
use futures::StreamExt;
use serde::Deserialize;
use url::Url;

#[cfg(doc)]
use crate::CosmosClientMethods;

/// Defines the methods provided by a [`DatabaseClient`]
///
/// This trait is intended to allow you to mock out the `DatabaseClient` when testing your application.
/// Rather than depending on `DatabaseClient`, you can depend on a generic parameter constrained by this trait, or an `impl DatabaseClientMethods` type.
pub trait DatabaseClientMethods {
    /// Reads the properties of the database.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional parameters for the request.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # async fn doc() {
    /// # use azure_data_cosmos::clients::{DatabaseClient, DatabaseClientMethods};
    /// # let database_client: DatabaseClient = panic!("this is a non-running example");
    /// let response = database_client.read(None)
    ///     .await.unwrap()
    ///     .deserialize_body()
    ///     .await.unwrap();
    /// # }
    /// ```
    #[allow(async_fn_in_trait)] // REASON: See https://github.com/Azure/azure-sdk-for-rust/issues/1796 for detailed justification
    async fn read(
        &self,
        options: Option<ReadDatabaseOptions>,
    ) -> azure_core::Result<Response<DatabaseProperties>>;

    /// Gets a [`ContainerClient`] that can be used to access the collection with the specified name.
    ///
    /// # Arguments
    /// * `name` - The name of the container.
    fn container_client(&self, name: impl AsRef<str>) -> ContainerClient;

    /// Returns the identifier of the Cosmos database.
    fn id(&self) -> &str;

    /// Executes a query against containers in the database.
    ///
    /// # Arguments
    ///
    /// * `query` - The query to execute.
    /// * `options` - Optional parameters for the request.
    ///
    /// # Examples
    ///
    /// The `query` parameter accepts anything that can be transformed [`Into`] a [`Query`].
    /// This allows simple queries without parameters to be expressed easily:
    ///
    /// ```rust,no_run
    /// # async fn doc() {
    /// # use azure_data_cosmos::clients::{DatabaseClient, DatabaseClientMethods};
    /// # let db_client: DatabaseClient = panic!("this is a non-running example");
    /// let containers = db_client.query_containers(
    ///     "SELECT * FROM dbs",
    ///     None).unwrap();
    /// # }
    /// ```
    ///
    /// See [`Query`] for more information on how to specify a query.
    fn query_containers(
        &self,
        query: impl Into<Query>,
        options: Option<QueryContainersOptions>,
    ) -> azure_core::Result<Pager<ContainerQueryResults>>;

    #[allow(async_fn_in_trait)] // REASON: See https://github.com/Azure/azure-sdk-for-rust/issues/1796 for detailed justification
    async fn read_throughput(
        &self,
        options: Option<ThroughputOptions>,
    ) -> azure_core::Result<Option<Response<ThroughputProperties>>>;
}

/// A client for working with a specific database in a Cosmos DB account.
///
/// You can get a `DatabaseClient` by calling [`CosmosClient::database_client()`](crate::CosmosClient::database_client()).
pub struct DatabaseClient {
    endpoint: Url,
    database_id: String,
    database_url: Url,
    pipeline: CosmosPipeline,
}

impl DatabaseClient {
    pub(crate) fn new(pipeline: CosmosPipeline, base_url: &Url, database_id: &str) -> Self {
        let database_id = database_id.to_string();
        let database_url = base_url.with_path_segments(["dbs", &database_id]);

        Self {
            endpoint: base_url.clone(),
            database_id,
            database_url,
            pipeline,
        }
    }
}

impl DatabaseClientMethods for DatabaseClient {
    async fn read(
        &self,

        #[allow(unused_variables)]
        // REASON: This is a documented public API so prefixing with '_' is undesirable.
        options: Option<ReadDatabaseOptions>,
    ) -> azure_core::Result<Response<DatabaseProperties>> {
        let mut req = Request::new(self.database_url.clone(), Method::Get);
        self.pipeline
            .send(Context::new(), &mut req, ResourceType::Databases)
            .await
    }

    fn container_client(&self, name: impl AsRef<str>) -> ContainerClient {
        ContainerClient::new(self.pipeline.clone(), &self.database_url, name.as_ref())
    }

    fn id(&self) -> &str {
        &self.database_id
    }

    fn query_containers(
        &self,
        query: impl Into<Query>,

        #[allow(unused_variables)]
        // REASON: This is a documented public API so prefixing with '_' is undesirable.
        options: Option<QueryContainersOptions>,
    ) -> azure_core::Result<Pager<ContainerQueryResults>> {
        let mut url = self.database_url.clone();
        url.append_path_segments(["colls"]);
        let base_request = Request::new(url, Method::Post);

        self.pipeline
            .send_query_request(query.into(), base_request, ResourceType::Containers)
    }

    async fn read_throughput(
        &self,

        #[allow(unused_variables)]
        // REASON: This is a documented public API so prefixing with '_' is undesirable.
        options: Option<ThroughputOptions>,
    ) -> azure_core::Result<Option<Response<ThroughputProperties>>> {
        #[derive(Model, Deserialize)]
        struct OfferResults {
            #[serde(rename = "Offers")]
            pub offers: Vec<ThroughputProperties>,
        }

        // We need to get the RID for the database.
        let db = self.read(None).await?.deserialize_body().await?;
        let rid = db
            .system_properties
            .resource_id
            .expect("service should always return a '_rid' for a database");

        // Now, query for the offer for this resource.
        let query = Query::from("SELECT * FROM c WHERE c.offerResourceId = @rid")
            .with_parameter("@rid", rid)?;
        let offers_url = self.endpoint.with_path_segments(["offers"]);
        let mut results: Pager<OfferResults> = self.pipeline.send_query_request(
            query,
            Request::new(offers_url, Method::Post),
            ResourceType::Offers,
        )?;
        let offers = results
            .next()
            .await
            .expect("the first pager result should always be Some, even when there's an error")?
            .deserialize_body()
            .await?
            .offers;

        if offers.len() == 0 {
            // No offers found for this resource.
            return Ok(None);
        }
        println!("Offers");
        println!("{:#?}", offers);

        // Now we can read the offer itself
        let mut req = Request::new(
            format!(
                "{}{}",
                self.endpoint,
                offers[0].system_properties.self_link.as_ref().unwrap()
            )
            .parse()
            .unwrap(),
            Method::Get,
        );
        self.pipeline
            .send(Context::new(), &mut req, ResourceType::Offers)
            .await
            .map(Some)
    }
}
