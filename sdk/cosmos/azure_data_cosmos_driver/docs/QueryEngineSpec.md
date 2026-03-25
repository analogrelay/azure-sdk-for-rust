# Query Engine Specification

## Problem Statement

The Cosmos DB driver (`azure_data_cosmos_driver`) needs a client-side query execution pipeline that can:

- Model query execution as a DAG of typed nodes
- Support concurrent execution of source nodes with configurable limits
- Stream results with back-pressure when possible, buffer when required
- Produce per-node and pipeline-level metrics
- Handle varying data shapes efficiently through a closed type system
- Support inline partition-split recovery

The pipeline is internal to the driver crate (`pub(crate)`). Plans are constructed either
programmatically (e.g., ReadMany) or by translating a backend-provided query plan JSON, or both.

**This document uses illustrative row shapes that may not directly represent the actual rows returned by the backend.**

## Module Structure

```text
sdk/cosmos/azure_data_cosmos_driver/src/
└── pipeline/
    ├── mod.rs              # Module root, re-exports, PipelineStream type alias
    ├── row.rs              # PipelineRow enum + OrderByRow, etc.
    ├── plan.rs             # QueryPlan, PlanNode enum, PlanEdge, validation
    ├── executor.rs         # build_pipeline() — assembles a plan into a composed Stream
    ├── metrics.rs          # NodeMetrics, PipelineMetrics, MetricsSnapshot
    ├── node/
    │   ├── mod.rs          # Common node types
    │   ├── partition_query.rs  # Stream impl: pages through a single-partition query
    │   ├── point_read.rs       # Stream impl: yields a single item
    │   ├── order_by_merge.rs   # Stream impl: k-way merge over input streams
    │   ├── buffer_and_sort.rs  # Stream impl: collects all inputs, sorts, yields
    │   └── aggregate.rs        # Stream impl: accumulates partials, yields final
    └── concurrency.rs      # ConcurrencyLimiter (semaphore-based)
```

## Design

### Row Shapes (`row.rs`)

A closed enum representing the possible data shapes flowing through the pipeline.
Nodes declare their expected input/output shapes at plan construction time.
Debug-mode validation checks compatibility; runtime nodes fail the pipeline on shape mismatch.

```rust
/// A single value in an ORDER BY clause, as provided by the backend.
/// Used for comparison during k-way merge.
pub(crate) struct OrderByValue {
    /// The raw JSON value (could be string, number, bool, null, etc.)
    /// Type is inferred from the JSON for sorting purposes.
    pub value: Box<RawValue>,
}

/// A row from a backend ORDER BY query.
pub(crate) struct OrderByRow {
    /// One value per ORDER BY clause, in clause order.
    pub order_by_items: Vec<OrderByValue>,
    /// The actual document, deferred for parsing.
    pub payload: Box<RawValue>,
}

/// The shapes that can flow between pipeline nodes.
pub(crate) enum PipelineRow {
    /// A plain JSON document (point reads, final output).
    RawItem(Box<RawValue>),
    /// A row from an ORDER BY query with sort keys + payload.
    OrderBy(OrderByRow),
    /// A partial or final aggregate result.
    Aggregate(Box<RawValue>),
}

/// Declares which PipelineRow variant(s) a node accepts/produces.
/// Used for plan validation.
pub(crate) enum RowShape {
    RawItem,
    OrderBy,
    Aggregate,
    /// Node accepts multiple shapes (e.g., a collect node).
    AnyOf(Vec<RowShape>),
}
```

### Query Plan (`plan.rs`)

The plan is a DAG represented as a flat list of nodes + edges. Each node has an ID
(index into the vec). The plan is validated at construction time (debug builds) and
is immutable once built.

