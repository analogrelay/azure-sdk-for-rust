use std::sync::Arc;

use azure_core::http::{
    headers::HeaderValue, request::options::ContentType, ClientMethodOptions, Context, Method,
    Request, Response,
};
use serde::de::DeserializeOwned;

use crate::{
    constants,
    pipeline::CosmosPipeline,
    query_engine::QueryEngine,
    resource_context::{ResourceLink, ResourceType},
    FeedPager, Query,
};

pub struct CrossPartitionQueryExecutor<T> {
    phantom: std::marker::PhantomData<T>,
    pipeline: CosmosPipeline,
    link: ResourceLink,
    items_link: ResourceLink,
    query: Query,
    method_options: ClientMethodOptions<'static>,
    query_engine: Arc<dyn QueryEngine>,
}

impl<T: DeserializeOwned> CrossPartitionQueryExecutor<T> {
    pub fn new(
        pipeline: CosmosPipeline,
        link: ResourceLink,
        query: Query,
        method_options: ClientMethodOptions<'static>,
        query_engine: Arc<dyn QueryEngine>,
    ) -> Self {
        let items_link = link.feed(ResourceType::Items);
        Self {
            pipeline,
            link,
            items_link,
            query,
            method_options,
            query_engine,
            phantom: std::marker::PhantomData,
        }
    }

    pub fn execute_query(mut self) -> FeedPager<T> {
        todo!()
        // Pager::from_callback(move |state: Option<PagerState>| async {
        //     let state = match state {
        //         Some(state) => state,
        //         None => {
        //             // Fetch the plan and pk ranges, then create the pipeline.
        //             let plan = self
        //                 .query_plan(context, &query, engine.supported_features().into())
        //                 .await?
        //                 .into_raw_body();
        //             let pk_ranges = self.partition_key_ranges(context).await?.into_raw_body();
        //             let pipeline = engine.create_pipeline(, pk_ranges, self.items_link.clone())?;

        //             // The pipeline might have rewritten the query, so we need to update it.
        //             query.replace_text(pipeline.query().to_string());
        //             PagerState {
        //                 query_pipeline: pipeline,

        //                 // Create a synthetic empty response to use as the initial response
        //                 // in the unlikely event we have items before we make requests.
        //                 last_response: (StatusCode::OK, Headers::new()),
        //             }
        //         }
        //     };

        //     loop {
        //         if state.query_pipeline.complete() {
        //             todo!("Handle completion");
        //         }

        //         let pipeline_result = state.query_pipeline.next_batch()?;

        //         // If we got items, return them now. There may also be requests, but the pipeline guarantees it'll return the same requests next loop.
        //         let items = pipeline_result.items;
        //         if !items.is_empty() {
        //             // This is awkward. For now, we have to synthesize the request body.
        //             // I'm opening a bug to discuss alternatives.
        //             let mut values = items
        //                 .into_iter()
        //                 .map(|item| serde_json::from_slice(&item))
        //                 .collect::<Result<Vec<_>, _>>()?;
        //             let resp = SyntheticItemsResponse {
        //                 items: values,
        //             };
        //             let body = serde_json::to_vec(&resp)?;
        //             let mut response = azure_core::Response::new(
        //                 body,
        //             );
        //         }

        //         todo!()
        //     }
        // })
    }

    async fn query_plan(
        &self,
        context: Context<'_>,
        query: &Query,
        supported_features: HeaderValue,
    ) -> azure_core::Result<Response> {
        let url = self.pipeline.url(&self.items_link);
        let mut req = Request::new(url, Method::Post);
        req.insert_header(constants::IS_QUERY_PLAN, "True");

        req.insert_header(constants::SUPPORTED_QUERY_FEATURES, supported_features);
        req.insert_header(constants::QUERY, "True");
        req.add_mandatory_header(&ContentType::APPLICATION_JSON);
        req.set_json(query)?;

        self.pipeline
            .send(context.clone(), &mut req, self.items_link.clone())
            .await
    }

    async fn partition_key_ranges(&self, context: Context<'_>) -> azure_core::Result<Response> {
        let link = self.link.feed(ResourceType::PartitionKeyRanges);
        let url = self.pipeline.url(&link);
        let mut req = Request::new(url, Method::Get);
        self.pipeline.send(context.clone(), &mut req, link).await
    }
}
