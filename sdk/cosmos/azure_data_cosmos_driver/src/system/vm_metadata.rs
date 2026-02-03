// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Azure VM metadata from the Instance Metadata Service (IMDS).

use azure_core::http::{new_http_client, Method, Request};
use serde::Deserialize;
use std::sync::{Arc, OnceLock, RwLock};
use url::Url;

/// Azure Instance Metadata Service endpoint.
const IMDS_ENDPOINT: &str = "http://169.254.169.254/metadata/instance?api-version=2020-06-01";

/// Prefix for VM ID in machine identifiers.
pub const VM_ID_PREFIX: &str = "vmId_";

/// Global singleton for Azure VM metadata.
static VM_METADATA: OnceLock<Arc<VmMetadataServiceInner>> = OnceLock::new();

/// Azure VM metadata retrieved from IMDS.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct AzureVmMetadata {
    /// Compute metadata.
    compute: ComputeMetadata,
}

impl AzureVmMetadata {
    /// Returns the Azure region/location.
    pub fn location(&self) -> &str {
        &self.compute.location
    }

    /// Returns the VM SKU.
    pub fn sku(&self) -> &str {
        &self.compute.sku
    }

    /// Returns the Azure environment (e.g., "AzurePublicCloud").
    pub fn az_environment(&self) -> &str {
        &self.compute.az_environment
    }

    /// Returns the OS type (e.g., "Linux", "Windows").
    pub fn os_type(&self) -> &str {
        &self.compute.os_type
    }

    /// Returns the VM size (e.g., "Standard_D2s_v3").
    pub fn vm_size(&self) -> &str {
        &self.compute.vm_size
    }

    /// Returns the VM ID.
    pub fn vm_id(&self) -> &str {
        &self.compute.vm_id
    }

    /// Returns the machine ID with the VM ID prefix.
    pub fn machine_id(&self) -> String {
        if self.compute.vm_id.is_empty() {
            String::new()
        } else {
            format!("{}{}", VM_ID_PREFIX, self.compute.vm_id)
        }
    }

    /// Returns the host environment info string.
    pub fn host_env_info(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.os_type(),
            self.sku(),
            self.vm_size(),
            self.az_environment()
        )
    }
}

/// Compute-specific metadata from IMDS.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ComputeMetadata {
    location: String,
    sku: String,
    #[serde(rename = "azEnvironment")]
    az_environment: String,
    os_type: String,
    vm_size: String,
    vm_id: String,
}

/// Handle to the VM metadata service singleton.
///
/// Provides access to cached Azure VM metadata fetched from IMDS.
/// The metadata is fetched once on first initialization and cached.
#[derive(Clone, Debug)]
pub struct VmMetadataService {
    /// Cached metadata (None if fetch failed or not on Azure).
    metadata: Option<Arc<AzureVmMetadata>>,
}

impl VmMetadataService {
    /// Gets or creates the VM metadata service singleton.
    ///
    /// On first call, this will attempt to fetch metadata from IMDS.
    /// This is an async operation since it uses azure_core's HTTP client.
    pub async fn get_or_init() -> Self {
        // Use OnceLock to ensure we only fetch once
        let inner = VM_METADATA.get_or_init(|| Arc::new(VmMetadataServiceInner::new()));

        // Check if we need to fetch metadata
        if !inner.is_fetch_complete() {
            // Fetch metadata (this will be a no-op if already fetched by another task)
            inner.fetch_metadata().await;
        }

        // Extract the cached metadata
        let metadata = inner.get_metadata();

        Self { metadata }
    }

    /// Creates an empty VM metadata service with no metadata.
    ///
    /// This is primarily for testing scenarios where VM metadata is not needed.
    #[cfg(test)]
    pub(crate) fn empty() -> Self {
        Self { metadata: None }
    }

    /// Returns the cached VM metadata, if available.
    ///
    /// Returns `None` if:
    /// - The fetch failed (not running on Azure)
    /// - IMDS access is disabled
    pub fn metadata(&self) -> Option<&AzureVmMetadata> {
        self.metadata.as_deref()
    }

