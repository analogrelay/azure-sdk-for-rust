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

impl PartitionKeyRange {
    pub fn new(
        id: impl Into<String>,
        min_inclusive: impl Into<String>,
        max_exclusive: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            min_inclusive: min_inclusive.into(),
            max_exclusive: max_exclusive.into(),
            parents: Vec::new(),
        }
    }
}
