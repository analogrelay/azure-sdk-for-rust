# Task 7: final integrated API and feature verification

**Status:** Pending.
**Dependencies:** Task 6; all six implementation tasks must be complete.
**Intended commit:** None. This is read-only final verification.

Read `tasks/README.md` for path abbreviations, confirmed decisions, commit boundaries, and per-task validation commands. Confirm those checks ran for the affected crates. Confirm the native ABI/C suite described in task 6 ran against the generated header.

## Feature matrix

Check SDK configurations separately; do not rely only on `--all-features` or on a build with Native, which intentionally enables driver PATCH.

```bash
cargo check -p azure_data_cosmos --lib --no-default-features
cargo check -p azure_data_cosmos --lib
cargo check -p azure_data_cosmos --lib --no-default-features --features preview_patch
cargo check -p azure_data_cosmos --lib --no-default-features --features preview_dtx
cargo check -p azure_data_cosmos --lib --no-default-features --features preview_opentelemetry
cargo check -p azure_data_cosmos --lib --no-default-features --features preview_patch,preview_dtx,preview_opentelemetry
cargo check -p azure_data_cosmos --lib --all-features
cargo check -p azure_data_cosmos_driver --lib --no-default-features
cargo check -p azure_data_cosmos_driver --lib --no-default-features --features preview_patch
```

Run default/minimal and all-feature documentation checks to detect links to gated APIs. Build existing PATCH/DTX/observability examples with their required feature combinations.

## Downstream API contract checks

Use positive integration tests and the existing Rustdoc test mechanism for durable compile-fail checks rather than adding a new testing dependency. Negative feature cases can live in a private `#[cfg(doctest)]` test module with feature-conditional examples. Always run the relevant doctest feature configuration; all-features tests alone cannot verify disabled APIs. These tests belong with the corresponding implementation task; this task verifies them.

Also validate with a temporary standalone downstream Cargo package outside the workspace, depending only on the SDK by path and with its own feature selection. Do not give it a direct driver dependency except in the explicit feature-unification case. This prevents workspace/dev feature unification from producing false negatives or false confidence. Remove temporary probe artifacts afterwards.

### Positive contracts

- One shared error identity for `FeedRange` parsing/building, driver-re-exported validation, and SDK result returns.
- Correct `Vec<RequestedRegion>` / `RequestedRegionReason` identity.
- Error-to-Azure-core downcasting and retained body/header/diagnostic metadata.
- Checked key/EPK conversions and unchanged common string/integer/tuple examples.
- Public token/exclusion construction and reading without tuple access.
- Direct throughput-field configuration in all final supported configurations.
- Preview tracking type identity and both OpenTelemetry handlers when enabled.

### Compile-fail contracts

- SDK imports of `CosmosClientOptions`, option `View` types, and `ConsistencyLevel`.
- Tuple construction and `.0` access for `SessionToken` / `ExcludedRegions`.
- Infallible float/vector/EPK-string conversion paths that were deliberately removed.
- Exhaustive downstream matching of driver `ResponseBody`, including access through SDK errors.
- Default/minimal PATCH imports, option field/setter, and tracking through all shared error/response/diagnostic/builder paths.
- Default/minimal concrete OpenTelemetry handlers/options.
- Throughput group imports/registration/`group_name`, added only in task 6.

For each negative probe, confirm compilation fails for the intended missing/private/gated API, not for an unrelated import, missing dependency, or invalid surrounding program. Include positive controls using the same harness.

## Release-facing completion record

1. Inspect final API diffs against the baseline and map every intentional breaking change to these tasks.
2. Confirm no driver-only API was changed beyond what was needed for the SDK-exposed contracts, propagation, native group removal, or preview consumers.
3. Confirm old telemetry feature names and current throughput-group APIs are gone, while historical documentation is not mistaken for live code.
4. Confirm the throughput-removal commit is isolated and the preceding hardening state builds with groups intact.
5. Confirm no incidental dependency upgrades, unexplained lockfile changes, generated-code edits, or probe artifacts remain.
6. Record actual checks and any external-environment limitations in the implementation handoff. Do not claim unrun service tests passed.
