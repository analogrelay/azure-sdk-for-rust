// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Fault injection framework for testing Cosmos DB client behavior under error conditions.
//!
//! This module provides a fault injection framework that intercepts HTTP requests at the
//! transport layer, below the retry policy. When a fault is injected, it triggers the same
//! retry and failover behavior as a real service error. This enables testing of:
//!
//! - CosmosError handling for various HTTP status codes (503, 500, 429, 408, etc.)
//! - Retry logic and backoff behavior
//! - Regional failover scenarios
//! - Operation-specific error handling
//!
//! # Core Components
//!
//! - [`FaultInjectionCondition`] — Defines when a fault should be applied, filtering by
//!   operation type, region, or container ID.
//! - [`FaultInjectionResult`] — Defines what error to inject, including error type, delay,
//!   and probability.
//! - [`FaultInjectionRule`] — Combines a condition with a result and additional controls
//!   like timing windows (`start_time`/`end_time`), `hit_limit`, and `probability`.
//! - [`FaultClient`] — A `TransportClient`
//!   implementation that evaluates rules and injects faults.
//! - `FaultInjectingHttpClientFactory` — An `HttpClientFactory`
//!   decorator that wraps created clients with fault injection.

mod condition;
mod evaluation;
mod fault_injecting_factory;
mod http_client;
mod result;
mod rule;

use std::fmt;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use crate::models::{OperationType, ResourceType};

pub use condition::{FaultInjectionCondition, FaultInjectionConditionBuilder};
pub use evaluation::FaultInjectionEvaluation;
pub(crate) use fault_injecting_factory::FaultInjectingHttpClientFactory;
pub use http_client::FaultClient;
pub use result::{
    CustomResponse, CustomResponseBuilder, FaultInjectionResult, FaultInjectionResultBuilder,
};
pub use rule::{FaultInjectionRule, FaultInjectionRuleBuilder};

/// Shared collector for fault injection evaluations.
///
/// Created by the transport pipeline and attached to [`HttpRequest`](crate::driver::transport::cosmos_transport_client::HttpRequest).
/// [`FaultClient`] writes evaluations into the collector during `send()`, and
/// the transport pipeline reads them after the request completes.
#[derive(Clone, Debug, Default)]
pub(crate) struct EvaluationCollector(Arc<Mutex<Vec<FaultInjectionEvaluation>>>);

impl EvaluationCollector {
    /// Appends all evaluations from `evals` into the collector, draining the source.
    pub fn push_all(&self, evals: &mut Vec<FaultInjectionEvaluation>) {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .append(evals);
    }

    /// Takes all collected evaluations, leaving the collector empty.
    pub fn take(self) -> Vec<FaultInjectionEvaluation> {
        match Arc::try_unwrap(self.0) {
            Ok(mutex) => mutex.into_inner().unwrap_or_else(|e| e.into_inner()),
            Err(arc) => {
                let mut evaluations = arc.lock().unwrap_or_else(|e| e.into_inner());
                std::mem::take(&mut *evaluations)
            }
        }
    }
}

/// The error condition a fault injection rule can simulate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FaultInjectionErrorType {
    /// Simulates an HTTP 500 Internal Server Error response.
    InternalServerError,
    /// Simulates an HTTP 429 Too Many Requests response.
    TooManyRequests,
    /// Simulates `404 / 1002`, which maps to read session not available.
    ReadSessionNotAvailable,
    /// Simulates an HTTP 408 Request Timeout response.
    Timeout,
    /// Simulates an HTTP 503 Service Unavailable response.
    ServiceUnavailable,
    /// Simulates `410 / 1002`, which maps to partition key range gone.
    PartitionIsGone,
    /// Simulates `403 / 3`, which indicates writes are not allowed in the current region.
    WriteForbidden,
    /// Simulates `403 / 1008`, which indicates the account is no longer owned by the current region.
    DatabaseAccountNotFound,
    /// Simulates a connection failure, such as connection refusal or name resolution failure.
    ///
    /// This produces [`crate::error::CosmosStatus::TRANSPORT_CONNECTION_FAILED`]
    /// instead of an HTTP response.
    ConnectionError,
    /// Simulates a timeout after the request is sent but before a response is received.
    ///
    /// This produces [`crate::error::CosmosStatus::TRANSPORT_IO_FAILED`]
    /// instead of an HTTP response.
    ResponseTimeout,
}

/// The request operation that a fault injection rule can target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FaultOperationType {
    /// Reads an item.
    ReadItem,
    /// Queries items.
    QueryItem,
    /// Creates an item.
    CreateItem,
    /// Upserts an item.
    UpsertItem,
    /// Replaces an item.
    ReplaceItem,
    /// Deletes an item.
    DeleteItem,
    /// Patches an item.
    PatchItem,
    /// Executes a batch operation.
    BatchItem,
    /// Reads change feed items.
    ChangeFeedItem,
    /// Reads container metadata.
    MetadataReadContainer,
    /// Reads account metadata.
    MetadataReadDatabaseAccount,
    /// Requests a query plan.
    MetadataQueryPlan,
    /// Reads partition key range metadata.
    MetadataPartitionKeyRanges,
}

