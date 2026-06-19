// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! [`ThroughputProperties`] for manual and autoscale throughput settings.

use std::borrow::Cow;

use azure_core::fmt::SafeDebug;
use serde::{Deserialize, Serialize};

use crate::models::SystemProperties;

const OFFER_VERSION_2: &str = "V2";

/// Throughput settings for a database or container.
///
/// Use [`ThroughputProperties::manual`] for fixed throughput or
/// [`ThroughputProperties::autoscale`] for autoscale throughput.
#[derive(Clone, SafeDebug, Deserialize, Serialize)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ThroughputProperties {
    resource: String,
    #[serde(rename = "content")]
    pub(crate) offer: Offer,
    #[serde(rename = "id")]
    pub(crate) offer_id: String,
    offer_resource_id: String,
    offer_type: String,
    offer_version: Cow<'static, str>, // When we serialize, this is always going to be a constant.
    #[serde(flatten)]
    pub(crate) system_properties: SystemProperties,
}

impl ThroughputProperties {
    /// Creates manual throughput settings.
    pub fn manual(throughput: usize) -> ThroughputProperties {
        ThroughputProperties {
            resource: String::new(),
            offer: Offer {
                offer_throughput: Some(throughput),
                offer_autopilot_settings: None,
            },
            offer_id: String::new(),
            offer_resource_id: String::new(),
            offer_type: String::new(),
            offer_version: OFFER_VERSION_2.into(),
            system_properties: SystemProperties::default(),
        }
    }

    /// Creates autoscale throughput settings.
    ///
    /// `starting_maximum_throughput` is the maximum throughput autoscale can
    /// reach. If `increment_percent` is set, the service can automatically
    /// raise that maximum by the specified percentage.
    pub fn autoscale(
        starting_maximum_throughput: usize,
        increment_percent: Option<usize>,
    ) -> ThroughputProperties {
        ThroughputProperties {
            resource: String::new(),
            offer: Offer {
                offer_throughput: None,
                offer_autopilot_settings: Some(OfferAutoscaleSettings {
                    max_throughput: starting_maximum_throughput,
                    auto_upgrade_policy: increment_percent.map(|p| AutoscaleAutoUpgradePolicy {
                        throughput_policy: Some(AutoscaleThroughputPolicy {
                            increment_percent: p,
                        }),
                    }),
                }),
            },
            offer_id: String::new(),
            offer_resource_id: String::new(),
            offer_type: String::new(),
            offer_version: OFFER_VERSION_2.into(),
            system_properties: SystemProperties::default(),
        }
    }

    /// Returns the manual throughput, if this is a manual throughput offer.
    pub fn throughput(&self) -> Option<usize> {
        self.offer.offer_throughput
    }

    /// Returns the autoscale maximum throughput, if autoscale is enabled.
    pub fn autoscale_maximum(&self) -> Option<usize> {
        Some(self.offer.offer_autopilot_settings.as_ref()?.max_throughput)
    }

    /// Returns the autoscale maximum-throughput increment percentage, if set.
    pub fn autoscale_increment(&self) -> Option<usize> {
        Some(
            self.offer
                .offer_autopilot_settings
                .as_ref()?
                .auto_upgrade_policy
                .as_ref()?
                .throughput_policy
                .as_ref()?
                .increment_percent,
        )
    }
}

#[derive(Clone, Default, SafeDebug, Deserialize, Serialize)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Offer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offer_throughput: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offer_autopilot_settings: Option<OfferAutoscaleSettings>,
}

#[derive(Clone, Default, SafeDebug, Deserialize, Serialize)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OfferAutoscaleSettings {
    pub max_throughput: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_upgrade_policy: Option<AutoscaleAutoUpgradePolicy>,
}

#[derive(Clone, Default, SafeDebug, Deserialize, Serialize)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AutoscaleAutoUpgradePolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throughput_policy: Option<AutoscaleThroughputPolicy>,
}

#[derive(Clone, Default, SafeDebug, Deserialize, Serialize)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AutoscaleThroughputPolicy {
    pub increment_percent: usize,
}

impl ThroughputProperties {
    /// Applies throughput settings to the given request headers.
    ///
    /// Sets either the manual throughput or autoscale settings header,
    /// depending on how this `ThroughputProperties` was constructed.
    pub(crate) fn apply_headers(
        &self,
        headers: &mut azure_data_cosmos_driver::models::CosmosRequestHeaders,
    ) {
        match (
            self.offer.offer_throughput,
            self.offer.offer_autopilot_settings.as_ref(),
        ) {
            (Some(t), _) => {
                headers.offer_throughput = Some(t);
            }
            (_, Some(ap)) => {
                let mut settings = azure_data_cosmos_driver::models::OfferAutoscaleSettings::new(
                    ap.max_throughput,
                );
                if let Some(policy) = ap.auto_upgrade_policy.as_ref() {
                    if let Some(tp) = policy.throughput_policy.as_ref() {
                        settings = settings.with_increment_percent(tp.increment_percent);
                    }
                }
                headers.offer_autopilot_settings = Some(settings);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use azure_data_cosmos_driver::models::CosmosRequestHeaders;

    #[test]
    fn manual_throughput_sets_offer_throughput() {
        let tp = ThroughputProperties::manual(400);
        let mut headers = CosmosRequestHeaders::new();
        tp.apply_headers(&mut headers);
        assert_eq!(headers.offer_throughput, Some(400));
        assert!(headers.offer_autopilot_settings.is_none());
    }

    #[test]
    fn autoscale_throughput_sets_autopilot_settings() {
        let tp = ThroughputProperties::autoscale(4000, None);
        let mut headers = CosmosRequestHeaders::new();
        tp.apply_headers(&mut headers);
        assert!(headers.offer_throughput.is_none());
        let settings = headers
            .offer_autopilot_settings
            .expect("should have autopilot settings");
        assert_eq!(settings.max_throughput, 4000);
    }
}
