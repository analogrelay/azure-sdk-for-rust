use std::{borrow::Cow, collections::VecDeque};

use crate::{
    query::{PartitionContinuation, PipelineDataRequest, QueryPlan, SortOrder},
    PartitionKeyRange,
};

pub enum Error {
    /// An error occurred due to an invalid query plan.
    ///
    /// The associated string is an error message describing the error.
    /// This error is not recoverable by the user.
    QueryPlanInvalid(Cow<'static, str>),
}

enum QueryStage {
    /// Indicates that the query has not started yet.
    /// The next query to be issued here is the first query for this partition, with no continuation token.
    NotStarted,

    /// Indicates that the last query returned a continuation token.
    /// The next query to be issued here is the next query for this partition, with the continuation token.
    Continuing(String),

    /// Indicates that we've performed at least one query, but the last query did not return a continuation token.
    /// The partition has been exhausted and there is no more data to return for this partition.
    Finished,
}

struct PartitionState {
    /// The partition key range ID for the partition that this state is tracking.
    pub partition_key_range: PartitionKeyRange,

    /// A queue of incoming results for this partition.
    pub queue: VecDeque<serde_json::Value>,

    /// The stage of the query for this partition. This is used to track whether the partition has more data to return.
    pub stage: QueryStage,
}

/// A function that can compare two partitions to determine which one has the next item to return without consuming said item.
type PartitionComparer =
    dyn Fn(&PartitionState, &PartitionState) -> Result<std::cmp::Ordering, Error>;

/// The query aggregator is a stateful component that receives results from multiple partitions and aggregates them into a single stream of results.
pub struct QueryPipeline {
    /// The query, rewritten by the query planner, that will be executed against each partition.
    rewritten_query: String,

    /// A list of partitions that the query is being executed against, with the necessary state tracking.
    partitions: Vec<PartitionState>,

    /// A buffer of results that have been processed and are ready to be returned to the caller.
    buffer: VecDeque<serde_json::Value>,

    /// A function used to compare two partitions and determine which one has the next item to return.
    ///
    /// This function should return an ordering that is descending, even if the sort ordering requested in the query is ascending.
    /// So, if the query is "ORDER BY a ASC", then the comparison function should return reversed [std::cmp::Ordering] values.
    comparer: Box<PartitionComparer>,
}

fn by_partition_key(a: &PartitionState, b: &PartitionState) -> Result<std::cmp::Ordering, Error> {
    Ok(a.partition_key_range
        .min_inclusive
        .cmp(&b.partition_key_range.min_inclusive))
}

fn get_order_by_items(
    value: &serde_json::Value,
) -> Result<impl Iterator<Item = &serde_json::Value>, Error> {
    Ok(value
        .as_object()
        .ok_or_else(|| Error::QueryPlanInvalid("query did not return an object".into()))?
        .get("orderByItems")
        .ok_or_else(|| Error::QueryPlanInvalid("row does not have 'orderByItems'".into()))?
        .as_array()
        .ok_or_else(|| Error::QueryPlanInvalid("'orderByItems' is not an array".into()))?
        .iter())
}

fn compare_json(a: &serde_json::Value, b: &serde_json::Value) -> std::cmp::Ordering {
    todo!();
}

fn by_order_by_items(orderings: Vec<SortOrder>) -> Box<PartitionComparer> {
    Box::new(move |a: &PartitionState, b: &PartitionState| {
        let (mut a_items, mut b_items) = match (a.queue.front(), b.queue.front()) {
            // Normally, they'll both have items.
            (Some(a), Some(b)) => (get_order_by_items(a)?, get_order_by_items(b)?),

            // Whichever one doesn't have an item is considered to be "less than" the other.
            (None, Some(_)) => return Ok(std::cmp::Ordering::Less),
            (Some(_), None) => return Ok(std::cmp::Ordering::Greater),
            (None, None) => return Ok(std::cmp::Ordering::Equal),
        };
        for ordering in orderings.iter() {
            let base_order = match (a_items.next(), b_items.next()) {
                (Some(a), Some(b)) => compare_json(a, b),
                _ => {
                    return Err(Error::QueryPlanInvalid(
                        "rows have inconsistent numbers of order by items".into(),
                    ))
                }
            };
            let order = match ordering {
                SortOrder::Ascending => base_order,
                SortOrder::Descending => base_order.reverse(),
            };
            if order != std::cmp::Ordering::Equal {
                return Ok(order);
            }
        }

        // If the values are equal, fall back to partition key order
        by_partition_key(a, b)
    })
}

impl QueryPipeline {
    pub fn from_plan(plan: QueryPlan, partition_key_ranges: Vec<PartitionKeyRange>) -> Self {
        let rewritten_query = format_query(&plan.query_info.rewritten_query);
        let partitions = partition_key_ranges
            .into_iter()
            .map(|partition_key_range| PartitionState {
                partition_key_range,
                queue: VecDeque::new(),
                stage: QueryStage::NotStarted,
            })
            .collect();

        let comparer = if plan.query_info.order_by.is_empty() {
            // Comparisons will be by partition key.
            Box::new(by_partition_key)
        } else {
            // Comparisons will be based on the "order_by_items" in the query
            by_order_by_items(plan.query_info.order_by)
        };

        Self {
            rewritten_query,
            partitions,
            buffer: VecDeque::new(),
            comparer,
        }
    }

