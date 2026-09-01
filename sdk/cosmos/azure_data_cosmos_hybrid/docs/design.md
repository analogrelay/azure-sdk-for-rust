# Azure Cosmos DB Hybrid Query SDK

## Status

- **Date:** September 1, 2026
- **Crate:** `azure_data_cosmos_hybrid`
- **Stage:** Prototype design
- **Primary dependency:** `azure_data_cosmos`
- **Implementation dependency:** `azure_data_cosmos_driver` query parser and planner,
  exposed behind a non-default prototype feature
- **Default integrations:** Microsoft `mssql-rs` and Azure OpenAI, each behind
  an independently configurable default-on feature

## 1. Summary

`azure_data_cosmos_hybrid` is a proposed SDK for Hybrid Transactional and
Analytical Processing (HTAP) over Azure Cosmos DB.

The SDK wraps an existing `azure_data_cosmos::CosmosClient`. Cosmos DB remains
the authoritative data store and the default query engine. Applications can
attach sidecar engines that provide additional query capabilities:

- execute lag-tolerant analytical queries against a Cosmos DB mirror exposed
  through a Microsoft Fabric SQL analytics endpoint;
- transform query parameters, such as embedding text before a Cosmos DB vector
  query;
- rerank Cosmos DB query candidates using an AI model; and
- eventually query and merge multiple data sources.

The customer-facing experience remains close to the Cosmos DB SDK:

```rust
let hybrid = HybridQueryClient::builder(cosmos_client)
    .with_query_source(fabric_source)
    .with_parameter_transform(embedding_provider)
    .with_reranker(semantic_reranker)
    .build()?;

let container = hybrid
    .container_client("catalog", "products")
    .await?;

let query = HybridQuery::from(
    "SELECT TOP 10 * FROM c WHERE VectorDistance(c.embedding, @vector) < 0.4",
)
    .with_transformed_parameter(
        "@vector",
        "trail running shoes",
        ParameterTransformDirective::new("azure-openai-embedding"),
    )?
    .with_semantic_reranking(
        SemanticRerankingOptions::new("azure-openai-reranker")
            .with_candidate_count(50)
            .with_result_count(10),
    );

let results = container
    .query_items::<Product>(query, FeedScope::full_container())
    .await?;
```

An ordinary `HybridQuery` with no hybrid directives is passed directly to
Cosmos DB, regardless of which engines are attached.

## 2. Goals

1. Preserve a simple, Cosmos-like query experience.
2. Keep Cosmos DB authoritative and the safe default.
3. Use Cosmos DB SQL text and parameterized queries as the common query model.
4. Support parameter transformation, alternate query sources, and result
   reranking without adding a new SQL dialect.
5. Allow customers and partners to implement engines through public traits.
6. Make planning decisions inspectable before execution and observable after
   execution.
7. Keep engine failures explicit while allowing policy-controlled fallback.
8. Avoid exposing driver AST and query plan types through the stable hybrid SDK
   API.
9. Leave room for multi-source execution and merging without requiring it in
   the first prototype.

## 3. Non-goals

The first prototype does not:

- replace the Cosmos DB SDK for writes, point reads, change feed, or resource
  management;
- define hybrid-specific Cosmos SQL syntax;
- guarantee that an analytical mirror has a specific maximum replication lag;
- translate the full Cosmos DB SQL language to T-SQL;
- provide continuation tokens for reranked, transformed, analytical, or
  multi-source plans;
- automatically offload ordinary queries merely because a sidecar is attached;
- expose driver parser, AST, or planner types as stable public API;
- implement arbitrary distributed joins or transactions across engines; or
- claim semantic equivalence when a target engine uses different null, missing,
  numeric, collation, or ordering behavior.

## 4. Design principles

### 4.1 Cosmos is authoritative

Cosmos DB is the only source eligible for the five standard Cosmos consistency
constraints. Sidecar sources are eligible only when the caller explicitly
selects the `Analytical` consistency constraint.

Writes always occur through `azure_data_cosmos`. The hybrid crate has no write
API.

### 4.2 Default execution is passthrough

The default `HybridQuery` produces a Cosmos-only plan. Parameter transforms,
analytical offload, and reranking are expressed directly on the query.

Attaching an engine changes capability, not behavior.

### 4.3 Typed options, not a new language

The query remains Cosmos DB SQL plus JSON parameters. Hybrid intent is carried
by typed options. This avoids creating SQL that Cosmos DB cannot execute and
keeps fallback possible.

### 4.4 Central planning

Engines advertise capabilities, but they do not independently execute based on
their own interpretation of a query. A central planner:

1. parses and analyzes the Cosmos DB SQL;
2. validates query options;
3. identifies eligible engines;
4. constructs an immutable execution plan; and
5. executes the selected stages.

This prevents conflicting engine decisions and makes `explain` deterministic.

### 4.5 Separate data from behavior

Execution requests, capabilities, plans, warnings, scores, and provenance are
plain data structures. Behavior is implemented by role-specific traits.

