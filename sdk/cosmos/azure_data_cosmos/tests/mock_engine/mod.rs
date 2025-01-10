use std::borrow::Cow;

use azure_data_cosmos::{
    query_engine::{QueryEngine, QueryPipeline},
    FeedPage,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MockItem {
    pub id: String,
    pub partition_key: String,
    pub merge_order: usize,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PartitionKeyRange {
    id: String,
    min_inclusive: String,
    max_exclusive: String,
}

pub struct MockQueryEngine {}

impl MockQueryEngine {
    pub fn new() -> Self {
        Self {}
    }
}

impl QueryEngine for MockQueryEngine {
    fn create_pipeline(
        &self,
        query: &str,
        _plan: &[u8],
        pkranges: &[u8],
    ) -> azure_core::Result<Box<dyn azure_data_cosmos::query_engine::QueryPipeline>> {
        #[derive(Deserialize, Serialize)]
        struct PartitionKeyRangeResult {
            #[serde(rename = "PartitionKeyRanges")]
            partition_key_ranges: Vec<PartitionKeyRange>,
        }

        let pkranges =
            serde_json::from_slice::<PartitionKeyRangeResult>(pkranges)?.partition_key_ranges;

        // We don't need to parse the plan in the mock engine.
        Ok(Box::new(MockQueryPipeline::new(
            query.to_string(),
            pkranges,
        )?))
    }

    fn supported_features(&self) -> Cow<'static, str> {
        "OrderBy".into()
    }
}

struct PartitionState {
    pub partition_key_range: PartitionKeyRange,
    pub started: bool,
    pub queue: Vec<MockItem>,
    pub next_continuation: Option<String>,
}

impl PartitionState {
    pub fn new(partition_key_range: PartitionKeyRange) -> Self {
        Self {
            partition_key_range,
            started: false,
            queue: vec![],
            next_continuation: None,
        }
    }

    pub fn is_exhausted(&self) -> bool {
        self.queue.is_empty() && self.started && self.next_continuation.is_none()
    }

    pub fn provide_data(&mut self, data: Vec<MockItem>, continuation: Option<String>) {
        self.started = true;
        self.next_continuation = continuation;
        self.queue.extend(data);
    }

    pub fn pop_item(&mut self) -> Option<azure_core::Result<Vec<u8>>> {
        if self.queue.is_empty() {
            return None;
        }
        let item = self.queue.remove(0);
        let item = serde_json::to_vec(&item).map_err(azure_core::Error::from);
        Some(item)
    }
}

struct MockQueryPipeline {
    query: String,
    completed: bool,
    partition_states: Vec<PartitionState>,
}

impl MockQueryPipeline {
    pub fn new(
        query: String,
        partition_key_ranges: Vec<PartitionKeyRange>,
    ) -> azure_core::Result<Self> {
        let partition_states = partition_key_ranges
            .into_iter()
            .map(PartitionState::new)
            .collect::<Vec<_>>();
        Ok(Self {
            query,
            completed: false,
            partition_states,
        })
    }
}

impl QueryPipeline for MockQueryPipeline {
    fn query(&self) -> &str {
        &self.query
    }

    fn complete(&self) -> bool {
        self.completed
    }

    fn next_batch(
        &mut self,
    ) -> azure_core::Result<azure_data_cosmos::query_engine::PipelineResult> {
        let mut items = Vec::new();

        'outer: loop {
            let mut lowest_merge_order = None;
            let mut lowest_partition_key_range = None;
            for (i, partition_state) in self.partition_states.iter_mut().enumerate() {
                if !partition_state.started {
                    // Break the outer loop if we find a partition that hasn't started yet.
                    break 'outer;
                }
                if partition_state.is_exhausted() {
                    // If the partition is exhausted, we can skip it.
                    continue;
                }
                if !partition_state.queue.is_empty()
                    && (lowest_partition_key_range.is_none()
                        || lowest_merge_order
                            .map(|mo| partition_state.queue[0].merge_order < mo)
                            .unwrap_or(false))
                {
                    lowest_partition_key_range = Some(i);
                    lowest_merge_order = Some(partition_state.queue[0].merge_order);
                }
            }

            if let Some(p) = lowest_partition_key_range {
                let partition_state = &mut self.partition_states[p];
                if let Some(item) = partition_state.pop_item() {
                    items.push(item?);
                }
            } else {
                // No more items to process.
                break;
            }
        }

        let mut requests = Vec::new();
        for partition_state in self.partition_states.iter_mut() {
            if partition_state.is_exhausted() {
                continue;
            }
            let request = azure_data_cosmos::query_engine::QueryRequest {
                partition_key_range_id: partition_state.partition_key_range.id.clone(),
                continuation: partition_state.next_continuation.clone(),
            };
            requests.push(request);
        }

        if items.is_empty() && requests.is_empty() {
            self.completed = true;
        }

        Ok(azure_data_cosmos::query_engine::PipelineResult {
            completed: self.completed,
            items,
            requests,
        })
    }

    fn provide_data(
        &mut self,
        data: azure_data_cosmos::query_engine::QueryResult,
    ) -> azure_core::Result<()> {
        #[derive(Deserialize)]
        struct Results<T> {
            #[serde(rename = "Documents")]
            items: Vec<T>,
        }
        let items = serde_json::from_slice::<Results<MockItem>>(&data.data)?.items;

        let partition_state = self
            .partition_states
            .iter_mut()
            .find(|state| state.partition_key_range.id == data.partition_key_range_id)
            .ok_or_else(|| {
                azure_core::Error::message(
                    typespec::error::ErrorKind::Other,
                    format!(
                        "Partition key range {} not found",
                        data.partition_key_range_id
                    ),
                )
            })?;

        partition_state.provide_data(items, data.next_continuation);
        Ok(())
    }
}
