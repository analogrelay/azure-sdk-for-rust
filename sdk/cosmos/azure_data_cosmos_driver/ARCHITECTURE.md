# Azure Cosmos DB Driver - API Overview

This document provides a comprehensive overview of the `azure_data_cosmos_driver` public API for team discussion and onboarding.

## Three-Layer Architecture

The Azure Cosmos DB Rust ecosystem consists of three distinct layers:

```text
┌─────────────────────────────────────────────────────────────────────────┐
│  Layer 3: Language-Specific SDKs                                        │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │ azure_data_cosmos (Rust)                                        │    │
│  │ - Idiomatic Rust API with serde serialization                   │    │
│  │ - Microsoft 24x7 support                                        │    │
│  └────────────────────────────┬────────────────────────────────────┘    │
│                               │ (direct Rust dependency)                │
│                               │                                         │
│  ┌─────────────────┐  ┌───────┼─────────┐  ┌─────────────────────────┐  │
│  │ Java SDK        │  │       │         │  │ .NET / Python SDKs      │  │
│  │ (via JNI)       │  │       │         │  │ (via native interop)    │  │
│  │ - Jackson types │  │       │         │  │ - native serialization  │  │
│  └────────┬────────┘  │       │         │  └────────────┬────────────┘  │
│           │           │       │         │               │               │
│           ▼           │       │         │               ▼               │
│  ┌────────────────────┴───────┼─────────┴───────────────────────────┐   │
│  │  Layer 2: azure_data_cosmos_native (C-FFI)                       │   │
│  │  - Stable C ABI for cross-language interop (non-Rust languages)  │   │
│  │  - Memory-safe wrappers around driver                            │   │
│  └─────────────────────────────┬────────────────────────────────────┘   │
│                                │                                        │
│                                ▼                                        │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  Layer 1: azure_data_cosmos_driver (This Crate)  ◄───────────────┼───┘
│  │  - Transport, routing, protocol handling                         │
│  │  - Schema-agnostic (raw bytes in/out)                            │
│  │  - Community support only                                        │
│  └──────────────────────────────────────────────────────────────────┘
└─────────────────────────────────────────────────────────────────────────┘
```

### Layer Responsibilities

| Layer | Crate | Responsibility | Support Level |
|-------|-------|----------------|---------------|
| 1 | `azure_data_cosmos_driver` | Transport, routing, protocol, retries | Community/GitHub |
| 2 | `azure_data_cosmos_native` | C-FFI wrapper for non-Rust languages | Internal |
| 3 | `azure_data_cosmos` | Idiomatic Rust API with serde (uses driver directly) | Microsoft 24x7 |

> **Note**: The Rust SDK (`azure_data_cosmos`) depends directly on `azure_data_cosmos_driver` - it does **not** go through the native layer. The native layer exists solely for cross-language interop (Java, .NET, Python, etc.) via C-FFI.

---

## High-Level Type Overview

```text
  CosmosDriverRuntime                    (Entry point - singleton per process)
          │
          │ get_or_create_driver()
          ▼
    CosmosDriver                         (Per-account driver instance)
          │
          │ execute_operation()
          │
          ▼
    CosmosOperation                      (Built via factory methods)
          │
          │ CosmosOperation::create(resource_ref)
          │ CosmosOperation::read(resource_ref)
          │ CosmosOperation::query(resource_ref)
          │ etc.
          │
          ▼
    CosmosResourceReference              (Typed resource targeting)
          │
          │ Built from typed references:
          │ - ContainerReference::from_name(...)
          │ - ItemReference::from_name(...)
          │ - DatabaseReference::from_name(...)
          │
          ▼
    CosmosResult                         (Response with diagnostics)
          │
          ├── response_bytes: Vec<u8>
          ├── headers: ResponseHeaders
          └── diagnostics: CosmosDiagnostics
```

### Core Flow

1. **Runtime** manages connection pools, background tasks, caching
2. **Driver** provides access to a specific Cosmos account
3. **Resource Reference** built from typed references (`ContainerReference`, `ItemReference`, etc.)
4. **Operation** created via factory methods: `CosmosOperation::create(resource_ref)`, `.read()`, `.query()`, etc.
5. **Execution** happens via `driver.execute_operation(operation)` - returns `CosmosResult`

---

## Code Examples