```rust
pub(crate) type NodeId = usize;

/// A node in the query plan DAG.
pub(crate) enum PlanNode {
    /// Issues a SQL query against a single physical partition, paging via continuations.
    PartitionQuery {
        /// The SQL query text + parameters.
        query: Query,
        /// Target physical partition range.
        pk_range_id: String,
        /// The continuation token, if resuming.
        continuation: Option<String>,
    },

    /// Reads a single item by ID + partition key.
    PointRead {
        item_id: String,
        partition_key: PartitionKey,
    },

    /// Streaming k-way merge of pre-sorted inputs using a binary heap.
    /// Inputs MUST produce OrderBy rows. Output is RawItem (extracted payload).
    OrderByMerge {
        /// Sort directions per ORDER BY clause (ascending/descending).
        sort_orders: Vec<SortOrder>,
    },

    /// Collects ALL input rows, sorts them, then emits.
    /// For vector/full-text queries where streaming isn't possible.
    BufferAndSort {
        sort_orders: Vec<SortOrder>,
    },

    /// Accumulates partial aggregates from multiple partitions.
    AggregateAccumulate {
        /// The aggregate functions to apply (COUNT, SUM, MIN, MAX, AVG, etc.)
        functions: Vec<AggregateFunction>,
    },
}

pub(crate) enum SortOrder {
    Ascending,
    Descending,
}

pub(crate) enum AggregateFunction {
    Count,
    Sum,
    Min,
    Max,
    Average,
}

/// Directed edge: output of `from` feeds into `to`.
pub(crate) struct PlanEdge {
    pub from: NodeId,
    pub to: NodeId,
}

/// A complete query execution plan.
pub(crate) struct QueryPlan {
    /// Nodes in topological order. Index = NodeId.
    pub nodes: Vec<PlanNode>,
    /// Directed edges (from → to).
    pub edges: Vec<PlanEdge>,
    /// The final node whose output is the pipeline result.
    pub output_node: NodeId,
    /// Container context for executing operations.
    pub container: ContainerReference,
}

impl QueryPlan {
    /// Validate the plan DAG:
    /// - Acyclic
    /// - All edges reference valid nodes
    /// - Row shape compatibility between connected nodes
    /// - Exactly one output node with no outgoing edges
    /// Only runs in debug builds for performance.
    pub fn validate(&self) -> Result<()> { ... }

    /// Returns source nodes (nodes with no incoming edges).
    pub fn source_nodes(&self) -> Vec<NodeId> { ... }

    /// Returns the input node IDs for a given node.
    pub fn inputs_for(&self, node: NodeId) -> Vec<NodeId> { ... }
}
```

#### Node Shape Declarations

Used by `validate` to check row shape compatibility between connected nodes:

| Node                | Input Shape(s) | Output Shape                             |
|---------------------|----------------|------------------------------------------|
| PartitionQuery      | (none)         | OrderBy or RawItem (configured per-plan) |
| PointRead           | (none)         | RawItem                                  |
| OrderByMerge        | OrderBy        | RawItem                                  |
| BufferAndSort       | OrderBy        | RawItem                                  |
| AggregateAccumulate | Aggregate      | RawItem (final result)                   |

### Pipeline Executor (`executor.rs`)

The executor takes a `QueryPlan` + a `CosmosDriver` reference and assembles a
composed `Stream` by wiring node streams together according to the plan's DAG
structure. The pipeline is **pull-based**: no tasks are spawned, no channels are
created. The consumer's `poll_next()` drives all I/O.

```rust
pub(crate) struct PipelineExecutor {
    concurrency_limit: usize,
}

impl PipelineExecutor {
    pub fn new(concurrency_limit: usize) -> Self { ... }

    /// Assemble a query plan into a composed stream of batched results.
    ///
    /// Walks the plan DAG bottom-up from the output node, constructing each
    /// node's Stream and passing upstream streams as inputs. Returns the
    /// output node's stream.
    pub fn build(
        &self,
        plan: QueryPlan,
        driver: Arc<CosmosDriver>,
        options: OperationOptions,
    ) -> PipelineStream { ... }
}
```

#### Pull-Based Execution Model

The pipeline uses a pull-based architecture where each node is a `Stream`
implementation. The consumer's `poll_next()` on the output stream propagates
through the DAG:

1. The executor walks the plan DAG from the output node, recursively constructing
   each node's `Stream`. Source node streams are passed as inputs to their
   downstream transform/merge node streams.
2. Each node implements `Stream<Item = Result<PipelineBatch>>`. When polled, a node
   pulls from its upstream inputs as needed.
3. Source nodes (`PartitionQuery`, `PointRead`) perform I/O only when polled. Each
   source acquires a `ConcurrencyLimiter` permit before issuing a request,
   ensuring bounded concurrent I/O across all sources in the pipeline.
4. When the consumer drops the `PipelineStream`, all node state is dropped with
   it — no orphaned tasks, no cleanup needed.

#### Batching

Each node drains as many ready items as possible from its inputs without blocking
on I/O, then yields the batch downstream. For example, an `OrderByMerge` node:

1. On `poll_next`, tries to pull from each input stream without awaiting I/O
   (using `poll_next` on each input — returns `Pending` if I/O is needed).
