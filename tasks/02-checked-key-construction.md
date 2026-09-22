# Task 2: checked partition keys, EPK parsing, and range construction

**Status:** Pending.
**Dependencies:** Task 1.
**Intended commit:** `Make Cosmos key construction fallible`.

Read `tasks/README.md` for path abbreviations, confirmed decisions, commit boundaries, and required documentation/API/validation work.

## Assessment: checked constructors without an API rewrite

The migration is reasonable for application call sites. Most existing string/integer keys and normal SDK calls remain unchanged. The repository diff will be larger because routing errors must propagate instead of panicking or silently falling back.

The following is an inventory of explicit constructor calls, not an exact edit count. It excludes comments/doc examples, can miss inferred `.into()` conversions and aliases, and classifies emulator/test-support/example code separately. Internal `Self::...` calls also need migration.

| Explicit call family | Production calls / files | Test, support, emulator, example calls / files |
| --- | --- | --- |
| `FeedRange::for_partition` | 5 / 3 | 21 / 10 |
| `EffectivePartitionKey::from` | 13 / 3 | 118 / 18 |
| `EffectivePartitionKey::compute` | 4 / 4 | 22 / 3 |
| Dependent `CosmosOperation` constructors | 16 / 3, including benchmarks | 259 / 35 |

The affected operation constructors are `create_item`, `read_item`, `delete_item`, `upsert_item`, `replace_item`, `patch_item`, `batch`, and `read_all_items`. The SDK's corresponding public operations already return `Result`; retain their signatures.

Do **not** replace every `From` with `TryFrom`. Keep the generic scalar and tuple `From` implementations for components that remain infallible. Add concrete `TryFrom<f32>`, `TryFrom<f64>`, and `TryFrom<Vec<PartitionKeyValue>>` implementations as specified below. A local compiler probe confirmed that these concrete implementations coexist with the retained generic scalar and tuple `From` implementations. A blanket tuple `TryFrom` is not the approach: it would overlap conversions already provided through `From`.

Examples of the intended migration:

```rust
// Unchanged.
let string_key = PartitionKey::from("tenant");
let integer_key = PartitionKey::from(42);
let tuple_key = PartitionKey::from(("tenant", 42));

// Checked inputs.
let float_key = PartitionKey::try_from(12.5_f64)?;
let component = PartitionKeyValue::try_from(12.5_f64)?;
let hierarchical_key = PartitionKey::from(("tenant", component));
let dynamic_key = PartitionKey::try_from(components)?;
let epk = EffectivePartitionKey::try_from(hex_text)?;
let parsed_epk: EffectivePartitionKey = hex_text.parse()?;
let range = FeedRange::for_partition(key, &definition)?;

// Driver construction, not a change to SDK operation signatures.
let operation = CosmosOperation::read_item(item)?;
let operation = CosmosOperation::create_item(item)?.with_body(body);
```

For optional floating-point components, validate first with `value.map(PartitionKeyValue::try_from).transpose()?`, then use the existing `From<Option<PartitionKeyValue>>`. Do not invent a family of generalized tuple/optional conversion traits.

## A. Floating-point and dynamic partition keys

1. In driver `src/models/partition_key.rs`, remove only `f32` and `f64` from `impl_from_number!`.
2. Implement `TryFrom<f32>` and `TryFrom<f64>` for `PartitionKeyValue`, with `Error = CosmosError`. Reject NaN and both infinities before constructing `FiniteF64`. Reuse the current finite-number representation and zero normalization. A checked finite value may use the existing internal `FiniteF64::new_strict`; do not pass unchecked input to it or use lossy conversion to accept invalid input.
3. Implement concrete `TryFrom<f32>` and `TryFrom<f64>` for `PartitionKey`, delegating validation to `PartitionKeyValue`.
4. Replace `From<Vec<PartitionKeyValue>> for PartitionKey` with `TryFrom<Vec<PartitionKeyValue>>`. Preserve vectors of lengths 0 through 3. Return the existing appropriate too-many-components error for length 4 or greater; remove the public-input assertion.
5. Leave integer conversion/rounding behavior, string/Boolean/null/undefined conversion, scalar generic `From`, and two-/three-component tuple `From` unchanged.
6. Migrate all inferred float/vector conversions as well as explicit constructor calls. Start with:
   - Driver `src/driver/dataflow/query_plan.rs`, especially JSON-number conversion.
   - Driver `src/query/local_plan_adapter.rs`.
   - Driver `src/in_memory_emulator/epk.rs`.
   - Native `src/partition_key.rs`.
   - Driver/SDK tests and examples using numeric keys.
7. At the native boundary, retain existing UTF-8, finite-number, arity, and output-handle checks. Translate the checked conversion's error using the native function's established error/status contract; preserve current invalid-input codes. Preserve the native empty-key factory.

## B. Strict EPK parsing

