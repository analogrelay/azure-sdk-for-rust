mod pipeline;
mod plan;

pub use plan::*;

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
