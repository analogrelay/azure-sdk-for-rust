// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Plan executor: runs an [`OperationPlan`] and produces a single page of
//! results.
//!
//! Phase 1 only handles single-node plans, which delegate directly to
//! [`execute_single_operation`] without any additional async machinery.

use crate::diagnostics::DiagnosticsContextBuilder;
use crate::driver::pipeline::operation_pipeline::{execute_single_operation, OperationContext};
use crate::driver::plan::{OperationPlan, PlanNode};
use crate::models::CosmosResponse;

/// Bundle of context passed to the plan executor for a single
/// [`PlanExecutor::execute`] call.
///
/// Today this is a thin wrapper around an [`OperationContext`] — the
/// executor needs everything required to execute a single Cosmos
/// operation, plus (in future phases) executor-only state such as PK
/// range cache handles and query plan information.
pub(crate) struct PlanExecutionContext<'a> {
    pub operation_context: OperationContext<'a>,
}

/// Runs operation plans.
pub(crate) struct PlanExecutor;

impl PlanExecutor {
    /// Executes a plan and returns a single [`CosmosResponse`].
    ///
    /// Phase 1: only [`OperationPlan::SingleNode`] is supported, which
    /// delegates straight to [`execute_single_operation`].
    pub(crate) async fn execute(
        plan: OperationPlan,
        ctx: PlanExecutionContext<'_>,
        diagnostics: DiagnosticsContextBuilder,
    ) -> azure_core::Result<CosmosResponse> {
        match plan {
            OperationPlan::SingleNode(node) => execute_single_node(node, ctx, diagnostics).await,
        }
    }
}

async fn execute_single_node(
    node: PlanNode,
    ctx: PlanExecutionContext<'_>,
    diagnostics: DiagnosticsContextBuilder,
) -> azure_core::Result<CosmosResponse> {
    let PlanNode::Request { operation } = node;
    let credential = operation.resource_reference().account().auth();
    execute_single_operation(&operation, credential, &ctx.operation_context, diagnostics).await
}
