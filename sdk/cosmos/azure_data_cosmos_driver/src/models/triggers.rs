// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Trigger invocation types for Cosmos DB operations.

use std::borrow::Cow;

/// Represents a trigger to be invoked before or after an operation.
///
/// Triggers are server-side scripts that can be automatically invoked
/// during create, update, and delete operations on items.
///
/// This type is serialized into request headers to specify which trigger to invoke.
/// For resource references to trigger definitions, see the resource reference types.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TriggerInvocation {
    /// The name/id of the trigger to invoke.
    pub name: Cow<'static, str>,
}

impl TriggerInvocation {
    /// Creates a new trigger invocation with the given name.
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self { name: name.into() }
    }
}

impl From<&'static str> for TriggerInvocation {
    fn from(name: &'static str) -> Self {
        Self::new(name)
    }
}

impl From<String> for TriggerInvocation {
    fn from(name: String) -> Self {
        Self::new(name)
    }
}
