# Cosmos Driver model investigation (`src/models`)

## Domain grouping (purpose + model kind)

### 1) Metadata/control-plane wire body models
- **Primary wire models** (`serde` request/response bodies):  
  `DatabaseProperties`, `ContainerProperties`, `PartitionKeyDefinition`, `PartitionKeyKind`, `IndexingPolicy`, `IndexingMode`, `SystemProperties` (from `mod.rs`).
- **Container policy/value wire submodels** (`container_properties.rs`):  
  `TimeToLive`, `ChangeFeedPolicy`, `UniqueKeyPolicy`, `UniqueKey`, `ConflictResolutionPolicy`, `ConflictResolutionMode`, `ComputedProperty`, `VectorEmbeddingPolicy`, `VectorEmbedding`, `VectorDataType`, `VectorDistanceFunction`, `FullTextPolicy`, `FullTextPath`.
- **Internal cached metadata helper**:  
  `ImmutableContainerProperties` (crate-private, extracted from `ContainerProperties`), `AccountProperties` (crate-private account metadata cache shape).

### 2) Resource identity + routing models (mostly helpers/value objects)
- **User/internal resource IDs**: `ResourceName`, `ResourceRid` + internal ID enums (`DatabaseId`, `ContainerId`, `ItemIdentifier`, `ItemId`, `StoredProcedureId`, `TriggerId`, `UdfId`, `PartitionKeyRangeId`, `ParsedResourceId`, `RidParseError`).
- **Typed references**: `DatabaseReference`, `ContainerReference`, `ItemReference`, `StoredProcedureReference`, `TriggerReference`, `UdfReference`, internal `PartitionKeyRangeReference`.
- **Generic routing reference**: `CosmosResourceReference` (normalizes all resource targeting to one shape).

### 3) Operation/response envelopes (driver runtime models)
- `CosmosOperation`: pre-wire operation envelope (operation type, target resource, optional PK/body/headers).
- `CosmosResult`: post-wire result envelope (raw body + parsed Cosmos headers + diagnostics).
- `CosmosHeaders`: typed extraction of Cosmos response headers.
- `CosmosStatus` + `SubStatusCode`: HTTP + Cosmos substatus pairing with disambiguation helpers/constants.

### 4) Header/auth/value helpers (wire-adjacent value objects)
- `PartitionKey`, `PartitionKeyValue` (header serialization for `x-ms-documentdb-partitionkey` and cross-partition behavior).
- `ETag`, `ETagCondition` (optimistic concurrency header semantics).
- `ActivityId`, `RequestCharge`, `SessionToken`, `TriggerInvocation`, `ThroughputControlGroupName` (header-facing value objects).
- `UserAgent` (computed user-agent header value object).
- `AccountReference`, `AuthOptions`, `AccountReferenceBuilder`, internal `AccountEndpoint` (account/auth endpoint identity).
- `ConnectionString` (parser/value object feeding account endpoint/key setup).
- `ResourceType`, `OperationType` (routing/dispatch enums used across references + operations).

## Key relationships/dependencies

1. `ResourceName`/`ResourceRid` are the base primitives for internal ID enums in `resource_id.rs`; those ID enums back all typed reference structs in `resource_reference.rs`.
2. `ContainerReference` embeds `Arc<ImmutableContainerProperties>` derived from wire `ContainerProperties`, giving runtime access to immutable schema bits (`partition_key`, `unique_key_policy`) without re-fetch.
3. `DatabaseReference`/`ContainerReference`/`ItemReference` convert into `CosmosResourceReference`; `CosmosOperation` then composes `OperationType + CosmosResourceReference + optional PartitionKey/body`.
4. `ItemReference` requires `PartitionKey`; item operations in `CosmosOperation` copy that key automatically.
5. `CosmosResult` holds raw body and `CosmosHeaders`; status interpretation is in diagnostics (which use `CosmosStatus`/`SubStatusCode` semantics).
6. `SubStatusCode` is intentionally context-sensitive; `CosmosStatus` resolves ambiguous values (for example `1002`) using HTTP status.

## Per-file/type breakdown

### `account_reference.rs`
- `AccountEndpoint` *(internal helper/value object)*: URL newtype used as cache/routing key; path-join helper.
- `AuthOptions` *(helper/value object)*: auth mode union (`MasterKey` or `TokenCredential`).
- `AccountReference` *(helper/reference object)*: account endpoint + auth bundle; equality/hash by endpoint.
- `AccountReferenceBuilder` *(helper/builder)*: enforces auth configuration before construction.

### `activity_id.rs`
- `ActivityId` *(wire header/value object)*: `x-ms-activity-id` wrapper (`serde` transparent), UUID generation/parsing/display.

### `connection_string.rs`
- `ConnectionString` *(helper/value object)*: parses `AccountEndpoint=...;AccountKey=...`; exposes endpoint/key.

