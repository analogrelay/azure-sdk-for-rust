// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Options for paged reads and queries.

use azure_data_cosmos_driver::models::{MaxItemCountHint, SessionToken};
use azure_data_cosmos_driver::options::OperationOptions;

use crate::feed::ContinuationToken;

/// Options shared by feed-style operations such as paged reads and queries.
///
/// These settings control page size and where iteration resumes.
/// [`QueryOptions`] includes these settings through its [`feed`](QueryOptions::feed)
/// field and also exposes matching convenience setters.
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct FeedOptions {
    /// Maximum number of items the service should return per page.
    ///
    /// `None` leaves page sizing up to the SDK and service defaults. See
    /// [`MaxItemCountHint`] for the explicit values you can send.
    ///
    /// This is a hint to the service, not a client-side guarantee of the page
    /// size returned.
    pub max_item_count: Option<MaxItemCountHint>,

    /// Continuation token from a prior page iterator, used to resume the feed.
    ///
    /// See [`QueryPageIterator::to_continuation_token`](crate::feed::QueryPageIterator::to_continuation_token).
    pub continuation_token: Option<ContinuationToken>,
}

impl FeedOptions {
    /// Sets the maximum number of items the service should return per page.
    ///
    /// Pass [`MaxItemCountHint::Limit`] with a concrete page size, or
    /// [`MaxItemCountHint::ServerDecides`] to let the service choose.
    pub fn with_max_item_count(mut self, max_item_count: MaxItemCountHint) -> Self {
        self.max_item_count = Some(max_item_count);
        self
    }

    /// Sets a continuation token to resume the feed at a previous position.
    pub fn with_continuation_token(mut self, continuation_token: ContinuationToken) -> Self {
        self.continuation_token = Some(continuation_token);
        self
    }
}

/// Options for [`ContainerClient::query_items`](crate::clients::ContainerClient::query_items).
///
/// Use [`operation`](Self::operation) for cross-cutting request settings,
/// [`feed`](Self::feed) for paging, and the metric flags when you want extra
/// diagnostics in each [`QueryFeedPage`](crate::feed::QueryFeedPage).
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct QueryOptions {
    /// Cross-cutting request settings for this query.
    ///
    /// See [`OperationOptions`] for the available settings.
    pub operation: OperationOptions,

    /// Paging settings for this query.
    ///
    /// See [`FeedOptions`].
    pub feed: FeedOptions,

    /// Session token for session-consistent queries.
    pub session_token: Option<SessionToken>,

    /// When `true`, asks the service to include index utilization metrics in each
    /// response page.
    ///
    /// Read them from [`QueryFeedPage::index_metrics`](crate::feed::QueryFeedPage::index_metrics).
    pub populate_index_metrics: Option<bool>,

    /// When `true`, asks the service to include query metrics in each response page.
    ///
    /// Read them from [`QueryFeedPage::query_metrics`](crate::feed::QueryFeedPage::query_metrics).
    pub populate_query_metrics: Option<bool>,
}

impl QueryOptions {
    /// Sets the session token for session-consistent queries.
    pub fn with_session_token(mut self, session_token: impl Into<SessionToken>) -> Self {
        self.session_token = Some(session_token.into());
        self
    }

    /// Sets the cross-cutting request settings for this query.
    pub fn with_operation_options(mut self, operation: OperationOptions) -> Self {
        self.operation = operation;
        self
    }

    /// Sets the paging settings for this query.
    pub fn with_feed_options(mut self, feed: FeedOptions) -> Self {
        self.feed = feed;
        self
    }

    /// Enables or disables index utilization metrics for this query.
    pub fn with_populate_index_metrics(mut self, enable: bool) -> Self {
        self.populate_index_metrics = Some(enable);
        self
    }

    /// Enables or disables query metrics for this query.
    pub fn with_populate_query_metrics(mut self, enable: bool) -> Self {
        self.populate_query_metrics = Some(enable);
        self
    }

    /// Sets the maximum number of items the service should return per page.
    ///
    /// Delegates to [`FeedOptions::with_max_item_count`] on the inner
    /// [`feed`](Self::feed). Pass [`MaxItemCountHint::Limit`] with a concrete
    /// page size, or [`MaxItemCountHint::ServerDecides`] to let the service
    /// choose.
    pub fn with_max_item_count(mut self, max_item_count: MaxItemCountHint) -> Self {
        self.feed = self.feed.with_max_item_count(max_item_count);
        self
    }

    /// Sets a continuation token to resume the query at a previous position.
    ///
    /// Delegates to [`FeedOptions::with_continuation_token`] on the inner
    /// [`feed`](Self::feed).
    pub fn with_continuation_token(mut self, continuation_token: ContinuationToken) -> Self {
        self.feed = self.feed.with_continuation_token(continuation_token);
        self
    }
}
