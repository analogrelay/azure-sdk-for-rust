# Realigning the driver pipelines around state and effects

**Status:** Proposed architectural direction
**Scope:** Internal Transport, Operation, and Dataflow pipeline refactoring
**Compatibility:** No user-visible behavior or public API changes

## Purpose and boundaries

Restore the design in `specs/0005-operation-and-transport-pipelines.md`: components
hold state, systems operate on the smallest necessary immutable inputs, and
state changes are returned as data and applied at an explicit boundary. Extend
that discipline consistently across all three pipelines, including the HTTP
send at the bottom of Transport.

The goals are human reviewability, better test coverage whose meaning
humans can understand, and better verifiability. A reviewer should be able to
identify a decision's inputs, its possible outcomes, and the place its effects
become visible without tracing a network of mutable contexts.

This is a shared architectural plan, not an implementation specification or a
sequence of pull requests. A separate planning pass will describe each
pipeline's refactoring using these concepts. The existing three-tier ownership
and retry scopes remain unchanged:

| Pipeline | Scope retained |
| --- | --- |
| Dataflow | Planning, pages, partition traversal, query operators, topology repair, and continuation state |
| Operation | One logical request across regional attempts, failover, hedging, sessions, and operation diagnostics |
| Transport | One endpoint's attempts, authorization, local retry, request-sent classification, and attempt diagnostics |

**Centralization means one shared effect-processing boundary with typed
state-owner handlers, not one global event loop, lock, or serialized driver.**
Most request execution remains sequential. Hedging and independently executing
requests retain their concurrency.

**Keep ordinary `async`/`await`.** Effects must not become a replacement async
runtime, a general instruction interpreter, or a manually resumed state machine
for every function. Readable async orchestration is part of the design, not
architectural drift.

## Current foundation and drift

The source already contains useful foundations. Reuse them rather than replacing
working domain logic with a new framework. Paths below are relative to
`sdk/cosmos/azure_data_cosmos_driver/src/`.

| Area | Existing foundation | Refactoring opportunity |
| --- | --- | --- |
| Operation decisions | `driver/pipeline/retry_evaluation.rs` returns actions and `LocationEffect` values. Hedging eligibility and terminal diagnostics have explicit types. | The ordinary attempt loop and hedge race also perform session capture, diagnostics accumulation, recovery tracking, and outcome updates. Make their ownership and ordering as visible as location effects. |
| Location state | `driver/routing/location_effects.rs` describes changes as data; `LocationStateStore` applies them against current state through compare-and-swap loops. | Snapshot acquisition, refresh claims, probes, clocks, and background updates remain environmental work. Centralize access without rewriting the synchronization algorithm. |
| Transport | Request-sent classification, signing inputs, request construction, and retry handling already have separable helpers. | Signing reads wall time; authorization can acquire credentials; dispatch, shard reservations, health accounting, and fault collectors carry environmental dependencies. Separate deterministic transformations from those dependencies. |
| Dataflow | `PageResult`, validated `SplitReplacements`, continuation snapshots, and range/ordering helpers make many contracts explicit. | `PipelineNode::next_page(&mut self, &mut PipelineContext)` combines child execution, topology access, decision-making, and state mutation. Split those responsibilities without replacing readable tree traversal. |
| Cross-cutting state | Runtime options use immutable `Arc` snapshots; caches provide coordinated fetching; page and operation diagnostics have data representations. | Passing a runtime, driver, manager, or mutable context gives helpers much more capability than they need. Diagnostics and recovery flags also cross boundaries through shared handles. |

For example, `driver/dataflow/request.rs` refreshes physical partition identity,
executes an operation, changes continuation state, and initiates topology
repair. `SkipTake` and `Distinct` preserve poisoning after a child has advanced;
`StreamingOrderedMerge` distinguishes buffered progress, emitted boundaries,
and deferred errors. These are important semantics to expose, not incidental
mutation to remove blindly.

