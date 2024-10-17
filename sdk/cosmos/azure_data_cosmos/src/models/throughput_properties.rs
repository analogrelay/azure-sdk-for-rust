// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use azure_core::Model;
use serde::{Deserialize, Serialize};

use crate::models::SystemProperties;

#[derive(Model, Clone, Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThroughputProperties {
    resource: String,
    #[serde(rename = "content")]
    offer: Offer,
    #[serde(rename = "id")]
    pub(crate) offer_id: String,
    offer_resource_id: String,
    offer_type: String,
    offer_version: String,
    #[serde(flatten)]
    pub(crate) system_properties: SystemProperties,
}

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Offer {
    pub offer_throughput: i32,
    pub offer_autopilot_settings: Option<AutoscaleSettings>,
}

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AutoscaleSettings {
    pub max_throughput: i32,
    pub auto_upgrade_policy: Option<AutoscaleAutoUpgradePolicy>,
}

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AutoscaleAutoUpgradePolicy {
    pub throughput_policy: Option<AutoscaleThroughputPolicy>,
}

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AutoscaleThroughputPolicy {
    pub increment_percent: i32,
}
