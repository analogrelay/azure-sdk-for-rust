// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Trigger types for Cosmos DB operations.

use std::borrow::Cow;

/// Represents a trigger to be executed before or after an operation.
///
/// Triggers are server-side scripts that can be automatically invoked
/// during create, update, and delete operations on items.
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

impl<T: Into<Cow<'static, str>>> From<T> for TriggerReference {
    fn from(name: T) -> Self {
        Self::new(name)
    }
}

/// Collection of triggers to include in a request.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TriggerOptions {
    /// Triggers to execute before the operation.
    pub pre_triggers: Vec<TriggerReference>,
    /// Triggers to execute after the operation.
    pub post_triggers: Vec<TriggerReference>,
}

impl TriggerOptions {
    /// Creates a new empty trigger options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a pre-trigger to execute before the operation.
    #[must_use]
    pub fn with_pre_trigger(mut self, trigger: impl Into<TriggerReference>) -> Self {
        self.pre_triggers.push(trigger.into());
        self
    }

    /// Adds a post-trigger to execute after the operation.
    #[must_use]
    pub fn with_post_trigger(mut self, trigger: impl Into<TriggerReference>) -> Self {
        self.post_triggers.push(trigger.into());
        self
    }
}