The original specifications are design references, not exact descriptions of
every current contract. In particular, the older pluggable-transport language
in Spec 0005 does not override the accepted internal-transport decision in
`adrs/0006-internal-http-transport.md`. Spec 0012 also contains historical
continuation proposals; the existing implementation and compatibility fixtures
must govern this refactoring.

## Shared vocabulary and execution shape

Use the same concepts in each pipeline without forcing their different
algorithms into one universal state machine.

| Concept | Meaning |
| --- | --- |
| Component | Focused domain data: retry counters, routing snapshot, node cursor, buffered rows, or diagnostics fragments. It contains no capability to perform I/O or mutate another owner. |
| System | A deterministic function over the required components and observations. It returns data; it does not obtain snapshots, read clocks, or update stores itself. |
| Observation | Data obtained at an environmental boundary: time, a random sample, a cache snapshot, an HTTP outcome, or a completed hedge leg. |
| Action | A control-flow choice such as complete, retry, fetch a child page, or send an attempt. It carries the data needed by ordinary async orchestration. |
| Effect | A materialized request to change retained state or perform an externally visible operation. It is a typed value, not a closure, future, or mutable service reference. |
| Transition | A decision together with its ordered effects, including any proposed next component values. Producing it does not publish those values. |
| Owner | The sole authority to publish a component's next state. It may be a request-local owner or a concurrency-safe shared store. |
| Environment | The internal boundary for environmental capabilities and shared-state access. `Env` is its borrowed handle. |

The common execution shape is:

```text
observe through Env at the existing observation point
    -> evaluate a pure system over minimal data
    -> obtain an action and ordered effects
    -> apply effects through the shared boundary
    -> await the selected work through Env / the next pipeline
    -> evaluate its outcome
```

This is a vocabulary for readable functions, not a requirement to allocate an
event for every expression. Lower-pipeline calls remain direct async calls with
`env` passed explicitly. A prepared HTTP request or retry action already
materializes the intended work; it does not need a second generic command
representation solely to permit `.await`.

### Immutability and local mutation

Systems borrow immutable inputs or consume owned data and return replacements.
An owned transformation may use local `mut`, build a buffer, or update a private
collection while computing its result. That is still pure when it cannot change
retained or shared state and depends only on its inputs.

Changing a retained retry component, node cursor, diagnostics accumulator, or
shared cache is different: propose the update as data and publish it through
the owner handler. Do not turn every vector push into an effect. Use meaningful
transitions such as replacing retry state, accepting a page, or splicing a
validated set of replacement ranges.

`Arc<T>` is suitable for immutable data sharing; `Arc<Mutex<T>>` is not an
immutable snapshot. Nor is an options view pure if its accessors consult a live
store. Resolve those dependencies at the existing boundary and pass values.
Prefer owned buffers and small updates over cloning entire plans or caches.

## One `Environment` and one `Env`

Introduce an internal `crate::env` module. `Environment`, `Env`, production
adapters, and testing helpers must not create a public injection API or change
existing public signatures.

The following is an illustrative internal declaration, not a new public API:

```rust
#[derive(Clone, Copy)]
pub(crate) struct Env<'a>(&'a dyn Environment);
```

`Env` has no state, cache, scheduler, or additional service fields. Do not create
`TransportEnv`, `OperationEnv`, or `DataflowEnv`, and do not replace it with a
generic parameter threaded through every type.

### Signature discipline

- An internal function requiring environmental access takes `env: Env<'_>` as
  its first parameter, or immediately after its receiver.
- A pure function takes the observations it needs instead: a time value, token,
  routing snapshot, effective options, or response. Do not pass `Env` just
  because a caller has it.
- Do not store `Env` in domain components or retrieve it from thread-local or
  global state. Avoid `env.driver()`, `env.cache()`, or similar accessors that
  leak mutable service capabilities.
- Public entry points keep their current signatures and construct the borrowed
  handle before calling internal env-first functions.
