# Query Engine Specification: ReadItems

## Problem Statement

The Cosmos DB driver (`azure_data_cosmos_driver`) needs a client-side query execution
pipeline. This spec focuses on the first operation: **ReadItems**, which reads multiple
items by their ID + partition key pairs, grouped and routed to physical partitions.

The pipeline is internal to the driver crate (`pub(crate)`). The general architecture
(pull-based composable streams, concurrency limiting, metrics) is designed to extend
to other operations (cross-partition ORDER BY, vector queries, aggregates) in the future.

## ReadItems Operation

### Public API

```rust
pub struct ItemIdentity {
    pub id: String,
    pub partition_key: PartitionKey,
}

impl CosmosDriver {
    pub fn read_items(
        &self,
        items: Vec<ItemIdentity>,
        container: &ContainerReference,
        options: OperationOptions,
    ) -> PipelineStream { ... }
}
```

### Planning

The planner takes the list of `ItemIdentity` and produces a `QueryPlan`:

1. Hash each PK → effective partition key using `ContainerReference::partition_key_definition()`.
2. Map each EPK → physical partition (pkrange) via the cached partition key range map.
3. Group items by pkrange.
4. For each pkrange group, choose a source node:
   - **1 item** → `PointRead` node
   - **2+ items, all same logical PK** → `QueryIdsInSinglePk` node
   - **2+ items, multiple logical PKs** → `QueryIdPkPairs` node
5. All sources feed into a single `UnsortedMerge` output node.

### Plan Example

```text
Input: [(id1, pkA), (id2, pkA), (id3, pkB), (id4, pkB), (id5, pkC)]

After grouping by physical partition:
  pkrange0: [(id1, pkA), (id2, pkA)]         → same logical PK → QueryIdsInSinglePk
  pkrange1: [(id3, pkB), (id4, pkB), (id5, pkC)] → mixed PKs  → QueryIdPkPairs
  (assume these are the only two physical partitions)

Plan DAG:
  QueryIdsInSinglePk(pkrange0, pkA, [id1, id2]) ──┐
                                                    ├── UnsortedMerge → output
  QueryIdPkPairs(pkrange1, [(id3,pkB),(id4,pkB),(id5,pkC)]) ──┘
```

If a pkrange group had exactly one item, it would be a `PointRead` instead.

## Module Structure

```text
sdk/cosmos/azure_data_cosmos_driver/src/
└── pipeline/
    ├── mod.rs              # Module root, re-exports, PipelineStream, PipelineBatch
    ├── row.rs              # PipelineRow enum, RowShape
    ├── plan.rs             # QueryPlan, PlanEdge, validation
    ├── executor.rs         # ExecutionContext, ExecutionOptions, IoPermitGuard
    ├── metrics.rs          # NodeMetrics, PipelineMetrics, MetricsSnapshot
    └── nodes/
        ├── mod.rs          # PlanNode enum, output_shape/input_shape, build() dispatch
        ├── point_read.rs   # PointRead build: returns impl Stream
        ├── query_ids_in_single_pk.rs  # QueryIdsInSinglePk build: returns impl Stream
        ├── query_id_pk_pairs.rs       # QueryIdPkPairs build: returns impl Stream
        └── unsorted_merge.rs          # UnsortedMerge build: returns impl Stream
```

## Design

### Row Shapes (`row.rs`)

A closed enum representing the possible data shapes flowing through the pipeline.

```rust
pub(crate) enum PipelineRow {
    RawItem(Box<RawValue>),
}

#[derive(PartialEq, Eq)]
pub(crate) enum RowShape {
    RawItem,
}
```

### Plan Nodes (`nodes/mod.rs`)

The `PlanNode` enum defines all node types. Each variant carries the data needed to
construct its stream. Shape declarations and the `build()` dispatch live here;
node-specific stream construction lives in submodules.

