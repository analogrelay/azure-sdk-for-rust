use std::fmt;

use azure_core::{Context, Method, Request, Response};
use azure_data_cosmos_driver::query::{PipelineResult, QueryPipeline, QueryPlan};
use futures::Stream;
use serde::de::DeserializeOwned;

use crate::{
    constants,
    models::{PartitionKeyRanges, QueryPage},
    pipeline::CosmosPipeline,
    resource_context::{ResourceLink, ResourceType},
    Query, QueryOptions,
};

enum QueryEngineState {
    Initial,
    Running(QueryPipeline),
    Done,
}

impl fmt::Debug for QueryEngineState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryEngineState::Initial => write!(f, "Initial"),
            QueryEngineState::Running(_) => write!(f, "Running"),
            QueryEngineState::Done => write!(f, "Done"),
        }
    }
}

pub struct QueryEngine<T> {
    query: Query,
    pipeline: CosmosPipeline,
    container_link: ResourceLink,
    items_link: ResourceLink,
    context: Context<'static>,
    phantom: std::marker::PhantomData<T>,
}

impl<T> QueryEngine<T> {
    pub fn new(
        query: Query,
        pipeline: CosmosPipeline,
        container_link: ResourceLink,
        items_link: ResourceLink,
        options: Option<QueryOptions<'_>>,
    ) -> Self {
        let context = options
            .unwrap_or_default()
            .method_options
            .context
            .into_owned();
        Self {
            query,
            pipeline,
            container_link,
            items_link,
            context,
            phantom: std::marker::PhantomData,
        }
    }

    // REVIEW: This is only public for testing purposes. It should be private.
    pub async fn query_plan(
        &self,
    ) -> azure_core::Result<Response<azure_data_cosmos_driver::query::QueryPlan>> {
        let url = self.pipeline.url(&self.items_link);
        let mut req = Request::new(url, Method::Post);
        req.insert_header(constants::IS_QUERY_PLAN, "True");

        let supported_features = azure_data_cosmos_driver::query::supported_query_features_string();

        req.insert_header(constants::SUPPORTED_QUERY_FEATURES, supported_features);
        req.insert_header(constants::QUERY, "True");
        req.set_json(&self.query)?;

        self.pipeline
            .send(self.context.clone(), &mut req, self.items_link.clone())
            .await
    }

    pub async fn partition_key_ranges(&self) -> azure_core::Result<Response<PartitionKeyRanges>> {
        let link = self.container_link.feed(ResourceType::PartitionKeyRanges);
        let url = self.pipeline.url(&link);
        let mut req = Request::new(url, Method::Get);
        self.pipeline
            .send(self.context.clone(), &mut req, link)
            .await
    }
}

impl<T: DeserializeOwned> QueryEngine<T> {
    pub fn into_stream(self) -> impl Stream<Item = azure_core::Result<QueryPage<T>>> {
        futures::stream::unfold(
            (self, QueryEngineState::Initial),
            |(mut this, state)| async move {
                let _ = tracing::debug_span!("QueryEngine::into_stream::next", ?state, query = ?this.query).entered();
                // Run the next step
                match state {
                    QueryEngineState::Initial => {
                        let r = this.initial().await.transpose()?;
                        match r {
                            Err(e) => Some((Err(e), (this, QueryEngineState::Done))),
                            Ok((page, pipeline)) => {
                                Some((Ok(page), (this, QueryEngineState::Running(pipeline))))
                            }
                        }
                    }
                    QueryEngineState::Running(mut query_pipeline) => {
                        let page = this.next_page(&mut query_pipeline).await.transpose()?;
                        Some((page, (this, QueryEngineState::Running(query_pipeline))))
                    }
                    QueryEngineState::Done => None,
                }
            },
        )
    }

    #[tracing::instrument(level = "trace", skip_all)]
    async fn initial(&mut self) -> azure_core::Result<Option<(QueryPage<T>, QueryPipeline)>> {
        // Get the query plan and pkranges.
        let plan: QueryPlan = self.query_plan().await?.into_json_body().await?;
        let ranges: PartitionKeyRanges =
            self.partition_key_ranges().await?.into_json_body().await?;
        tracing::debug!(?plan, ?ranges, "fetched query plan and ranges");

        let mut query_pipeline = QueryPipeline::from_plan(plan, ranges.ranges);

        let page = self.next_page(&mut query_pipeline).await?;
        let result = page.map(|page| (page, query_pipeline));

        Ok(result)
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn next_page(
        &mut self,
        query_pipeline: &mut QueryPipeline,
    ) -> azure_core::Result<Option<QueryPage<T>>> {
        loop {
            let next = query_pipeline.step_pipeline().map_err(to_azure_error)?;
            let Some(pipeline_result) = next else {
                // The pipeline is done.
                return Ok(None);
            };

            match pipeline_result {
                PipelineResult::Data(data) => {
                    let mut deserialized = Vec::with_capacity(data.len());
                    for item in data {
                        let item = serde_json::from_value(item)?;
                        deserialized.push(item);
                    }
                    return Ok(Some(QueryPage {
                        items: deserialized,
                    }));
                }
                PipelineResult::NeedsMoreData(requests) => {
                    tracing::debug!("pipeline is requesting more data");
                    for partition in requests.partitions {
                        let _ = tracing::debug_span!(
                            "QueryEngine::next_page::requesting_more_data",
                            pkrange_id = ?partition.partition_key_range_id,
                            query = ?requests.query,
                            continuation = ?partition.continuation,
                        )
                        .entered();

                        let url = self.pipeline.url(&self.items_link);
                        let mut req = Request::new(url, Method::Post);
                        req.insert_header(constants::QUERY, "True");
                        req.insert_header(
                            constants::PARTITION_KEY_RANGE_ID,
                            &partition.partition_key_range_id,
                        );
                        if let Some(continuation) = partition.continuation.clone() {
                            req.insert_header(constants::CONTINUATION, continuation);
                        }
                        req.add_mandatory_header(&constants::QUERY_CONTENT_TYPE);
                        req.insert_header("x-ms-documentdb-query-iscontinuationexpected", "False");

                        // TODO: Parameters
                        req.set_json(&Query::from(&requests.query))?;

                        tracing::debug!(pkrange_id = ?partition.partition_key_range_id, continuation=?partition.continuation, query=?requests.query, "sending query request");

                        let results: Response = self
                            .pipeline
                            .send(self.context.clone(), &mut req, self.items_link.clone())
                            .await?;
                        let continuation = results
                            .headers()
                            .get_optional_str(&constants::CONTINUATION)
                            .map(|s| s.to_owned());
                        let items: QueryPage<serde_json::Value> = results.into_json_body().await?;

                        tracing::debug!(
                            item_count = items.items.len(),
                            "inserting results into pipeline",
                        );
                        query_pipeline
                            .enqueue_data(
                                partition.partition_key_range_id.clone(),
                                items.items,
                                continuation,
                            )
                            .map_err(to_azure_error)?;
                    }
                }
            }
        }
    }
}

fn to_azure_error(e: azure_data_cosmos_driver::query::Error) -> azure_core::Error {
    azure_core::Error::full(
        azure_core::error::ErrorKind::Other,
        e,
        "query pipeline error",
    )
}
