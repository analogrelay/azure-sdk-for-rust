// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Operation planning and execution.
//!
//! Every call to [`CosmosDriver::execute_operation`] flows through this module:
//!
//! 1. The [`Planner`] turns a [`CosmosOperation`] into an [`OperationPlan`].
//! 2. The [`PlanExecutor`] runs the plan and produces a single
//!    [`CosmosResponse`].
//!
//! In Phase 1 the only supported plan is a single [`PlanNode::Request`]
//! wrapped in [`OperationPlan::SingleNode`], which delegates straight to
//! `execute_single_operation`. Multi-node plans (sequential drain, merges,
//! ...) will be added in follow-up phases.
//!
//! [`CosmosDriver::execute_operation`]: crate::driver::CosmosDriver::execute_operation
//! [`CosmosOperation`]: crate::models::CosmosOperation
//! [`CosmosResponse`]: crate::models::CosmosResponse

mod executor;
mod planner;
mod types;

pub(crate) use executor::{PlanExecutionContext, PlanExecutor};
pub(crate) use planner::Planner;
pub(crate) use types::{OperationPlan, PlanNode};
