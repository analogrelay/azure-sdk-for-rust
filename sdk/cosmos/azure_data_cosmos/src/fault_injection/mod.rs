// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Fault injection for testing Cosmos DB client behavior under failure.
//!
//! Use this module to simulate service errors, delays, and custom responses so
//! you can exercise retry logic, failover behavior, and error handling without
//! waiting for real faults.
//!
//! Add one or more [`FaultInjectionRule`] values to
//! [`CosmosClientBuilder::with_fault_injection_rules`](crate::CosmosClientBuilder::with_fault_injection_rules)
//! when you build the client.
//!
//! This module is useful for testing scenarios such as:
//!
//! - handling service errors like `503`, `500`, `429`, and `408`
//! - retry and backoff behavior
//! - regional failover
//! - operation-specific fault handling
//!
//! # Enabling fault injection
//!
//! Fault injection requires the `fault_injection` feature flag:
//!
//! ```toml
//! [dependencies]
//! azure_data_cosmos = { version = "...", features = ["fault_injection"] }
//! ```
//!
//! # Core types
//!
//! - [`FaultInjectionRule`] combines a condition with a result and optional
//!   timing or hit-count limits.
//! - [`FaultInjectionCondition`] decides which requests a rule applies to.
//! - [`FaultInjectionResult`] decides what the matching request should return.
//!
//! # Example
//!
//! ```rust,no_run
//! use azure_data_cosmos::fault_injection::{
//!     FaultInjectionConditionBuilder, FaultInjectionErrorType,
//!     FaultInjectionResultBuilder, FaultInjectionRuleBuilder, FaultOperationType,
//! };
//! use azure_data_cosmos::CosmosClientBuilder;
//! use azure_data_cosmos::AccountReference;
//! use azure_core::credentials::Secret;
//! use std::sync::Arc;
//! use std::time::{Duration, Instant};
//!
//! # async fn doc() {
//! // 1. Define what error to inject
//! let result = FaultInjectionResultBuilder::new()
//!     .with_error(FaultInjectionErrorType::ServiceUnavailable)
//!     .with_delay(Duration::from_millis(100))
//!     .with_probability(1.0)
//!     .build();
//!
//! // 2. Define when to inject it
//! let condition = FaultInjectionConditionBuilder::new()
//!     .with_operation_type(FaultOperationType::ReadItem)
//!     .with_region("West US".into())
//!     .build();
//!
//! // 3. Create a rule with timing constraints
//! let rule = Arc::new(FaultInjectionRuleBuilder::new("region-failover-test", result)
//!     .with_condition(condition)
//!     .with_hit_limit(5)
//!     .with_end_time(Instant::now() + Duration::from_secs(30))
//!     .build());
//!
//! // 4. Create the client with fault injection.
//! let client = CosmosClientBuilder::new()
//!     .with_fault_injection_rules(vec![rule])
//!     .unwrap()
//!     .build(
//!         AccountReference::with_authentication_key(
//!             "https://myaccount.documents.azure.com/".parse().unwrap(),
//!             Secret::new("my_account_key"),
//!         ),
//!         azure_data_cosmos::RoutingStrategy::ProximityTo("East US".into()),
//!     )
//!     .await
//!     .unwrap();
//! # }
//! ```
//!
//! # Rule evaluation
//!
//! Rules are evaluated in the order they were added. The first matching rule
//! is applied. All conditions in a [`FaultInjectionCondition`] must match. If
//! you do not set any conditions, the rule applies to every request.

/// A synthetic response returned by a fault injection rule.
#[doc(inline)]
pub use azure_data_cosmos_driver::fault_injection::CustomResponse;
/// Builds a [`CustomResponse`].
#[doc(inline)]
pub use azure_data_cosmos_driver::fault_injection::CustomResponseBuilder;
/// Describes which requests a fault injection rule applies to.
#[doc(inline)]
pub use azure_data_cosmos_driver::fault_injection::FaultInjectionCondition;
/// Builds a [`FaultInjectionCondition`].
#[doc(inline)]
pub use azure_data_cosmos_driver::fault_injection::FaultInjectionConditionBuilder;
/// The kind of error a fault injection rule should simulate.
#[doc(inline)]
pub use azure_data_cosmos_driver::fault_injection::FaultInjectionErrorType;
/// Describes what a matching fault injection rule should return.
#[doc(inline)]
pub use azure_data_cosmos_driver::fault_injection::FaultInjectionResult;
/// Builds a [`FaultInjectionResult`].
#[doc(inline)]
pub use azure_data_cosmos_driver::fault_injection::FaultInjectionResultBuilder;
/// A fault injection rule.
#[doc(inline)]
pub use azure_data_cosmos_driver::fault_injection::FaultInjectionRule;
/// Builds a [`FaultInjectionRule`].
#[doc(inline)]
pub use azure_data_cosmos_driver::fault_injection::FaultInjectionRuleBuilder;
/// The operation type targeted by a fault injection rule.
#[doc(inline)]
pub use azure_data_cosmos_driver::fault_injection::FaultOperationType;