The first extension roles are:

- `ParameterTransform`
- `QuerySource`
- `ResultReranker`

These roles have different inputs, outputs, and failure semantics. A single
catch-all engine trait would make valid stage ordering difficult to express and
would require most implementations to reject most callbacks.

## 5. Public client hierarchy

The hybrid client mirrors the Cosmos client hierarchy without wrapping every
Cosmos operation.

```text
CosmosClient
    |
    v
HybridQueryClient
    |
    +-- database and container identity
    v
HybridContainerClient
    |
    +-- query_items<T>
    +-- explain
```

Proposed construction:

```rust
pub struct HybridQueryClientBuilder {
    // Private registration and policy state.
}

impl HybridQueryClient {
    pub fn builder(cosmos_client: CosmosClient) -> HybridQueryClientBuilder;

    pub async fn container_client(
        &self,
        database: impl Into<ResourceIdentity>,
        container: impl Into<ResourceIdentity>,
    ) -> Result<HybridContainerClient>;
}
```

`HybridContainerClient` owns a clone of the native `ContainerClient`, the
engine registry, and an immutable snapshot of `ContainerProperties`.
Construction resolves the Cosmos container metadata needed for partition-key
analysis and extension planning.

The builder is preferred over mutable `attach_*` methods so a client is
immutable and safe to share after construction.

## 6. Hybrid query

The hybrid SDK owns a single `HybridQuery` type containing Cosmos SQL text,
parameters, Cosmos request settings, and hybrid execution directives:

```rust
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct HybridQuery {
    pub text: String,
    pub parameters: Vec<HybridQueryParameter>,
    pub cosmos: QueryOptions,
    pub consistency: ConsistencyConstraint,
    pub source: SourceSelection,
    pub parameter_transforms: Vec<ParameterTransformDirective>,
    pub reranking: Option<SemanticRerankingOptions>,
}
```

The public API uses consuming builder methods similar to
`azure_data_cosmos::Query`. Public fields are shown only to make the data model
clear.

Immediately before Cosmos execution, the hybrid SDK constructs an internal
`azure_data_cosmos::Query` from the final SQL text and transformed parameters.
The Cosmos query type does not need new inspection APIs.

Hybrid behavior belongs to the query rather than to a separate options object.
This makes the complete execution request portable, inspectable, and suitable
for `explain`.

Simple parameters and transformed parameters have parallel convenience methods:

```rust
impl HybridQuery {
    pub fn with_parameter(
        self,
        name: impl Into<String>,
        value: impl Serialize,
    ) -> Result<Self>;

    pub fn with_transformed_parameter(
        self,
        name: impl Into<String>,
        value: impl Serialize,
        transform: ParameterTransformDirective,
    ) -> Result<Self>;

    pub fn with_parameter_transform(
        self,
        parameter: impl Into<String>,
        transform: ParameterTransformDirective,
    ) -> Result<Self>;
}
```

Rust does not support an optional third positional argument, so
`with_transformed_parameter` is the concise form that atomically adds the
parameter and its first transform. `with_parameter_transform` remains available
for attaching a transform to an existing parameter or building a multi-stage
transform pipeline.

### 6.1 Consistency constraint

```rust
#[derive(Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConsistencyConstraint {
    Strong,
    BoundedStaleness,
    #[default]
    Session,
    ConsistentPrefix,
    Eventual,
    Analytical,
}
```

This type is a routing and validation constraint. It does not assert that a
sidecar can reproduce Cosmos consistency semantics.

| Constraint | Eligible source | Meaning |
| ---------- | --------------- | ------- |
| `Strong` | Cosmos only | Require Cosmos-authoritative execution compatible with strong consistency |
| `BoundedStaleness` | Cosmos only | Require Cosmos-authoritative bounded-staleness execution |
| `Session` | Cosmos only | Preserve Cosmos session behavior and session tokens |
| `ConsistentPrefix` | Cosmos only | Require Cosmos-authoritative consistent-prefix execution |
| `Eventual` | Cosmos only | Use Cosmos with eventual consistency semantics |
| `Analytical` | Analytical preferred, Cosmos fallback allowed | Caller accepts unspecified replication lag and prefers an eligible analytical source |

The planner must reject a request that attempts to strengthen consistency beyond
what the underlying Cosmos account and SDK operation can provide.

`Analytical` is intentionally symbolic. It does not accept a duration and does
not claim that actual lag can be measured. If no analytical source supports the
query, the default behavior is a Cosmos fallback with a structured warning.
Callers can make the selected source required.

### 6.2 Source selection and requirements

```rust
#[derive(Clone, Default)]
#[non_exhaustive]
pub enum SourceSelection {
    #[default]
    Cosmos,
    Prefer(EngineName),
    Require(EngineName),
    PreferAnalytical,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum EngineRequirement {
    #[default]
    Optional,
    Required,
}
```