```rust
pub(crate) enum PlanNode {
    PointRead {
        item_id: String,
        partition_key: PartitionKey,
    },

    QueryIdsInSinglePk {
        pk_range_id: String,
        partition_key: PartitionKey,
        item_ids: Vec<String>,
    },

    QueryIdPkPairs {
        pk_range_id: String,
        items: Vec<ItemIdentity>,
    },

    UnsortedMerge,
}

impl PlanNode {
    pub fn output_shape(&self) -> RowShape {
        match self {
            PlanNode::PointRead { .. } => RowShape::RawItem,
            PlanNode::QueryIdsInSinglePk { .. } => RowShape::RawItem,
            PlanNode::QueryIdPkPairs { .. } => RowShape::RawItem,
            PlanNode::UnsortedMerge => RowShape::RawItem,
        }
    }

    pub fn input_shape(&self) -> Option<RowShape> {
        match self {
            PlanNode::PointRead { .. } => None,
            PlanNode::QueryIdsInSinglePk { .. } => None,
            PlanNode::QueryIdPkPairs { .. } => None,
            PlanNode::UnsortedMerge => Some(RowShape::RawItem),
        }
    }

    pub fn is_source(&self) -> bool {
        self.input_shape().is_none()
    }

    pub fn build(
        self,
        inputs: Vec<BoxedPipelineStream>,
        ctx: ExecutionContext,
    ) -> BoxedPipelineStream {
        match self {
            PlanNode::PointRead { item_id, partition_key } => {
                Box::pin(point_read::build(item_id, partition_key, ctx))
            }
            PlanNode::QueryIdsInSinglePk { pk_range_id, partition_key, item_ids } => {
                Box::pin(query_ids_in_single_pk::build(
                    pk_range_id, partition_key, item_ids, ctx,
                ))
            }
            PlanNode::QueryIdPkPairs { pk_range_id, items } => {
                Box::pin(query_id_pk_pairs::build(pk_range_id, items, ctx))
            }
            PlanNode::UnsortedMerge => {
                Box::pin(unsorted_merge::build(inputs))
            }
        }
    }
}

type BoxedPipelineStream = Pin<Box<dyn Stream<Item = Result<PipelineBatch>> + Send>>;
```

Each submodule exposes a `build()` function that returns
`impl Stream<Item = Result<PipelineBatch>> + Send`. The node is free to use any
combination of `futures` stream combinators, custom `Stream` structs, or async
blocks to produce the stream.

### Query Plan (`plan.rs`)

The plan is a DAG of nodes + edges. `PlanNode` is defined in `nodes/mod.rs`;
`QueryPlan` is defined here alongside `PlanEdge` and the DAG validation logic.

```rust
// Derives relevant traits to act as a newtype.
pub(crate) struct NodeId(usize);

pub(crate) struct PlanEdge {
    pub from: NodeId,
    pub to: NodeId,
}

pub(crate) struct QueryPlan {
    pub nodes: Vec<PlanNode>,
    pub edges: Vec<PlanEdge>,
    pub output_node: NodeId,
    pub container: ContainerReference,
}
```

#### QueryPlan Methods

```rust
impl QueryPlan {
    // Only called in debug builds
    pub fn validate(&self) -> Result<()> {
        for edge in &self.edges {
            assert!(edge.from < self.nodes.len());
            assert!(edge.to < self.nodes.len());
        }

        assert!(self.output_node < self.nodes.len());
        assert!(!self.edges.iter().any(|e| e.from == self.output_node));

        for edge in &self.edges {
            let from_output = self.nodes[edge.from].output_shape();
            if let Some(to_input) = self.nodes[edge.to].input_shape() {
                assert!(from_output == to_input);
            }
        }

        // Check acyclicity via topological sort
        // (standard Kahn's algorithm over self.nodes / self.edges)

        Ok(())
    }

    pub fn source_nodes(&self) -> Vec<NodeId> {
        let has_incoming: HashSet<NodeId> = self.edges.iter().map(|e| e.to).collect();
        (0..self.nodes.len())
            .filter(|id| !has_incoming.contains(id))
            .collect()
    }

    pub fn inputs_for(&self, node: NodeId) -> Vec<NodeId> {
        self.edges.iter()
            .filter(|e| e.to == node)
            .map(|e| e.from)
            .collect()
    }
}
```

#### Node Shape Declarations

| Node                | Input Shape(s) | Output Shape |
|---------------------|----------------|--------------|
| PointRead           | (none)         | RawItem      |
| QueryIdsInSinglePk  | (none)         | RawItem      |
| QueryIdPkPairs      | (none)         | RawItem      |
| UnsortedMerge       | RawItem        | RawItem      |

### Execution Context and Options (`executor.rs`)

#### ExecutionOptions

