use std::collections::VecDeque;

use crate::{
    query::{Error, PartitionContinuation, PipelineDataRequest, QueryPlan, QueryRange, SortOrder},
    PartitionKeyRange,
};

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
    buffer: Vec<serde_json::Value>,

    /// A function used to compare two partitions and determine which one has the next item to return.
    ///
    /// This function should return an ordering that is ascending, regardless of the order requested in the ORDER BY (which means descending sort orders should reverse the comparison they return)
    comparer: Box<PartitionComparer>,

    /// A function that can extract the payload (the value that should be returned to the user) from the result of a rewritten query.
    // TODO: Replace this with a more generic set of pipeline nodes to handle things like TOP, OFFSET, Aggregates, etc.
    extractor: fn(serde_json::Value) -> Result<serde_json::Value, Error>,
}

fn by_partition_key(a: &PartitionState, b: &PartitionState) -> Result<std::cmp::Ordering, Error> {
    Ok(a.partition_key_range
        .min_inclusive
        .cmp(&b.partition_key_range.min_inclusive))
}

fn by_order_by_items(orderings: Vec<SortOrder>) -> Box<PartitionComparer> {
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
                (Some(a), Some(b)) => compare_json(a, b)?,
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
        // TODO: Filter incoming partition key ranges by the query ranges specified in the query plan.
        // For now, panic if we have a query that doesn't span the entire range of partition key ranges.
        if plan.query_ranges
            != vec![QueryRange {
                is_min_inclusive: true,
                is_max_inclusive: false,
                min: "".into(),
                max: "FF".into(),
            }]
        {
            panic!("query plan has query ranges, but this implementation does not support that");
        }

        let rewritten_query = format_query(&plan.query_info.rewritten_query);
        let partitions = partition_key_ranges
            .into_iter()
            .map(|partition_key_range| PartitionState {
                partition_key_range,
                queue: VecDeque::new(),
                stage: QueryStage::NotStarted,
            })
            .collect();

        let (comparer, extractor) = if plan.query_info.order_by.is_empty() {
            // Comparisons will be by partition key.
            fn identity(i: serde_json::Value) -> Result<serde_json::Value, Error> {
                Ok(i)
            }
            (
                Box::new(by_partition_key) as Box<PartitionComparer>,
                identity as fn(serde_json::Value) -> Result<serde_json::Value, Error>,
            )
        } else {
            // Comparisons will be based on the "order_by_items" in the query
            fn payload_extractor(mut i: serde_json::Value) -> Result<serde_json::Value, Error> {
                i.as_object_mut()
                    .and_then(|o| o.remove("payload"))
                    .ok_or(Error::QueryPlanInvalid(
                        "item returned by rewritten query does not have 'payload' property".into(),
                    ))
            }
            (
                by_order_by_items(plan.query_info.order_by),
                payload_extractor as fn(serde_json::Value) -> Result<serde_json::Value, Error>,
            )
        };

        Self {
            rewritten_query,
            partitions,
            buffer: Vec::new(),
            comparer,
            extractor,
        }
    }

    /// Adds data to the start of the pipeline for a given partition.
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn enqueue_data(
        &mut self,
        partition_key_range_id: String,
        values: Vec<serde_json::Value>,
        continuation: Option<String>,
    ) -> Result<(), Error> {
        let partition = self
            .partitions
            .iter_mut()
            .find(|p| p.partition_key_range.id == partition_key_range_id)
            .ok_or(Error::PartitionNotFound(partition_key_range_id))?;

        partition.queue.extend(values);
        partition.stage = match continuation {
            Some(s) => QueryStage::Continuing(s),
            None => QueryStage::Finished,
        };

        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn step_pipeline(&mut self) -> Result<Option<PipelineResult>, Error> {
        self.ensure_buffer()?;

        if self.buffer.is_empty() {
            Ok(self.more_data())
        } else {
            // TODO: Preserve the capacity?
            let new_buffer = Vec::new();
            let data = std::mem::replace(&mut self.buffer, new_buffer);
            Ok(Some(PipelineResult::Data(data)))
        }
    }

    /// Ensures the buffer has data to return to the caller. If the buffer is empty, it will request more data from the partitions.
    /// If the buffer is still empty after this call, then there is no more data available in the partition buffers.
    #[tracing::instrument(level = "trace", skip(self))]
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
                // Skip partitions that have no queued items.
                if partition.queue.is_empty() {
                    continue;
                }

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
                    tracing::debug!(partition_key_range_id = %partition.partition_key_range.id, "found item to return");
                    // If we found an item to return, pop it from the partition and push it into the buffer.
                    self.buffer.push((self.extractor)(value)?);
                }
            } else {
                // If we didn't find an item to return, then all partitions have empty queues and we can stop filling the buffer.
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
                    tracing::debug!(partition_key_range_id = %partition.partition_key_range.id, "partition has not started yet, starting");
                    // If the partition has not been started yet, we need to issue a query for it.
                    partition_continuations.push(PartitionContinuation {
                        partition_key_range_id: partition.partition_key_range.id.clone(),
                        continuation: None,
                    });
                }
                QueryStage::Continuing(ref continuation) => {
                    tracing::debug!(partition_key_range_id = %partition.partition_key_range.id, "partition is continuing, requesting more data");
                    // If the partition has a continuation token, we need to issue a query for it.
                    partition_continuations.push(PartitionContinuation {
                        partition_key_range_id: partition.partition_key_range.id.clone(),
                        continuation: Some(continuation.clone()),
                    });
                }
                QueryStage::Finished => {
                    tracing::debug!(partition_key_range_id = %partition.partition_key_range.id, "partition exhausted");
                    // If the partition has been exhausted, we don't need to issue a query for it.
                    continue;
                }
            }
        }

        if partition_continuations.is_empty() {
            None
        } else {
            Some(PipelineResult::NeedsMoreData(PipelineDataRequest {
                query: self.rewritten_query.clone(),
                partitions: partition_continuations,
            }))
        }
    }
}

