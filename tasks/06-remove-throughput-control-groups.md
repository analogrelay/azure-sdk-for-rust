# Task 6: remove throughput groups in an independent final commit

**Status:** Pending.
**Dependencies:** Task 5.
**Intended commit:** `Remove throughput control groups`.

Read `tasks/README.md` for path abbreviations, confirmed decisions, commit boundaries, and required documentation/API/validation work.

This task must contain only throughput-group removal and its directly related migration, tests, docs, changelogs, and generated artifacts. Reverting this one commit must restore groups on top of all the other hardening changes.

## Mechanical changes

1. Remove SDK `CosmosClientBuilder::register_throughput_control_group` and any SDK storage/forwarding used solely for groups. Remove `ThroughputControlGroupName` and `ThroughputControlGroupOptions` SDK exports.
2. In driver `src/options/operation_options.rs`, remove `ThroughputControlOptions.group_name` and associated docs/attributes. The derive removes its generated builder/view API. Keep `throughput_bucket`, `priority_level`, their generated methods, and independent per-field layer resolution.
3. Remove driver `ThroughputControlGroupName` from `src/models/mod.rs` and all group config/registry/key/snapshot types and registration methods from:
   - `src/options/throughput_control.rs`.
   - `src/options/driver_options.rs`.
   - `src/options/mod.rs`.
   - Any remaining runtime/builder entry points found by a scoped symbol search.
4. Do **not** delete `src/options/throughput_control.rs` wholesale if it still contains `ResolvedThroughputControl` or direct-field helpers. Keep the smallest appropriate module for surviving code.
5. In driver `src/driver/cosmos_driver.rs`, remove registry storage/initialization and group/default-group lookup branches. Resolve bucket and priority directly from the layered throughput view. Retain container-applicability behavior and emission of direct values in both gateway transports.
6. If the removal makes `effective_throughput_control` infallible or eliminates a parameter, simplify that private signature and its callers in this commit. Do not invent a placeholder group path or leave unreachable group errors.
7. Remove group-only error/status constants and their native mirrors. Do not renumber or reuse the numeric values of surviving statuses.
8. In Native `src/op_request.rs`, remove `CosmosOperationOptions.throughput_control_group`, parsing, default initialization, tests, imports, and group documentation. Update the construction test in `c_tests/operation_construction.c`. Update stale registration references in `src/runtime_builder.rs` and group-specific status references in `src/error.rs`.
9. This deliberately changes the unreleased native options-struct layout. Regenerate `include/azurecosmosdriver.h` through Native's `build.rs`/cbindgen build, never by hand. Rebuild native tests/consumers against the matching header and library. Do not preserve a misleading ignored group-name field or add new native throughput fields as unrelated scope.
10. Update mutable docs `docs/specs/0025-throughput-control.md`, affected sections of `0005-operation-and-transport-pipelines.md` and `0019-native-wrapper.md`, and current crate READMEs/examples. Describe the surviving direct bucket/priority options and their layering, not unimplemented group registration. Do not rewrite historical changelog entries, reports, or accepted ADRs.
11. Add removal-specific public API negative checks and direct-field behavior tests in this commit. Include all corresponding SDK/driver/native changelog and generated API/header changes here, not in an earlier or later hardening commit.

## Acceptance criteria

- No current SDK/driver/native callable group registration, group-name option, group config type, or registry remains.
- Bucket-only, priority-only, both, neither, and split-layer overrides preserve their pre-removal direct-field behavior.
- Header/protocol tests cover direct values for Gateway and Gateway V2; removal must not erase or reinterpret them.
- Native C construction tests build with the generated header and pass.
- No unrelated feature, constructor, error-identity, or tuple-privacy change is introduced in this commit.
- Verify the commit can be cleanly reversed relative to the hardening branch and that its parent was already validated with groups present. Do not perform a destructive revert in the user's worktree merely to demonstrate this.

## Native ABI validation

In addition to the per-task validation in `tasks/README.md`, first build Native to refresh the source header, then configure/build/run its C suite:

```bash
cargo build -p azure_data_cosmos_driver_native
cmake -S sdk/cosmos/azure_data_cosmos_driver_native -B target/cosmos-native-api-hardening
cmake --build target/cosmos-native-api-hardening
ctest --test-dir target/cosmos-native-api-hardening --output-on-failure
```

Use a fresh, task-specific build directory so CMake copies the newly generated header, not a stale previous layout.
