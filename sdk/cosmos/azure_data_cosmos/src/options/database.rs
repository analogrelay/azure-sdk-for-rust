// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Options for database operations.

use azure_data_cosmos_driver::options::OperationOptions;

/// Options for [`CosmosClient::create_database`](crate::CosmosClient::create_database).
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct CreateDatabaseOptions {
    /// Cross-cutting request settings for this operation.
    ///
    /// See [`OperationOptions`] for the available settings.
    pub operation: OperationOptions,
}

impl CreateDatabaseOptions {
    /// Sets the cross-cutting request settings for this operation.
    pub fn with_operation_options(mut self, operation: OperationOptions) -> Self {
        self.operation = operation;
        self
    }
}

/// Options for [`DatabaseClient::delete`](crate::clients::DatabaseClient::delete).
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct DeleteDatabaseOptions {
    /// Cross-cutting request settings for this operation.
    ///
    /// See [`OperationOptions`] for the available settings.
    pub operation: OperationOptions,
}

impl DeleteDatabaseOptions {
    /// Sets the cross-cutting request settings for this operation.
    pub fn with_operation_options(mut self, operation: OperationOptions) -> Self {
        self.operation = operation;
        self
    }
}

/// Options for [`DatabaseClient::read`](crate::clients::DatabaseClient::read).
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct ReadDatabaseOptions {
    /// Cross-cutting request settings for this operation.
    ///
    /// See [`OperationOptions`] for the available settings.
    pub operation: OperationOptions,
}

impl ReadDatabaseOptions {
    /// Sets the cross-cutting request settings for this operation.
    pub fn with_operation_options(mut self, operation: OperationOptions) -> Self {
        self.operation = operation;
        self
    }
}

/// Options for [`CosmosClient::query_databases`](crate::CosmosClient::query_databases).
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct QueryDatabasesOptions {
    /// Cross-cutting request settings for this operation.
    ///
    /// See [`OperationOptions`] for the available settings.
    pub operation: OperationOptions,
}

impl QueryDatabasesOptions {
    /// Sets the cross-cutting request settings for this operation.
    pub fn with_operation_options(mut self, operation: OperationOptions) -> Self {
        self.operation = operation;
        self
    }
}