### Example 1: Simple Master Key Authentication

```rust,no_run
use azure_data_cosmos_driver::{
    CosmosDriverRuntime,
    models::{
        AccountReference, ContainerReference, CosmosOperation, PartitionKey,
    },
};
use url::Url;

#[tokio::main]
async fn main() -> azure_core::Result<()> {
    // Create runtime (typically once per application)
    let runtime = CosmosDriverRuntime::builder().build().await?;

    // Configure account with master key
    let account = AccountReference::with_master_key(
        Url::parse("https://myaccount.documents.azure.com:443/").unwrap(),
        "your-master-key-here",
    );

    // Get or create driver for this account
    let driver = runtime.get_or_create_driver(account.clone(), None).await?;

    // Create a JSON document payload
    let document_json = r#"{
        "id": "doc_001",
        "pk": "HelloWorld",
        "message": "Hello from Cosmos DB!",
        "count": 42
    }"#;

    // Build typed container reference (no raw resource link strings!)
    let container = ContainerReference::from_name(
        account,
        "myDatabase",
        "myContainer",
    );

    // Create a document in the container (target is the container itself)
    let operation = CosmosOperation::create(container)
        .with_partition_key(PartitionKey::from("HelloWorld"))
        .with_body(document_json.as_bytes().to_vec());

    // Execute the operation
    let result = driver.execute_operation(operation, None).await?;

    // Access the response
    println!("Request charge: {} RUs", result.headers().request_charge());
    println!("Activity ID: {}", result.headers().activity_id());
    println!("Response: {}", String::from_utf8_lossy(result.response_bytes()));

    // Access diagnostics for debugging
    let diagnostics = result.diagnostics();
    println!("Total latency: {:?}", diagnostics.elapsed());
    println!("Regions contacted: {:?}", diagnostics.regions_contacted());

    Ok(())
}
```

### Example 2: AAD Authentication with Configuration Mutation

```rust,no_run
use azure_data_cosmos_driver::{
    CosmosDriverRuntime,
    models::{
        AccountReference, ContainerReference, CosmosOperation,
        CosmosResourceReference, ItemReference, PartitionKey,
    },
    options::{DriverOptions, RetryOptions, ConnectionPoolOptions},
};
use azure_identity::DefaultAzureCredential;
use url::Url;
use std::time::Duration;
use std::sync::Arc;

#[tokio::main]
async fn main() -> azure_core::Result<()> {
    // Build runtime with custom options
    let runtime = CosmosDriverRuntime::builder()
        .driver_options(
            DriverOptions::builder()
                .retry_options(
                    RetryOptions::builder()
                        .max_retries(5)
                        .initial_delay(Duration::from_millis(100))
                        .max_delay(Duration::from_secs(30))
                        .build()
                )
                .connection_pool_options(
                    ConnectionPoolOptions::builder()
                        .max_idle_connections_per_host(20)
                        .idle_timeout(Duration::from_secs(90))
                        .build()
                )
                .build()
        )
        .build()
        .await?;

    // Use AAD credential (recommended for production)
    let credential = Arc::new(DefaultAzureCredential::new()?);

    let account = AccountReference::with_credential(
        Url::parse("https://myaccount.documents.azure.com:443/").unwrap(),
        credential,
    );

    // Driver-level option override
    let driver_opts = DriverOptions::builder()
        .retry_options(
            RetryOptions::builder()
                .max_retries(10)  // More aggressive retry for this account
                .build()
        )
        .build();

    let driver = runtime.get_or_create_driver(account.clone(), Some(driver_opts)).await?;

    // Read an existing document using typed references
    let item_ref = ItemReference::from_name(
        account,
        "myDatabase",
        "myContainer",
        "doc_001",
    );

    let read_operation = CosmosOperation::read(item_ref)
        .with_partition_key(PartitionKey::from("HelloWorld"));

    let result = driver.execute_operation(read_operation, None).await?;
    println!("Document: {}", String::from_utf8_lossy(result.response_bytes()));

    Ok(())
}
```

---

## Configuration Hierarchy

Configuration cascades from most general to most specific:

```text
Environment Variables (COSMOS_*)
        │
        ▼
Runtime-Level Options (DriverOptions on runtime)
        │
        ▼
Driver-Level Options (per-account overrides)
        │
        ▼
Operation-Level Options (per-request overrides)
```