- `Environment` implementations are the actual boundary: their trait methods
  use their own receiver, not an additional recursive `Env` argument. Required
  external trait signatures and `Drop` similarly need adapters rather than
  signature changes.

A borrowed `Env` cannot be moved into an arbitrary `'static` task. The production
boundary owns the resources needed by background tasks; each task borrows a
fresh `Env` from its owned environment for its execution. Domain state and
continuation tokens never retain environment lifetimes or task handles.

Keep `Environment` compatible with `dyn` dispatch and shared access across
concurrent requests. Use shared receivers with synchronization private to the
boundary, retaining the existing futures' `Send` requirements and runtime
feature support. Box futures only where the dynamic async boundary requires it;
do not introduce boxing into pure systems or change public type bounds.

### Keep the trait small and concrete

The trait describes irreducible environmental capabilities, not every driver
workflow. Keep domain algorithms in bare functions. Add a primitive when it
hides a real external dependency or an atomic shared-state operation, not just
to make a helper mockable.

| Capability | Boundary contract |
| --- | --- |
| Time and waiting | Observe monotonic and wall time separately; await a duration or deadline using the existing runtime behavior. Pure deadline arithmetic receives explicit times. |
| Identifiers and randomness | Generate UUIDs by delegating to `uuid`; obtain samples for jitter and fault probability from the existing production mechanisms. Do not implement UUIDs from random bytes or change sampling/distributions during extraction. |
| Credentials | Acquire an access token or other externally obtained credential material. Header formatting, canonical signing text, and deterministic HMAC remain ordinary functions over supplied data. |
| HTTP | Send one prepared request through an identified internal client and return a fully buffered response or the existing classified transport failure. |
| Shared state | Read typed snapshots and perform narrowly scoped, atomic updates or reservations. Return data or boundary-owned resource leases, never locks or general-purpose store handles to systems. |
| Runtime and observation | Own required client/task lifecycle operations and external logging or telemetry calls. Host/configuration reads occur here, at the existing initialization or refresh boundary. |

Shared-state methods should retain domain types and scope, for example an
account's routing snapshot or a shard reservation. Do not use untyped keys,
`Any`, generic key-value bags, or one opaque `execute_everything` method to make
the trait look small. Conversely, do not add `retry_operation`, `execute_query`,
or a separate trait method for every composition.

Common compositions live as bare functions in `crate::env`: deadline-aware
waiting, obtaining authorization inputs, metadata refresh coordination,
executing a prepared HTTP attempt, and effect application. Pipeline-specific
retry and routing decisions stay in their own pure systems. The production
implementation delegates to existing runtime, account, cache, credential, and
transport owners rather than copying their state into a second source of truth.

For example, the current jitter helper uses wall time and a process-wide atomic
seed. Move that dependency behind the boundary before considering any change
to its generator. Deterministic scaling of a supplied sample stays pure.

### HTTP is the bottom boundary, not a bypass

The intended call direction is:

```text
Dataflow -> Operation -> Transport
    -> crate::env helpers and Environment::send_http
    -> existing private HTTP backend -> network
```

Routing, retry eligibility, protocol selection, Gateway V2 framing, shard
selection policy, and fault policy remain visible driver logic. Environmental
resource acquisition, reservation, dispatch, and accounting go through `Env`.
The raw send primitive must not call back into the Transport pipeline or
silently introduce retries of its own.

Retain the existing buffered response boundary; do not introduce a separate
virtual instruction for reading each response body. A body-read failure must
still retain its existing classification and request-sent status. Preserve
TLS, proxy, backend-feature, pooling, and internal mocking/fault-injection
behavior. Metadata requests, connectivity probes, credential-provider calls,
and background work must not become alternate paths around the boundary;
third-party providers may retain their own internals behind their adapters.

## Effect processing and state ownership

Put the shared effect-processing facility alongside the environment helpers,
with small typed handlers grouped by owner. All pipelines use this facility;
none maintains a separate, subtly different implementation of shared mutations.
The facility may span modules. Centralization is about authority and call paths,
not putting every match arm into one enormous file.