pub enum PipelineResult {
    /// Indicates that the pipeline has data available for consumption.
    Data(Vec<serde_json::Value>),

    /// Indicates that the pipeline needs more data to continue processing.
    NeedsMoreData(PipelineDataRequest),
}

fn format_query(query: &str) -> String {
    query.replace("{documentdb-formattableorderbyquery-filter}", "true")
}

/// This method unwraps the 'item' property from the JSON value, AND computes an "ordinal" for it's type.
/// The type ordinal is used to determine the order of the types when comparing them.
/// If the type ordinals for two JSON values are not equal, then the values are not equal and we can return the ordering of the type ordinals instead of comparing the values.
fn unwrap_json_and_get_type_ordinal(
    j: &serde_json::Value,
) -> Result<(usize, &serde_json::Value), Error> {
    // The Python SDK returns a type ordinal of '0' if there's no item property, but it is an error case, so we're returning an error here.
    let item = j.as_object().and_then(|o| o.get("item")).ok_or_else(|| {
        Error::QueryPlanInvalid("orderByItem does not have 'item' property".into())
    })?;

    let type_ordinal = match item {
        serde_json::Value::Null => 0,
        serde_json::Value::Bool(_) => 1,
        serde_json::Value::Number(_) => 2,
        serde_json::Value::String(_) => 3,
        _ => {
            return Err(Error::QueryPlanInvalid(
                "cannot order by non-primitive type".into(),
            ))
        }
    };
    Ok((type_ordinal, item))
}

