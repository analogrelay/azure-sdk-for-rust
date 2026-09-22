# Cosmos SDK 1.0 public API hardening tasks

Make the last practical compatibility fixes before GA, without redesigning the SDK or isolating it from the driver. These files are implementation instructions, not authorization to begin implementation. All seven tasks are pending.

Read this file before executing any task. Each numbered file contains that task's steps and acceptance criteria. Execute them sequentially.

## Paths and baseline

Implementation paths in these tasks are relative to `sdk/cosmos/`, unless explicitly identified otherwise. Abbreviations:

- **SDK**: `azure_data_cosmos/`
- **Driver**: `azure_data_cosmos_driver/`
- **Native**: `azure_data_cosmos_driver_native/`
- **Harness**: `azure_data_cosmos_observability_harness/`

Run validation commands from the repository root. The inspected baseline is commit `038b557b4d`. Recheck the worktree before execution; integrate unrelated user changes rather than reverting them.

## Task index and commit boundaries

| Task file | Intended commit title | Dependency |
| --- | --- | --- |
| `01-shared-errors-and-diagnostics.md` | `Unify Cosmos error and diagnostics types` | None |
| `02-checked-key-construction.md` | `Make Cosmos key construction fallible` | 1 |
| `03-patch-preview-boundaries.md` | `Close Cosmos PATCH preview boundaries` | 1, 2 |
| `04-opentelemetry-preview.md` | `Gate OpenTelemetry behind one preview feature` | 3 |
| `05-api-encapsulation-and-documentation.md` | `Encapsulate Cosmos options and document limits` | 4 |
| `06-remove-throughput-control-groups.md` | `Remove throughput control groups` | 5 |
| `07-final-api-verification.md` | Final integrated API/feature verification; no implementation commit | 6 |

Keep all throughput-group removal, native ABI changes, removal-specific tests, documentation, changelog entries, and API artifact changes in task 6. Do not delete group exports, types, tests, or status codes in earlier tasks. Earlier tasks may mechanically update existing group code when needed for shared error/construction changes, but must preserve its behavior.

Do not automatically commit simply because these instructions contain commit boundaries. When execution includes creating commits, use the repository commit requirements and session trailers. If execution is requested without commits, stop before mixing task 6 into an uncommitted bundle and obtain the necessary commit/staging direction.

## Confirmed decisions

1. Continue re-exporting reasonable driver types. Their SDK-reachable API is public even though the driver is an internal implementation dependency. Do not introduce wrappers solely to remove driver dependencies from signatures.
2. Re-export driver `CosmosError` as SDK `CosmosError`. Move the necessary SDK error conversions to the driver. Keep SDK `Result<T>` using that one error identity.
3. Share driver diagnostics. Replace the unused SDK region wrappers with driver re-exports, exporting `ExecutionContext` under the SDK name `RequestedRegionReason`.
4. Re-export driver `PatchTrackingId` under `preview_patch`, replacing the duplicate SDK wrapper.
5. Make only the problematic partition-key/EPK conversions fallible. Retain existing safe string, integer, Boolean, and non-floating-point tuple conveniences.
6. Make `FeedRange::for_partition` return `Result` without renaming it. Propagate errors through dependent driver operation constructors. Mechanical test migrations are approved.
7. Preserve empty-vector partition-key construction. Reject an empty key when constructing a logical partition feed range; do not retain the old malformed-input fallback.
8. Keep the driver's internal PATCH engine. Gate SDK-reachable PATCH configuration and tracking accessors with a driver `preview_patch` feature forwarded by the SDK. Both features are off by default. Explicitly enabling the driver feature through another dependency is an accepted preview opt-in under Cargo feature unification.
9. Replace both existing SDK features, `metrics` and `distributed_tracing`, with **one** off-by-default `preview_opentelemetry` feature enabling both handlers. There is no existing SDK `opentelemetry` feature to rename.
10. Remove throughput-control groups from SDK, driver, and native wrapper. Preserve direct `ThroughputControlOptions.throughput_bucket` and `.priority_level`. Put the complete removal in its own final implementation commit.
11. Preserve feed-range serialization behavior, including loss of logical-partition identity. Document the limitation for application users. Do not change point-range overlap semantics in this work.
12. Make `CosmosClientOptions` SDK-private and the option `View` types driver-only. Remove SDK `ConsistencyLevel`. Keep and document `OperationOptions::custom_headers`. Privatize `SessionToken` and `ExcludedRegions` tuple fields.

## Documentation, changelog, and API artifacts for every task

- Read the applicable repository/Cosmos instructions before editing. For affected specs, follow `docs/AGENTS.md`. Accepted ADRs remain immutable; update mutable architecture/spec text that inaccurately describes the current implementation, without rewriting historical decisions.
- Update directly related Rustdocs, crate documentation, and current examples alongside each change. Keep SDK-facing text application-oriented.
- Read the repository's changelog instructions. Add concise public API entries to existing unreleased categories, one line per change; do not add category headings or rewrite past releases.
- Invoke the `export-api` skill after public API edits, using its workflow for affected Rust crates. Regenerate SDK and driver API artifacts and metadata; inspect for accidental API additions/leaks. Do not hand-edit artifacts.
- Review both SDK exports and signatures/methods reachable through shared types. A clean export list alone is not sufficient.
- Keep API artifact changes with their owning implementation commit; task 6 must own the complete throughput-removal artifact delta.
- Do not edit `generated/` directories or `eng/common/`. Native's checked-in C header is regenerated by its build, not manually patched.

## Per-task validation

After all edits for the task, format first. Then run the required validation on the crates actually changed; expand to affected dependents when constructor/features changed. Avoid a whole-workspace build unless targeted results require it.

Typical commands for SDK/driver/native work:

```bash
cargo fmt -p azure_data_cosmos -p azure_data_cosmos_driver -p azure_data_cosmos_driver_native
cargo build -p azure_data_cosmos -p azure_data_cosmos_driver -p azure_data_cosmos_driver_native
cargo clippy -p azure_data_cosmos -p azure_data_cosmos_driver -p azure_data_cosmos_driver_native --all-features --all-targets
cargo doc -p azure_data_cosmos -p azure_data_cosmos_driver --no-deps --all-features
cargo test -p azure_data_cosmos -p azure_data_cosmos_driver -p azure_data_cosmos_driver_native --all-features
```

Include Harness, benchmarks, perf, and any other changed consumer in the relevant formatting/build/Clippy checks. Use the repository's in-memory tests for routing/protocol behavior; run credential/service-dependent tests only with their documented environment. Report unavailable external test prerequisites accurately rather than implying those tests ran.

Task 7 specifies the feature matrix and downstream API checks. Apply relevant checks during each implementation task; do not defer discovering broken feature boundaries until the final task. Native ABI validation commands are in task 6.

## Explicit non-goals

- No wholesale driver/SDK decoupling or repatriation of all exported types.
- No client/response/query/page/change-feed redesign.
- No generic `TryInto<PartitionKey>` retrofit across every SDK method.
- No integer precision policy change, new partition-key data model, or removal of empty internal keys.
- No logical-feed-range serialization format change, marker preservation, point-overlap fix, or session-token-coalescing redesign.
- No removal of internal driver PATCH execution and no new public macro abstraction.
- No OpenTelemetry dependency upgrade, new telemetry architecture, or loss of ordinary logging.
- No throughput-control replacement framework or new native direct-option API.
- No unrelated review/fix of the driver's general public API.
