use std::borrow::Cow;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QueryPlan {
    #[serde(default)]
    pub version: Option<usize>,
    pub query_info: QueryInfo,
    pub query_ranges: Vec<QueryRange>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QueryInfo {
    pub order_by: Vec<SortOrder>,
    pub order_by_expressions: Vec<String>,
    pub has_select_value: bool,
    pub has_non_streaming_order_by: bool,
    pub rewritten_query: String,
}

impl QueryInfo {
    /// Creates an empty QueryInfo with the given rewritten query.
    ///
    /// This is primarily used for testing purposes. In production, the QueryInfo will be populated by the server.
    pub fn from_query(query: &str) -> Self {
        Self {
            order_by: Vec::new(),
            order_by_expressions: Vec::new(),
            has_select_value: false,
            has_non_streaming_order_by: false,
            rewritten_query: query.to_string(),
        }
    }

    /// Sets the order by clauses for this query info.
    ///
    /// This is primarily used for testing purposes. In production, the QueryInfo will be populated by the server.
    pub fn with_order_by(mut self, order_by: Vec<SortOrder>) -> Self {
        self.order_by = order_by;
        self
    }
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QueryRange {
    pub is_max_inclusive: bool,
    pub is_min_inclusive: bool,
    pub min: Cow<'static, str>,
    pub max: Cow<'static, str>,
}

impl QueryRange {
    pub const FULL_SPAN: QueryRange = QueryRange {
        is_max_inclusive: false,
        is_min_inclusive: true,
        min: Cow::Borrowed(""),
        max: Cow::Borrowed("FF"),
    };
}
