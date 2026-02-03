// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Trigger reference types for Cosmos DB operations.

use std::borrow::Cow;

/// Represents a trigger to be executed before or after an operation.
///
/// Triggers are server-side scripts that can be automatically invoked
/// during create, update, and delete operations on items.
///
/// This type is serialized into request headers to specify which trigger to invoke.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TriggerReference {
    /// The name/id of the trigger to invoke.
    pub name: Cow<'static, str>,
}

impl TriggerReference {
    /// Creates a new trigger reference with the given name.
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self { name: name.into() }
    }
}

impl From<&'static str> for TriggerReference {
    fn from(name: &'static str) -> Self {
        Self::new(name)
    }
}

impl From<String> for TriggerReference {
    fn from(name: String) -> Self {
        Self::new(name)
    }
}