Each level can selectively override settings from the level above.

---

## Public API Reference

### Root Module (`azure_data_cosmos_driver`)

#### Core Types

| Type | Description |
|------|-------------|
| `CosmosDriverRuntime` | Entry point; manages drivers, pools, background tasks |
| `CosmosDriverRuntimeBuilder` | Builder for `CosmosDriverRuntime` |
| `CosmosDriver` | Per-account driver for executing operations |
| `CosmosOperation` | Single operation with context and options |
| `CosmosResult` | Response containing bytes, headers, diagnostics |

---

### Module: `options`

Configuration types with builder pattern throughout.

#### Option Types

| Type | Description |
|------|-------------|
| `DriverOptions` | Top-level driver configuration |
| `DriverOptionsBuilder` | Builder for `DriverOptions` |
| `RetryOptions` | Retry policy configuration |
| `RetryOptionsBuilder` | Builder for `RetryOptions` |
| `ConnectionPoolOptions` | HTTP connection pool settings |
| `ConnectionPoolOptionsBuilder` | Builder for `ConnectionPoolOptions` |
| `TimeoutOptions` | Request/operation timeout settings |
| `TimeoutOptionsBuilder` | Builder for `TimeoutOptions` |
| `TelemetryOptions` | Distributed tracing and logging |
| `TelemetryOptionsBuilder` | Builder for `TelemetryOptions` |

#### Key Configuration Fields

```rust
// RetryOptions
struct RetryOptions {
    max_retries: u32,           // Default: 3
    initial_delay: Duration,    // Default: 100ms
    max_delay: Duration,        // Default: 30s
    retry_on_throttle: bool,    // Default: true
}

// ConnectionPoolOptions
struct ConnectionPoolOptions {
    max_idle_connections_per_host: usize,  // Default: 10
    idle_timeout: Duration,                 // Default: 90s
}

// TimeoutOptions
struct TimeoutOptions {
    request_timeout: Duration,     // Per-request timeout
    operation_timeout: Duration,   // Total operation timeout (incl. retries)
}
```

---

### Module: `models`

Resource definitions and metadata types.

#### Account & Connection

| Type | Description |
|------|-------------|
| `AccountReference` | Account endpoint + credentials |
| `AccountProperties` | Account metadata (regions, capabilities) |
| `ConsistencyLevel` | Strong, BoundedStaleness, Session, Eventual, ConsistentPrefix |

#### Database & Container

| Type | Description |
|------|-------------|
| `DatabaseProperties` | Database metadata |
| `ContainerProperties` | Container configuration |
| `ContainerPropertiesBuilder` | Builder for `ContainerProperties` |
| `PartitionKeyDefinition` | Partition key path(s) and kind |
| `PartitionKeyDefinitionBuilder` | Builder for `PartitionKeyDefinition` |
| `PartitionKeyKind` | Hash, Range, MultiHash |

#### Indexing

| Type | Description |
|------|-------------|
| `IndexingPolicy` | Container indexing configuration |
| `IndexingPolicyBuilder` | Builder for `IndexingPolicy` |
| `IndexingMode` | Consistent, Lazy, None |
| `IncludedPath` | Paths to include in index |
| `ExcludedPath` | Paths to exclude from index |
| `SpatialIndex` | Geospatial index configuration |
| `CompositeIndex` | Multi-property composite index |
| `CompositeIndexOrder` | Ascending, Descending |

#### Throughput & Scaling

| Type | Description |
|------|-------------|
| `ThroughputProperties` | Provisioned or autoscale throughput |
| `ThroughputPropertiesBuilder` | Builder for `ThroughputProperties` |
| `AutoscaleSettings` | Autoscale max throughput |

#### Conflicts & TTL

| Type | Description |
|------|-------------|
| `ConflictResolutionPolicy` | LastWriterWins, Custom, Manual |
| `ConflictResolutionPolicyBuilder` | Builder for conflict policy |
| `DefaultTimeToLive` | Off, NoDefault, Seconds(i32) |

---

### Module: `diagnostics`

Operational telemetry for debugging and monitoring.

#### Core Diagnostics

| Type | Description |
|------|-------------|
| `CosmosDiagnostics` | Top-level diagnostics container |
| `OperationDiagnostics` | Per-operation summary |
| `RequestDiagnostics` | Per-HTTP-request details |