### `container_properties.rs`
- `TimeToLive` *(wire model value enum)*: TTL semantics mapped to wire integer/null forms.
- `ChangeFeedPolicy` *(wire model value enum)*: latest-only vs all-versions/deletes with retention.
- `UniqueKeyPolicy`, `UniqueKey` *(wire body models)*: uniqueness constraints.
- `ConflictResolutionPolicy`, `ConflictResolutionMode` *(wire body model + enum)*: multi-region conflict handling.
- `ComputedProperty` *(wire body model)*: SQL-derived computed fields.
- `VectorEmbeddingPolicy`, `VectorEmbedding`, `VectorDataType`, `VectorDistanceFunction` *(wire body models/enums)*: vector search config.
- `FullTextPolicy`, `FullTextPath` *(wire body models)*: full-text search config.

### `cosmos_operation.rs`
- `CosmosOperation` *(runtime helper envelope)*: internal representation of an operation pre-HTTP request; stores `OperationType`, `ResourceType`, `CosmosResourceReference`, optional `PartitionKey`, headers, body.

### `cosmos_resource_reference.rs`
- `CosmosResourceReference` *(runtime routing helper)*: generic canonical target descriptor spanning account/database/container/document/sproc/trigger/udf/pkrange/offer; computes request/signing paths and supports feed vs item semantics.

### `cosmos_result.rs`
- `CosmosResult` *(runtime response envelope)*: raw bytes + typed Cosmos headers + diagnostics context.
- `CosmosHeaders` *(wire-header projection object)*: activity id, RU charge, session token, etag, continuation, item count.

### `cosmos_status.rs`
- `SubStatusCode` *(wire-status value object)*: Cosmos substatus code wrapper with large named constant catalog and context-aware name mapping.
- `CosmosStatus` *(runtime/wire-status helper)*: paired HTTP status + optional substatus; convenience predicates for common Cosmos retry/error conditions.

### `etag.rs`
- `ETag` *(wire header/value object)*: ETag wrapper for concurrency tokens.
- `ETagCondition` *(wire request condition object)*: if-match / if-none-match semantics.

### `mod.rs`
- `AccountProperties` *(internal helper model)*: cached account routing metadata (write/read regions, optional rid).
- `DatabaseProperties` *(wire body model)*: database resource shape.
- `ContainerProperties` *(wire body model)*: container resource shape composed with policies from `container_properties.rs`.
- `ImmutableContainerProperties` *(internal helper model)*: non-mutable subset of container metadata cached on references.
- `PartitionKeyDefinition`, `PartitionKeyKind` *(wire body model + enum)*: container partitioning schema.
- `IndexingPolicy`, `IndexingMode` *(wire body model + enum)*: indexing configuration.
- `SystemProperties` *(wire body submodel)*: `_rid`, `_ts`, `_etag`.
- `ResourceType`, `OperationType` *(runtime routing enums)*: resource category + operation category with helpers.
- `SessionToken` *(wire header/value object)*: session consistency token wrapper.
- `TriggerInvocation` *(wire header/value object)*: trigger name to invoke during operations.
- `ThroughputControlGroupName` *(wire header/value object)*: throughput-control group header value.

### `partition_key.rs`
- `PartitionKeyValue` *(wire header/value object)*: one PK component (null/string/number/bool).
- `PartitionKey` *(wire header/value object)*: single or hierarchical PK; serializes to Cosmos partition-key headers and cross-partition toggle.
- *(private helpers)* `FiniteF64`, `InnerPartitionKeyValue` for hash-safe numeric encoding.

### `request_charge.rs`
- `RequestCharge` *(wire header/value object)*: RU charge newtype (`f64` normalization + arithmetic/ordering traits).

### `resource_id.rs`
- `ResourceName`, `ResourceRid` *(helper/value objects)*: canonical name vs RID wrappers.
- `DatabaseId`, `ContainerId`, `ItemIdentifier`, `ItemId`, `StoredProcedureId`, `TriggerId`, `UdfId`, `PartitionKeyRangeId` *(internal helper enums)*: enforce all-name or all-RID consistency across hierarchy levels.
- `ParsedResourceId` *(internal helper model)*: parsed hierarchical RID parts (db/container/document).
- `RidParseError` *(internal helper error type)*: invalid RID parse conditions.

### `resource_reference.rs`
- `DatabaseReference` *(typed routing helper)*: account + database identifier (name or RID).
- `ContainerReference` *(typed routing helper)*: resolved container carrying both name+RID (db/container) plus immutable container properties.
- `ItemReference` *(typed routing helper)*: container + required partition key + item identifier + precomputed resource link.
- `StoredProcedureReference`, `TriggerReference`, `UdfReference` *(typed routing helpers)*: account + hierarchical identifier for programmable resources.
- `PartitionKeyRangeReference` *(internal typed routing helper)*: internal partition key range reference.