2. From all currently-buffered head items, determines which items can be emitted
   (all items that sort before any pending/unknown input).
3. Yields those items as a `PipelineBatch`.
4. On the next `poll_next`, awaits I/O for inputs that were `Pending`.

#### Concurrency Model

```text
Consumer calls poll_next() on PipelineStream
   │
   ▼
Output node (e.g., OrderByMerge) Stream
   │  Polls upstream source streams via FuturesUnordered
   │
   ├── PartitionQuery(pkrange1) Stream
   │     └── poll_next() → acquire semaphore permit → fetch page → yield rows
   ├── PartitionQuery(pkrange2) Stream
   │     └── poll_next() → acquire semaphore permit → fetch page → yield rows
   └── PartitionQuery(pkrange3) Stream
         └── poll_next() → acquire semaphore permit → fetch page → yield rows

Concurrency is achieved by polling multiple source streams through
FuturesUnordered. The semaphore bounds how many sources can perform
I/O simultaneously. When the consumer stops polling, all I/O stops.
```

**Key properties:**

- **No task spawning** — runtime-agnostic; works on any async executor.
- **Natural back-pressure** — I/O only happens when the consumer polls.
- **Partial consumption is efficient** — if the consumer only needs N items,
  only the sources needed to produce those N items are polled.
- **Cancellation is trivial** — dropping the stream drops all node state.

### Pipeline Stream (`stream.rs`)

The consumer-facing output type. This is a type alias (or thin wrapper) for the
output node's composed `Stream`. Since the pipeline is pull-based, the stream IS
the pipeline — polling it drives all upstream nodes.

```rust
/// A batch of results from the pipeline.
pub(crate) struct PipelineBatch {
    /// The result rows in this batch.
    pub rows: Vec<PipelineRow>,
    /// Metrics snapshot as of this batch.
    pub metrics: MetricsSnapshot,
}

/// The output stream from a pipeline execution.
/// Implements `Stream<Item = Result<PipelineBatch>>`.
///
/// The pipeline is pull-based: polling this stream drives all upstream
/// node I/O. Dropping this stream drops all node state and stops all work.
pub(crate) struct PipelineStream {
    /// The composed stream from the output node.
    inner: Pin<Box<dyn Stream<Item = Result<PipelineBatch>> + Send>>,
}

impl Stream for PipelineStream {
    type Item = Result<PipelineBatch>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().inner.as_mut().poll_next(cx)
    }
}
```

No `Drop` implementation is needed — when the stream is dropped, all owned node
state (including source node page buffers, merge heaps, etc.) is dropped with it.

### Metrics (`metrics.rs`)

Metrics are collected inline by each node as it processes rows. Since the pipeline
is pull-based and single-threaded from the consumer's perspective (no spawned tasks),
metrics can be updated with simple mutable state rather than atomics.

```rust
/// Per-node metrics, updated during execution.
pub(crate) struct NodeMetrics {
    pub node_id: NodeId,
    pub node_type: &'static str,  // "PartitionQuery", "OrderByMerge", etc.

    // Timing
    pub start_time: Option<Instant>,
    pub end_time: Option<Instant>,
    pub time_waiting_for_input: Duration,

    // Throughput
    pub items_received: u64,
    pub items_emitted: u64,
    pub bytes_received: u64,
    pub bytes_emitted: u64,

    // Source-node specific
    pub pages_fetched: u64,
    pub requests_issued: u64,
    pub request_units_consumed: f64,
    pub throttled_responses: u64,
}

/// Pipeline-level metrics aggregated from all nodes.
pub(crate) struct PipelineMetrics {
    pub nodes: Vec<NodeMetrics>,
    pub total_wall_clock: Duration,
    pub total_request_units: f64,
    pub peak_concurrency: usize,
}

/// A point-in-time snapshot of pipeline metrics, attached to each batch.
pub(crate) struct MetricsSnapshot {
    pub total_items_emitted: u64,
    pub total_request_units: f64,
    pub elapsed: Duration,
    /// Per-node metrics snapshots.
    pub nodes: Vec<NodeMetrics>,
}
```

Each node holds a mutable reference (or owned instance) of its `NodeMetrics` and
updates it as rows flow through. When producing a `PipelineBatch`, the output node
collects snapshots from all nodes to build the `MetricsSnapshot`.

### Concurrency Limiter (`concurrency.rs`)