### Common structures to build

| Structure | Responsibility |
| --- | --- |
| `Env` / `Environment` | A single borrowed capability handle and the production/test implementations of the environmental contract |
| Transition return convention | An action plus ordered, typed effects; a small `Transition<Action, Effect>` record where it removes duplication, without a mandatory system trait |
| Domain effect enums | Reuse `LocationEffect`; add focused update values for session state, diagnostics, recovery, transport accounting, and dataflow progress as needed |
| Shared effect handlers | One application path per owner, callable from every pipeline and background producer; request-local handlers receive only the relevant mutable owner |
| Observation and completion values | Typed snapshots, prepared requests, attempt outcomes, and diagnostic fragments; no callback-based escape hatches |
| Boundary-owned leases | Cancellation-safe ownership of inflight reservations, hedge permits, refresh claims, and task lifetimes |
| Scripted test environment | Controlled observations and completions plus an exact record of requested environmental operations |

Effects can be grouped into a small outer enum at the shared boundary when
needed, but systems should return their narrow domain type rather than import
every pipeline's effects. Do not introduce a universal mutable request context
to service that enum.

There must be one authoritative representation of an update. For example,
either a replacement retry component is carried by a transition's update field
or it is carried by a typed effect; it must not be encoded in both an action
and an independently applied effect.

### Ordering, errors, and cancellation

An effect list is ordered work, not a transaction or an unordered set.
The handler preserves the current point at which each effect becomes visible:
before retry, before returning an error, after a response, or when releasing a
reservation. Do not batch all effects until successful request completion.

Separate local publication, shared updates, and awaited work where their order
matters. For each handler, define what commits before an await, what remains if
it fails, and what is released if its future is dropped. Effects that must
survive an error must not disappear inside a discarded `Result::Err`; apply
them at the established boundary or carry them with the terminal outcome.
Preserve existing best-effort versus propagated-error behavior explicitly,
including its diagnostics/logging, rather than making all effects fallible or
silently ignoring their failures.

Cancellation may prevent any completion value from being returned. Therefore,
cleanup cannot rely on a later `Release` effect being processed. Keep resource
guards inside the environmental boundary, with existing synchronous `Drop`
cleanup and cancellation semantics. Do not equate cancelling an HTTP future
with proving its request was not sent, or with undoing a service write.

Retained plan state must also survive dropping an awaited sub-operation. Do not
move the only copy of that state into a cancellable future or add automatic
rollback over progress that is already committed. The local owner and explicit
commit points must preserve today's behavior at each suspension point.

### Shared-state alternatives

| State | Preferred direction | Alternative and tradeoff |
| --- | --- | --- |
| Request retry, node progress, page buffers | One request/plan owner publishes returned component updates through local handlers. Systems do not receive `&mut` access to retained state. | Replacing whole immutable components can be simpler for small state; large buffers need moves or focused updates to avoid repeated cloning. |
| Routing, partition health, options | Observe immutable snapshots; apply typed intent against the latest shared state behind `Env`. Retain existing CAS/lock ownership. | A single actor or global lock simplifies serialization but changes scheduling, contention, and possibly behavior. It is not part of this refactor. |
| Session vectors and metadata caches | Centralize merge/invalidation/refresh access, retaining scope, refresh coordination, and admission timing. | Copying and writing back an old snapshot can lose another request's update. Moving all caches into one store adds coupling without making policy clearer. |
| Shards, fault counters, hedge budgets | Atomic reservation/accounting inside the boundary; expose immutable facts and keep cleanup guards there. | An immutable counter snapshot alone cannot admit work safely under contention. A new queue or actor would alter existing non-blocking behavior. |
| Diagnostics and recovery flags | Return facts and proposed updates to one owner; aggregate at defined boundaries. Keep shared synchronization only where concurrent producers actually require it. | A mutex around a shared builder preserves convenience but hides who records what and risks duplicate or missing accounting. |