```rust
pub(crate) struct ExecutionOptions {
    pub max_concurrent_sources: usize,
}
```

#### ExecutionContext

Bundles the shared resources that every node needs. Cheaply cloneable.

```rust
#[derive(Clone)]
pub(crate) struct ExecutionContext {
    driver: Arc<CosmosDriver>,
    container: ContainerReference,
    options: OperationOptions,
    execution_options: ExecutionOptions,
    semaphore: Arc<tokio::sync::Semaphore>,
}

impl ExecutionContext {
    pub fn new(
        driver: Arc<CosmosDriver>,
        container: ContainerReference,
        options: OperationOptions,
        execution_options: ExecutionOptions,
    ) -> Self {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(execution_options.max_concurrent_sources));
        Self { driver, container, options, execution_options, semaphore }
    }

    pub fn driver(&self) -> &CosmosDriver { ... }
    pub fn container(&self) -> &ContainerReference { ... }
    pub fn options(&self) -> &OperationOptions { ... }
    pub fn execution_options(&self) -> &ExecutionOptions { ... }

    pub async fn acquire_io_permit(&self) -> IoPermitGuard {
        let permit = self.semaphore.clone().acquire_owned().await
            .expect("semaphore closed");
        IoPermitGuard { _permit: permit }
    }
}

pub(crate) struct IoPermitGuard {
    _permit: tokio::sync::OwnedSemaphorePermit,
}
```

### Pipeline Executor (`executor.rs`)

The executor assembles a `QueryPlan` into a `PipelineStream`. Source nodes are
spawned as independent `tokio` tasks for multi-threaded execution, communicating
results through a bounded `mpsc` channel. The consumer receives results by polling
the channel's receiver.

```rust
pub(crate) struct PipelineExecutor;

impl PipelineExecutor {
    pub fn build(
        plan: QueryPlan,
        driver: Arc<CosmosDriver>,
        options: OperationOptions,
        execution_options: ExecutionOptions,
    ) -> PipelineStream {
        let ctx = ExecutionContext::new(
            driver,
            plan.container.clone(),
            options,
            execution_options,
        );

        let stream = Self::build_node(plan.output_node, &plan, &ctx);
        PipelineStream::new(stream)
    }

    fn build_node(
        node_id: NodeId,
        plan: &QueryPlan,
        ctx: &ExecutionContext,
    ) -> BoxedPipelineStream {
        let node = plan.nodes[node_id].clone();

        if node.is_source() {
            // Source nodes: build the stream, spawn a task to drain it
            // into a channel, return the receiver as a stream.
            let source_stream = node.build(vec![], ctx.clone());
            Self::spawn_source(source_stream)
        } else {
            // Non-source nodes: recursively build inputs, then build this node.
            let inputs: Vec<_> = plan.inputs_for(node_id)
                .into_iter()
                .map(|input_id| Self::build_node(input_id, plan, ctx))
                .collect();
            node.build(inputs, ctx.clone())
        }
    }

    fn spawn_source(source: BoxedPipelineStream) -> BoxedPipelineStream {
        let (tx, rx) = tokio::sync::mpsc::channel(1);

        let handle = tokio::spawn(async move {
            let mut source = source;
            while let Some(batch) = source.next().await {
                if tx.send(batch).await.is_err() {
                    break; // receiver dropped, consumer is done
                }
            }
        });

        Box::pin(ReceiverStream::new(rx, handle))
    }
}
```

The executor distinguishes source nodes (no inputs) from non-source nodes. Source
nodes get their stream wrapped in a spawned task + channel. Non-source nodes (like
`UnsortedMerge`) receive already-spawned source streams as inputs.

#### Concurrency Model

```text
PipelineExecutor::build()
   │
   ├── tokio::spawn ─── PointRead(id1, pk1) task
   │                       └── acquire_io_permit → read → tx.send(batch)
   │
   ├── tokio::spawn ─── QueryIdsInSinglePk(pkrange0, ...) task
   │                       └── acquire_io_permit → fetch page → tx.send(batch) → loop
   │
   ├── tokio::spawn ─── QueryIdPkPairs(pkrange1, ...) task
   │                       └── acquire_io_permit → fetch page → tx.send(batch) → loop
   │
   └── UnsortedMerge (not spawned, runs on consumer's task)
         └── Receives from all source channels, yields in arrival order

Consumer calls poll_next() on PipelineStream
   → polls UnsortedMerge
   → gets next batch from whichever source task finished first
```

