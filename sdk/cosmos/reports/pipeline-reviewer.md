# Cosmos Driver Operation Pipeline Review

## Scope and Entry Point

- **File analyzed:** `sdk/cosmos/azure_data_cosmos_driver/src/driver/cosmos_driver.rs`
- **Primary entry point:** `CosmosDriver::execute_operation(operation, options)`
- **Scope:** request construction, pipeline/policy execution, retries, routing, transport, response handling, diagnostics integration.

## High-Level Call Flow

- `CosmosDriver::execute_operation`
  - `effective_runtime_options(&options)`
  - `effective_throughput_control_group(&effective_options, container)` (if container-scoped)
  - `DiagnosticsContextBuilder::new(activity_id, diagnostics_options)`
  - `CosmosResourceReference::link_for_signing()` + `request_path()`
  - `AccountEndpoint::join_path(request_path)`
  - `Request::new(url, operation_type.http_method())`
  - `CosmosTransport::{create_dataplane_pipeline|create_metadata_pipeline}`
    - `CosmosTransport::create_authenticated_pipeline`
    - `CosmosPipeline::new([CosmosHeadersPolicy, AuthorizationPolicy], transport)`
  - `CosmosPipeline::send(&ctx, &mut request)`
    - `CosmosHeadersPolicy::send`
    - `AuthorizationPolicy::send`
    - `TrackedTransportPolicy::send`
    - `try_into_raw_response()` buffering
  - Success path: parse headers/body, finalize diagnostics, return `CosmosResult`
  - Error path: classify sent/not-sent, fail diagnostics request record, return error

## Stage 1: Option Resolution and Pre-Execution Setup

- `effective_runtime_options()` merges `RuntimeOptions` in priority order:
  - operation-level (`OperationOptions::runtime()`)
  - driver-level (`DriverOptions::runtime_options().snapshot()`)
  - runtime-level (`CosmosDriverRuntime::runtime_options().snapshot()`)
- `effective_throughput_control_group()` resolves throughput control group by:
  - explicit group name in effective options, then
  - default group for the container.
- In current `execute_operation`, resolved throughput group is computed but not yet applied (`_effective_control_group`).

## Stage 2: Diagnostics Initialization

- Generates operation-scoped `ActivityId` via `ActivityId::new_uuid()`.
- Creates mutable `DiagnosticsContextBuilder` with runtime diagnostics settings.
- Starts a per-request diagnostics record later via `start_request(...)` with:
  - `ExecutionContext::Initial`
  - selected `PipelineType`
  - selected `TransportSecurity`
  - placeholder region `Region::new("Unknown")`
  - endpoint host string.

## Stage 3: Resource, Signing, and URL Construction

- Gets account/auth from `operation.resource_reference().account()`.
- Computes signing path using `resource_ref.link_for_signing()`:
  - feed operations sign with **parent** path (Cosmos signature requirement),
  - item operations sign with full path.
- Computes request URL path using `resource_ref.request_path()`.
- Builds absolute URL with `AccountEndpoint::join_path(request_path)`.

## Stage 4: HTTP Request Construction

- Determines method from `operation.operation_type().http_method()`.
- Creates `azure_core::http::Request`.
- Populates request data from `CosmosOperation`:
  - body via `operation.body()` -> `request.set_body(...)`
  - operation headers via `operation.headers()` -> `request.insert_header(...)`
  - partition key via `operation.partition_key()` -> `PartitionKey::as_headers()`.
- Constructs `AuthorizationContext` with:
  - HTTP method,
  - `ResourceType`,
  - signing link (leading `/` stripped).

## Stage 5: Routing and Pipeline Selection

- **Pipeline routing decision:** `uses_dataplane_pipeline(resource_type, operation_type)`.
  - `true` for `ResourceType::Document`.
  - `true` for `ResourceType::StoredProcedure` when operation is `Execute`.
  - otherwise metadata pipeline.
- Selects pipeline via transport:
  - dataplane: `create_dataplane_pipeline`
  - metadata: `create_metadata_pipeline`.
- Selects diagnostics classifications:
  - `PipelineType::{DataPlane|Metadata}`
  - `TransportSecurity::{Secure|EmulatorWithInsecureCertificates}` using `is_emulator_host(endpoint)`.
- **Current routing limitations in this flow:**
  - no region selection/failover inside `execute_operation`;
  - diagnostics region hard-coded to `Unknown`;
  - account metadata cache is not consulted here.

## Stage 6: Context Wiring and Event Tracking

