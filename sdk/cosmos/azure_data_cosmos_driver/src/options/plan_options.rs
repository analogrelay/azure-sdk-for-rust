// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Plan-time options for [`CosmosDriver::plan_operation`](crate::driver::CosmosDriver::plan_operation).

/// Default maximum fan-out for a fresh cross-partition operation.
///
/// A plan that would fan out to more than this many leaf request nodes is
/// rejected unless the caller raises [`PlanOptions::max_fan_out`].
pub const DEFAULT_MAX_FAN_OUT: usize = 100;

/// Options that shape how an operation is planned into a dataflow pipeline.
///
/// Unlike [`OperationOptions`](crate::options::OperationOptions), which controls
/// per-request behavior (consistency, routing, retries), `PlanOptions` controls
/// the *shape* of the plan itself. For example, how many partitions a plan
/// may fan out to.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PlanOptions {
    /// Maximum number of leaf request nodes a fresh cross-partition plan may
    /// fan out to.
    ///
    /// Cross-partition operations are expensive by design; an accidental broad
    /// query can span a very large number of physical partitions. When a fresh
    /// plan would exceed this limit, planning fails with
    /// [`CosmosStatus::CLIENT_CROSS_PARTITION_FAN_OUT_EXCEEDED`](crate::error::CosmosStatus::CLIENT_CROSS_PARTITION_FAN_OUT_EXCEEDED).
    /// Resuming from a continuation token does not re-check this limit — the
    /// caller already opted in when the operation was first planned.
    ///
    /// Defaults to [`DEFAULT_MAX_FAN_OUT`].
    pub max_fan_out: usize,
}

impl Default for PlanOptions {
    fn default() -> Self {
        Self {
            max_fan_out: DEFAULT_MAX_FAN_OUT,
        }
    }
}

impl PlanOptions {
    /// Sets the maximum fan-out for a fresh cross-partition plan.
    pub fn with_max_fan_out(mut self, max_fan_out: usize) -> Self {
        self.max_fan_out = max_fan_out;
        self
    }
}
