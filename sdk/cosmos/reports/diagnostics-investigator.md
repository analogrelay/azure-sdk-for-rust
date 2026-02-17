# Cosmos Driver Diagnostics Investigation

## Scope

Investigated `sdk/cosmos/azure_data_cosmos_driver/src/diagnostics/` and traced how diagnostics data flows through driver execution paths.

## Module: `src/diagnostics/mod.rs`

### Responsibility
- Declares the diagnostics module boundary and public surface.
- Re-exports core diagnostics types for crate consumers.
- Keeps `DiagnosticsContextBuilder` internal (`pub(crate)`) while exposing immutable diagnostics types publicly.

### Exports
- Internal-only: `DiagnosticsContextBuilder`
- Public: `DiagnosticsContext`, `ExecutionContext`, `PipelineType`, `RequestDiagnostics`, `RequestEvent`, `RequestEventType`, `RequestHandle`, `RequestSentStatus`, `TransportSecurity`

---

## Module: `src/diagnostics/diagnostics_context.rs`

### `ExecutionContext` (enum)
**Responsibility:** Captures *why* a request happened (`Initial`, `Retry`, `Hedging`, `RegionFailover`, `CircuitBreakerProbe`) so retries/failovers are diagnosable.

### `PipelineType` (enum)
**Responsibility:** Classifies request pipeline (`Metadata` vs `DataPlane`) so diagnostics show control-plane vs data-plane path selection.

### `TransportSecurity` (enum)
**Responsibility:** Records whether request used secure TLS or emulator-insecure cert mode.

### `RequestSentStatus` (enum)
**Responsibility:** Tri-state retry-safety signal (`Sent`, `NotSent`, `Unknown`) used to decide whether a failed request may be safely retried.

### `RequestDiagnostics` (struct)
**Responsibility:** Per-request record containing execution context, endpoint/region, status/substatus, RU charge, timings, transport events, timeout flag, send status, and error details.

**Lifecycle methods:**
- `new(...)` when request starts
- `complete(...)` on HTTP response
- `timeout()` for client operation timeout
- `fail(...)` for transport-level failures
- `add_event(...)` for pipeline milestones

### `RequestHandle` (struct)
**Responsibility:** Opaque index into builder state, allowing in-flight request updates without exposing mutable internals.

### `RequestEventType` (enum)
**Responsibility:** Standard transport milestones:
- `TransportStart`
- `ResponseHeadersReceived`
- `TransportComplete`
- `TransportFailed`

Also exposes `indicates_request_sent()` to support send-status inference.

### `RequestEvent` (struct)
**Responsibility:** Timestamped event entry with optional duration/details for request timeline diagnostics.

### `DiagnosticsContextBuilder` (internal struct)
**Responsibility:** Mutable operation-scoped collector used during execution. Tracks operation start, request list, operation status, and diagnostics options.

**Key APIs:**
- `start_request(...)`
- `update_request(...)`
- `add_event(...)`
- `complete_request(...)`
- `timeout_request(...)`
- `fail_request(...)`
- `set_operation_status(...)`
- `complete()` -> immutable `DiagnosticsContext`

### `DiagnosticsContext` (public struct)
**Responsibility:** Immutable finalized diagnostics artifact returned to consumers (usually via `Arc`). Contains operation-level activity ID, duration, status, and all request diagnostics.

**Query/output APIs:**
- `status()`, `total_request_charge()`, `request_count()`, `regions_contacted()`, `requests()`
- `to_json_string(...)` with cached `Detailed`/`Summary` JSON

### Internal summary/output helper types
These are internal serialization helpers for diagnostics JSON shaping and truncation:
- `DetailedDiagnosticsOutput`
- `SummaryDiagnosticsOutput`
- `RegionSummary`
- `RequestSummary`
- `DeduplicatedGroup`
- `TruncatedOutput`
- `DeduplicationKey`
- helper `percentile_sorted(...)`

---

## Diagnostics-related configuration types (outside `diagnostics/`)

### `options::DiagnosticsVerbosity`
Controls output style: `Default`, `Summary`, `Detailed`.

### `options::DiagnosticsOptions` + `DiagnosticsOptionsBuilder`
Controls diagnostics JSON defaults (`default_verbosity`) and summary size truncation (`max_summary_size_bytes`) including env-based defaults.

### `options::DiagnosticsThresholds`
Defines optional latency/RU/payload thresholds and is plumbed through runtime/operation options.

---

## Integration points in driver execution paths

## 1) Runtime construction path
- `CosmosDriverRuntimeBuilder` accepts `diagnostics_options(...)`.
- Built runtime stores `Arc<DiagnosticsOptions>` and exposes `diagnostics_options()`.

**Files:**
- `src/driver/runtime.rs`

## 2) Operation execution path (`CosmosDriver::execute_operation`)
1. Create operation activity ID.
2. Create `DiagnosticsContextBuilder::new(activity_id, runtime.diagnostics_options())`.
3. Determine `PipelineType` (metadata/data-plane) and `TransportSecurity` (secure/emulator).
4. `start_request(...)` with initial execution context and endpoint host.
5. Execute pipeline.
6. Collect transport events from `TrackedRequestState` and add them via `add_event(...)`.
7. On success:
   - update request RU from headers (`update_request`)
   - `complete_request(...)`
   - `set_operation_status(...)`
   - `complete()` builder and attach `Arc<DiagnosticsContext>` to `CosmosResult`
8. On transport error:
   - compute `RequestSentStatus` via `request_sent_status_with_error(...)`
   - `fail_request(...)`
   - set synthetic operation status (503 + transport substatus)
   - return error

**Files:**
- `src/driver/cosmos_driver.rs`

## 3) Transport instrumentation path
- `EventEmitter` + mpsc channel are inserted into request `Context`.
- `TrackedTransportPolicy` emits `TransportStart`, `ResponseHeadersReceived`, `TransportFailed`.
- `CosmosPipeline::send` emits `TransportComplete` after buffering response body.
- `TrackedRequestState::collect(...)` drains channel and provides send-status inference APIs.

**Files:**
- `src/driver/transport/pipeline.rs`
- `src/driver/transport/tracked_transport.rs`
- `src/driver/transport/mod.rs`

## 4) Result surfacing path
- `CosmosResult` stores `Arc<DiagnosticsContext>` and exposes it through `diagnostics()`.
- Crate root re-exports core diagnostics types for users.

**Files:**
- `src/models/cosmos_result.rs`
- `src/lib.rs`

---

## Observed wiring status and gaps

- `DiagnosticsThresholds` is currently configuration-only; no execution-path reads found in `driver/` yet.
- `DiagnosticsContextBuilder::timeout_request(...)` exists but no active call site in runtime operation execution.
- In the current error branch of `execute_operation`, diagnostics are updated in the builder but not finalized/returned to caller (error is returned directly).