Do not replace current-state updates by writing back a whole store from the
decision snapshot. CAS retries may recompute a pure reducer, but must not repeat
I/O, generate new IDs, or duplicate diagnostics. Observe nondeterministic inputs
outside the retryable reducer at the appropriate existing boundary.

An immutable snapshot gives a stable view to its consumer, not automatically an
atomic view across every store. Do not claim or introduce
stronger cross-store consistency as part of this refactoring. Preserve current
snapshot scopes, generations, refresh points, account isolation, and publication
semantics. Never hold a state lock or epoch guard across network I/O.

## Applying the design to each pipeline

### Transport

Separate attempt preparation, outcome classification, and retry decisions from
time, credentials, client resources, and dispatch. A prepared request is data;
retry systems consume outcome, request-sent status, retry state, options, and
explicit time/sample observations.

Keep shard-selection and health policy inspectable over data while performing
reservation, pool growth, and accounting atomically at the boundary. Retain the
current one-shard connectivity retry, failed-shard exclusion, throttle budgets,
deadline checks, and fault-injection ordering.

The existing inflight guard is finished when the buffered send returns, before
Gateway V2 unwrapping. Moving it outward through decoding would change resource
lifetime. Similarly, do not collapse `NotSent`, `Unknown`, and `Sent` into a
single success/failure flag; write retry safety depends on these distinctions.

### Operation

Keep the ordinary attempt loop as a readable async coordinator. Extract pure
systems around routing, session selection, retry classification, recovery
eligibility, and hedge result selection. Pass the relevant component snapshots,
not a driver or session manager. Produce session, location, recovery, and
diagnostic updates as typed data.

Preserve the main loop's capture of session tokens before result classification,
and its distinction between immediate effects and deferred single-master write
effects. Do not unify this mechanically with hedge handling: hedged session
capture is winner-specific, while completed losing legs can contribute
diagnostics and routing effects. Preserve the current harvest, cancellation,
deadline, and terminal-result rules.

Hedging needs an explicit owner, not a new general scheduler. Each leg owns
its local attempt state and returns completion facts. The parent coordinates
with ordinary futures, selects the result, and routes updates through the same
effect handlers. Any effect that currently becomes visible before a winner is
selected must retain that timing. Preserve metadata hedge admission and
drop-to-release permits; do not serialize the two sends behind a mutable
environment borrow.

PATCH and container-recreation paths use the same update/diagnostic conventions,
but retain their distinct retry safety, precondition, and verification rules.
Uniform structure is not permission to merge different retry policies.

### Dataflow

Separate node state from systems that choose a child, classify a fetched page,
transform rows, advance a cursor, or construct topology replacements. Keep the
owned tree and async child traversal unless a subsequent pipeline plan shows a
clearer equivalent. This plan does not require replacing every node trait with
an enum or flattening the tree into an instruction graph.

Replace the mutable capability-bearing `PipelineContext` with explicit `Env`
access at execution boundaries and data parameters in pure systems. Request
execution still invokes Operation; topology acquisition uses environment helpers.
Pure range resolution and plan construction consume the returned topology data.
Keep the distinction between optional topology identity for logical-key routing
and required topology for physical-range repair.

Return meaningful updates for child replacement, continuation advancement,
buffer acceptance, row emission, poisoning, and deferred errors. A leaf's
`SplitRequired` should describe validated replacement state, not hide live
execution capabilities. Retain the exact-coverage invariant.

Live execution state is not the continuation-token schema. Preserve the existing
token encoding, validation, legacy decoding, scope binding, and supported versus
unsupported resume cases while changing internal ownership. Preserve the
difference between a fetched row and a delivered row: failed encoding cannot
silently advance a resume boundary. Keep partial-page/deferred-error behavior,
poisoning, change-feed ETags and priming, terminal-child handling, ordering ties,
and request-charge/session/diagnostic aggregation.

## Patterns to prefer and avoid

The following signature sketches illustrate the intended dependency direction;
they are not complete implementations.