Setting `ConsistencyConstraint::Analytical` changes the default source
selection from `Cosmos` to `PreferAnalytical`. An explicit `SourceSelection`
always wins.

Every requested transform or reranking stage carries an `EngineRequirement`.
An optional stage may fall back with a warning. A required stage fails planning
or execution when it cannot run.

## 7. Stable query analysis model

The hybrid crate depends on driver parsing and planning, but adapts driver types
into a smaller stable model:

```rust
#[derive(Clone)]
#[non_exhaustive]
pub struct QueryAnalysis {
    pub features: QueryFeatures,
    pub result_shape: QueryResultShape,
    pub partition_scope: PartitionScopeAnalysis,
    pub referenced_paths: Vec<QueryPath>,
    pub parameters: Vec<ParameterAnalysis>,
    pub container: ContainerQueryMetadata,
}

#[derive(Clone)]
#[non_exhaustive]
pub struct ParameterAnalysis {
    pub name: String,
    pub usages: Vec<ParameterUsage>,
}

#[derive(Clone)]
#[non_exhaustive]
pub enum ParameterUsage {
    Comparison {
        operator: ComparisonOperator,
        other: QueryOperand,
    },
    FunctionArgument {
        function: QueryFunction,
        argument_index: usize,
        arguments: Vec<QueryOperand>,
    },
    ResultWindow {
        clause: ResultWindowClause,
    },
    Other,
}

#[derive(Clone)]
#[non_exhaustive]
pub enum QueryOperand {
    Parameter(String),
    Property(QueryPath),
    Literal(serde_json::Value),
    Expression(QueryExpressionSummary),
}

#[derive(Clone, Default)]
#[non_exhaustive]
pub struct QueryFeatures {
    pub has_where: bool,
    pub has_join: bool,
    pub has_subquery: bool,
    pub has_udf: bool,
    pub has_distinct: bool,
    pub has_group_by: bool,
    pub has_aggregates: bool,
    pub has_order_by: bool,
    pub top: Option<u64>,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}

#[derive(Clone)]
#[non_exhaustive]
pub struct ContainerQueryMetadata {
    pub database: ResourceIdentity,
    pub container: ResourceIdentity,
    pub properties: Arc<ContainerProperties>,
}
```

Engines receive `QueryAnalysis`, including semantic usage information for every
parameter, plus the original SQL and parameter values. The hybrid planner uses
the complete driver AST to resolve aliases, property paths, operators, function
names, and function argument positions into these stable summaries.

Engines do not receive the driver AST directly. This allows the driver parser
to evolve without breaking third-party engines while still giving extensions
the context needed for intelligent decisions.

`ContainerQueryMetadata` provides read-only access to the container identity
and `ContainerProperties`, including the partition-key definition, indexing
policy, vector embedding policy, full-text policy, and service metadata retained
in extensible properties. Extensions can use this metadata to infer how to
process a query instead of requiring every detail to be repeated by the caller.

When an engine needs richer syntax information, the hybrid crate should first
add a stable, purpose-specific data type rather than publishing the AST.

## 8. Engine model

### 8.1 Registration

Engines are registered under stable names:

```rust
let client = HybridQueryClient::builder(cosmos)
    .with_query_source(FabricSqlQuerySource::new("fabric-sales", fabric_options)?)
    .with_parameter_transform(
        AzureOpenAiEmbeddingProvider::new("azure-openai-embedding", ai_options)?,
    )
    .with_reranker(
        AzureOpenAiReranker::new("azure-openai-reranker", reranker_options)?,
    )
    .build()?;
```

Duplicate names within the same role are rejected. Different roles may share
an implementation but are registered independently.

### 8.2 Parameter transforms

```rust
#[async_trait]
pub trait ParameterTransform: Send + Sync {
    fn name(&self) -> &EngineName;

    fn supports(
        &self,
        analysis: &QueryAnalysis,
        request: &ParameterTransformPlanRequest,
    ) -> EngineSupport;

    async fn transform(
        &self,
        context: &EngineContext,
        request: ParameterTransformRequest,
    ) -> Result<TransformedParameter>;
}

#[derive(Clone)]
#[non_exhaustive]
pub struct ParameterTransformPlanRequest {
    pub parameter: HybridQueryParameter,
    pub usages: Vec<ParameterUsage>,
    pub directive: ParameterTransformDirective,
    pub container: ContainerQueryMetadata,
}
```

The initial transform scenario replaces a string parameter with an embedding
vector before Cosmos execution. Every directive names the transformer to
activate:

```rust
ParameterTransformDirective::new("azure-openai-embedding")
    .with_requirement(EngineRequirement::Required)
```

The directive can also carry transformer-specific options:

```rust
ParameterTransformDirective::new("azure-openai-embedding")
    .with_options(EmbeddingTransformOptions::default().with_model("embedding-v3"))
```

There is no transformer-agnostic `auto` directive. The caller always selects
which registered transformer or transform pipeline is allowed to run. This is
necessary because embedding is only one possible parameter transformation, and
different transforms can interpret the same value in incompatible ways.

