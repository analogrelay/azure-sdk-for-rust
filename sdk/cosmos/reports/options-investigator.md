# Cosmos Native `options/` Investigation

Scope: `sdk/cosmos/azure_data_cosmos_native/src/options/mod.rs`, plus call sites in `src/clients/*.rs` and the generated C header `include/azurecosmos.h`.

## Option types in `src/options/mod.rs`

| Rust type | C header type | Layout | Current behavior |
|---|---|---|---|
| `ClientOptions` | `cosmos_client_options` | `#[repr(C)]` with `danger_allow_invalid_certificates: bool` (non-wasm only) | The only options type that is actually converted and applied. |
| `QueryOptions` | `cosmos_query_options` | `#[repr(C)]` with `_unused: u8` placeholder | Accepted by query APIs but currently ignored. |
| `CreateDatabaseOptions` | `cosmos_create_database_options` | `#[repr(C)]` with `_unused: u8` placeholder | Accepted by create-database API but currently ignored. |
| `ReadDatabaseOptions` | `cosmos_read_database_options` | `#[repr(C)]` with `_unused: u8` placeholder | Accepted by read-database API but currently ignored. |
| `DeleteDatabaseOptions` | `cosmos_delete_database_options` | `#[repr(C)]` with `_unused: u8` placeholder | Accepted by delete-database API but currently ignored. |
| `CreateContainerOptions` | `cosmos_create_container_options` | `#[repr(C)]` with `_unused: u8` placeholder | Accepted by create-container API but currently ignored. |
| `ReadContainerOptions` | `cosmos_read_container_options` | `#[repr(C)]` with `_unused: u8` placeholder | Accepted by read-container API but currently ignored. |
| `DeleteContainerOptions` | `cosmos_delete_container_options` | `#[repr(C)]` with `_unused: u8` placeholder | Accepted by delete-container API but currently ignored. |
| `ItemOptions` | `cosmos_item_options` | `#[repr(C)]` with `_unused: u8` placeholder | Accepted by item CRUD APIs but currently ignored. |

The `_unused` field comment explicitly says it exists only because empty C structs are non-standard and may be removed later.

## Layout + FFI implications

- All option structs are `#[repr(C)]`, so field order/layout is C-compatible.
- Placeholder option structs are intentionally non-empty (`u8`) to avoid empty-struct ABI hazards in C.
- `ClientOptions` is the only struct with a real setting today.
- On `target_family = "wasm"`, `ClientOptions.danger_allow_invalid_certificates` is compiled out; conversion ignores the input and returns defaults.

## Ownership model across FFI

- Option parameters are passed as borrowed `*const ...Options` pointers (nullable).
- No ownership is transferred to Rust; there are no corresponding `*_options_free` APIs.
- For **`ClientOptions` only**, non-null pointers are dereferenced and converted (`convert_optional_ptr` + `TryFrom<&ClientOptions>`), so pointer validity must hold for the call.
- For placeholder options, parameters are present for forward compatibility but marked/used as reserved and currently not dereferenced in call sites.

## Defaults and effective runtime behavior

### Client options defaulting/adaptation

`impl TryFrom<&ClientOptions> for azure_data_cosmos::CosmosClientOptions` is the only conversion in this module:

- If `danger_allow_invalid_certificates == false`: returns `CosmosClientOptions::default()`.
- If `true` (non-wasm):
  - builds a `reqwest::Client` with `danger_accept_invalid_certs(true)`,
  - wraps it into `azure_core::http::Transport`,
  - returns `CosmosClientOptions` with that transport override and remaining fields defaulted.
- If `true` but `reqwest` feature is disabled: it panics with `at least one HTTP transport feature must be enabled`.
- On wasm target: always returns default options.

This conversion is wired into:

- `cosmos_client_create_with_key`
- `cosmos_client_create_with_connection_string`

Both call `convert_optional_ptr(options)?` and pass `Option<CosmosClientOptions>` into `azure_data_cosmos::CosmosClient` constructors.

### All other options today

Even though these pointers appear in exported C APIs, call sites currently pass `None` to the underlying Rust SDK operations (`read`, `delete`, `create_*`, `query_*`, `*_item`), so behavior is effectively SDK defaults:

- `QueryOptions`: query APIs on client/database/container
- `CreateDatabaseOptions`: create database
- `ReadDatabaseOptions`, `DeleteDatabaseOptions`: database read/delete
- `CreateContainerOptions`, `ReadContainerOptions`, `DeleteContainerOptions`: container management
- `ItemOptions`: item CRUD

## Conversion/adaptation roles summary

1. **Active adapter**: `ClientOptions -> CosmosClientOptions` (real policy adaptation into transport config).
2. **ABI placeholders**: the other eight option structs currently act as stable signature slots for future per-operation configuration, without current runtime adaptation.
