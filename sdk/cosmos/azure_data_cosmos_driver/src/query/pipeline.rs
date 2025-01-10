use std::marker::PhantomData;

use crate::query::QueryPlan;

/// The query aggregator is a stateful component that receives results from multiple partitions and aggregates them into a single stream of results.
pub struct QueryPipeline<T> {
    phantom: PhantomData<T>,
}

impl<T> QueryPipeline<T> {
    pub fn from_plan(_plan: QueryPlan) -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

/// Represents a request for data from a partition.
///
/// When the query pipeline runs out of data, it will return a collection of these requests describing the HTTP requests
/// that need to be made to retrieve more data.
///
/// The Rust driver intentionally avoids performing I/O or network operations in the query pipeline.
/// Instead, it is the responsibility of the caller to perform these operations and pass the results back into the pipeline before reading the next items.
pub struct PartitionQueryRequest {
    /// The partition range ID for the partition that this request is for.
    pub partition_range_id: String,

    /// The query to be executed against the partition.
    pub query: String,

    /// The continuation token to be used for the next request, if any.
    pub continuation: Option<String>,
}

/// Represents a single result from a query.
pub struct QueryResult<T> {
    value: T,
}

pub enum PipelineResult<T> {
    /// Indicates that the pipeline has data available for consumption.
    Data(Vec<QueryResult<T>>),

    /// Indicates that the pipeline needs more data to continue processing.
    NeedsMoreData(Vec<PartitionQueryRequest>),

    /// Indicates that the pipeline has completed and no more data will be available.
    Complete,
}

impl<T> Iterator for QueryPipeline<T> {
    type Item = PipelineResult<T>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
