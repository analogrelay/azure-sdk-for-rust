// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Options for throughput operations.

use azure_data_cosmos_driver::options::OperationOptions;

/// Options for reading or replacing throughput on a database or container.
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct ThroughputOptions {
    /// Cross-cutting request settings for this operation.
    ///
    /// See [`OperationOptions`] for the available settings.
    pub operation: OperationOptions,
}

impl ThroughputOptions {
    /// Sets the cross-cutting request settings for this operation.
    pub fn with_operation_options(mut self, operation: OperationOptions) -> Self {
        self.operation = operation;
        self
    }
}