fn compare_json(a: &serde_json::Value, b: &serde_json::Value) -> Result<std::cmp::Ordering, Error> {
    // Extract the item property from each value.
    let (a_ordinal, a_value) = unwrap_json_and_get_type_ordinal(a)?;
    let (b_ordinal, b_value) = unwrap_json_and_get_type_ordinal(b)?;

    // Compare ordinals
    if a_ordinal != b_ordinal {
        // If the ordinals are not equal, the values differ by type and we can just sort based on the type ordinal.
        return Ok(a_ordinal.cmp(&b_ordinal));
    }

    // Now we know the types match, but we need to compare the values.
    let ordering = match (a_value, b_value) {
        (serde_json::Value::Null, serde_json::Value::Null) => std::cmp::Ordering::Equal,
        (serde_json::Value::Bool(a), serde_json::Value::Bool(b)) => a.cmp(b),
        (serde_json::Value::Number(a), serde_json::Value::Number(b)) => {
            // Try integer comparison first. This will fail if either value is a float
            if let (Some(a_int), Some(b_int)) = (a.as_i128(), b.as_i128()) {
                a_int.cmp(&b_int)
            } else {
                // as_f64 is nominally fallible, but failure is only possible if the value is somehow an invalid float, which serde_json should forbid.
                let a = a.as_f64().expect("expected to unwrap as a float");
                let b = b.as_f64().expect("expected to unwrap as a float");
                a.total_cmp(&b)
            }
        }
        (serde_json::Value::String(a), serde_json::Value::String(b)) => a.cmp(b),
        _ => {
            panic!("expected the types to match because we checked that in the ordinal comparison")
        }
    };
    Ok(ordering)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        query::{
            pipeline::PipelineResult, PartitionContinuation, PipelineDataRequest, QueryInfo,
            QueryPlan, QueryRange, SortOrder,
        },
        PartitionKeyRange,
    };

    use super::QueryPipeline;

    // The content of the query doesn't matter to us in these tests.
    const TEST_QUERY: &str = "SELECT * FROM c WHERE foo = bar";

    fn make_item(value: usize, title: &impl std::fmt::Debug) -> serde_json::Value {
        json!({"id": value, "name": format!("Item {value} / {title:?}")})
    }

    fn make_order_by_item<
        V: Into<serde_json::Value>,
        I: IntoIterator<Item = V> + std::fmt::Debug,
    >(
        comparands: I,
        id: usize,
    ) -> serde_json::Value {
        let item = make_item(id, &comparands);
        let comparands = comparands
            .into_iter()
            .map(|v| json!({"item": v.into()}))
            .collect::<Vec<_>>();
        json!({"orderByItems": comparands, "payload": item})
    }

    fn create_test_pipeline(orderings: impl IntoIterator<Item = SortOrder>) -> QueryPipeline {
        let plan = QueryPlan {
            version: Some(1),
            query_info: QueryInfo::from_query(TEST_QUERY)
                .with_order_by(orderings.into_iter().collect()),
            query_ranges: vec![QueryRange::FULL_SPAN],
        };
        let pk_ranges = vec![
            // Our partitions are reversed, with partition 1 having a lower range than partition 0.
            // This is designed to exercise our partition key range comparison logic.
            PartitionKeyRange::new("0", "99", "FF"),
            PartitionKeyRange::new("1", "00", "99"),
        ];
        QueryPipeline::from_plan(plan, pk_ranges)
    }

    #[test]
    pub fn initial_call_to_next_always_returns_more_data() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut pipeline = create_test_pipeline([]);
        let result = pipeline.step_pipeline()?;
        let Some(PipelineResult::NeedsMoreData(request)) = result else {
            return Err("expected a PipelineResult::NeedsMoreData".into());
        };
        assert_eq!(
            PipelineDataRequest {
                query: TEST_QUERY.into(),
                partitions: vec![
                    PartitionContinuation {
                        partition_key_range_id: "0".into(),
                        continuation: None,
                    },
                    PartitionContinuation {
                        partition_key_range_id: "1".into(),
                        continuation: None,
                    },
                ]
            },
            request
        );
        Ok(())
    }

    #[test]
    pub fn next_returns_data_ordered_by_partition_if_no_order_by(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut pipeline = create_test_pipeline([]);
        pipeline.enqueue_data(
            "0".into(),
            vec![
                json!(make_item(1, &1)),
                json!(make_item(2, &2)),
                json!(make_item(3, &3)),
                json!(make_item(4, &4)),
            ],
            None,
        )?;
        pipeline.enqueue_data(
            "1".into(),
            vec![
                json!(make_item(5, &5)),
                json!(make_item(6, &6)),
                json!(make_item(7, &7)),
                json!(make_item(8, &8)),
            ],
            None,
        )?;

        let result = pipeline.step_pipeline()?;
        let Some(PipelineResult::Data(data)) = result else {
            return Err("expected a PipelineResult::Data".into());
        };
        assert_eq!(
            // Reminder: The partitions (0,1) are in reverse order, and should appear as (1,0)
            vec![
                json!({"id": 5, "name": "Item 5"}),
                json!({"id": 6, "name": "Item 6"}),
                json!({"id": 7, "name": "Item 7"}),
                json!({"id": 8, "name": "Item 8"}),
                json!({"id": 1, "name": "Item 1"}),
                json!({"id": 2, "name": "Item 2"}),
                json!({"id": 3, "name": "Item 3"}),
                json!({"id": 4, "name": "Item 4"}),
            ],
            data,
        );
        Ok(())
    }

    #[test]
    pub fn order_by_integer() -> Result<(), Box<dyn std::error::Error>> {
        let mut pipeline = create_test_pipeline([SortOrder::Ascending]);

        // We're enqueing data that is interleaved by the order by item
        // Reminder: The pipeline ASSUMES data in each partition is ordered!
        pipeline.enqueue_data(
            "0".into(),
            vec![
                make_order_by_item([1], 1),
                make_order_by_item([3], 3),
                make_order_by_item([5], 5),
                make_order_by_item([7], 7),
            ],
            None,
        )?;
        pipeline.enqueue_data(
            "1".into(),
            vec![
                make_order_by_item([2], 2),
                make_order_by_item([4], 4),
                make_order_by_item([6], 6),
                make_order_by_item([8], 8),
            ],
            None,
        )?;

        let result = pipeline.step_pipeline()?;
        let Some(PipelineResult::Data(data)) = result else {
            return Err("expected a PipelineResult::Data".into());
        };
        assert_eq!(
            vec![
                json!({"id": 1, "name": "Item 1 / [1]"}),
                json!({"id": 2, "name": "Item 2 / [2]"}),
                json!({"id": 3, "name": "Item 3 / [3]"}),
                json!({"id": 4, "name": "Item 4 / [4]"}),
                json!({"id": 5, "name": "Item 5 / [5]"}),
                json!({"id": 6, "name": "Item 6 / [6]"}),
                json!({"id": 7, "name": "Item 7 / [7]"}),
                json!({"id": 8, "name": "Item 8 / [8]"}),
            ],
            data,
        );
        Ok(())
    }

    #[test]
    pub fn order_by_string() -> Result<(), Box<dyn std::error::Error>> {
        let mut pipeline = create_test_pipeline([SortOrder::Ascending]);

        // We're enqueing data that is interleaved by the order by item
        // Reminder: The pipeline ASSUMES data in each partition is ordered!
        pipeline.enqueue_data(
            "0".into(),
            vec![
                make_order_by_item(["aaaa"], 1),
                make_order_by_item(["aaac"], 3),
                make_order_by_item(["aaae"], 5),
                make_order_by_item(["aaag"], 7),
            ],
            None,
        )?;
        pipeline.enqueue_data(
            "1".into(),
            vec![
                make_order_by_item(["aaab"], 2),
                make_order_by_item(["aaad"], 4),
                make_order_by_item(["aaaf"], 6),
                make_order_by_item(["aaah"], 8),
            ],
            None,
        )?;

        let result = pipeline.step_pipeline()?;
        let Some(PipelineResult::Data(data)) = result else {
            return Err("expected a PipelineResult::Data".into());
        };
        assert_eq!(
            vec![
                json!({"id": 1, "name": "Item 1 / [\"aaaa\"]"}),
                json!({"id": 2, "name": "Item 2 / [\"aaab\"]"}),
                json!({"id": 3, "name": "Item 3 / [\"aaac\"]"}),
                json!({"id": 4, "name": "Item 4 / [\"aaad\"]"}),
                json!({"id": 5, "name": "Item 5 / [\"aaae\"]"}),
                json!({"id": 6, "name": "Item 6 / [\"aaaf\"]"}),
                json!({"id": 7, "name": "Item 7 / [\"aaag\"]"}),
                json!({"id": 8, "name": "Item 8 / [\"aaah\"]"}),
            ],
            data,
        );
        Ok(())
    }

    #[test]
    pub fn order_by_bool() -> Result<(), Box<dyn std::error::Error>> {
        let mut pipeline = create_test_pipeline([SortOrder::Ascending]);

        // We're enqueing data that is interleaved by the order by item
        // Reminder: The pipeline ASSUMES data in each partition is ordered!
        pipeline.enqueue_data(
            "0".into(),
            vec![
                make_order_by_item([false], 1),
                make_order_by_item([false], 5),
                make_order_by_item([true], 3),
                make_order_by_item([true], 7),
            ],
            None,
        )?;
        pipeline.enqueue_data(
            "1".into(),
            vec![
                make_order_by_item([false], 4),
                make_order_by_item([false], 8),
                make_order_by_item([true], 2),
                make_order_by_item([true], 6),
            ],
            None,
        )?;

        let result = pipeline.step_pipeline()?;
        let Some(PipelineResult::Data(data)) = result else {
            return Err("expected a PipelineResult::Data".into());
        };
        assert_eq!(
            // False is less than true, and we're sorting ascending.
            // In the case of equality, the lower partition key range goes first.
            // And, as a reminder, our partitions are in reverse order (to further "exercise" our comparison logic, so partition 1 has a lower range than partition 0)
            vec![
                json!({"id": 4, "name": "Item 4 / [false]"}),
                json!({"id": 8, "name": "Item 8 / [false]"}),
                json!({"id": 1, "name": "Item 1 / [false]"}),
                json!({"id": 5, "name": "Item 5 / [false]"}),
                json!({"id": 2, "name": "Item 2 / [true]"}),
                json!({"id": 6, "name": "Item 6 / [true]"}),
                json!({"id": 3, "name": "Item 3 / [true]"}),
                json!({"id": 7, "name": "Item 7 / [true]"}),
            ],
            data
        );
        Ok(())
    }

    #[test]
    pub fn order_by_null() -> Result<(), Box<dyn std::error::Error>> {
        let mut pipeline = create_test_pipeline([SortOrder::Ascending]);

        // We're enqueing data that is interleaved by the order by item
        // Reminder: The pipeline ASSUMES data in each partition is ordered!
        pipeline.enqueue_data(
            "0".into(),
            vec![
                make_order_by_item([serde_json::Value::Null], 1),
                make_order_by_item([serde_json::Value::Null], 3),
                make_order_by_item([1], 5),
                make_order_by_item([3], 7),
            ],
            None,
        )?;
        pipeline.enqueue_data(
            "1".into(),
            vec![
                make_order_by_item([serde_json::Value::Null], 2),
                make_order_by_item([serde_json::Value::Null], 4),
                make_order_by_item([2], 6),
                make_order_by_item([4], 8),
            ],
            None,
        )?;

        let result = pipeline.step_pipeline()?;
        let Some(PipelineResult::Data(data)) = result else {
            return Err("expected a PipelineResult::Data".into());
        };
        assert_eq!(
            // Null is less than non-Null, and we're sorting ascending.
            // In the case of equality, the lower partition key range goes first.
            // And, as a reminder, our partitions are in reverse order (to further "exercise" our comparison logic, so partition 1 has a lower range than partition 0)
            vec![
                json!({"id": 2, "name": "Item 2 / [Null]"}),
                json!({"id": 4, "name": "Item 4 / [Null]"}),
                json!({"id": 1, "name": "Item 1 / [Null]"}),
                json!({"id": 3, "name": "Item 3 / [Null]"}),
                json!({"id": 5, "name": "Item 5 / [1]"}),
                json!({"id": 6, "name": "Item 6 / [2]"}),
                json!({"id": 7, "name": "Item 7 / [3]"}),
                json!({"id": 8, "name": "Item 8 / [4]"}),
            ],
            data
        );
        Ok(())
    }

    #[test]
    pub fn order_by_mixed() -> Result<(), Box<dyn std::error::Error>> {
        let mut pipeline = create_test_pipeline([SortOrder::Ascending]);

        // We're enqueing data that is interleaved by the order by item
        // Reminder: The pipeline ASSUMES data in each partition is ordered!
        pipeline.enqueue_data(
            "0".into(),
            vec![make_order_by_item([false], 2), make_order_by_item([1], 3)],
            None,
        )?;
        pipeline.enqueue_data(
            "1".into(),
            vec![
                make_order_by_item([serde_json::Value::Null], 1),
                make_order_by_item([1.1], 4),
                make_order_by_item(["aaa"], 5),
            ],
            None,
        )?;

        let result = pipeline.step_pipeline()?;
        let Some(PipelineResult::Data(data)) = result else {
            return Err("expected a PipelineResult::Data".into());
        };
        assert_eq!(
            // Null is less than non-Null, and we're sorting ascending.
            // In the case of equality, the lower partition key range goes first.
            // And, as a reminder, our partitions are in reverse order (to further "exercise" our comparison logic, so partition 1 has a lower range than partition 0)
            vec![
                json!({"id": 1, "name": "Item 1 / [Null]"}),
                json!({"id": 2, "name": "Item 2 / [false]"}),
                json!({"id": 3, "name": "Item 3 / [1]"}),
                json!({"id": 4, "name": "Item 4 / [1.1]"}),
                json!({"id": 5, "name": "Item 5 / [\"aaa\"]"}),
            ],
            data
        );
        Ok(())
    }

    #[test]
    pub fn order_by_descending() -> Result<(), Box<dyn std::error::Error>> {
        let mut pipeline = create_test_pipeline([SortOrder::Descending]);

        // We're enqueing data that is interleaved by the order by item
        // Reminder: The pipeline ASSUMES data in each partition is ordered!
        pipeline.enqueue_data(
            "0".into(),
            vec![make_order_by_item([1], 3), make_order_by_item([false], 2)],
            None,
        )?;
        pipeline.enqueue_data(
            "1".into(),
            vec![
                make_order_by_item(["aaa"], 5),
                make_order_by_item([1.1], 4),
                make_order_by_item([serde_json::Value::Null], 1),
            ],
            None,
        )?;

        let result = pipeline.step_pipeline()?;
        let Some(PipelineResult::Data(data)) = result else {
            return Err("expected a PipelineResult::Data".into());
        };
        assert_eq!(
            // In the case of equality, the lower partition key range goes first, even when sorting descending.
            // And, as a reminder, our partitions are in reverse order (to further "exercise" our comparison logic, so partition 1 has a lower range than partition 0)
            vec![
                json!({"id": 5, "name": "Item 5 / [\"aaa\"]"}),
                json!({"id": 4, "name": "Item 4 / [1.1]"}),
                json!({"id": 3, "name": "Item 3 / [1]"}),
                json!({"id": 2, "name": "Item 2 / [false]"}),
                json!({"id": 1, "name": "Item 1 / [Null]"}),
            ],
            data
        );
        Ok(())
    }

    #[test]
    pub fn order_by_multiple() -> Result<(), Box<dyn std::error::Error>> {
        let mut pipeline = create_test_pipeline([SortOrder::Ascending, SortOrder::Descending]);

        // We're enqueing data that is interleaved by the order by item
        // Reminder: The pipeline ASSUMES data in each partition is ordered!
        pipeline.enqueue_data(
            "0".into(),
            vec![
                make_order_by_item(
                    [
                        serde_json::Value::Number(1.into()),
                        serde_json::Value::String("zzzz".into()),
                    ],
                    1,
                ),
                make_order_by_item(
                    [
                        serde_json::Value::Number(1.into()),
                        serde_json::Value::String("bbbb".into()),
                    ],
                    3,
                ),
                make_order_by_item(
                    [
                        serde_json::Value::Number(2.into()),
                        serde_json::Value::String("yyyy".into()),
                    ],
                    6,
                ),
                make_order_by_item(
                    [
                        serde_json::Value::Number(3.into()),
                        serde_json::Value::String("aaaa".into()),
                    ],
                    8,
                ),
            ],
            None,
        )?;
        pipeline.enqueue_data(
            "1".into(),
            vec![
                make_order_by_item(
                    [
                        serde_json::Value::Number(1.into()),
                        serde_json::Value::String("yyyy".into()),
                    ],
                    2,
                ),
                make_order_by_item(
                    [
                        serde_json::Value::Number(1.into()),
                        serde_json::Value::String("aaaa".into()),
                    ],
                    4,
                ),
                make_order_by_item(
                    [
                        serde_json::Value::Number(2.into()),
                        serde_json::Value::String("zzzz".into()),
                    ],
                    5,
                ),
                make_order_by_item(
                    [
                        serde_json::Value::Number(3.into()),
                        serde_json::Value::String("zzzz".into()),
                    ],
                    7,
                ),
            ],
            None,
        )?;

        let result = pipeline.step_pipeline()?;
        let Some(PipelineResult::Data(data)) = result else {
            return Err("expected a PipelineResult::Data".into());
        };
        assert_eq!(
            vec![
                json!({"id": 1, "name": "Item 1 / [Number(1), String(\"zzzz\")]"}),
                json!({"id": 2, "name": "Item 2 / [Number(1), String(\"yyyy\")]"}),
                json!({"id": 3, "name": "Item 3 / [Number(1), String(\"bbbb\")]"}),
                json!({"id": 4, "name": "Item 4 / [Number(1), String(\"aaaa\")]"}),
                json!({"id": 5, "name": "Item 5 / [Number(2), String(\"zzzz\")]"}),
                json!({"id": 6, "name": "Item 6 / [Number(2), String(\"yyyy\")]"}),
                json!({"id": 7, "name": "Item 7 / [Number(3), String(\"zzzz\")]"}),
                json!({"id": 8, "name": "Item 8 / [Number(3), String(\"aaaa\")]"}),
            ],
            data
        );
        Ok(())
    }
}
