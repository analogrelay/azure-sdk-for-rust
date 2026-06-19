// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Options for container operations.

use azure_data_cosmos_driver::options::OperationOptions;

use crate::models::ThroughputProperties;

/// Options for [`DatabaseClient::create_container`](crate::clients::DatabaseClient::create_container).
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct CreateContainerOptions {
    /// Cross-cutting request settings for this operation.
    ///
    /// See [`OperationOptions`] for the available settings.
    pub operation: OperationOptions,

    pub(crate) throughput: Option<ThroughputProperties>,
}

impl CreateContainerOptions {
    /// Sets the throughput configuration for the new container.
    pub fn with_throughput(mut self, throughput: ThroughputProperties) -> Self {
        self.throughput = Some(throughput);
        self
    }

    /// Sets the cross-cutting request settings for this operation.
    pub fn with_operation_options(mut self, operation: OperationOptions) -> Self {
        self.operation = operation;
        self
    }
}

/// Options for [`ContainerClient::replace`](crate::clients::ContainerClient::replace).
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct ReplaceContainerOptions {
    /// Cross-cutting request settings for this operation.
    ///
    /// See [`OperationOptions`] for the available settings.
    pub operation: OperationOptions,
}

impl ReplaceContainerOptions {
    /// Sets the cross-cutting request settings for this operation.
    pub fn with_operation_options(mut self, operation: OperationOptions) -> Self {
        self.operation = operation;
        self
    }
}

/// Options for [`ContainerClient::delete`](crate::clients::ContainerClient::delete).
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct DeleteContainerOptions {
    /// Cross-cutting request settings for this operation.
    ///
    /// See [`OperationOptions`] for the available settings.
    pub operation: OperationOptions,
}

impl DeleteContainerOptions {
    /// Sets the cross-cutting request settings for this operation.
    pub fn with_operation_options(mut self, operation: OperationOptions) -> Self {
        self.operation = operation;
        self
    }
}

/// Options for [`ContainerClient::read`](crate::clients::ContainerClient::read).
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct ReadContainerOptions {
    /// Cross-cutting request settings for this operation.
    ///
    /// See [`OperationOptions`] for the available settings.
    pub operation: OperationOptions,
}

impl ReadContainerOptions {
    /// Sets the cross-cutting request settings for this operation.
    pub fn with_operation_options(mut self, operation: OperationOptions) -> Self {
        self.operation = operation;
        self
    }
}

/// Options for [`DatabaseClient::query_containers`](crate::clients::DatabaseClient::query_containers).
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct QueryContainersOptions {
    /// Cross-cutting request settings for this operation.
    ///
    /// See [`OperationOptions`] for the available settings.
    pub operation: OperationOptions,
}

impl QueryContainersOptions {
    /// Sets the cross-cutting request settings for this operation.
    pub fn with_operation_options(mut self, operation: OperationOptions) -> Self {
        self.operation = operation;
        self
    }
}
