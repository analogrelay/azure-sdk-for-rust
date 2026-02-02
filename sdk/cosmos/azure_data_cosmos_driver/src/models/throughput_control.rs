// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Throughput control types for rate limiting.

use crate::models::PriorityLevel;
use std::borrow::Cow;

/// Unique name identifying a throughput control group.
///
/// This name is used both when defining a group and when referencing it in request options.
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

impl<T: Into<Cow<'static, str>>> From<T> for ThroughputControlGroupName {
    fn from(name: T) -> Self {
        Self::new(name)
    }
}

impl std::fmt::Display for ThroughputControlGroupName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Specifies the throughput target for a control group.
///
/// Either an absolute RU/s value or a percentage threshold of the provisioned throughput.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum ThroughputTarget {
    /// Absolute throughput limit in Request Units per second.
    Absolute(u32),
    /// Percentage threshold of provisioned throughput (0.0 to 1.0].
    Threshold(f64),
}

/// Configuration for a throughput control group.
///
/// Registered at the environment level and associated with a container.
/// Throughput control can be enforced either client-side or server-side,
/// and these modes are mutually exclusive.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ThroughputControlGroupOptions {
    /// Client-side enforced throughput control.
    ///
    /// The SDK enforces the throughput limits locally before sending requests.
    ClientSide {
        /// Unique name identifying this control group.
        name: ThroughputControlGroupName,
        /// Target throughput limit (required).
        target_throughput: ThroughputTarget,
        /// Optional priority level for throttling decisions.
        priority_level: Option<PriorityLevel>,
        /// Whether this group is used by default for requests without explicit assignment.
        is_default: bool,
    },

    /// Server-side enforced throughput control using throughput buckets.
    ///
    /// The Cosmos DB service enforces the throughput limits.
    /// See <https://learn.microsoft.com/azure/cosmos-db/nosql/throughput-buckets>
    ServerSide {
        /// Unique name identifying this control group.
        name: ThroughputControlGroupName,
        /// Throughput bucket assignment.
        throughput_bucket: u32,
        /// Whether this group is used by default for requests without explicit assignment.
        is_default: bool,
    },
}

impl ThroughputControlGroupOptions {
    /// Returns the name of the throughput control group.
    pub fn name(&self) -> &ThroughputControlGroupName {
        match self {
            Self::ClientSide { name, .. } => name,
            Self::ServerSide { name, .. } => name,
        }
    }

    /// Returns whether this group is the default.
    pub fn is_default(&self) -> bool {
        match self {
            Self::ClientSide { is_default, .. } => *is_default,
            Self::ServerSide { is_default, .. } => *is_default,
        }
    }
}