After selection, the transformer inspects both the parsed query and
`ContainerQueryMetadata`. It may use backend metadata to supply defaults,
validate explicit options, or determine whether it supports the request. An
embedding extension can, for example:

- detect that `@vector` is argument 1 of
  `VectorDistance(c.embedding, @vector)`;
- resolve the sibling argument `c.embedding` to the normalized Cosmos path
  `/embedding`;
- find the matching entry in the container's `VectorEmbeddingPolicy`;
- validate the configured vector dimensions and data type;
- inspect extension metadata associated with the vector policy for a model or
  deployment hint; and
- accept the request only when its capabilities satisfy that metadata.

The normal embedding path therefore requires only a named transformer:

```rust
ParameterTransformDirective::new("azure-openai-embedding")
```

Explicit options remain available when the property cannot be inferred or when
metadata does not contain enough configuration:

```rust
ParameterTransformDirective::new("azure-openai-embedding")
    .with_options(
        EmbeddingTransformOptions::default()
            .with_vector_path("/imageEmbedding")
            .with_model("multimodal-embedding"),
    )
```

Explicit options take precedence over metadata-derived defaults. An explicit
property override can resolve an absent or ambiguous property reference. If it
contradicts an unambiguous property found in the parsed query, planning fails
rather than embedding for a property the query does not use.

If the same parameter has incompatible usages, such as being compared to two
different vector properties, the transformer rejects the plan unless explicit
configuration resolves the ambiguity. It must never silently switch to a
different registered transformer.

The transform may replace only the named parameter. It cannot rewrite arbitrary
SQL in the first prototype. The planner validates that:

- the parameter exists;
- the transform supports its current JSON value;
- the transformed value satisfies configured dimension and numeric checks; and
- the query still parses after parameter substitution rules are applied.

### 8.3 Query sources

```rust
#[async_trait]
pub trait QuerySource: Send + Sync {
    fn name(&self) -> &EngineName;

    fn capabilities(&self) -> QuerySourceCapabilities;

    fn supports(&self, request: &QuerySourcePlanRequest) -> EngineSupport;

    async fn execute(
        &self,
        context: &EngineContext,
        request: QuerySourceRequest,
    ) -> Result<QuerySourceResponse>;
}
```

The prototype executes exactly one primary source. The public plan represents a
vector of source stages so multi-source execution can be added later without
redesigning the plan format.

`CosmosQuerySource` is built into the SDK and is not registered by the caller.

### 8.4 Result rerankers

```rust
#[async_trait]
pub trait ResultReranker: Send + Sync {
    fn name(&self) -> &EngineName;

    fn supports(&self, request: &RerankPlanRequest) -> EngineSupport;

    async fn rerank(
        &self,
        context: &EngineContext,
        request: RerankRequest,
    ) -> Result<RerankResponse>;
}
```

Rerankers operate on raw JSON candidate envelopes, not on the caller's generic
`T`:

```rust
#[derive(Clone)]
#[non_exhaustive]
pub struct QueryCandidate {
    pub identity: ItemIdentity,
    pub document: serde_json::Value,
    pub source_score: Option<f64>,
    pub provenance: ResultProvenance,
}
```

This preserves item identity and source metadata while allowing final
deserialization into any `T`.

`SemanticRerankingOptions` requires bounded values:

```rust
SemanticRerankingOptions::new("azure-openai-reranker")
    .with_candidate_count(50)
    .with_result_count(10)
```

The candidate count is distinct from SQL `TOP` or `LIMIT`. The planner rewrites
or wraps the source request when necessary to retrieve the candidate window,
then applies the final result count after reranking. The plan explanation must
show this rewrite.

## 9. Execution plan

The public plan is immutable, serializable for diagnostics, and independent of
driver types:

```rust
#[derive(Clone)]
#[non_exhaustive]
pub struct HybridQueryPlan {
    pub analysis: QueryAnalysis,
    pub stages: Vec<QueryStage>,
    pub streaming: StreamingMode,
    pub continuation: ContinuationSupport,
    pub warnings: Vec<HybridQueryWarning>,
}

#[derive(Clone)]
#[non_exhaustive]
pub enum QueryStage {
    TransformParameters(ParameterTransformPlan),
    ExecuteSource(QuerySourcePlan),
    Rerank(ResultRerankPlan),
    ProjectResults(ResultProjectionPlan),
}
```

Stage order in the first prototype is fixed:

1. parse and validate;
2. transform selected parameters;
3. select and execute one primary query source;
4. collect a bounded candidate window when reranking is requested;
5. rerank and truncate;
6. deserialize results; and
7. emit pages or items with diagnostics.

### 9.1 Explain

```rust
pub async fn explain(
    &self,
    query: HybridQuery,
    scope: FeedScope,
) -> Result<HybridQueryPlan>;
```

`explain` performs parsing, capability negotiation, translation, and validation
without executing a source or AI model. Secrets, access tokens, and parameter
values marked sensitive must not appear in debug or serialized plan output.

