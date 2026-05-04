// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Plan model: [`OperationPlan`] and [`PlanNode`].
//!
//! Phase 1 carries a single node variant ([`PlanNode::Request`]) wrapped in a
//! single plan variant ([`OperationPlan::SingleNode`]). Future phases will add
//! a multi-node `Graph` plan variant and additional node kinds (sequential
//! drain, merges, ...).

use crate::models::CosmosOperation;

/// A plan for executing an operation.
///
/// In Phase 1 only the [`SingleNode`](OperationPlan::SingleNode) variant
/// exists, which is used for both point operations and the (future)
/// trivial single-partition feed cases.
pub(crate) enum OperationPlan {
    /// A single-node plan, stored inline. No heap allocation.
    SingleNode(PlanNode),
}

/// A node in an operation plan.
///
/// In Phase 1 only the [`Request`](PlanNode::Request) variant exists.
/// Composite nodes (sequential drain, merges, ...) will be added when feed
/// operations land.
///
/// Per-call options and other composite-wide invariants (the resolved
/// [`OperationOptionsView`](crate::options::OperationOptionsView), throughput
/// control snapshot, custom headers) are *not* stored on the node — they
/// belong to the executor context because they are constant for the whole
/// composite operation.
pub(crate) enum PlanNode {
    /// Execute a single Cosmos request via the operation pipeline.
    ///
    /// Holds the operation inline. When multi-node plans are introduced,
    /// `operation` will become `Arc<CosmosOperation>` so sibling `Request`
    /// nodes can share the base operation without cloning headers and
    /// resource references.
    Request { operation: CosmosOperation },
}
