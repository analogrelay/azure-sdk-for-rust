mod error;
mod pipeline;
mod plan;

pub use error::Error;
pub use pipeline::*;
pub use plan::*;

/// Represents a request for additional data for the pipeline
///
/// When the query pipeline runs out of data, it will return a collection of these requests describing the HTTP requests
/// that need to be made to retrieve more data.
///
/// The Rust driver intentionally avoids performing I/O or network operations in the query pipeline.
/// Instead, it is the responsibility of the caller to perform these operations and pass the results back into the pipeline before reading the next items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionContinuation {
    /// The partition key range ID for the partition that this request is for.
    pub partition_key_range_id: String,

    /// The continuation token to be used for the next request, if any.
    pub continuation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineDataRequest {
    /// The query to be executed
    pub query: String,

    /// The individual partitions to query, and the continuation tokens for each partition.
    pub partitions: Vec<PartitionContinuation>,
}

pub enum QueryFeature {
    Aggregate,
    CompositeAggregate,
    Distinct,
    MultipleOrderBy,
    OffsetAndLimit,
    OrderBy,
    Top,
    NonStreamingOrderBy,
}

impl QueryFeature {
    pub fn as_str(&self) -> &'static str {
        match self {
            QueryFeature::Aggregate => "Aggregate",
            QueryFeature::CompositeAggregate => "CompositeAggregate",
            QueryFeature::Distinct => "Distinct",
            QueryFeature::MultipleOrderBy => "MultipleOrderBy",
            QueryFeature::OffsetAndLimit => "OffsetAndLimit",
            QueryFeature::OrderBy => "OrderBy",
            QueryFeature::Top => "Top",
            QueryFeature::NonStreamingOrderBy => "NonStreamingOrderBy",
        }
    }
}

pub const fn supported_query_features() -> &'static [QueryFeature] {
    const SUPPORTED_QUERY_FEATURES: &[QueryFeature] = &[QueryFeature::OrderBy];
    SUPPORTED_QUERY_FEATURES
}

// TODO: Is there a const way to do this?
pub fn supported_query_features_string() -> String {
    supported_query_features()
        .iter()
        .map(|feature| feature.as_str())
        .collect::<Vec<&str>>()
        .join(",")
}