## 10. Fabric analytical source

### 10.1 Query surface

The initial real analytical integration targets the read-only SQL analytics
endpoint associated with a Cosmos DB mirrored database in Microsoft Fabric.
The connection uses TDS and Microsoft Entra ID authentication.

Direct OneLake DFS access is a storage integration, not the query execution
surface for this prototype.

### 10.2 Explicit mapping

Each attached Fabric source is configured with one or more explicit
Cosmos-container-to-Fabric-table mappings:

```rust
#[non_exhaustive]
pub struct FabricSqlSourceOptions {
    pub endpoint: Url,
    pub credential: Arc<dyn TokenCredential>,
    pub mappings: Vec<FabricContainerMapping>,
}

#[non_exhaustive]
pub struct FabricContainerMapping {
    pub cosmos: CosmosContainerReference,
    pub fabric: FabricTableReference,
    pub columns: Vec<FabricColumnMapping>,
}

#[non_exhaustive]
pub struct CosmosContainerReference {
    pub database: ResourceIdentity,
    pub container: ResourceIdentity,
}

#[non_exhaustive]
pub struct FabricTableReference {
    pub database: String,
    pub schema: String,
    pub table: String,
}

#[non_exhaustive]
pub struct FabricColumnMapping {
    pub cosmos_path: QueryPath,
    pub fabric_column: String,
}
```

The Fabric endpoint and credential belong to the source. Each mapping identifies
one Cosmos database and container and the corresponding Fabric database, schema,
and table. This allows one authenticated Fabric source to serve several mirrored
containers.

Column mappings are optional for top-level scalar properties whose Fabric
column name is identical to the Cosmos property name. Explicit column mappings
are required when a property is renamed, escaped differently, or projected into
a different relational column.

The mapping used for a query is selected from the `HybridContainerClient`
resource identity. Mapping validation occurs when the hybrid client is built:

- a source cannot contain duplicate mappings for the same Cosmos container;
- Fabric database, schema, table, and column identifiers are validated before
  they are used to construct SQL;
- referenced Cosmos paths must map unambiguously to Fabric columns;
- the partition-key and item-ID columns can be identified when result
  provenance requires them; and
- a missing mapping makes the Fabric source unsupported for that container.

The prototype does not infer Fabric artifact names or call Fabric management
APIs. Discovery can be added later as a separate configuration provider that
produces the same mapping data.

### 10.3 Translation subset

The first translator supports flat scalar fields and a restricted subset:

- `SELECT *`, `SELECT VALUE`, and scalar projection lists;
- `WHERE` with supported comparisons, Boolean operators, literals, and bound
  parameters;
- `TOP`;
- `GROUP BY`;
- `COUNT`, `SUM`, `AVG`, `MIN`, and `MAX`;
- `ORDER BY`; and
- `OFFSET` and `LIMIT` when an equivalent deterministic T-SQL form is
  available.

The first translator rejects:

- joins and array iteration;
- subqueries;
- UDFs;
- Cosmos-specific functions without an explicit translation;
- nested objects and arrays;
- vector and full-text functions;
- ambiguous missing-versus-null behavior; and
- queries whose ordering or numeric semantics cannot be preserved.

An unsupported translation is not an execution error. It is a planning result.
An optional analytical source falls back to Cosmos with a warning. A required
analytical source returns an error before I/O.

### 10.4 TDS dependency and extension boundary

The Fabric source uses a public `FabricSqlExecutor` transport extension point.
It allows customers to replace `mssql-rs` without reimplementing Cosmos SQL
translation, Fabric mapping, or row conversion. The trait does not expose
`mssql-rs` types:

```rust
#[async_trait]
pub trait FabricSqlExecutor: Send + Sync {
    async fn query(
        &self,
        request: FabricSqlRequest,
    ) -> Result<FabricSqlRowStream>;
}

impl FabricSqlQuerySource {
    pub fn with_executor(
        options: FabricSqlSourceOptions,
        executor: Arc<dyn FabricSqlExecutor>,
    ) -> Result<Self>;
}
```

The built-in implementation uses the `mssql-tds` crate from Microsoft's
`mssql-rs` project. It is enabled by the default `mssql-rs` crate feature:

```toml
[features]
default = ["azure-openai", "mssql-rs"]
mssql-rs = ["dep:mssql-tds"]
```

Disabling `mssql-rs` removes the built-in Fabric TDS adapter while preserving
both `FabricSqlExecutor` and the higher-level `QuerySource` extension point. A
customer can replace only the TDS transport or register a completely different
analytical source.

The `mssql-rs` adapter must support:

- TLS certificate validation;
- Microsoft Entra access-token authentication;
- Fabric endpoint redirects if required;
- asynchronous row streaming;
- cancellation and timeouts;
- supported Rust and repository MSRV; and
- a dependency and security posture acceptable for an Azure SDK prototype.

