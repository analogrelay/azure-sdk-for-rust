// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Region ordering strategies for Azure Cosmos DB clients.

use super::Region;

/// Controls how the SDK orders Azure regions when routing requests.
///
/// Pass a value of this type when building a [`CosmosClient`](crate::CosmosClient).
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum RoutingStrategy {
    /// Starts with the Azure region nearest the given region and then falls back
    /// to other regions based on the SDK's built-in proximity list.
    ///
    /// If the region name is not recognized, the SDK falls back to the account's
    /// region order.
    ProximityTo(Region),

    /// Tries the listed regions in the order you provide.
    ///
    /// This order does not prevent the SDK from trying other account regions after
    /// the preferred list is exhausted. To keep requests out of specific regions,
    /// use [`OperationOptions::excluded_regions`](crate::options::OperationOptions::excluded_regions).
    PreferredRegions(Vec<Region>),
}