Source tasks run independently on tokio worker threads. The semaphore
(`max_concurrent_sources`) gates how many can perform I/O at once — others
park on the semaphore's wait list (not consuming a worker thread). The
bounded channel (capacity 1) provides back-pressure: if the consumer is
slow, source tasks block on `tx.send()` after producing one batch.

**Key properties:**

- **Multi-threaded execution** — source tasks run on tokio worker threads,
  utilizing all available cores for request setup and response parsing.
- **Back-pressure** — bounded channel prevents sources from racing ahead
  of the consumer.
- **Cancellation** — dropping the `PipelineStream` drops the channel
  receivers, causing source tasks to observe a closed channel and exit.
- **Semaphore limits I/O** — at most `max_concurrent_sources` HTTP
  requests in flight, regardless of how many source tasks exist.

### Pipeline Stream

The consumer-facing output type. Wraps the output node's stream and owns
the spawned task handles for cancellation.

```rust
pub(crate) struct PipelineBatch {
    pub rows: Vec<PipelineRow>,
    pub metrics: MetricsSnapshot,
}

pub(crate) struct PipelineStream {
    inner: BoxedPipelineStream,
}

impl Stream for PipelineStream {
    type Item = Result<PipelineBatch>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().inner.as_mut().poll_next(cx)
    }
}
```

`ReceiverStream` (used internally by `spawn_source`) wraps a
`tokio::sync::mpsc::Receiver` as a `Stream` and aborts its associated
`JoinHandle` on drop, ensuring spawned tasks are cancelled when the
consumer drops the pipeline.

### Metrics (`metrics.rs`)

Metrics are collected inline by each node. Since source nodes run on separate tasks,
per-node metrics must be `Send`. Metrics can be sent alongside batches in the
channel, or aggregated when the task completes.

```rust
pub(crate) struct NodeMetrics {
    pub node_id: NodeId,
    pub node_type: &'static str,

    pub start_time: Option<Instant>,
    pub end_time: Option<Instant>,
    pub time_waiting_for_input: Duration,

    pub items_received: u64,
    pub items_emitted: u64,
    pub bytes_received: u64,
    pub bytes_emitted: u64,

    pub pages_fetched: u64,
    pub requests_issued: u64,
    pub request_units_consumed: f64,
    pub throttled_responses: u64,
}

pub(crate) struct PipelineMetrics {
    pub nodes: Vec<NodeMetrics>,
    pub total_wall_clock: Duration,
    pub total_request_units: f64,
}

pub(crate) struct MetricsSnapshot {
    pub total_items_emitted: u64,
    pub total_request_units: f64,
    pub elapsed: Duration,
    pub nodes: Vec<NodeMetrics>,
}
```

### Node Implementations

Each node module exposes a `build()` function that returns
`impl Stream<Item = Result<PipelineBatch>> + Send`. Nodes are free to use any
combination of `futures` stream combinators, custom `Stream` structs, or async
blocks to produce the stream.

#### PointRead (`nodes/point_read.rs`)

A one-shot source. Yields a single `RawItem` and completes.

```rust
pub(super) fn build(
    item_id: String,
    partition_key: PartitionKey,
    ctx: ExecutionContext,
) -> impl Stream<Item = Result<PipelineBatch>> + Send {
    futures::stream::once(async move {
        let _permit = ctx.acquire_io_permit().await;
        let operation = CosmosOperation::read_item(
            ctx.container(), &partition_key, &item_id,
        );
        let response = ctx.driver().execute_operation(operation, ctx.options().clone()).await?;
        let raw = RawValue::from_string(String::from_utf8_lossy(response.body()).into_owned())?;
        Ok(PipelineBatch {
            rows: vec![PipelineRow::RawItem(raw)],
            metrics: MetricsSnapshot::default(),
        })
    })
}
```

#### QueryIdsInSinglePk (`nodes/query_ids_in_single_pk.rs`)

Queries items sharing a single logical PK within a physical partition. Pages
through results via continuation tokens.

