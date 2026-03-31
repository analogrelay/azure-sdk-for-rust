// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Async runtime abstraction for the Cosmos DB driver.
//!
//! Provides concrete newtype wrappers around runtime-specific primitives
//! (currently tokio), selected at compile time via feature flags.
//!
//! All types in this module are `pub(crate)`.

#[cfg(feature = "tokio")]
mod tokio;

#[cfg(feature = "tokio")]
pub(crate) use tokio::*;
