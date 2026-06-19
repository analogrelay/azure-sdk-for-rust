// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Conditional request types built on top of [`Etag`].

use azure_core::http::Etag;

/// A conditional request based on an [`Etag`].
///
/// Use [`Precondition::IfMatch`] for "update if unchanged" behavior and
/// [`Precondition::IfNoneMatch`] for "only if missing" or "only if changed"
/// behavior, depending on the ETag value you provide.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Precondition {
    /// Operation succeeds only if the resource's current ETag matches.
    ///
    /// Used for "update if unchanged" semantics (optimistic concurrency).
    IfMatch(Etag),

    /// Operation succeeds only if the resource's current ETag does NOT match.
    ///
    /// Use `Etag::from("*")` for "create if not exists" semantics.
    IfNoneMatch(Etag),
}

impl Precondition {
    /// Creates an [`IfMatch`](Self::IfMatch) condition.
    pub fn if_match(etag: impl Into<Etag>) -> Self {
        Self::IfMatch(etag.into())
    }

    /// Creates an [`IfNoneMatch`](Self::IfNoneMatch) condition.
    ///
    /// Use `"*"` for "create if not exists" behavior.
    pub fn if_none_match(etag: impl Into<Etag>) -> Self {
        Self::IfNoneMatch(etag.into())
    }

    /// Returns the ETag if this is an If-Match condition.
    pub fn as_if_match(&self) -> Option<&Etag> {
        match self {
            Self::IfMatch(etag) => Some(etag),
            Self::IfNoneMatch(_) => None,
        }
    }

    /// Returns the ETag if this is an If-None-Match condition.
    pub fn as_if_none_match(&self) -> Option<&Etag> {
        match self {
            Self::IfNoneMatch(etag) => Some(etag),
            Self::IfMatch(_) => None,
        }
    }

    /// Returns `true` if this is an If-Match condition.
    pub fn is_if_match(&self) -> bool {
        matches!(self, Self::IfMatch(_))
    }

    /// Returns `true` if this is an If-None-Match condition.
    pub fn is_if_none_match(&self) -> bool {
        matches!(self, Self::IfNoneMatch(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn if_match_accessors() {
        let etag = Etag::from("abc123");
        let condition = Precondition::if_match(etag.clone());

        assert!(condition.is_if_match());
        assert!(!condition.is_if_none_match());
        assert_eq!(condition.as_if_match(), Some(&etag));
        assert_eq!(condition.as_if_none_match(), None);
    }

    #[test]
    fn if_none_match_accessors() {
        let etag = Etag::from("*");
        let condition = Precondition::if_none_match(etag.clone());

        assert!(!condition.is_if_match());
        assert!(condition.is_if_none_match());
        assert_eq!(condition.as_if_match(), None);
        assert_eq!(condition.as_if_none_match(), Some(&etag));
    }
}