At the time of this design revision, `mssql-tds` is sourced from the
`microsoft/mssql-rs` repository rather than crates.io, and that repository uses
Rust 1.95 while this workspace declares Rust 1.88. The implementation cannot add
the dependency until it either compiles on the workspace MSRV, provides a
compatible revision, or the repository makes an explicit MSRV decision.
`mssql-rs` remains the selected adapter; this is a compatibility gate, not a
request to select a different TDS client.

## 11. Azure AI providers

The public traits remain service-neutral. The prototype includes real Azure
OpenAI embedding and reranking adapters behind the default-on `azure-openai`
feature:

```toml
[features]
default = ["azure-openai", "mssql-rs"]
azure-openai = ["dep:azure-core"]
```

Disabling `azure-openai` removes the built-in adapters without removing the
`ParameterTransform` or `ResultReranker` extension points.

### 11.1 Embeddings

`AzureOpenAiEmbeddingProvider` calls the Azure OpenAI v1 embeddings endpoint
using `azure_core` HTTP and credential abstractions. It accepts a model
deployment name, validates the returned vector, and never logs input text or
embedding values by default.

### 11.2 Reranking

The generic `ResultReranker` contract does not assume a particular model API.
The first Azure adapter may use a configured model prompt to return item
identities and scores. Its response is validated against the submitted
candidate set:

- unknown or duplicate identities are rejected;
- missing candidates are either appended in original order or dropped according
  to options;
- non-finite scores are rejected; and
- model output cannot modify source documents.

This is prototype behavior, not a claim of equivalence to Azure AI Search
semantic ranker.

## 12. Results, paging, and continuation

The SDK exposes one query method:

```rust
pub async fn query_items<T>(
    &self,
    query: HybridQuery,
    scope: FeedScope,
) -> Result<HybridQueryItemIterator<T>>;
```

`HybridQueryItemIterator<T>` yields:

```rust
#[non_exhaustive]
pub struct HybridQueryItem<T> {
    pub item: T,
    pub score: Option<f64>,
    pub provenance: ResultProvenance,
    pub extensions: ExtensionData,
}
```

`ExtensionData` is a namespaced collection of JSON-compatible values attached
by query sources, transforms, rerankers, or future merge stages. It provides
typed deserialization helpers without exposing engine-specific Rust types in the
core API. Reserved SDK namespaces prevent two engines from silently
overwriting each other's values.

Pages contain `HybridQueryItem<T>` values plus page-level diagnostics, warnings,
and extension data. Cosmos passthrough therefore remains slightly more explicit
than the native SDK: callers access the document through `item.item`, while all
execution modes share one stable result shape.

The prototype reuses native Cosmos paging only for a Cosmos-only plan without
transforms that change resumability. Analytical and reranked plans have no
continuation support.

Passing a continuation token to a non-resumable plan fails during planning.
The SDK does not silently restart or buffer an unbounded result set.

## 13. Fallback, warnings, and errors

### 13.1 Fallback rules

Fallback is allowed only when all of the following are true:

1. the failed or unsupported stage is optional;
2. a fallback plan preserves the requested consistency constraint;
3. the fallback does not omit a required transform or reranker; and
4. execution has not emitted user-visible results.

Examples:

- unsupported optional Fabric translation: fall back to Cosmos;
- Fabric connection failure before results: fall back to Cosmos;
- required Fabric source unavailable: fail;
- optional reranker failure after candidates are buffered: return original
  source order with a warning;
- required embedding transform failure: fail; and
- sidecar failure after partial streaming: fail the stream, because transparent
  restart could duplicate results.

### 13.2 Structured warnings

Optional fallback returns success with structured warnings:

```rust
#[non_exhaustive]
pub enum HybridQueryWarning {
    EngineUnsupported {
        engine: EngineName,
        reason: String,
    },
    EngineFailed {
        engine: EngineName,
        stage: QueryStageKind,
        message: String,
    },
    FellBackToCosmos {
        requested: EngineName,
    },
    ContinuationUnavailable,
}
```

Warnings are attached to the plan and execution diagnostics. They are not
silently discarded.

### 13.3 Error model

The hybrid error type preserves the original source error:

```rust
#[non_exhaustive]
pub enum HybridQueryErrorKind {
    InvalidQuery,
    InvalidOptions,
    UnsupportedPlan,
    EngineNotFound,
    EngineRejected,
    EngineExecution,
    Cosmos,
    Deserialization,
}
```

Engine errors include engine name and stage. Cosmos errors remain inspectable
as Cosmos errors so callers can access status, substatus, activity ID, request
charge, and diagnostics.

## 14. Diagnostics

Every execution produces `HybridQueryDiagnostics` containing:

- plan ID and stage timings;
- selected and considered engines;
- source and fallback decisions;
- query translation summary;
- candidate and result counts;
- Cosmos diagnostics and request charge when Cosmos executes;
- Fabric endpoint activity metadata when available;
- AI request IDs and token usage when available;
- structured warnings; and
- cancellation or timeout information.

