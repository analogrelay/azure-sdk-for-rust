use serde::Deserialize;

pub mod query;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PartitionKeyRange {
    pub id: String,
    pub max_exclusive: String,
    pub min_inclusive: String,
    pub parents: Vec<String>,
}
