use azure_core::Model;
use serde::Deserialize;

#[derive(Model, Deserialize, Debug)]
pub struct PartitionKeyRanges {
    #[serde(rename = "PartitionKeyRanges")]
    pub ranges: Vec<PartitionKeyRange>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PartitionKeyRange {
    pub id: String,
    pub max_exclusive: String,
    pub min_inclusive: String,
    pub parents: Vec<String>,
}
