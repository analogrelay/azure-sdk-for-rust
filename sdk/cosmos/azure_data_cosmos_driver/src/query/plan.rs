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

#[derive(Deserialize, Debug)]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QueryRange {
    pub is_max_inclusive: bool,
    pub is_min_inclusive: bool,
    pub min: String,
    pub max: String,
}
