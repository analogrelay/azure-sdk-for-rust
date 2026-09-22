# Task 1: one error identity and shared diagnostics

**Status:** Pending.
**Dependencies:** None.
**Intended commit:** `Unify Cosmos error and diagnostics types`.

Read `tasks/README.md` for path abbreviations, confirmed decisions, commit boundaries, and required documentation/API/validation work.

## Mechanical changes

1. In SDK `src/error.rs`, replace the SDK `CosmosError` newtype with `pub use azure_data_cosmos_driver::error::CosmosError`. Retain the SDK's `CosmosStatus`, `SubStatusCode`, and `Result<T>` paths. Update root exports in `src/lib.rs` as necessary; do not rename the public error.
2. Move these implementations and their focused tests from SDK `src/error.rs` into driver `src/error/`:
   - `From<CosmosError> for azure_core::Error`.
   - `classify_for_azure_core`.
   - `From<serde_json::Error> for CosmosError`.
   - `From<url::ParseError> for CosmosError`.
   The driver already depends on the required crates. Update internal imports to `crate`, rather than introducing a driver-to-SDK dependency.
3. Preserve error classification exactly: actual wire responses become HTTP errors with original bytes/headers where supported; credential, connection, I/O, serialization, and timeout mappings retain their meanings. Preserve the shared `CosmosError` as the downcastable source and preserve its underlying source chain/backtrace.
4. Keep the SDK-local request-encoding helpers `convert_json_encode_error` and `convert_binary_encode_error`. Return the shared driver's builder result directly. Do not let request-body encoding errors accidentally become response-body errors through the blanket JSON conversion.
5. Replace SDK-only `.with_diagnostics(...)` and `.with_patch_tracking_id(...)` error helpers with `CosmosErrorBuilder::from_error(error)...build()` at the relevant call sites. Combine metadata updates in one builder chain where both are present. Relevant files include SDK `src/models/cosmos_response.rs` and `src/clients/offers_client.rs`.
6. Remove now-redundant wrapper constructions, identity `map_err(CosmosError::from)` calls, and identity `.into()` calls where Clippy identifies them in affected code. Do not perform unrelated style cleanup.
7. In SDK `src/diagnostics/mod.rs`, export driver `RequestedRegion` and `ExecutionContext as RequestedRegionReason`. Keep the shared `DiagnosticsContext`. Remove `src/diagnostics/region.rs` if nothing remains after moving its useful user example to the diagnostics documentation.
8. Delete obsolete conversion tests that only exercise the removed SDK region copies. Add type-identity tests instead. Preserve all region reasons; do not retain the old future-variant-to-`Initial` fallback.
9. Replace SDK `src/models/patch_tracking.rs` with gated re-exports of the driver's `PatchTrackingId` and matching public constants, or remove the module and export from `src/models/mod.rs`. Remove `.into_driver()` / `.from_driver()` calls in SDK responses, errors, and `src/clients/container_client.rs`.
10. Keep the driver error builder opaque: its fields are already private. Its existing public builder/accessors now become SDK-reachable; do not add public convenience mutation methods merely to reproduce the removed SDK-private helpers. PATCH members are gated in task 3.
11. Correct the known non-Cosmos validation errors on SDK-re-exported types:
    - Driver `PartitionKeyVersion::TryFrom<u32>` in `src/models/mod.rs`: replace `&'static str` error with `CosmosError`.
    - Driver `DiagnosticsVerbosity::FromStr` in `src/options/diagnostics_options.rs`: replace `String` error with `CosmosError`.
    - Driver `PatchTrackingId::FromStr` in `src/models/patch.rs`: map invalid UUID input to `CosmosError`, retaining the UUID error as source.
    Use existing appropriate client-validation statuses; prefer `CLIENT_BAD_REQUEST` where no specific existing status fits. Keep Serde's required `D::Error` / `S::Error` contracts and genuine `Infallible` conversions unchanged. Do not expand this task into unrelated driver-only parser/error modernization.
12. Add `#[non_exhaustive]` to driver `src/models/response_body.rs::ResponseBody`. Find matches in sibling crates and integration tests. Prefer existing `single`, `items`, `into_single`, and `into_items` helpers to hand-written decoding. Where a genuinely exhaustive external match remains necessary, return an explicit unsupported-body error for an unknown shape; do not convert it into empty content.
    - SDK `src/models/response_body.rs` already delegates its non-`Bytes` branch to driver `into_items`; preserve that behavior.
    - The moved Azure-core classifier is now inside the enum's defining crate and can remain exhaustive there.
    - Preserve the existing handling of known `Items` bodies on error paths; do not concatenate feed items into a fabricated HTTP payload.

## Acceptance criteria

- An SDK-only function can return `"invalid".parse::<FeedRange>()` directly as `azure_data_cosmos::Result<FeedRange>`, without a wrapper conversion.
- The same direct-return identity works for newly standardized validation errors.
- `DiagnosticsContext::requested_regions()` can be returned directly as `Vec<azure_data_cosmos::diagnostics::RequestedRegion>`. Its `.reason` has the SDK alias type.
- Response/error/handler diagnostics continue to share the same `Arc`; status, charge, activity ID, region information, and source metadata are not reconstructed or lost.
- With PATCH enabled, the tracking ID returned by errors, diagnostics, and SDK responses is assignable to the SDK request option's tracking-ID type.
- Azure-core conversion regression tests cover wire body/headers, synthetic classifications, source downcasting, and retained diagnostics.
- Downstream exhaustive matching of the driver response body requires a wildcard.