- Creates `Context` and inserts:
  - `AuthorizationContext` (required by `AuthorizationPolicy`),
  - `EventEmitter` backed by `event_channel()` for lifecycle events.
- Event capture is later materialized with `TrackedRequestState::collect(receiver)`.

## Stage 7: Policy Chain and Transport Execution

- `CosmosPipeline` is custom and intentionally excludes azure_core default pipeline policies.
  - No built-in azure_core retry/logging/telemetry policies are injected.
- Effective chain order:
  1. `CosmosHeadersPolicy`
     - sets Cosmos headers (`x-ms-version`, capabilities, accept/cache-control),
     - preserves existing content-type,
     - overrides user-agent.
  2. `AuthorizationPolicy`
     - reads `AuthorizationContext` from `Context`,
     - creates `x-ms-date`,
     - computes and sets `Authorization` header:
       - AAD token (`type=aad...`) or
       - master key HMAC signature (`type=master...`).
  3. `TrackedTransportPolicy` (last)
     - emits `TransportStart`,
     - calls underlying `Transport` (`reqwest` client),
     - emits `ResponseHeadersReceived` or `TransportFailed`.
- `CosmosPipeline::send` then buffers full response body via `try_into_raw_response()` and emits `TransportComplete`.

## Stage 8: Retry Behavior (Current State)

- `execute_operation` performs a **single send attempt** (`pipeline.send(...).await`) with no retry loop.
- `CosmosPipeline` explicitly avoids default retry policies.
- Diagnostics model includes retry-aware structures (`ExecutionContext::Retry`, request sent-state tracking), but this function currently always records `ExecutionContext::Initial`.
- Retry-safety support that already exists:
  - request lifecycle events,
  - `RequestSentStatus` classification through events + error heuristics (`TrackedRequestState::request_sent_status_with_error`).

## Stage 9: Response Handling (Success Path)

- On `Ok(response)`:
  - reads HTTP status and optional `x-ms-substatus` -> `SubStatusCode`.
  - extracts optional `x-ms-request-charge` and updates request diagnostics before completion.
  - attaches all tracked transport events to request diagnostics.
  - marks request complete with `complete_request(handle, status, sub_status)`.
  - sets operation-level status via `set_operation_status(status, sub_status)`.
  - parses Cosmos headers into `CosmosHeaders::from_headers(response.headers())`.
  - buffers body bytes (`response.into_body()` -> `Vec<u8>` clone).
  - finalizes immutable diagnostics (`Arc<DiagnosticsContext>`) via `diagnostics_builder.complete()`.
  - returns `CosmosResult::new(body, cosmos_headers, diagnostics)`.

## Stage 10: Error Handling (Transport/Policy Error Path)

- On `Err(e)` from pipeline send:
  - computes request-sent status from events + error kind heuristics.
  - records all captured events into diagnostics request entry.
  - marks request as failed: `fail_request(handle, e.to_string(), request_sent)`.
  - sets operation status to synthetic transport mapping:
    - HTTP `503 ServiceUnavailable`
    - sub-status `TRANSPORT_GENERATED_503`.
  - returns original error (`Err(e)`).
- Note: diagnostics are not returned to caller on error in this API shape; only successful `CosmosResult` carries diagnostics.

## Key Data Structures in the Flow

- **Input/operation model**
  - `CosmosOperation`: operation type, resource type/reference, optional body, headers, partition key.
  - `CosmosResourceReference`: builds signing link and request path.
  - `OperationOptions`/`RuntimeOptions`: merged config context (partially consumed in this function).
- **Transport/pipeline model**
  - `CosmosTransport`: chooses metadata/dataplane and emulator/secure transport.
  - `CosmosPipeline`: ordered policy chain + tracked transport final policy.
  - `AuthorizationContext`: context payload required for signature generation.
- **Diagnostics model**
  - `DiagnosticsContextBuilder` -> immutable `DiagnosticsContext`.
  - `RequestDiagnostics` per request attempt.
  - `RequestEvent`/`RequestEventType` lifecycle events.
  - `RequestSentStatus` tri-state for retry safety.
- **Output model**
  - `CosmosResult`: raw body bytes + `CosmosHeaders` + `Arc<DiagnosticsContext>`.

## Practical Observations

- The pipeline split (metadata vs dataplane) is implemented and active.
- Request signing and header policy integration are fully wired through `Context` + policies.
- Transport event tracking is integrated and contributes to diagnostics.
- Retry and region-aware routing are structurally prepared in types/diagnostics but not yet implemented in this `execute_operation` path.
