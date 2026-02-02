// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! ETag types for optimistic concurrency control.

use std::borrow::Cow;

/// An ETag value used for optimistic concurrency control.
///
/// ETags are opaque identifiers representing a specific version of a resource.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ETag(pub Cow<'static, str>);

impl ETag {
    /// Creates a new ETag with the given value.
    pub fn new(value: impl Into<Cow<'static, str>>) -> Self {
        Self(value.into())
    }

    /// Returns the ETag value as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<Cow<'static, str>>> From<T> for ETag {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for ETag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Conditional request options based on ETag values.
///
/// Used for optimistic concurrency control on write operations.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ETagCondition {
    /// If set, the operation succeeds only if the resource's current ETag matches.
    /// Used for "update if unchanged" semantics.
    pub if_match: Option<ETag>,
    /// If set, the operation succeeds only if the resource's current ETag does NOT match.
    /// Used for "create if not exists" or conditional reads.
    pub if_none_match: Option<ETag>,
}

impl ETagCondition {
    /// Creates a new empty ETag condition.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the If-Match condition.
    pub fn with_if_match(mut self, etag: impl Into<ETag>) -> Self {
        self.if_match = Some(etag.into());
        self
    }

    /// Sets the If-None-Match condition.
    pub fn with_if_none_match(mut self, etag: impl Into<ETag>) -> Self {
        self.if_none_match = Some(etag.into());
        self
    }
}
