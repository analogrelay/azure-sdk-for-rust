// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Defines conditions for when fault injection rules should be applied.

use super::FaultOperationType;
use crate::options::Region;

/// Filters when a [`FaultInjectionRule`](super::FaultInjectionRule) applies.
///
/// Leave a field unset to match any value for that part of the request.
#[derive(Clone, Default, Debug)]
#[non_exhaustive]
pub struct FaultInjectionCondition {
    operation_type: Option<FaultOperationType>,
    region: Option<Region>,
    container_id: Option<String>,
}

impl FaultInjectionCondition {
    /// Returns the operation type to which the fault injection applies.
    pub fn operation_type(&self) -> Option<FaultOperationType> {
        self.operation_type
    }

    /// Returns the region to which the fault injection applies.
    pub fn region(&self) -> Option<&Region> {
        self.region.as_ref()
    }

    /// Returns the container ID to which the fault injection applies.
    pub fn container_id(&self) -> Option<&str> {
        self.container_id.as_deref()
    }
}

/// Builder for creating a [`FaultInjectionCondition`].
#[derive(Default)]
pub struct FaultInjectionConditionBuilder {
    operation_type: Option<FaultOperationType>,
    region: Option<Region>,
    container_id: Option<String>,
}

impl FaultInjectionConditionBuilder {
    /// Creates a builder with no filters.
    pub fn new() -> Self {
        Self {
            operation_type: None,
            region: None,
            container_id: None,
        }
    }

    /// Sets the operation type to which the fault injection applies.
    pub fn with_operation_type(mut self, operation_type: FaultOperationType) -> Self {
        self.operation_type = Some(operation_type);
        self
    }

    /// Sets the region to which the fault injection applies.
    pub fn with_region(mut self, region: Region) -> Self {
        self.region = Some(region);
        self
    }

    /// Sets the container ID to which the fault injection applies.
    pub fn with_container_id(mut self, container_id: impl Into<String>) -> Self {
        self.container_id = Some(container_id.into());
        self
    }

    /// Builds the [`FaultInjectionCondition`].
    pub fn build(self) -> FaultInjectionCondition {
        FaultInjectionCondition {
            operation_type: self.operation_type,
            region: self.region,
            container_id: self.container_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FaultInjectionConditionBuilder;

    #[test]
    fn builder_default() {
        let builder = FaultInjectionConditionBuilder::default();
        let condition = builder.build();
        assert!(condition.operation_type().is_none());
        assert!(condition.region().is_none());
        assert!(condition.container_id().is_none());
    }
}
