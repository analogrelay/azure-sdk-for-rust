# Task 5: encapsulation and user-facing contract documentation

**Status:** Pending.
**Dependencies:** Task 4.
**Intended commit:** `Encapsulate Cosmos options and document limits`.

Read `tasks/README.md` for path abbreviations, confirmed decisions, commit boundaries, and required documentation/API/validation work.

## Mechanical changes

1. Make SDK `src/options/client.rs::CosmosClientOptions` `pub(crate)` and remove its public re-export. Retain a crate-private import/re-export where needed by `CosmosClientBuilder`. Its helper methods can be crate-private; do not expose a replacement public builder-storage type.
2. Remove SDK public re-exports of `OperationOptionsView`, `ThrottlingRetryOptionsView`, and `ThroughputControlOptionsView`. Use direct driver imports or `pub(crate) use` where SDK internals need them. Keep the driver view types and layered resolution machinery intact.
3. Remove SDK `ConsistencyLevel`, its module/re-export, and dead references. Keep `ReadConsistencyStrategy` and its operational behavior. Do not remove the driver's private wire-level consistency representation.
4. In driver `src/models/mod.rs`, change `SessionToken(pub Cow<'static, str>)` to a private tuple field. Use its existing `new`, `From`, `as_str`, `AsRef<str>`, and `Display`; no new public getter is needed.
5. In driver `src/options/policies.rs`, privatize `ExcludedRegions`'s `Vec<Region>` field. Existing `new`, `with_region`, `FromIterator`, `iter`, `len`, and `is_empty` are sufficient. Replace Native `ExcludedRegions(out)` with collection through `FromIterator`; do not add `Deref<Vec<_>>`, mutable accessors, or a public tuple field substitute.
6. Migrate tuple construction/`.0` access outside the defining modules, including driver PATCH tests and `tests/in_memory_emulator_tests/patch_verification_routing.rs`. Preserve `Some(empty)` as clearing exclusions and `None` as inheritance.
7. Replace the non-doc comments on driver `OperationOptions::custom_headers` with public Rustdoc:
   - Extra headers are for purposes not represented by Cosmos-specific options.
   - Never use this field to set Cosmos-specific HTTP headers. Gateway V2 may ignore them.
   - Use typed SDK options for consistency, session/routing, throughput, and other Cosmos settings.
   Do not reject custom headers at runtime or invent a header denylist.
8. Add user-facing documentation to driver `FeedRange` and `FeedRange::for_partition`, and relevant SDK `FeedScope`/feed-range docs:
   - Serializing/stringifying a logical-partition range retains only effective bounds.
   - Parsing/deserializing it does not restore its partition-key identity or logical routing semantics.
   - Do not persist a single-partition feed range to recreate logical scope later. Preserve the partition key and recreate the scope/range with the current container definition.
   - Use the operation's continuation token to resume an existing feed/query, following that operation's documented scope requirements; a serialized range is not a replacement for a continuation token.
   Do not mention this review, the user requesting the change, driver internals, or a promise to recover the lost marker.
9. Add a regression test demonstrating bounds survive a logical-range serialization round trip while `is_logical_partition()` becomes false. Preserve ordinary non-logical range round trips. Do not change serialization, overlap, or session-token coalescing behavior.

## Acceptance criteria

SDK-only consumers cannot import the removed types or use tuple construction/fields. They can still construct/read tokens and excluded-region sets through the existing methods. Documented range behavior matches tests. Custom headers remain available with the warning and no behavioral change.
