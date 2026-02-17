# Cosmos Driver PR Iteration Plan

## Why this split

The current branch touches 151 files in `sdk/cosmos` with ~35k insertions, spanning a new driver crate, SDK integration changes, native/FFI updates, diagnostics, routing, and test/doc infra.  
This plan breaks that work into reviewable PRs aligned to architectural seams and dependency order.

## Agent reports created for follow-up work

- `sdk/cosmos/reports/pipeline-reviewer.md`
- `sdk/cosmos/reports/runtime-lifecycle.md`
- `sdk/cosmos/reports/model-investigator.md`
- `sdk/cosmos/reports/diagnostics-investigator.md`
- `sdk/cosmos/reports/options-investigator.md`

## Iteration sequence

### Iteration 1: Driver crate skeleton + packaging

**PR goal:** Introduce `azure_data_cosmos_driver` crate skeleton and wiring without deep behavior.

**Tasks**
- Add crate scaffolding (`Cargo.toml`, `src/lib.rs`, module layout, README/ARCHITECTURE baseline).
- Add minimal CI/build integration for the new crate in `sdk/cosmos/ci.yml`.
- Add crate-level smoke test wiring (compile/test only, minimal runtime behavior).
- Keep `azure_data_cosmos` behavior unchanged in this PR.

---

### Iteration 2: Core model foundation (value/reference/wire types)

**PR goal:** Land driver model types as an isolated, testable foundation.

**Tasks**
- Add and review model modules under `azure_data_cosmos_driver/src/models/` in focused chunks:
  - identity/reference models (`resource_id.rs`, `resource_reference.rs`, `cosmos_resource_reference.rs`, `account_reference.rs`)
  - wire/value helpers (`partition_key.rs`, `etag.rs`, `request_charge.rs`, `activity_id.rs`, `connection_string.rs`, `user_agent.rs`)
  - operation/result/status envelopes (`cosmos_operation.rs`, `cosmos_result.rs`, `cosmos_status.rs`, `mod.rs`)
- Add targeted unit tests for parsing, serialization, path/signing link generation, and status/substatus mapping.
- Ensure no runtime/transport logic is coupled into this PR.

---

### Iteration 3: Driver options system

**PR goal:** Add runtime/driver/operation option types and builders independently from transport execution.

**Tasks**
- Add `azure_data_cosmos_driver/src/options/*` modules and exports.
- Validate env parsing and defaults (`env_parsing.rs`, diagnostics defaults, runtime/driver merge semantics).
- Add tests for precedence and builder invariants.
- Defer active execution-path consumption to later iterations.

---

### Iteration 4: Diagnostics subsystem (types + builder + output)

**PR goal:** Land diagnostics model and builder as a standalone subsystem.

**Tasks**
- Add `src/diagnostics/mod.rs` and `src/diagnostics/diagnostics_context.rs`.
- Add tests for request lifecycle transitions (`start`, `complete`, `fail`), status aggregation, and JSON output (`Summary`/`Detailed`).
- Keep integration thin: diagnostics types available but not fully connected to operation pipeline yet.

---

### Iteration 5: Runtime lifecycle + shared caches

**PR goal:** Introduce runtime creation, shared transport ownership, cache infrastructure, and driver registry.

**Tasks**
- Add `driver/runtime.rs`, cache modules (`driver/cache/*`), and `system/*` dependencies.
- Add tests for:
  - endpoint-keyed `get_or_create_driver` behavior
  - container/account cache hit/miss behavior
  - runtime options snapshot/merge mechanics
- Explicitly document lifecycle semantics and known retention constraints (registry/ownership model).

---

### Iteration 6: Transport and policy pipeline primitives

**PR goal:** Add transport layer and policy chain independent of full `execute_operation`.

**Tasks**
- Add transport modules (`driver/transport/{mod,pipeline,tracked_transport,headers_policy,authorization_policy,emulator}.rs`).
- Add tests for:
  - header population policy
  - authorization context/signature policy behavior
  - tracked transport event emission ordering
  - metadata vs dataplane pipeline construction
- Keep final operation orchestration (`execute_operation`) minimal or stubbed.

---

### Iteration 7: `execute_operation` orchestration path

**PR goal:** Land end-to-end operation execution in `cosmos_driver.rs`, now that models/options/runtime/transport exist.

**Tasks**
- Implement and review `CosmosDriver::execute_operation` in one PR focused on:
  - option resolution
  - request construction
  - pipeline selection
  - response mapping into `CosmosResult`
  - diagnostics event wiring and completion
- Add operation-level tests (mock transport/emulator) for success, transport failure, request-sent classification.
- Keep retries/failover enhancements out of this iteration unless already fully complete.

---

### Iteration 8: `azure_data_cosmos` integration with driver

**PR goal:** Move SDK client flow to use the new driver while preserving existing external behavior.

**Tasks**
- Update `azure_data_cosmos/src/clients/*`, request/context/pipeline integration points, and response plumbing.
- Land query/routing/fault-injection changes in **separate sub-PRs** if they exceed reviewable scope.
- Migrate or adapt tests in small PRs:
  - framework updates
  - emulator test migration (`tests/emulator_tests/*`)
  - multi-write/fault-injection scenarios
- Verify no unintended public API breaks in `azure_data_cosmos`.

---

### Iteration 9: `azure_data_cosmos_native` options and FFI alignment

**PR goal:** Isolate C/native wrapper changes from driver+SDK internals.

**Tasks**
- Land native options module updates (`azure_data_cosmos_native/src/options/mod.rs`) and header updates.
- Clearly separate:
  - active `ClientOptions` conversion path
  - placeholder ABI-only options for future behavior
- Add FFI-focused tests/validation for pointer conversion safety and defaults.

---

### Iteration 10: Docs, guidance, and release metadata

**PR goal:** Keep documentation/process changes in a dedicated, low-risk PR.

**Tasks**
- Land docs/instructions updates (`sdk/cosmos/AGENTS.md`, skills, contributing notes, design principles docs).
- Land changelog updates only after functional PR sequence stabilizes.
- Cross-link the five generated reports and architecture docs for future follow-up work.

## Suggested dependency graph

1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8  
7 -> 9  
All functional iterations -> 10

## Guardrails per PR

- Keep each PR focused to one architectural seam.
- Include targeted tests only for the seam introduced in that PR.
- Avoid mixing docs/changelog churn into functional PRs.
- Prefer follow-up PRs for retries/region failover/lifecycle cleanup if they are not complete and test-backed in the same iteration.