    pub fn next(&mut self) -> Result<Option<PipelineResult>, Error> {
        self.ensure_buffer()?;

        if self.buffer.is_empty() {
            Ok(self.more_data())
        } else {
            // TODO: Preserve the capacity?
            let new_buffer = VecDeque::new();
            let data = std::mem::replace(&mut self.buffer, new_buffer);
            Ok(Some(PipelineResult::Data(data)))
        }
    }

    /// Ensures the buffer has data to return to the caller. If the buffer is empty, it will request more data from the partitions.
    /// If the buffer is still empty after this call, then there is no more data available in the partition buffers.
    fn ensure_buffer(&mut self) -> Result<(), Error> {
        // If the buffer is non-empty, return early.
        if !self.buffer.is_empty() {
            return Ok(());
        }

        // Fill the buffers from data in the partitions, using the comparator to sort the results.
        loop {
            // Figure out which partition has the next item to return, using the comparator to determine which item to return next.
            let mut next_partition = None;
            for partition in self.partitions.iter_mut() {
                next_partition = match (next_partition, partition) {
                    (None, p) => Some(p),
                    (Some(p1), p2) => {
                        if (self.comparer)(p1, p2)? == std::cmp::Ordering::Greater {
                            Some(p2)
                        } else {
                            Some(p1)
                        }
                    }
                };
            }

            if let Some(partition) = next_partition {
                if let Some(value) = partition.queue.pop_front() {
                    // If we found an item to return, pop it from the partition and push it into the buffer.
                    self.buffer.push_back(value);
                }
            } else {
                // If we didn't find an item to return, then the partitions are exhausted and we can stop.
                break;
            }
        }
        Ok(())
    }

    fn more_data(&self) -> Option<PipelineResult> {
        // The buffer is empty, so we need to request more data from the partitions.
        let mut partition_continuations = Vec::new();
        for partition in self.partitions.iter() {
            match partition.stage {
                QueryStage::NotStarted => {
                    // If the partition has not been started yet, we need to issue a query for it.
                    partition_continuations.push(PartitionContinuation {
                        partition_key_range_id: &partition.partition_key_range.id,
                        continuation: None,
                    });
                }
                QueryStage::Continuing(ref continuation) => {
                    // If the partition has a continuation token, we need to issue a query for it.
                    partition_continuations.push(PartitionContinuation {
                        partition_key_range_id: &partition.partition_key_range.id,
                        continuation: Some(continuation.as_str()),
                    });
                }
                QueryStage::Finished => {
                    // If the partition has been exhausted, we don't need to issue a query for it.
                    continue;
                }
            }
        }

        if partition_continuations.is_empty() {
            None
        } else {
            Some(PipelineResult::NeedsMoreData(PipelineDataRequest {
                query: self.rewritten_query.as_str(),
                partitions: partition_continuations,
            }))
        }
    }
}

pub enum PipelineResult<'a> {
    /// Indicates that the pipeline has data available for consumption.
    Data(VecDeque<serde_json::Value>),

    /// Indicates that the pipeline needs more data to continue processing.
    NeedsMoreData(PipelineDataRequest<'a>),
}

fn format_query(query: &str) -> String {
    query.replace("{documentdb-formattableorderbyquery-filter}", "true")
}
