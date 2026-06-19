// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Options for transactional batch requests and batch sub-operations.
//!
//! Use [`BatchOptions`] for settings that apply to the whole request passed to
//! [`ContainerClient::execute_transactional_batch`](crate::clients::ContainerClient::execute_transactional_batch).
//! Use the other types in this module for settings on individual batch operations.

use azure_data_cosmos_driver::models::{Precondition, SessionToken};
use azure_data_cosmos_driver::options::OperationOptions;

/// Options for [`ContainerClient::execute_transactional_batch`](crate::clients::ContainerClient::execute_transactional_batch).
///
/// Use this type for settings that apply to the whole batch request. Conditional
/// ETag checks are configured on each sub-operation instead.
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct BatchOptions {
    /// Cross-cutting request settings for the batch request.
    ///
    /// See [`OperationOptions`] for the available settings.
    pub operation: OperationOptions,

    /// Session token to use for session-consistent execution.
    pub session_token: Option<SessionToken>,
}

impl BatchOptions {
    /// Sets the session token for session-consistent execution.
    pub fn with_session_token(mut self, session_token: impl Into<SessionToken>) -> Self {
        self.session_token = Some(session_token.into());
        self
    }

    /// Sets the cross-cutting request settings for the batch request.
    pub fn with_operation_options(mut self, operation: OperationOptions) -> Self {
        self.operation = operation;
        self
    }
}

/// Options for an upsert sub-operation in a transactional batch.
///
/// Supports both [`Precondition::IfMatch`] and [`Precondition::IfNoneMatch`].
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct BatchUpsertOptions {
    /// Conditional ETag check for this upsert operation.
    pub precondition: Option<Precondition>,
}

impl BatchUpsertOptions {
    /// Sets the conditional ETag check for this upsert operation.
    pub fn with_precondition(mut self, precondition: Precondition) -> Self {
        self.precondition = Some(precondition);
        self
    }
}

/// Options for a replace sub-operation in a transactional batch.
///
/// Only [`Precondition::IfMatch`] is applied. Other preconditions are ignored.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct BatchReplaceOptions {
    /// Conditional ETag check for this replace operation.
    pub precondition: Option<Precondition>,
}

impl BatchReplaceOptions {
    /// Sets the conditional ETag check for this replace operation.
    pub fn with_precondition(mut self, precondition: Precondition) -> Self {
        self.precondition = Some(precondition);
        self
    }
}

/// Options for a read sub-operation in a transactional batch.
///
/// Supports both [`Precondition::IfMatch`] and [`Precondition::IfNoneMatch`].
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct BatchReadOptions {
    /// Conditional ETag check for this read operation.
    pub precondition: Option<Precondition>,
}

impl BatchReadOptions {
    /// Sets the conditional ETag check for this read operation.
    pub fn with_precondition(mut self, precondition: Precondition) -> Self {
        self.precondition = Some(precondition);
        self
    }
}

/// Options for a delete sub-operation in a transactional batch.
///
/// Only [`Precondition::IfMatch`] is applied. Other preconditions are ignored.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct BatchDeleteOptions {
    /// Conditional ETag check for this delete operation.
    pub precondition: Option<Precondition>,
}

impl BatchDeleteOptions {
    /// Sets the conditional ETag check for this delete operation.
    pub fn with_precondition(mut self, precondition: Precondition) -> Self {
        self.precondition = Some(precondition);
        self
    }
}
