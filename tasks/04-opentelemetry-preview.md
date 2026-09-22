# Task 4: one OpenTelemetry preview feature

**Status:** Pending.
**Dependencies:** Task 3.
**Intended commit:** `Gate OpenTelemetry behind one preview feature`.

Read `tasks/README.md` for path abbreviations, confirmed decisions, commit boundaries, and required documentation/API/validation work.

## Mechanical changes

1. In SDK `Cargo.toml`, remove `metrics` and `distributed_tracing`; add `preview_opentelemetry = ["dep:opentelemetry"]`. Do not retain aliases, and keep it out of defaults.
2. Change SDK cfgs for the metrics module, `MetricsOptions`, `CosmosMetricsHandler`, tracing module, `CosmosTracingHandler`, and shared semantic-convention attributes to the new feature. Both handlers are enabled together.
3. Keep `DiagnosticsHandler`, the handler chain, diagnostics data, `SamplingLogHandler`, and ordinary `TracingLogHandler` available without this feature.
4. Update docs.rs feature metadata, feature tables, Rustdocs, examples, and public API feature metadata. Say that these concrete OpenTelemetry integrations are preview APIs; do not describe the OpenTelemetry crate/version itself as an SDK-stable contract.
5. Update Harness `Cargo.toml`, `src/main.rs`, `src/client.rs`, `src/config.rs`, and README feature forwarding/cfgs. Enabling the replacement Harness feature must enable both SDK handlers; preserve existing runtime selection of which telemetry to emit.
6. Search other consumers, including perf `src/bin/binary_payload_ab.rs`, CI/example commands, and manifests, for the old feature names. Change only feature references, not ordinary metric/tracing terminology or wire headers.

## Acceptance criteria

Neither concrete handler nor `MetricsOptions` is accessible in a default/minimal SDK build; both are accessible with the one preview feature. Existing metrics and span-exporter tests pass, while ordinary logging/diagnostics remain usable without OpenTelemetry.
