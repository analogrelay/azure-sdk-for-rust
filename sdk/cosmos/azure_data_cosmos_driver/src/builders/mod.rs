// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Builder types for constructing driver instances.
//!
//! # Deprecated
//!
//! The `DriverBuilder` is deprecated. Use [`CosmosDriverRuntime::get_or_create_driver()`]
//! instead for creating driver instances. This ensures proper singleton management.

// This module is kept for backwards compatibility but the preferred way to create
// drivers is now through CosmosDriverRuntime::get_or_create_driver().