Diagnostics must not contain credentials, bearer tokens, full prompts, raw
documents, embedding vectors, or sensitive query parameter values.

## 15. Security and privacy

- Each engine receives only the data needed for its stage.
- Parameter transforms receive only selected parameters plus read-only query and
  container metadata needed for capability negotiation.
- Rerankers receive only the configured document fields where possible.
- Sending item content to an AI service requires explicit reranking options.
- Credentials are supplied through `TokenCredential` or engine-private
  credential abstractions, never query options.
- Engine names and endpoint hosts may appear in diagnostics; secrets may not.
- The SDK should support cancellation and per-stage timeouts to limit external
  service exposure and cost.

## 16. Driver and Cosmos SDK changes

### 16.1 Driver prototype feature

Add a non-default feature such as `preview_hybrid_query` to
`azure_data_cosmos_driver`. Behind it, expose only the parser and local plan
entry points needed by the hybrid crate.

The feature is experimental and not enabled by `azure_data_cosmos`.

The hybrid crate adapts all driver values immediately. No driver AST or plan
type appears in a public hybrid signature.

### 16.2 Dependency direction

```text
azure_data_cosmos_hybrid
    +-- azure_data_cosmos
    +-- azure_data_cosmos_driver [preview_hybrid_query]
    +-- mssql-tds [default feature: mssql-rs]
    +-- Azure OpenAI adapter [default feature: azure-openai]
```

`azure_data_cosmos` must not depend on the hybrid crate.

The built-in integrations are batteries-included defaults, but neither
third-party transport type appears in the public core API.

The design-only crate skeleton declares both default features now. Their
dependency edges are added when the corresponding implementation phase meets
the compatibility gates below.

## 17. Prototype implementation plan

### Phase 0: Validate external transports

**Purpose:** Validate the selected default integrations before committing their
dependency revisions.

1. Connect to a real Fabric mirrored Cosmos SQL analytics endpoint from Rust
   using an Entra token.
2. Execute a parameterized `SELECT`, stream rows, cancel a query, and capture a
   server request identifier.
3. Validate `mssql-rs` against the workspace MSRV and pin a reviewed compatible
   revision.
4. Call the Azure OpenAI v1 embeddings endpoint using `azure_core` and a
   `TokenCredential`.
5. Prototype the configured-model reranking request and strict response
   validation.

**Exit criteria:**

- a compatible `mssql-rs` revision is identified;
- TLS and Entra authentication work against Fabric;
- Azure OpenAI authentication and embeddings work;
- dependency risks are documented; and
- no public hybrid API depends on transport-specific types.

### Phase 1: Crate and query-analysis foundation

1. Add the `preview_hybrid_query` driver feature.
2. Expose parser and local-plan entry points behind the feature.
3. Implement `HybridQuery`, parameters, and consuming directive builders.
4. Load `ContainerProperties` into stable `ContainerQueryMetadata`.
5. Implement hybrid-owned `QueryAnalysis`.
6. Adapt parameter comparisons, function arguments, and related property paths
   into stable `ParameterUsage` values.
7. Implement validation and stable error conversion.
8. Add parser-to-analysis unit tests covering supported and unsupported query
   shapes.

**Exit criteria:**

- the hybrid crate parses a `HybridQuery` and can construct a native Cosmos
  `Query` internally;
- no driver query type leaks publicly;
- malformed queries return location-aware errors; and
- analysis is deterministic and safe to include in `explain`.

### Phase 2: Cosmos passthrough and explain

1. Implement `HybridQueryClientBuilder` and `HybridContainerClient`.
2. Implement the built-in Cosmos query source.
3. Implement `ConsistencyConstraint`, source selection, engine requirements,
   and immutable plans as parts of `HybridQuery`.
4. Implement `explain`.
5. Implement `query_items<T>` Cosmos passthrough.
6. Preserve Cosmos page diagnostics and continuation tokens for eligible plans.

**Exit criteria:**

- default hybrid queries behave like native Cosmos queries;
- attaching engines does not change default execution;
- `explain` selects Cosmos by default; and
- Cosmos errors and diagnostics remain accessible.

### Phase 3: Parameter embedding

1. Implement the `ParameterTransform` trait and registry.
2. Implement transform plan validation.
3. Add deterministic mock embedding tests.
4. Add metadata-driven transform configuration and validation using
   `ContainerProperties`.
5. Infer vector property paths from `ParameterUsage` and match them to
   `VectorEmbeddingPolicy`.
6. Add the default-on Azure OpenAI embedding adapter.
7. Validate vector dimensions, finite values, and parameter replacement.

**Exit criteria:**

- a string parameter can be added and assigned a named transform in one call;
- named transforms can infer the related property from the parsed query;
- inferred properties resolve container vector policy defaults and validation;
- explicit options resolve absent or ambiguous inference;
- only the selected parameter is sent to the provider;
- provider failures obey required/optional policy; and
- the transformed query executes against Cosmos.

