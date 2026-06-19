// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Options for item reads, writes, and patch operations.

use azure_data_cosmos_driver::models::{Precondition, SessionToken};
use azure_data_cosmos_driver::options::OperationOptions;

/// Options for [`ContainerClient::read_item`](crate::clients::ContainerClient::read_item).
///
/// Use [`operation`](Self::operation) for cross-cutting request settings and
/// [`precondition`](Self::precondition) for conditional reads.
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct ItemReadOptions {
    /// Cross-cutting request settings for this operation.
    ///
    /// See [`OperationOptions`] for the available settings.
    pub operation: OperationOptions,

    /// Session token to use for session-consistent reads.
    pub session_token: Option<SessionToken>,

    /// Conditional ETag check for this read.
    ///
    /// [`Precondition::IfNoneMatch`] is commonly used to avoid returning an item
    /// that has not changed.
    pub precondition: Option<Precondition>,
}

impl ItemReadOptions {
    /// Sets the session token for session-consistent reads.
    pub fn with_session_token(mut self, session_token: impl Into<SessionToken>) -> Self {
        self.session_token = Some(session_token.into());
        self
    }

    /// Sets the conditional ETag check for this read.
    pub fn with_precondition(mut self, precondition: Precondition) -> Self {
        self.precondition = Some(precondition);
        self
    }

    /// Sets the cross-cutting request settings for this operation.
    pub fn with_operation_options(mut self, operation: OperationOptions) -> Self {
        self.operation = operation;
        self
    }
}

/// Options for item write operations.
///
/// Used by [`ContainerClient::create_item`](crate::clients::ContainerClient::create_item),
/// [`ContainerClient::replace_item`](crate::clients::ContainerClient::replace_item),
/// [`ContainerClient::upsert_item`](crate::clients::ContainerClient::upsert_item), and
/// [`ContainerClient::delete_item`](crate::clients::ContainerClient::delete_item).
///
/// Use [`operation`](Self::operation) for cross-cutting request settings and
/// [`precondition`](Self::precondition) for conditional writes.
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct ItemWriteOptions {
    /// Cross-cutting request settings for this operation.
    ///
    /// See [`OperationOptions`] for the available settings.
    pub operation: OperationOptions,

    /// Session token to use for session-consistent writes.
    pub session_token: Option<SessionToken>,

    /// Conditional ETag check for this write.
    ///
    /// [`Precondition::IfMatch`] is commonly used for optimistic concurrency.
    pub precondition: Option<Precondition>,
}

impl ItemWriteOptions {
    /// Sets the session token for session-consistent writes.
    pub fn with_session_token(mut self, session_token: impl Into<SessionToken>) -> Self {
        self.session_token = Some(session_token.into());
        self
    }

    /// Sets the conditional ETag check for this write.
    pub fn with_precondition(mut self, precondition: Precondition) -> Self {
        self.precondition = Some(precondition);
        self
    }

    /// Sets the cross-cutting request settings for this operation.
    pub fn with_operation_options(mut self, operation: OperationOptions) -> Self {
        self.operation = operation;
        self
    }
}

/// Options for [`ContainerClient::patch_item`](crate::clients::ContainerClient::patch_item).
///
/// Patch currently reads the item, applies your [`PatchInstructions`](crate::models::PatchInstructions),
/// and then writes the updated item back with an ETag check. If another writer
/// changes the item first, the SDK retries the operation.
///
/// Use [`max_attempts`](Self::max_attempts) to cap those retries. `None` uses
/// the default limit of 5.
///
/// This type does not expose a [`Precondition`] because the SDK manages the
/// ETag used for retries internally.
///
/// Because each patch includes at least one read and one write, it can take
/// longer than a point read or replace.
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct PatchItemOptions {
    /// Cross-cutting request settings for this operation.
    ///
    /// See [`OperationOptions`] for the available settings.
    pub operation: OperationOptions,

    /// Session token to use for session-consistent patch operations.
    pub session_token: Option<SessionToken>,

    /// Maximum number of retry attempts the SDK makes if the item changes during
    /// the patch.
    ///
    /// `None` uses the default limit of 5.
    pub max_attempts: Option<std::num::NonZeroU8>,
}

impl PatchItemOptions {
    /// Sets the session token for session-consistent patch operations.
    pub fn with_session_token(mut self, session_token: impl Into<SessionToken>) -> Self {
        self.session_token = Some(session_token.into());
        self
    }

    /// Caps how many times the SDK retries the patch if the item changes.
    pub fn with_max_attempts(mut self, max_attempts: std::num::NonZeroU8) -> Self {
        self.max_attempts = Some(max_attempts);
        self
    }

    /// Sets the cross-cutting request settings for this operation.
    pub fn with_operation_options(mut self, operation: OperationOptions) -> Self {
        self.operation = operation;
        self
    }
}
