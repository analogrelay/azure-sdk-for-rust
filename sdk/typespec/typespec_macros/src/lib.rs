// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use syn::{parse_macro_input, DeriveInput};

extern crate proc_macro;

mod safe_debug;

type Result<T> = ::std::result::Result<T, syn::Error>;

// NOTE: Proc macros must appear in the root of the crate. Just re-exporting them with `pub use` is **not sufficient**.
// So, all the top-level entry functions for the proc macros will appear here, but they just call inner "impl" functions in the modules.

/// Defines the function signature expected by run_derive_macro
type DeriveImpl = fn(DeriveInput) -> Result<proc_macro2::TokenStream>;

/// Runs the provided derive macro implementation, automatically generating errors if it returns errors.
fn run_derive_macro(input: proc_macro::TokenStream, imp: DeriveImpl) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match imp(ast) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Derive macro for implementing the `SafeDebug` trait.
///
/// Deriving this trait will derive a [`std::fmt::Debug`] implementation that should not leak personally identifiable information (PII).
/// By default, only the structure or enumeration name will be returned.
///
/// # Examples
///
/// ```
/// # use typespec_macros::SafeDebug;
/// #[derive(SafeDebug)]
/// struct MyModel {
///     name: Option<String>,
/// }
///
/// let model = MyModel {
///     name: Some("Kelly Smith".to_string()),
/// };
/// assert_eq!(format!("{model:?}"), "MyModel { .. }");
/// ```
#[proc_macro_derive(SafeDebug)]
pub fn derive_safe_debug(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    run_derive_macro(input, safe_debug::derive_safe_debug_impl)
}