### Phase 4: Semantic reranking

1. Implement raw JSON candidate envelopes.
2. Implement bounded candidate buffering.
3. Implement `ResultReranker` and deterministic mock tests.
4. Implement the default-on Azure OpenAI reranking adapter.
5. Implement `HybridQueryItem<T>`, per-item extension data, and enriched page
   metadata.
6. Implement optional reranker fallback to original source ordering.

**Exit criteria:**

- candidate and result counts are enforced;
- item identity survives reranking;
- invalid model output is rejected;
- all execution modes produce the same `HybridQueryItem<T>` shape; and
- reranked plans reject continuation tokens.

### Phase 5: Fabric analytical source

1. Implement explicit Cosmos-container-to-Fabric-table registration.
2. Implement the restricted Cosmos SQL to T-SQL translator.
3. Implement `FabricSqlExecutor` and the default-on `mssql-rs` adapter.
4. Implement Entra token acquisition, refresh, TLS, timeouts, cancellation, and
   retries appropriate for read-only queries.
5. Implement `Analytical` source preference and Cosmos fallback.
6. Convert Fabric rows to JSON while preserving supported Cosmos-compatible
   value shapes.
7. Test against a real mirrored Cosmos database and SQL analytics endpoint.

**Exit criteria:**

- supported analytical queries execute against Fabric;
- unsupported queries fall back or fail according to requirement;
- standard Cosmos consistency variants never select Fabric;
- `Analytical` prefers Fabric and reports fallback;
- translated SQL is visible in redacted explain diagnostics; and
- no query is sent to Fabric without explicit analytical intent.

### Phase 6: Hardening and evaluation

1. Add per-stage cancellation and timeouts.
2. Add diagnostics redaction tests.
3. Add concurrency and client-sharing tests.
4. Measure parser, planning, embedding, reranking, Fabric, and Cosmos latency.
5. Measure candidate buffering memory.
6. Review API surface for SemVer and Azure SDK guideline compatibility.
7. Decide whether to publish the extension traits or keep the crate as an
   experimental package for another iteration.

**Exit criteria:**

- all prototype scenarios have end-to-end tests;
- failure and fallback behavior is deterministic;
- security review finds no credential or document leakage;
- performance costs are documented; and
- unresolved production requirements are explicitly listed.

## 18. Testing strategy

### Unit tests

- Cosmos SQL analysis and stable adaptation;
- engine capability negotiation;
- consistency and source eligibility;
- stage ordering;
- source and engine requirement behavior;
- Cosmos SQL to T-SQL translation;
- row-to-JSON conversion;
- candidate identity and reranking validation;
- diagnostics redaction; and
- continuation eligibility.

### Integration tests

- native Cosmos passthrough using the Cosmos emulator;
- parameter embedding using a deterministic provider;
- reranking using a deterministic provider;
- Fabric execution using a TDS test server where possible;
- real Fabric SQL analytics endpoint smoke tests behind environment gates; and
- real Azure OpenAI smoke tests behind environment gates.

Live tests must not run by default and must use repository test conventions for
credentials and recordings where applicable.

## 19. Open questions after the prototype

1. Should `Analytical` remain a consistency variant or become a separate source
   freshness policy before a stable release?
2. Can Fabric expose reliable mirror freshness metadata that the SDK can use
   without a management-plane dependency?
3. Which Cosmos SQL functions have sufficiently equivalent T-SQL behavior to
   add safely?
4. Should nested JSON translation use `JSON_VALUE` and `OPENJSON`, or remain an
   explicit engine extension?
5. How should hybrid continuation tokens encode multiple source cursors and
   reranker state?
6. Should multi-source plans use union, keyed merge, rank fusion, or
   caller-provided merge policies?
7. Should engine traits remain in this crate or move to a lower-support
   companion crate to allow faster evolution?
8. Which fields may be sent to AI rerankers, and should field selection be
   mandatory rather than optional?
9. Should an Azure AI Search semantic ranker adapter be distinct from the
   configured-model Azure OpenAI reranker?

## 20. References

- [Illustrative hybrid query samples](samples.md)
- [Azure Cosmos DB Rust SDK](../../azure_data_cosmos/README.md)
- [Cosmos DB driver query implementation](../../azure_data_cosmos_driver/src/query/)
- [Connect to OneLake](https://learn.microsoft.com/fabric/onelake/onelake-access-api)
- [Fabric Warehouse connectivity](https://learn.microsoft.com/fabric/data-warehouse/connectivity)
- [Fabric mirrored Cosmos DB limitations](https://learn.microsoft.com/fabric/mirroring/azure-cosmos-db-limitations)
- [Azure OpenAI embeddings](https://learn.microsoft.com/azure/foundry/openai/how-to/embeddings)
- [Azure OpenAI v1 API](https://learn.microsoft.com/azure/foundry/openai/api-version-lifecycle)
- [Microsoft Rust TDS implementation](https://github.com/microsoft/mssql-rs)