impl FaultOperationType {
    /// Returns the wire-format name for this operation type.
    pub fn as_str(&self) -> &'static str {
        match self {
            FaultOperationType::ReadItem => "ReadItem",
            FaultOperationType::QueryItem => "QueryItem",
            FaultOperationType::CreateItem => "CreateItem",
            FaultOperationType::UpsertItem => "UpsertItem",
            FaultOperationType::ReplaceItem => "ReplaceItem",
            FaultOperationType::DeleteItem => "DeleteItem",
            FaultOperationType::PatchItem => "PatchItem",
            FaultOperationType::BatchItem => "BatchItem",
            FaultOperationType::ChangeFeedItem => "ChangeFeedItem",
            FaultOperationType::MetadataReadContainer => "MetadataReadContainer",
            FaultOperationType::MetadataReadDatabaseAccount => "MetadataReadDatabaseAccount",
            FaultOperationType::MetadataQueryPlan => "MetadataQueryPlan",
            FaultOperationType::MetadataPartitionKeyRanges => "MetadataPartitionKeyRanges",
        }
    }

    /// Maps an operation type and resource type to a [`FaultOperationType`].
    ///
    /// Returns `None` when the request does not have a matching fault injection operation.
    pub fn from_operation_and_resource(
        operation_type: &OperationType,
        resource_type: &ResourceType,
    ) -> Option<Self> {
        match (operation_type, resource_type) {
            (OperationType::Read, ResourceType::Document) => Some(FaultOperationType::ReadItem),
            (OperationType::Query, ResourceType::Document) => Some(FaultOperationType::QueryItem),
            (OperationType::Create, ResourceType::Document) => Some(FaultOperationType::CreateItem),
            (OperationType::Upsert, ResourceType::Document) => Some(FaultOperationType::UpsertItem),
            (OperationType::Replace, ResourceType::Document) => {
                Some(FaultOperationType::ReplaceItem)
            }
            (OperationType::Delete, ResourceType::Document) => Some(FaultOperationType::DeleteItem),
            (OperationType::Batch, ResourceType::Document) => Some(FaultOperationType::BatchItem),
            (OperationType::ReadFeed, ResourceType::Document) => {
                Some(FaultOperationType::ChangeFeedItem)
            }
            (OperationType::Read, ResourceType::DocumentCollection) => {
                Some(FaultOperationType::MetadataReadContainer)
            }
            (OperationType::Read, ResourceType::DatabaseAccount) => {
                Some(FaultOperationType::MetadataReadDatabaseAccount)
            }
            (OperationType::QueryPlan, ResourceType::Document) => {
                Some(FaultOperationType::MetadataQueryPlan)
            }
            (OperationType::ReadFeed, ResourceType::PartitionKeyRange) => {
                Some(FaultOperationType::MetadataPartitionKeyRanges)
            }
            // PatchItem will be mapped when OperationType::Patch is added to the driver.
            _ => None,
        }
    }
}

impl fmt::Display for FaultOperationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for FaultOperationType {
    type Err = crate::error::CosmosError;

    /// Parses a string into a [`FaultOperationType`].
    ///
    /// Returns an error if `s` does not name a supported operation type.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ReadItem" => Ok(FaultOperationType::ReadItem),
            "QueryItem" => Ok(FaultOperationType::QueryItem),
            "CreateItem" => Ok(FaultOperationType::CreateItem),
            "UpsertItem" => Ok(FaultOperationType::UpsertItem),
            "ReplaceItem" => Ok(FaultOperationType::ReplaceItem),
            "DeleteItem" => Ok(FaultOperationType::DeleteItem),
            "PatchItem" => Ok(FaultOperationType::PatchItem),
            "BatchItem" => Ok(FaultOperationType::BatchItem),
            "ChangeFeedItem" => Ok(FaultOperationType::ChangeFeedItem),
            "MetadataReadContainer" => Ok(FaultOperationType::MetadataReadContainer),
            "MetadataReadDatabaseAccount" => Ok(FaultOperationType::MetadataReadDatabaseAccount),
            "MetadataQueryPlan" => Ok(FaultOperationType::MetadataQueryPlan),
            "MetadataPartitionKeyRanges" => Ok(FaultOperationType::MetadataPartitionKeyRanges),
            _ => Err(crate::error::CosmosError::builder()
                .with_status(crate::error::CosmosStatus::new(
                    azure_core::http::StatusCode::BadRequest,
                ))
                .with_message(format!("unknown fault operation type: {s}"))
                .build()),
        }
    }
}

impl fmt::Display for FaultInjectionErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InternalServerError => write!(f, "InternalServerError"),
            Self::TooManyRequests => write!(f, "TooManyRequests"),
            Self::ReadSessionNotAvailable => write!(f, "ReadSessionNotAvailable"),
            Self::Timeout => write!(f, "Timeout"),
            Self::ServiceUnavailable => write!(f, "ServiceUnavailable"),
            Self::PartitionIsGone => write!(f, "PartitionIsGone"),
            Self::WriteForbidden => write!(f, "WriteForbidden"),
            Self::DatabaseAccountNotFound => write!(f, "DatabaseAccountNotFound"),
            Self::ConnectionError => write!(f, "ConnectionError"),
            Self::ResponseTimeout => write!(f, "ResponseTimeout"),
        }
    }
}

impl FromStr for FaultInjectionErrorType {
    type Err = crate::error::CosmosError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "InternalServerError" => Ok(Self::InternalServerError),
            "TooManyRequests" => Ok(Self::TooManyRequests),
            "ReadSessionNotAvailable" => Ok(Self::ReadSessionNotAvailable),
            "Timeout" => Ok(Self::Timeout),
            "ServiceUnavailable" => Ok(Self::ServiceUnavailable),
            "PartitionIsGone" => Ok(Self::PartitionIsGone),
            "WriteForbidden" => Ok(Self::WriteForbidden),
            "DatabaseAccountNotFound" => Ok(Self::DatabaseAccountNotFound),
            "ConnectionError" => Ok(Self::ConnectionError),
            "ResponseTimeout" => Ok(Self::ResponseTimeout),
            _ => Err(crate::error::CosmosError::builder()
                .with_status(crate::error::CosmosStatus::new(
                    azure_core::http::StatusCode::BadRequest,
                ))
                .with_message(format!("unknown fault injection error type: {s}"))
                .build()),
        }
    }
}
