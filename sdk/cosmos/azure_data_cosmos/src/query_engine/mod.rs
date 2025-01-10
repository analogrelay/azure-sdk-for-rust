//! Provides the interface to integrate an external query engine with the SDK.
//! This is an UNSTABLE feature and is subject to change.
//!
//! The Cosmos DB SDK has to manually fan-out cross-partition queries to all partitions.
//! The "query engine" is the component that handles processing the individual partition results and aggregating them into a single set of results.
//! Currently, this is a separate component from the SDK, being developed to share across several Cosmos SDKs.
//! As a result, it must be integrated separately with the SDK.
//!
//! This is preview functionality and may change dramatically in the future.
//! Eventually, we expect it will be integrated directly into the SDK itself.

use std::borrow::Cow;

/// Provides an interface to allow the SDK to integrate with an external query engine, such as the one provided by the Azure Cosmos DB Client Engine.
pub trait QueryEngine {
    /// Creates a new query pipeline for the given query, plan, and partition key ranges.
    fn create_pipeline(
        &self,
        query: &str,
        plan: &[u8],
        pkranges: &[u8],
    ) -> azure_core::Result<Box<dyn QueryPipeline>>;

    /// Gets a comma-separated list of supported features for the query engine.
    /// This list can be used when requesting a query plan from the gateway.
    fn supported_features(&self) -> Cow<'static, str>;
}

/// Represents a request, produced by the query engine, for the SDK to execute a particular query.
pub struct QueryRequest {
    /// The partition key range ID to use for the query.
    pub partition_key_range_id: String,

    /// The continuation token to use for the query, if any.
    pub continuation: Option<String>,
}

/// Represents the result of executing a query, as a result of completing the query requested by a [`QueryRequest`].
pub struct QueryResult {
    /// The partition key range ID these results are from.
    pub partition_key_range_id: String,

    /// The next continuation token to use for the query, if any.
    pub next_continuation: Option<String>,

    /// The data returned by the query.
    pub data: Vec<u8>,
}

/// Represents the result of executing a single turn of the query pipeline.
pub struct PipelineResult {
    /// A boolean indicating whether the pipeline is done (after emitting any items in this result).
    pub completed: bool,

    /// Items to be returned to the user.
    pub items: Vec<Vec<u8>>,

    /// Requests that must be sent to the server before the pipeline can continue.
    pub requests: Vec<QueryRequest>,
}

pub trait QueryPipeline: Send {
    /// Gets the, possibly rewritten, query to execute.
    fn query(&self) -> &str;

    /// Indicates whether the pipeline has completed.
    /// If this is true, there is no need to call [`next_batch()`] or [`provide_data()`] again.
    fn complete(&self) -> bool;

    /// Executes a single turn of the query pipeline, returning the next batch of results.
    fn next_batch(&mut self) -> azure_core::Result<PipelineResult>;

    /// Provides new data to the query pipeline, in response to a [`QueryRequest`] returned by [`next_batch()`].
    fn provide_data(&mut self, data: QueryResult) -> azure_core::Result<()>;
}