```rust
/// Controls the number of concurrent I/O operations in a pipeline.
pub(crate) struct ConcurrencyLimiter {
    semaphore: Arc<Semaphore>,
    peak: AtomicUsize,
}

impl ConcurrencyLimiter {
    pub fn new(max_concurrent: usize) -> Self { ... }

    /// Acquire a permit before performing I/O. Blocks if at limit.
    pub async fn acquire(&self) -> SemaphorePermit { ... }

    /// Current peak concurrency observed.
    pub fn peak_concurrency(&self) -> usize { ... }
}
```

Uses `async_lock::Semaphore` (already a dependency) rather than `tokio::sync::Semaphore`
to stay runtime-agnostic.

### Node Implementation

Rather than a `Node` trait with dynamic dispatch, each node type is a struct that
implements `Stream<Item = Result<PipelineBatch>>`. The executor constructs the
appropriate node based on the `PlanNode` variant and composes them:

```rust
// Source node example: PartitionQuery
pub(crate) struct PartitionQueryStream {
    query: Query,
    pk_range_id: String,
    driver: Arc<CosmosDriver>,
    container: ContainerReference,
    options: OperationOptions,
    limiter: Arc<ConcurrencyLimiter>,
    metrics: NodeMetrics,

    // Internal state
    page_buffer: VecDeque<PipelineRow>,
    continuation: Option<String>,
    exhausted: bool,
    pending_fetch: Option<Pin<Box<dyn Future<Output = Result<CosmosResponse>> + Send>>>,
}

impl Stream for PartitionQueryStream {
    type Item = Result<PipelineBatch>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // 1. If page_buffer has items, drain and yield a batch
        // 2. If exhausted (no continuation), return None
        // 3. If no pending fetch, acquire semaphore permit + start fetch
        // 4. Poll pending fetch — when ready, parse page, fill buffer, loop
    }
}

// Merge node example: OrderByMerge
pub(crate) struct OrderByMergeStream {
    sort_orders: Vec<SortOrder>,
    /// Input source streams, polled concurrently via FuturesUnordered.
    inputs: Vec<Pin<Box<dyn Stream<Item = Result<PipelineBatch>> + Send>>>,
    metrics: NodeMetrics,

    // Internal state
    heap: BinaryHeap<HeapEntry>,  // min-heap of (OrderByRow, source_index)
    pending_polls: FuturesUnordered<...>,  // concurrent input polling
}

impl Stream for OrderByMergeStream {
    type Item = Result<PipelineBatch>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // 1. Poll all pending input futures (FuturesUnordered)
        // 2. Push newly arrived items into the heap
        // 3. Drain items from heap that are guaranteed to be in order
        //    (all inputs must have at least one item buffered, or be exhausted)
        // 4. Yield drained items as a batch, or Pending if waiting on inputs
    }
}
```

The executor's `build()` method constructs these streams bottom-up:

```rust
// In executor.rs — simplified sketch
fn build_node_stream(
    node: PlanNode,
    inputs: Vec<Pin<Box<dyn Stream<Item = Result<PipelineBatch>> + Send>>>,
    driver: Arc<CosmosDriver>,
    container: ContainerReference,
    options: OperationOptions,
    limiter: Arc<ConcurrencyLimiter>,
) -> Pin<Box<dyn Stream<Item = Result<PipelineBatch>> + Send>> {
    match node {
        PlanNode::PartitionQuery { query, pk_range_id, continuation } => {
            Box::pin(PartitionQueryStream::new(
                query, pk_range_id, continuation,
                driver, container, options, limiter,
            ))
        }
        PlanNode::PointRead { item_id, partition_key } => {
            Box::pin(PointReadStream::new(
                item_id, partition_key,
                driver, container, options, limiter,
            ))
        }
        PlanNode::OrderByMerge { sort_orders } => {
            Box::pin(OrderByMergeStream::new(sort_orders, inputs))
        }
        PlanNode::BufferAndSort { sort_orders } => {
            Box::pin(BufferAndSortStream::new(sort_orders, inputs))
        }
        PlanNode::AggregateAccumulate { functions } => {
            Box::pin(AggregateStream::new(functions, inputs))
        }
    }
}
```

This avoids trait objects for dispatch — the `PlanNode` enum is the single point of
dispatch and we get exhaustive match checking from the compiler. Each node's stream
is type-erased into `Pin<Box<dyn Stream + Send>>` only at the composition boundary.

### Ordering Semantics