```text
Avoid:  choose_retry(env, &Driver, &mut Context, result)
Prefer: choose_retry(&RetryState, &AttemptOutcome, &RetryOptions, now, sample)
            -> Transition<RetryAction, RetryEffect>

Avoid:  sign_request(&mut request, credentials)   // secretly reads wall time
Prefer: build_signed_request(request, signing_time, authorization_material)
            -> Result<PreparedRequest>

Avoid:  node.next_page(&mut PipelineContext)     // I/O and arbitrary writes
Prefer: next_page(env, node_owner)              // ordinary async coordination
            -> node outcome and explicit updates
        classify_page(&NodeProgress, &PageFacts)
            -> Transition<PageAction, NodeUpdate>

Avoid:  evaluate_failure(snapshot) -> closure capturing a live state store
Prefer: evaluate_failure(snapshot, failure) -> typed effects
        apply_effects(env, relevant_owner, effects).await

Avoid:  cancel loser; infer NotSent; discard all loser accounting
Prefer: apply the observed leg facts under the existing hedge rules;
        let boundary-owned guards release resources on cancellation
```

Additional anti-patterns are a newtype hiding a mutable god object, a trait
method per business workflow, direct `Instant::now()` or `Uuid::new_v4()` in a
system, and logging macros inside functions described as pure. Obtain
observations through `Env`; produce diagnostic data in systems; perform external
emission through the boundary while preserving existing event semantics.

Moving a long function into an `Environment` implementation without separating
policy has not improved reviewability. Neither has replacing a short async
function with dozens of scheduler states.

## Compatibility and evidence

Behavior includes more than the final response. Preserve request bytes and
headers, routing and retry order, timing policies, deadline scope, credential
refresh, resource lifetimes, cancellation classification, diagnostic content,
errors, page boundaries, and continuation compatibility. Effective options must
remain pinned or refreshed at the same points; environment-variable reads must
not migrate from initialization to every request.

Clock values, IDs, and network race outcomes naturally vary. Compatibility
means preserving their production sources, distributions, observation points,
and permitted scheduling behavior, not demanding equal wall-clock durations
between real runs. With identical scripted observations and completion order,
the old and new paths should produce equivalent observable traces.

Existing tests are the starting regression oracle, not proof of complete
coverage. Keep them, and add characterization where ownership changes expose a
contract not covered by assertions. Do not update expectations merely to make the refactor
pass. Existing shortcomings are separate behavior work: for example, the EPK
split path in `Request` currently documents incomplete aggregation of prior
attempt diagnostics. Do not silently repair that while claiming equivalence.

### Human-readable test coverage

Use named, table-driven scenarios at three levels:

| Level | Inputs | Assertions |
| --- | --- | --- |
| Pure system | Components, observation, and resolved options | Exact action, next-state/update values, ordered effects, and absence of unrelated effects |
| Async boundary | Scripted clock, credentials, HTTP outcomes, shared-state changes, and completion order | Requested work, effect order, resulting state, errors, diagnostics, and cleanup |
| Integrated driver | Existing in-memory emulator, fault injection, protocol adapters, and appropriate recorded/live tiers | End-to-end behavior across the real pipeline composition and compatibility boundaries |

Reuse the existing scenario-catalog approach in
`tests/skip_take_scenario_catalog.rs`, `tests/distinct_scenario_catalog.rs`, and
`tests/streaming_order_by_scenario_catalog.rs`, alongside dataflow resume tests,
retry/hedge unit tests, transport accounting tests, and
`tests/in_memory_emulator_tests/`. A fake environment complements these tests;
it does not replace real transport classification or service evidence.

Each table row should have a stable scenario ID, a plain-language rule, initial
state, input observations, expected result and effects, and the invariant it
exercises. Coverage reporting should map rules to rows and test levels, identify
untested combinations, and explain deliberate exclusions. Line coverage and
scenario counts alone do not explain whether a policy is covered.

Representative cases to carry into that inventory:

