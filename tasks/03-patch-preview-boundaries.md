# Task 3: close PATCH preview boundaries without gating the internal engine

**Status:** Pending.
**Dependencies:** Tasks 1 and 2.
**Intended commit:** `Close Cosmos PATCH preview boundaries`.

Read `tasks/README.md` for path abbreviations, confirmed decisions, commit boundaries, and required documentation/API/validation work.

## Mechanical changes

1. Add driver Cargo feature `preview_patch = []`, not included in defaults. Forward it from SDK `preview_patch`.
2. In SDK `src/models/mod.rs`, move `CosmosNumber`, `PatchInstructions`, and `PatchOperation` out of the unconditional export block. Gate these shared models with `any(feature = "preview_patch", feature = "preview_dtx")`: existing preview DTX `patch_item` consumes them. DTX-only builds must keep working without enabling ordinary item PATCH/tracking options.
3. Keep SDK `PatchItemOptions`, ordinary `ContainerClient::patch_item`, `PatchStrategy`, tracking ID/constants, and tracking accessors behind `preview_patch`. Re-export the shared tracking identity from task 1.
4. In driver `src/options/operation_options.rs`, put `#[cfg(feature = "preview_patch")]` on `patch_strategy`. Gate its now-optional import and any feature-specific test/doc references. The generated builder and view must omit `with_patch_strategy` and `patch_strategy` when disabled.
   - Rust removes cfg-disabled fields before invoking derive macros; a local compiler probe confirmed this. Start with the field attribute only.
   - Do not modify the macros crate merely to copy cfg attributes. The driver currently uses published `azure_data_cosmos_macros = "0.2.0"`, not the local crate.
   - Validate the real generated builder/view in both configurations. If an actual macro deficiency appears, stop and narrow it before adding a local macro dependency or changing its API.
5. In driver `src/driver/cosmos_driver.rs` PATCH strategy resolution, read the layered option only with `preview_patch`. Without it, retain the internal engine's existing default `PatchStrategy::Auto`; the public strategy option/environment setting is unavailable. Do not disable ordinary non-PATCH operations or rewrite the engine.
6. Gate the following public methods on shared driver types:
   - `error::CosmosError::patch_tracking_id`.
   - `error::CosmosErrorBuilder::with_patch_tracking_id`.
   - `diagnostics::DiagnosticsContext::patch_tracking_id`.
   - `models::CosmosResponse::patch_tracking_id`.
   Keep private tracking storage intact.
7. Where internal code still needs these functions without the feature, extract the existing body into a `pub(crate)` helper (for example, `patch_tracking_id_internal` / `with_patch_tracking_id_internal`). Make the feature-gated public method a thin delegate. Migrate intra-driver engine calls to the internal helper. Do not duplicate state/behavior or use `Deref`/hidden public methods to bypass the gate.
8. Check inherited/reachable paths, not just SDK exports: an SDK user can reach driver responses through `CosmosError::response()` and the error builder through `CosmosError::builder()`. Both must obey the shared driver feature boundary.
9. Native currently exposes PATCH. Explicitly enable driver `preview_patch` in Native's dependency rather than deleting native PATCH functionality. Do the same for existing direct-driver consumers that intentionally use gated configuration/tracking, or gate their feature-dependent test modules/targets. Do not enable the driver feature in SDK defaults or unconditional SDK dev-dependencies.
10. Update driver integration test module cfgs/required features as necessary so default tests compile and all-feature tests still exercise PATCH behavior. Keep the internal engine's ungated unit coverage where useful.
11. Once the SDK tracking wrapper is gone, check whether the SDK normal optional `uuid` dependency is still needed. Remove it and `dep:uuid` from `preview_patch` only if no normal SDK code uses it; keep test dependencies where used. Do not change UUID versions.
12. Document preview status and Cargo feature behavior. Ordinary non-preview builds must not advertise PATCH strategy environment configuration that they cannot apply.

## Acceptance criteria

- An SDK-only default/minimal consumer cannot import the PATCH models (unless it opts into DTX), name `PatchItemOptions`, call ordinary item PATCH, set `OperationOptions.patch_strategy`, call its generated builder setter, or access PATCH tracking through errors/diagnostics/responses/builders.
- SDK `preview_patch` makes all intended PATCH paths usable, with one tracking-ID type.
- SDK `preview_dtx` alone exposes the existing transaction PATCH models/method, but not ordinary item PATCH or tracking/strategy configuration.
- Explicit driver `preview_patch` unification makes shared-type members available as agreed; SDK-local cfg-gated methods still follow the SDK feature.
- No driver PATCH feature is enabled accidentally by SDK test dependencies. Run negative tests in a standalone downstream consumer, not only a workspace build containing Native.
- Native PATCH and all existing preview behavior remain functional.
