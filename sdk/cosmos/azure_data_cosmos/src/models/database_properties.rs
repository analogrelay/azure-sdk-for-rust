// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! [`DatabaseProperties`] for Cosmos DB databases.

use azure_core::fmt::SafeDebug;
use serde::{Deserialize, Serialize};

use crate::models::SystemProperties;

/// Properties of a Cosmos DB database.
#[non_exhaustive]
#[derive(Clone, Default, SafeDebug, Deserialize, Serialize, PartialEq, Eq)]
#[safe(true)]
pub struct DatabaseProperties {
    /// The database ID, if present in the service response.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Common system properties for the database.
    #[serde(flatten)]
    pub system_properties: SystemProperties,
}
