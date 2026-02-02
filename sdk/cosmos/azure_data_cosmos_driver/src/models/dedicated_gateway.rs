// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Dedicated gateway options for integrated cache.

use std::time::Duration;

/// Options for requests routed through the Azure Cosmos DB dedicated gateway.
///
/// The dedicated gateway provides integrated caching capabilities.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DedicatedGatewayOptions {
    /// Maximum staleness for cached responses.
    ///
    /// For requests with Eventual or Session consistency, responses from the
    /// integrated cache are guaranteed to be no staler than this duration.
    ///
    /// Staleness is supported at millisecond granularity.
    pub max_integrated_cache_staleness: Option<Duration>,

    /// Whether to bypass the integrated cache for this request.
    pub bypass_integrated_cache: bool,
}

impl DedicatedGatewayOptions {
    /// Creates new dedicated gateway options with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum staleness for cached responses.
    pub fn with_max_integrated_cache_staleness(mut self, staleness: Duration) -> Self {
        self.max_integrated_cache_staleness = Some(staleness);
        self
    }

    /// Sets whether to bypass the integrated cache.
    pub fn with_bypass_integrated_cache(mut self, bypass: bool) -> Self {
        self.bypass_integrated_cache = bypass;
        self
    }
}
