// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Throughput control group name type.

use std::borrow::Cow;

/// Unique name identifying a throughput control group.
///
/// This name is serialized into request headers when referencing a control group.
/// The group configuration is defined separately via [`ThroughputControlGroupOptions`](crate::options::ThroughputControlGroupOptions).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ThroughputControlGroupName(pub Cow<'static, str>);

impl ThroughputControlGroupName {
    /// Creates a new throughput control group name.
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self(name.into())
    }

    /// Returns the name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for ThroughputControlGroupName {
    fn from(name: &'static str) -> Self {
        Self::new(name)
    }
}

impl From<String> for ThroughputControlGroupName {
    fn from(name: String) -> Self {
        Self::new(name)
    }
}

impl From<Cow<'static, str>> for ThroughputControlGroupName {
    fn from(name: Cow<'static, str>) -> Self {
        Self::new(name)
    }
}

impl std::fmt::Display for ThroughputControlGroupName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