#### Metrics & Timing

| Type | Description |
|------|-------------|
| `RequestCharge` | RU consumption (total, per-request breakdown) |
| `RetryInfo` | Retry count, reasons, delays |
| `TimingInfo` | Request/response timing breakdown |
| `RegionInfo` | Which region(s) handled the request |

#### Request Tracking

| Type | Description |
|------|-------------|
| `RequestSentStatus` | Sent, NotSent, Unknown - tracks if request left the client |
| `RequestEvent` | Lifecycle events (headers received, body buffered, etc.) |

#### Key Diagnostic Fields

```rust
struct CosmosDiagnostics {
    operation_id: String,
    total_request_charge: f64,
    total_duration: Duration,
    retry_count: u32,
    requests: Vec<RequestDiagnostics>,
}

struct RequestDiagnostics {
    request_id: String,
    status_code: Option<u16>,
    sub_status_code: Option<u32>,
    request_charge: f64,
    region: String,
    request_sent: RequestSentStatus,
    duration: Duration,
    events: Vec<RequestEvent>,
}
```

---

### Module: `builders`

Fluent builders for complex type construction.

| Type | Description |
|------|-------------|
| `PointReadBuilder` | Build point read operations |
| `QueryBuilder` | Build query operations |
| `UpsertBuilder` | Build upsert operations |
| `DeleteBuilder` | Build delete operations |
| `PatchBuilder` | Build patch operations |
| `BulkBuilder` | Build bulk operation batches |

---

### Enums Summary

| Enum | Variants | Description |
|------|----------|-------------|
| `ConsistencyLevel` | Strong, BoundedStaleness, Session, Eventual, ConsistentPrefix | Read consistency |
| `PartitionKeyKind` | Hash, Range, MultiHash | Partition strategy |
| `IndexingMode` | Consistent, Lazy, None | When to index |
| `CompositeIndexOrder` | Ascending, Descending | Sort order |
| `DefaultTimeToLive` | Off, NoDefault, Seconds(i32) | Document expiration |
| `RequestSentStatus` | Sent, NotSent, Unknown | Request lifecycle state |

---

## Error Handling

All fallible operations return `azure_core::Result<T>` (alias for `Result<T, azure_core::Error>`).

### Error Categories

| Category | When | Retryable? |
|----------|------|------------|
| `HttpError` | Network/transport failures | Usually yes |
| `ServiceError` | Cosmos DB returned error | Depends on status |
| `CredentialError` | Auth token acquisition failed | Usually no |
| `ConfigurationError` | Invalid options/setup | No |

### Status Code Handling

```rust
match result {
    Ok(response) => { /* success */ }
    Err(e) if e.is_throttling() => { /* 429 - back off */ }
    Err(e) if e.is_not_found() => { /* 404 - item missing */ }
    Err(e) if e.is_conflict() => { /* 409 - ETag mismatch */ }
    Err(e) => { /* other error */ }
}
```

---

## Thread Safety

All core types are `Send + Sync`:

- `CosmosDriverRuntime` - safe to share across threads
- `CosmosDriver` - safe to share across threads
- Operations should be created per-request (not shared)

Recommended pattern:

```rust
// Create once at startup
let runtime = Arc::new(CosmosDriverRuntime::builder().build().await?);

// Share across request handlers
let runtime_clone = runtime.clone();
tokio::spawn(async move {
    let driver = runtime_clone.get_or_create_driver(account, None).await?;
    // ... use driver
});
```

---

## Performance Considerations

1. **Runtime is expensive to create** - create once, reuse globally
2. **Driver is cached per-account** - `get_or_create_driver` returns existing instance
3. **Connection pooling is automatic** - configured via `ConnectionPoolOptions`
4. **Retries have backoff** - exponential with jitter, configurable limits
5. **Diagnostics are always collected** - no runtime cost to enable

---

## See Also

- [README.md](README.md) - Quick start and basic usage
- [CHANGELOG.md](CHANGELOG.md) - Version history
- [azure_data_cosmos](https://docs.rs/azure_data_cosmos) - High-level Rust SDK
- [Azure Cosmos DB Documentation](https://docs.microsoft.com/azure/cosmos-db/)
