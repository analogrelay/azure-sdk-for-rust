// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Session token types for session consistency.

use std::borrow::Cow;

/// A session token for maintaining session consistency.
///
/// Session tokens track the logical sequence number of operations, enabling
/// read-your-writes consistency within a session.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SessionToken(pub Cow<'static, str>);

impl SessionToken {
    /// Creates a new session token with the given value.
    pub fn new(value: impl Into<Cow<'static, str>>) -> Self {
        Self(value.into())
    }

    /// Returns the session token value as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<Cow<'static, str>>> From<T> for SessionToken {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for SessionToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