1. In driver `src/models/effective_partition_key.rs`, remove `From<&str>` and `From<String>`.
2. Add `FromStr<Err = CosmosError>`, `TryFrom<&str>`, and `TryFrom<String>`. Delegate all three to one strict implementation based on the existing `try_hex_to_bytes`, rather than adding a second parser.
3. Accept even-length upper/lowercase hex, `""` as MIN, and `"FF"` as MAX. Reject odd length, invalid ASCII, non-ASCII, and malformed suffixes. Do not impose a fixed byte width; existing hierarchical bounds have different valid widths.
4. Make `Deserialize` use the strict parser and `serde::de::Error::custom`. Remove the lenient `hex_to_bytes`; migrate its valid-literal test uses.
5. Preserve EPK `Eq`, `Ord`, hashing, trailing-zero canonicalization, `PartialEq<str>`, and serialized wire spelling. String comparison already uses strict parsing; retain `false` for malformed comparison strings.
6. In `increment_be`, remove bytes-to-hex-to-parse round trips. Use the existing `from_bytes` for the empty successor and incremented buffer.
7. In `src/driver/dataflow/planner.rs`, propagate parse errors from resumed ranges/query bounds rather than dropping a range, substituting MIN/MAX, or changing query scope. Migrate aliases and inferred `.into()` calls exposed by compilation too.

## C. Checked hashing and logical feed ranges

1. Change private driver `EffectivePartitionKey::compute` to return `crate::error::Result<Self>`. Reject the invalid `MultiHash` + `V1` combination explicitly instead of asserting. Preserve valid Hash V1/V2, MultiHash V2, legacy behavior, and internal MIN/MAX sentinel semantics.
2. Make `compute_range` propagate `compute` errors. Retain its current empty-key and arity validation and full-key/prefix range shapes.
3. Change `FeedRange::for_partition(...) -> crate::error::Result<Self>`. Delete the fallback from failed `compute_range` to `compute`; construct the logical representation only on success.
4. Change private `FeedRange::for_item` and `CosmosOperation::for_item` to return `Result`. Change the eight dependent driver constructors listed in the assessment to return `Result<Self>`.
5. In `CosmosOperation::retarget_container`, use `.map(...).transpose()?` for the optional replacement target. Complete fallible target validation before mutating the resource reference, so an error does not leave partially updated routing.
6. In SDK `src/feed/query.rs`, change `FeedScope::into_feed_range` to return SDK `Result<FeedRange>`; wrap already-valid explicit/full ranges in `Ok`. Propagate with `?` from query and change-feed construction in `src/clients/container_client.rs`.
7. Update SDK item/batch operations by inserting `?` immediately after the driver constructor and before `.with_body(...)` or other builder methods. Preserve SDK method names/signatures.
8. Migrate driver hashing callers in `src/driver/dataflow/query_plan.rs`, `src/driver/pipeline/operation_pipeline.rs`, `src/driver/cache/partition_key_range_cache.rs`, and emulator helpers. Follow each new `Result` to an existing fallible boundary. Do not add `unwrap`, `expect`, or success-shaped routing fallbacks in production.
9. Treat cache "not applicable/not found" separately from invalid routing input. An existing optional cache optimization may decline a lookup, but it must not make invalid definitions/keys proceed successfully: the authoritative operation/routing path must return the validation error. Add regression coverage for this distinction.
10. Migrate native `src/feed_range.rs` and `src/op_request.rs`, preserving explicit error translation, initialization/ownership of out parameters, and completion behavior. Update `azure_data_cosmos_benchmarks/src/lib.rs` and all dependent tests/examples.
11. Use `?` in fallible test helpers, or `expect` with a clear valid-fixture assumption in infallible test helpers. Do not change assertions testing invalid inputs into unconditional unwraps. Update doc examples' return signatures/hidden `Ok(())` lines as needed.
12. Leave `PartitionKeyDefinition::new/with_kind/with_version` signatures unchanged. Callers may assemble a definition in stages; invalid combinations are rejected at checked routing/hash boundaries. This avoids a second constructor redesign.

## Acceptance criteria

- Finite floats, negative zero, strings, integers, Boolean, null, undefined, and valid tuples retain their existing encoding/routing behavior.
- NaN/infinities and four-component vectors return errors without panicking; empty-vector construction remains available.
- Strict EPK construction and Serde reject malformed input, including invalid suffixes and non-ASCII, without partial parsing or panics.
- Mixed-width EPK ordering and canonical equality tests still pass.
- Full logical keys and valid MultiHash prefixes produce the same bounds as before.
- Empty logical keys, too many components relative to a definition, non-MultiHash prefixes, and MultiHash/V1 return errors.
- Point, batch, feed, resumed query, and metadata-retarget paths propagate errors without transport submission or partial retarget mutation where construction fails.
- No SDK async operation needs a new public signature solely because of this migration.