```rust
pub(super) fn build(
    pk_range_id: String,
    partition_key: PartitionKey,
    item_ids: Vec<String>,
    ctx: ExecutionContext,
) -> impl Stream<Item = Result<PipelineBatch>> + Send {
    futures::stream::unfold(
        State { pk_range_id, partition_key, item_ids, ctx, continuation: None, done: false },
        |mut state| async move {
            if state.done { return None; }

            let _permit = state.ctx.acquire_io_permit().await;
            let query = build_in_list_query(&state.partition_key, &state.item_ids);
            let response = execute_query(
                &state.ctx, &state.pk_range_id, query, state.continuation.take(),
            ).await;

            match response {
                Ok((rows, continuation)) => {
                    state.done = continuation.is_none();
                    state.continuation = continuation;
                    Some((Ok(PipelineBatch { rows, metrics: MetricsSnapshot::default() }), state))
                }
                Err(e) => {
                    state.done = true;
                    Some((Err(e), state))
                }
            }
        },
    )
}
```

#### QueryIdPkPairs (`nodes/query_id_pk_pairs.rs`)

Queries items with different logical PKs within a single physical partition.
Pages through results via continuation tokens.

```rust
pub(super) fn build(
    pk_range_id: String,
    items: Vec<ItemIdentity>,
    ctx: ExecutionContext,
) -> impl Stream<Item = Result<PipelineBatch>> + Send {
    futures::stream::unfold(
        State { pk_range_id, items, ctx, continuation: None, done: false },
        |mut state| async move {
            if state.done { return None; }

            let _permit = state.ctx.acquire_io_permit().await;
            let query = build_id_pk_pairs_query(&state.items);
            let response = execute_query(
                &state.ctx, &state.pk_range_id, query, state.continuation.take(),
            ).await;

            match response {
                Ok((rows, continuation)) => {
                    state.done = continuation.is_none();
                    state.continuation = continuation;
                    Some((Ok(PipelineBatch { rows, metrics: MetricsSnapshot::default() }), state))
                }
                Err(e) => {
                    state.done = true;
                    Some((Err(e), state))
                }
            }
        },
    )
}
```

#### UnsortedMerge (`nodes/unsorted_merge.rs`)

Concurrently polls all input streams and yields results in arrival order.
When inputs are channel receivers from spawned source tasks, this effectively
collects results from all source tasks as they complete.

```rust
pub(super) fn build(
    inputs: Vec<BoxedPipelineStream>,
) -> impl Stream<Item = Result<PipelineBatch>> + Send {
    let mut merged = futures::stream::select_all(inputs);
    futures::stream::poll_fn(move |cx| merged.poll_next_unpin(cx))
}
```

`select_all` from `futures` merges multiple streams into one, polling all of
them and yielding items as they become available. Since each input is backed
by a channel receiver from a spawned source task, this naturally yields
results in arrival order with no additional coordination.

## Design Considerations

### Error Handling

Source nodes use driver APIs that already handle retries and failover. The pipeline
only sees terminal failures — when all retry/failover options have been exhausted,
the source node yields an `Err`, which propagates through the merge to the consumer.
The entire pipeline fails on the first error.

### Partition Split Recovery

Source nodes may encounter 410 Gone (partition split). In the pull-based model, a
source can internally refresh the routing table and restart with updated pkrange
IDs. Since source nodes own their state and are only accessed via `poll_next`,
recovery is local — the merge node is unaware. **Deferred for detailed design but
the architecture naturally supports it.**

### Pipeline Resumability

For stateless web apps with pagination, we'll eventually need to serialize pipeline
state (continuation tokens per source, which sources are exhausted, etc.) to resume
later. **Deferred but kept in mind** — the query node types already carry
continuation state internally.

### Runtime Dependency

The pipeline requires a `tokio` multi-threaded runtime for spawning source tasks.
A compile-time abstraction using feature flags to support other runtimes is planned
separately.

### Future Node Types

The architecture is designed to support additional nodes for other operations:

- **OrderByMerge** — streaming k-way merge for cross-partition ORDER BY queries
- **BufferAndSort** — buffered sort for vector/full-text queries
- **AggregateAccumulate** — partial aggregate accumulation for COUNT/SUM/etc.

These will introduce additional `PipelineRow` variants and new `PlanNode` variants.
Each new node adds a variant to the enum in `nodes/mod.rs` and a corresponding
submodule with its `build()` function.
