# Cosmos Driver Runtime Lifecycle Analysis

## Scope

This report examines lifecycle behavior in:

- `sdk/cosmos/azure_data_cosmos_driver/src/driver/runtime.rs`
- `sdk/cosmos/azure_data_cosmos_driver/src/driver/cosmos_driver.rs`

It also references directly invoked components (`transport`, VM metadata, CPU/memory monitor) where needed to explain runtime startup/background/shutdown behavior.

## 1) Driver creation lifecycle

### Runtime creation entrypoint

`CosmosDriverRuntime::builder().build().await` is the root lifecycle entrypoint.

Build path (`CosmosDriverRuntimeBuilder::build`):

1. Compute `UserAgent` from (priority):
   - `user_agent_suffix`
   - `workload_id`
   - `correlation_id`
   - default base user agent
2. Resolve `ConnectionPoolOptions` defaults.
3. Create shared `CosmosTransport` (`Arc<CosmosTransport>`).
4. Wrap diagnostics and runtime defaults:
   - `diagnostics_options: Arc<DiagnosticsOptions>`
   - `runtime_options: SharedRuntimeOptions`
5. Initialize process-wide system services:
   - `CpuMemoryMonitor::get_or_init()`
   - `VmMetadataService::get_or_init().await`
6. Initialize registries/caches:
   - `driver_registry: Arc<RwLock<HashMap<String, Arc<CosmosDriver>>>>`
   - `AccountMetadataCache`
   - `ContainerCache`

### Driver creation entrypoint

`CosmosDriverRuntime::get_or_create_driver(account, driver_options).await` performs singleton creation per endpoint.

Flow:

1. Compute key from `account.endpoint().to_string()`.
2. Read-lock `driver_registry` and return existing driver if found.
3. Upgrade to write-lock; double-check key to avoid races.
4. Build driver options if absent (`DriverOptions::builder(account).build()`).
5. Create `Arc<CosmosDriver>` with `CosmosDriver::new(self.clone(), options)`.
6. Insert into registry and return cloned `Arc`.

Important behavior: if a driver already exists for that endpoint, provided `driver_options` are ignored.

## 2) Runtime ownership / borrowing model

## Ownership graph

- `CosmosDriverRuntime` is an owned, cloneable value type.
- Shared mutable/global-ish internals are behind `Arc` + lock wrappers:
  - `transport: Arc<CosmosTransport>`
  - `driver_registry: Arc<RwLock<HashMap<...>>>`
  - caches and diagnostics options in `Arc`
  - `SharedRuntimeOptions(Arc<RwLock<RuntimeOptions>>)`
- `CosmosDriver` owns:
  - a cloned `CosmosDriverRuntime` value
  - `DriverOptions`
- public APIs mostly borrow immutably (`&self`), with interior mutability for shared state.

### Practical borrowing behavior

- `CosmosDriver::runtime()` returns `&CosmosDriverRuntime` (borrow only).
- Runtime defaults are read via `snapshot()` and merged per operation.
- Runtime-level mutability (e.g., changing defaults) happens through `SharedRuntimeOptions` setter methods, guarded by `RwLock`.

## 3) Startup and initialization paths

## Runtime startup

Startup is mostly eager at `build()` time for core runtime objects, with selective lazy internals:

- Eager:
  - base metadata/dataplane reqwest transports
  - cache/registry structures
  - CPU monitor handle + VM metadata service handle
- Lazy:
  - emulator-specific insecure transports (`OnceLock`) inside `CosmosTransport`
  - actual VM metadata fetch only once; fallback machine ID if unavailable

## First operation path (`execute_operation`)

Each operation call does per-request initialization:

1. Merge effective options (operation > driver > runtime).
2. Resolve effective throughput control group for operation container (if any).
3. Build diagnostics context (`ActivityId`, request tracking).
4. Build request URL/path/method/headers/body.
5. Create authorization context.
6. Select transport pipeline (metadata vs dataplane) based on resource/operation type.
7. Create per-request event channel and attach tracked transport emitter.
8. Send request and translate result into `CosmosResult` + diagnostics.

This path is stateless per request except for reads from runtime-managed shared structures.

## 4) Background management

## Process-level background components

1. **CPU/memory monitor**
   - `CpuMemoryMonitor::get_or_init()` starts a background thread (`cosmos-cpu-monitor`) once.
   - Thread samples periodically and updates history.
   - Sampling is listener-aware (skips refresh when no listeners are registered).

2. **VM metadata service**
   - `VmMetadataService::get_or_init().await` performs one-time IMDS fetch attempt.
   - Metadata is cached; machine ID is always available (IMDS VM ID or generated UUID fallback).

## Request-level background/event tracking

- `execute_operation` creates an event channel per request (`event_channel()`), collects transport events after send, and feeds diagnostics.
- This is ephemeral per operation and not a long-lived background task.

## 5) Shutdown and cleanup semantics

## Explicit shutdown API

There is no explicit `shutdown()` / `close()` on `CosmosDriverRuntime` or `CosmosDriver` in the analyzed files.

## Drop/cleanup behavior observed

- `CosmosDriverRuntime`, `CosmosDriver`, and `CosmosTransport` have no custom `Drop`.
- `CpuMemoryMonitor` does implement `Drop` and unregisters a listener.
- VM metadata service is cached and reused; no explicit teardown path.
- Caches/registry can be invalidated/cleared via internal methods, but no public runtime teardown is exposed.

## Lifetime implications

- Driver instances are retained in `driver_registry` for the runtime lifetime; no eviction/removal API is present.
- Because each driver stores a cloned runtime and the runtime stores the registry containing the driver `Arc`, the structure forms a strong reference cycle (`registry -> driver -> runtime clone -> registry`).
- In practice, this means drivers/runtime internals are effectively process-lifetime unless structural cleanup is introduced.

## 6) Instance management across operations

## Driver instances

- One logical driver per account endpoint key.
- Repeated `get_or_create_driver` calls return the same `Arc<CosmosDriver>`.
- Driver-level options are fixed at first creation for a given endpoint key.

## Runtime options and operation overrides

- Per operation, effective runtime options are recomputed by three-layer merge:
  - operation overrides
  - driver defaults
  - runtime defaults
- This enables dynamic runtime default changes without recreating drivers.

## Container metadata across operations

- `resolve_container(db, container)` checks runtime container cache first.
- On miss, it issues read-database/read-container operations, constructs a resolved `ContainerReference`, and caches it for future operations.
- Subsequent operations can reuse resolved container references to avoid repeated metadata fetches.

## Transport/pipeline reuse

- Transport infrastructure is runtime-shared (`Arc<CosmosTransport>`).
- Pipelines are created per request from shared transport and auth context.
- Underlying reqwest transports/connection pools are reused across operations.

## Summary

The design is runtime-centric and share-heavy: one runtime owns shared transport/options/caches, and per-endpoint singleton drivers borrow that shared state through cloned runtime handles. Operation execution is request-local with deterministic option merging, while background services (CPU monitor and VM metadata) are process-oriented. Cleanup is minimal/implicit, and current ownership structure retains driver/runtime instances for process lifetime.
