// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Consistency levels reported for a Cosmos DB account.

use std::fmt::{self, Display};

/// Represents the consistency level configured on a Cosmos DB account.
///
/// For per-request consistency settings, use [`ReadConsistencyStrategy`](crate::options::ReadConsistencyStrategy)
/// through [`OperationOptions`](crate::options::OperationOptions).
///
/// Learn more at [Consistency Levels](https://learn.microsoft.com/azure/cosmos-db/consistency-levels).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ConsistencyLevel {
    /// Reads preserve write order but may lag behind the latest write.
    ConsistentPrefix,
    /// Reads may lag behind and may not preserve the latest write order.
    Eventual,
    /// Reads are consistent within a client session.
    Session,
    /// Reads may lag behind the latest write within a configured bound.
    BoundedStaleness,
    /// Reads always reflect the latest committed write.
    Strong,
}

impl Display for ConsistencyLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            ConsistencyLevel::ConsistentPrefix => "ConsistentPrefix",
            ConsistencyLevel::Eventual => "Eventual",
            ConsistencyLevel::Session => "Session",
            ConsistencyLevel::BoundedStaleness => "BoundedStaleness",
            ConsistencyLevel::Strong => "Strong",
        };
        write!(f, "{}", value)
    }
}
