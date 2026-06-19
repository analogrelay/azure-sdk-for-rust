// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Options for feed range lookup operations.

use azure_data_cosmos_driver::options::OperationOptions;

/// Options for [`ContainerClient::read_feed_ranges`](crate::clients::ContainerClient::read_feed_ranges)
/// and [`ContainerClient::feed_range_from_partition_key`](crate::clients::ContainerClient::feed_range_from_partition_key).
#[derive(Clone, Default, Debug)]
#[non_exhaustive]
pub struct ReadFeedRangesOptions {
    /// Cross-cutting request settings for this operation.
    ///
    /// See [`OperationOptions`] for the available settings.
    pub operation: OperationOptions,

    force_refresh: bool,
}

impl ReadFeedRangesOptions {
    /// When `true`, refreshes cached partition range metadata before resolving feed ranges.
    pub fn with_force_refresh(mut self, force_refresh: bool) -> Self {
        self.force_refresh = force_refresh;
        self
    }

    /// Sets the cross-cutting request settings for this operation.
    pub fn with_operation_options(mut self, operation: OperationOptions) -> Self {
        self.operation = operation;
        self
    }

    pub(crate) fn force_refresh(&self) -> bool {
        self.force_refresh
    }
}