| Rule family | Distinctions reviewers should see in rows |
| --- | --- |
| Transport retry | Not-sent versus ambiguous/sent failure; safe versus unsafe retry; throttle count and cumulative delay boundaries; expiry before and during dispatch |
| Shared routing | Stale decision snapshot plus another request's update; immediate versus deferred write effects; failed or cancelled refresh claim |
| Hedging | Primary before threshold; either leg wins; both transient; completed loser versus cancelled loser; deadline race; exhausted admission budget |
| Feed progress | Empty versus terminal page; change-feed 304 with an ETag; split replacement coverage; buffered but not emitted rows |
| Error after progress | Partial page followed by deferred error; encoding/transcoding failure; poisoned continuation and subsequent execution |
| Resume and configuration | Legacy token compatibility; topology change on resume; fresh versus resumed limits; options changed between operations but not within a pinned scope |

The scripted environment must fail on unexpected calls and unused required
expectations, not return convenient defaults. Use controllable futures and
virtual time to exercise cancellation and alternative hedge completion orders.
Observations should be addressable by request/leg identity where concurrency
would make a single global reply queue ambiguous. Do not assert an ordering
between independent requests that production does not guarantee.

### Verifiability

Pure transitions allow property tests and bounded exploration of reachable
states without credentials, real time, or network fixtures. Candidate properties
include no unsafe write retry after an ambiguous send, no lost concurrent
session update, exact split coverage, no resume cursor beyond undelivered rows,
and no leaked reservation after cancellation.

State the assumptions for each property: service continuation semantics,
transport classification, atomic-store operations, and the modeled completion
orders. Small reducers and explicit effect ordering make formal verification
more practical; they do not prove the runtime, service, codecs, or whole driver
correct. Bounded models must state their bounds and need conformance tests
against the production boundary. A violated property discovered during
characterization is a separate defect decision, not authorization to change
behavior in this refactor.

## Value, risks, and follow-on planning

The plan has high value where correctness currently depends on following writes
through several helpers: retry/hedge interaction, shared routing/session
updates, and dataflow progress versus resumability. The benefit is not merely
smaller files. It is a visible separation between facts, decisions, and commits,
with scenario tables that explain which decisions were exercised.

The principal risks are overbuilding `Environment`, hiding policy behind
helpers, changing effect timing, and paying for whole-state copies or excessive
async boxing. Keep the trait's operations concrete, retain native async control
flow, share immutable data, move owned buffers, and preserve existing concurrent
store implementations. Compare point-operation overhead, paging allocations,
and contention using the existing performance tooling when those paths change.

The following choices are settled for subsequent pipeline plans:

- One internal `Env`, passed first where needed; pure functions receive data.
- One shared effect boundary with typed owners, without global serialization.
- Ordinary async orchestration, including explicit hedge races.
- State updates as inspectable values; cancellation cleanup retained inside the
  boundary rather than dependent on returning an effect.
- No new public API, continuation format, transport extension point, retry
  policy, or behavior correction.

The later plans must identify concrete component boundaries and effect commit
points, map every environmental dependency to the shared contract, and document
the retained-state behavior at each await and failure boundary. They must also
decide whether a particular hot component is clearer as a returned replacement
or a focused update, and whether any shared tracker truly needs synchronization.
Those choices require pipeline-local evidence; they do not require another
execution framework.

The refactoring is successful when reviewers can read each system with only its
declared inputs, find a single application path for each effect, and trace an
async request without an interpreter manual. Completion evidence must include
the existing regression suites, mapped decision/interaction scenarios, unchanged
public API exports and feature surfaces, continuation compatibility, and
unchanged observable behavior under controlled environment traces.

Related design references: `specs/0005-operation-and-transport-pipelines.md`,
`specs/0012-feed-operations-and-dataflow.md`,
`specs/0018-diagnostics-contract.md`, `specs/0026-session-consistency.md`,
`adrs/0004-three-tier-execution-pipeline.md`, and
`adrs/0006-internal-http-transport.md`.
