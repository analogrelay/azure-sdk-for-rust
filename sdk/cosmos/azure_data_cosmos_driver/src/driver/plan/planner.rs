// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Planner: turns a [`CosmosOperation`] into an [`OperationPlan`].
//!
//! Phase 1 only handles point operations, so planning is synchronous and
//! trivial: the operation is wrapped in a single
//! [`PlanNode::Request`](super::PlanNode::Request).

use crate::driver::plan::{OperationPlan, PlanNode};
use crate::models::CosmosOperation;

/// Builds [`OperationPlan`]s from operations.
pub(crate) struct Planner;

impl Planner {
    /// Builds a plan for the given operation.
    ///
    /// Phase 1: always produces a single-node `Request` plan. When feed
    /// operations are introduced, this will inspect `operation.target()` and
    /// `operation.operation_type()` to decide between trivial and multi-node
    /// plans (and will become `async` to allow PK range cache lookups).
    pub(crate) fn plan(operation: CosmosOperation) -> OperationPlan {
        OperationPlan::SingleNode(PlanNode::Request { operation })
    }
}
