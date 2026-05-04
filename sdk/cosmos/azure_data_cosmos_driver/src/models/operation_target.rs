// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Partition targeting for [`CosmosOperation`].
//!
//! [`CosmosOperation`]: crate::models::CosmosOperation

use crate::models::PartitionKey;

/// How an operation is targeted to partitions.
///
/// Replaces the `partition_key: Option<PartitionKey>` field on
/// [`CosmosOperation`](crate::models::CosmosOperation). Future variants
/// (e.g. `FeedRange`) will be added when feed operations are introduced.
#[derive(Clone, Debug, Default)]
pub enum OperationTarget {
    /// No partition targeting (account-level or database-level operations,
    /// such as CreateDatabase or ReadContainer).
    #[default]
    None,

    /// Target a specific logical partition key.
    ///
    /// Used for point operations (read, create, delete, upsert, replace)
    /// and single-partition feed operations where the raw partition key
    /// value must be included in the request headers.
    PartitionKey(PartitionKey),
}

impl OperationTarget {
    /// Returns the partition key if this target carries one.
    pub fn as_partition_key(&self) -> Option<&PartitionKey> {
        match self {
            OperationTarget::None => None,
            OperationTarget::PartitionKey(pk) => Some(pk),
        }
    }
}
