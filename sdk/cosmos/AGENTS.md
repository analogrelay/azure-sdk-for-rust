---
applyTo: "sdk/cosmos/**/*.*"
---

# Cosmos SDK Instructions

Treat `sdk/cosmos` as a mature, multi-crate codebase. Make the smallest coherent
change, preserve established ownership boundaries, and extend existing patterns
rather than introducing parallel abstractions.

## Start with project context

Before investigating code:

1. Read [docs/Project.md](docs/Project.md) for product boundaries, crate roles,
   and support expectations.
2. Read [docs/Architecture.md](docs/Architecture.md) for layer ownership,
   request flow, pipelines, and shared state.
3. Inventory the available design material through
   [docs/README.md](docs/README.md).
4. Read **only** the specs, ADRs, and reports relevant to the task. Do not load
   every document.

When changing durable documentation, also follow
[docs/AGENTS.md](docs/AGENTS.md). Detailed Rust conventions live in
[docs/CodeStyle.md](docs/CodeStyle.md).

## Respect crate and layer ownership

- `azure_data_cosmos` owns the supported Rust API, typed models, `serde`
  serialization, responses, options, and telemetry emission.
- `azure_data_cosmos_driver` owns schema-agnostic execution: routing, retries,
  failover, hedging, sessions, transport, query dataflow, caches, and
  diagnostics collection.
- `azure_data_cosmos_driver_native` is an unpublished C ABI over the driver.
  The Rust SDK does not use it.
- Emulator, observability, performance, and benchmark crates are engineering
  tools, not supported Azure products.

Keep ordinary item bodies opaque in the driver: requests use bytes and
responses return bytes. Service-defined metadata and protocol envelopes may be
parsed there. PATCH is the single approved application-body exception; do not
generalize it.

Do not expose driver models through the SDK's public API. Keep public models
independent and convert explicitly at the boundary. A narrow protocol type may
be shared only when the relevant architecture decision permits it.

Place behavior in the pipeline that owns its retry and state scope:

- dataflow: pages, partitions, plans, ordering, and continuation;
- operation: logical requests across regions and attempts;
- transport: one endpoint attempt, signing, deadlines, and local retry.

Consult the relevant spec or ADR before changing these boundaries, wire
contracts, feature categories, configuration precedence, or error taxonomy.

## Compatibility and generated code

- During pre-GA `0.x` releases, always prefer a clean breaking change over a
  compatibility shim.
- After GA, preserve public compatibility unless the task explicitly approves a
  breaking change.
- Never edit files under a `generated/` directory. Change the TypeSpec source,
  generator, or customization layer, then regenerate.
- Do not hand-write generated clients, models, or operations when TypeSpec owns
  them.

## Implementation rules

- Follow the repository instructions and
  [docs/CodeStyle.md](docs/CodeStyle.md) for public APIs, imports, errors,
  documentation, and tests.
- Prefer typed, explicit boundaries over implicit conversion or shared mutable
  state.
- Use immutable execution snapshots; update long-lived routing, cache, session,
  and diagnostics state through their established owners.
- If an infallible function needs to produce an error, raise this with the user
  to confirm intent. Strongly prefer making the function fallible rather than
  hiding, logging, or panicking on the error.
- Do not add speculative abstractions, compatibility layers, or fallback paths.
- Update the relevant spec when behavior changes. Add or supersede an ADR only
  for a finalized cross-cutting architecture decision; do not rewrite accepted
  ADRs.

## Tests and validation

Add coverage at the narrowest useful level:

- unit tests for isolated parsing, planning, routing, retry, and conversion
  behavior;
- in-memory emulator tests for deterministic service behavior and faults;
- hosted or legacy emulator tests for transport and end-to-end behavior;

Do not test trivial field assignment, getters, or derived traits. Preserve
existing test-category gates and intentional emulator exclusions.

For repository-specific commands, follow the non-standard skills directly:

- [Cosmos validation skill](.github/skills/validate/SKILL.md)
- [Cosmos emulator-test skill](.github/skills/emulator-tests/SKILL.md)

Do not consider the task complete until the validation skill's acceptance
criteria pass for every affected package or for the workspace.