    /// Returns the machine ID (VM ID with prefix), if available.
    pub fn machine_id(&self) -> Option<&str> {
        self.metadata.as_ref().map(|m| m.vm_id())
    }

    /// Returns `true` if metadata has been fetched successfully.
    pub fn is_available(&self) -> bool {
        self.metadata.is_some()
    }
}

/// Internal state for the VM metadata service (used for async initialization).
#[derive(Debug)]
struct VmMetadataServiceInner {
    /// Cached metadata.
    metadata: RwLock<Option<Arc<AzureVmMetadata>>>,
    /// Whether fetch has completed (success or failure).
    fetch_complete: RwLock<bool>,
}

impl VmMetadataServiceInner {
    fn new() -> Self {
        Self {
            metadata: RwLock::new(None),
            fetch_complete: RwLock::new(false),
        }
    }

    fn is_fetch_complete(&self) -> bool {
        *self.fetch_complete.read().unwrap()
    }

    fn get_metadata(&self) -> Option<Arc<AzureVmMetadata>> {
        self.metadata.read().unwrap().clone()
    }

    async fn fetch_metadata(&self) {
        // Check if already fetched (race condition protection)
        {
            let complete = self.fetch_complete.read().unwrap();
            if *complete {
                return;
            }
        }

        // Check if IMDS access is disabled via environment variable
        if std::env::var("COSMOS_DISABLE_IMDS").is_ok() {
            tracing::info!("IMDS access disabled via COSMOS_DISABLE_IMDS");
            *self.fetch_complete.write().unwrap() = true;
            return;
        }

        let result = Self::do_fetch().await;

        match result {
            Ok(metadata) => {
                tracing::debug!("Fetched Azure VM metadata: {:?}", metadata);
                *self.metadata.write().unwrap() = Some(Arc::new(metadata));
            }
            Err(e) => {
                tracing::debug!("Failed to fetch Azure VM metadata (not on Azure?): {}", e);
            }
        }

        *self.fetch_complete.write().unwrap() = true;
    }

    async fn do_fetch() -> azure_core::Result<AzureVmMetadata> {
        let url: Url = IMDS_ENDPOINT.parse().expect("valid IMDS URL");
        let mut request = Request::new(url, Method::Get);
        request.insert_header("Metadata", "true");

        let http_client = new_http_client();
        let response = http_client.execute_request(&request).await?;
        let body = response.into_body().collect_string().await?;
        let metadata: AzureVmMetadata = serde_json::from_str(&body)?;
        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn azure_vm_metadata_deserialize() {
        let json = r#"{
            "compute": {
                "location": "eastus",
                "sku": "Standard",
                "azEnvironment": "AzurePublicCloud",
                "osType": "Linux",
                "vmSize": "Standard_D2s_v3",
                "vmId": "12345678-1234-1234-1234-123456789012"
            }
        }"#;

        let metadata: AzureVmMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(metadata.location(), "eastus");
        assert_eq!(metadata.sku(), "Standard");
        assert_eq!(metadata.az_environment(), "AzurePublicCloud");
        assert_eq!(metadata.os_type(), "Linux");
        assert_eq!(metadata.vm_size(), "Standard_D2s_v3");
        assert_eq!(metadata.vm_id(), "12345678-1234-1234-1234-123456789012");
        assert_eq!(
            metadata.machine_id(),
            "vmId_12345678-1234-1234-1234-123456789012"
        );
    }

    #[test]
    fn azure_vm_metadata_empty() {
        let metadata = AzureVmMetadata::default();
        assert_eq!(metadata.location(), "");
        assert_eq!(metadata.machine_id(), "");
    }

    #[test]
    fn azure_vm_metadata_host_env_info() {
        let json = r#"{
            "compute": {
                "osType": "Linux",
                "sku": "18.04-LTS",
                "vmSize": "Standard_D2s_v3",
                "azEnvironment": "AzurePublicCloud"
            }
        }"#;

        let metadata: AzureVmMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(
            metadata.host_env_info(),
            "Linux|18.04-LTS|Standard_D2s_v3|AzurePublicCloud"
        );
    }
}