All `OrderByValue` comparisons follow the Cosmos DB type ordering:

```text
null < boolean < number < string < array < object
```

Within a type, standard comparison rules apply. This is implemented as a custom
`Ord` on `OrderByValue`. The type of each value is inferred from the raw JSON at
comparison time — values may be heterogeneous types across partitions.

## Scenario Walkthroughs

### Scenario 1: ReadMany

```text
User provides: [(id1, pk1), (id2, pk2), (id3, pk3), ...]

Planner:
  1. Hash each PK → effective partition key
  2. Map EPK → pkrange via cached routing table
  3. Group items by pkrange
  4. For groups with 1 item → PointRead node
  5. For groups with N items → PartitionQuery node (SELECT ... WHERE id IN ...)
  6. All sources → OrderByMerge (sort by id + pk, implicit)

Plan DAG:
  PointRead(id1, pk1) ─┐
  PartitionQuery(Q1)  ──┤
  PartitionQuery(Q2)  ──┼── OrderByMerge(asc id, asc pk) → output
  PointRead(id5, pk5) ──┘
```

### Scenario 2: Cross-Partition ORDER BY

```text
User query: SELECT * FROM c ORDER BY c.price ASC

Planner:
  1. Get all pkranges for the container
  2. For each pkrange → PartitionQuery node (same query)
     Backend returns OrderByRow { orderByItems: [price], payload: doc }
  3. All sources → OrderByMerge(asc)

Plan DAG:
  PartitionQuery(pkrange1) ──┐
  PartitionQuery(pkrange2) ──┼── OrderByMerge(asc) → output
  PartitionQuery(pkrange3) ──┘
```

### Scenario 3: Vector/Full-Text Query (TOP N)

```text
User query: SELECT TOP 10 * FROM c ORDER BY VectorDistance(c.embedding, [...])

Planner:
  1. For each pkrange → PartitionQuery (same query, TOP 10 per partition)
  2. All sources → BufferAndSort (must collect all before sorting)

Plan DAG:
  PartitionQuery(pkrange1) ──┐
  PartitionQuery(pkrange2) ──┼── BufferAndSort(sort by vector distance) → output
  PartitionQuery(pkrange3) ──┘
```

### Scenario 4: Aggregate (COUNT)

```text
User query: SELECT VALUE COUNT(1) FROM c

Planner:
  1. For each pkrange → PartitionQuery (aggregate query, returns partial count)
  2. All sources → AggregateAccumulate(Sum) — sum the partial counts

Plan DAG:
  PartitionQuery(pkrange1) ──┐
  PartitionQuery(pkrange2) ──┼── AggregateAccumulate(Sum) → output
  PartitionQuery(pkrange3) ──┘
```

## Design Considerations

### Error Handling

Source nodes use driver APIs that already handle retries and failover. The pipeline
only sees terminal failures — when all retry/failover options have been exhausted,
the entire pipeline fails with the underlying error.

### Partition Split Recovery

Source nodes need a mechanism to detect 410 Gone (partition split), refresh the
routing table, and re-target the new sub-partitions. In the pull-based model, a
source node that encounters a split can internally replace itself with two child
streams (or re-query the routing table and restart with the correct pkrange ID).
Since source nodes own their state and are only accessed via `poll_next`, this
recovery is local to the source — the merge node upstream is unaware. **Deferred
for detailed design but the pull-based architecture naturally supports it** — a
source node can transparently become a fan-out internally.

### Pipeline Resumability

For stateless web apps with pagination, we'll eventually need to serialize enough
pipeline state (continuation tokens per source, merge position, etc.) to resume later.
The plan structure + per-node continuation state should be serializable. **Deferred but
kept in mind** — e.g., `PlanNode::PartitionQuery` already has an optional `continuation`.

### Runtime Agnosticism

The pull-based model avoids task spawning entirely. All I/O is driven by the
consumer's `poll_next()` calls, so the pipeline works on any async executor
(tokio, async-std, smol, etc.). The only async primitive required is
`async_lock::Semaphore` for the concurrency limiter, which is already a
dependency and is runtime-agnostic.

### Plan Construction

Plans can be constructed in two ways:

1. **Programmatically** — the driver builds a plan directly (e.g., for ReadMany,
   where the plan shape is known statically).
2. **From backend query plan** — the backend parses the user's SQL and returns a
   query plan in a custom JSON format. The driver translates this into a `QueryPlan`.
   The planner module handles this translation.
