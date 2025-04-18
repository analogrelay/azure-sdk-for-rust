// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

// Don't spell-check header names (which should start with 'x-').
// cSpell:ignoreRegExp /x-[^\s]+/

//! Constants defining HTTP headers and other values relevant to Azure Cosmos DB APIs.

use azure_core::http::{
    headers::{HeaderName, HeaderValue},
    request::options::ContentType,
};

pub const QUERY: HeaderName = HeaderName::from_static("x-ms-documentdb-query");
pub const PARTITION_KEY: HeaderName = HeaderName::from_static("x-ms-documentdb-partitionkey");
pub const CONTINUATION: HeaderName = HeaderName::from_static("x-ms-continuation");
pub const INDEX_METRICS: HeaderName = HeaderName::from_static("x-ms-cosmos-index-utilization");
pub const QUERY_METRICS: HeaderName = HeaderName::from_static("x-ms-documentdb-query-metrics");
pub const IS_UPSERT: HeaderName = HeaderName::from_static("x-ms-documentdb-is-upsert");
pub const OFFER_THROUGHPUT: HeaderName = HeaderName::from_static("x-ms-offer-throughput");
pub const OFFER_AUTOPILOT_SETTINGS: HeaderName =
    HeaderName::from_static("x-ms-cosmos-offer-autopilot-settings");

pub const QUERY_CONTENT_TYPE: ContentType = ContentType::from_static("application/query+json");

pub(crate) const PREFER_MINIMAL: HeaderValue = HeaderValue::from_static("return=minimal");
