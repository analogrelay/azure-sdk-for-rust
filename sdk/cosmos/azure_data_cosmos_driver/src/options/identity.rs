// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Identity and telemetry types for environment configuration.

use std::fmt;

/// Maximum length for the full user agent string (HTTP header limit).
const MAX_USER_AGENT_LENGTH: usize = 255;

/// Azure SDK user agent prefix.
const AZSDK_USER_AGENT_PREFIX: &str = "azsdk-rust-";

/// SDK name used in the user agent.
const SDK_NAME: &str = "cosmos-driver";

/// SDK version, retrieved from Cargo.toml at compile time.
const SDK_VERSION: &str = env!("CARGO_PKG_VERSION");

/// User agent string for HTTP requests.
///
/// The user agent is automatically computed with a static prefix containing:
/// - Azure SDK identifier (`azsdk-rust-`)
/// - SDK name and version
/// - OS name and version (if available)
/// - Rust version (compile time)
///
/// An optional suffix can be appended (typically from [`UserAgentSuffix`],
/// [`WorkloadId`], or [`CorrelationId`]).
///
/// # Example
///
/// Without suffix: `azsdk-rust-cosmos-driver/0.1.0 Windows/10.0 rustc/1.85.0`
/// With suffix: `azsdk-rust-cosmos-driver/0.1.0 Windows/10.0 rustc/1.85.0 myapp-westus2`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserAgent {
    /// The full computed user agent string.
    full_user_agent: String,
    /// The suffix that was appended (if any).
    suffix: Option<String>,
}

impl Default for UserAgent {
    fn default() -> Self {
        Self::new(None::<&str>)
    }
}

impl UserAgent {
    /// Returns the base user agent prefix (without suffix).
    ///
    /// Format: `azsdk-rust-{sdk-name}/{version} {os}/{os-version} rustc/{rust-version}`
    fn base_user_agent() -> String {
        let os_name = std::env::consts::OS;
        let os_arch = std::env::consts::ARCH;

        // Rust version is available at compile time via rustc_version_runtime or we use a const
        // For simplicity, we'll use the compile-time RUSTC version if available
        let rust_version = option_env!("RUSTC_VERSION").unwrap_or("unknown");

        format!(
            "{}{}/{} {}/{} rustc/{}",
            AZSDK_USER_AGENT_PREFIX, SDK_NAME, SDK_VERSION, os_name, os_arch, rust_version
        )
    }

    /// Creates a new user agent with an optional suffix.
    ///
    /// The suffix is appended to the base user agent, separated by a space.
    /// If the resulting string exceeds 255 characters, the suffix is truncated.
    pub fn new(suffix: Option<impl Into<String>>) -> Self {
        let base = Self::base_user_agent();
        let suffix = suffix.map(Into::into);

        let full_user_agent = match &suffix {
            Some(s) if !s.is_empty() => {
                let proposed = format!("{} {}", base, s);
                if proposed.len() <= MAX_USER_AGENT_LENGTH {
                    proposed
                } else {
                    // Truncate suffix to fit within limit
                    let max_suffix_len = MAX_USER_AGENT_LENGTH.saturating_sub(base.len() + 1);
                    if max_suffix_len > 0 {
                        format!("{} {}", base, &s[..max_suffix_len.min(s.len())])
                    } else {
                        base
                    }
                }
            }
            _ => base,
        };

        Self {
            full_user_agent: strip_non_ascii(&full_user_agent),
            suffix,
        }
    }

    /// Creates a user agent from a [`UserAgentSuffix`].
    pub fn from_suffix(suffix: &UserAgentSuffix) -> Self {
        Self::new(Some(suffix.as_str()))
    }

    /// Creates a user agent from a [`WorkloadId`].
    pub fn from_workload_id(workload_id: WorkloadId) -> Self {
        Self::new(Some(format!("w{}", workload_id.value())))
    }

    /// Creates a user agent from a [`CorrelationId`].
    pub fn from_correlation_id(correlation_id: &CorrelationId) -> Self {
        Self::new(Some(correlation_id.as_str()))
    }

    /// Returns the full user agent string.
    pub fn as_str(&self) -> &str {
        &self.full_user_agent
    }

    /// Returns the suffix that was used, if any.
    pub fn suffix(&self) -> Option<&str> {
        self.suffix.as_deref()
    }

    /// Returns the base user agent prefix (without any suffix).
    pub fn base_prefix() -> String {
        Self::base_user_agent()
    }
}

impl fmt::Display for UserAgent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.full_user_agent)
    }
}

/// Strips non-ASCII characters from a string, replacing them with underscores.
fn strip_non_ascii(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_ascii() && !c.is_ascii_control() {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Workload identifier for resource governance.
///
/// Must be a value between 1 and 50 (inclusive) if set.
/// Used for workload-based resource allocation and tracking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkloadId(u8);

impl WorkloadId {
    /// The minimum allowed workload ID value.
    pub const MIN: u8 = 1;
    /// The maximum allowed workload ID value.
    pub const MAX: u8 = 50;

    /// Creates a new workload ID.
    ///
    /// # Panics
    ///
    /// Panics if the value is not between 1 and 50 (inclusive).
    pub fn new(value: u8) -> Self {
        assert!(
            (Self::MIN..=Self::MAX).contains(&value),
            "WorkloadId must be between {} and {} (inclusive), got {}",
            Self::MIN,
            Self::MAX,
            value
        );
        Self(value)
    }

    /// Creates a new workload ID, returning `None` if the value is out of range.
    pub fn try_new(value: u8) -> Option<Self> {
        if (Self::MIN..=Self::MAX).contains(&value) {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Returns the workload ID value.
    pub fn value(&self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for WorkloadId {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::try_new(value).ok_or("WorkloadId must be between 1 and 50 (inclusive)")
    }
}

impl fmt::Display for WorkloadId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validates that a string contains only HTTP header-safe characters.
///
/// Allowed characters: alphanumeric, hyphen, underscore, dot, and tilde.
fn is_http_header_safe(s: &str) -> bool {
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~'))
}

/// Correlation ID for client-side metrics.
///
/// Used as a dimension for client-side metrics to correlate requests.
/// Limited to 50 characters and must contain only HTTP header-safe characters
/// (alphanumeric, hyphen, underscore, dot, tilde).
///
/// # Cardinality Warning
///
/// If the cardinality of correlation IDs is too high, metrics aggregation may
/// ignore or truncate this dimension. Choose values that provide meaningful
/// grouping without excessive uniqueness (e.g., cluster names, environment IDs,
/// deployment identifiers).
///
/// # Examples
///
/// Good values (low to moderate cardinality):
/// - AKS cluster name: `"aks-prod-eastus-001"`
/// - Environment: `"production"`, `"staging"`
/// - Deployment ID: `"deploy-2024-01-15"`
///
/// Avoid (high cardinality):
/// - Request IDs
/// - Timestamps
/// - User IDs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorrelationId(String);

impl CorrelationId {
    /// Maximum length for a correlation ID.
    pub const MAX_LENGTH: usize = 50;

    /// Creates a new correlation ID.
    ///
    /// # Panics
    ///
    /// Panics if the value exceeds 50 characters or contains invalid characters.
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(
            value.len() <= Self::MAX_LENGTH,
            "CorrelationId must be at most {} characters, got {}",
            Self::MAX_LENGTH,
            value.len()
        );
        assert!(
            is_http_header_safe(&value),
            "CorrelationId must contain only HTTP header-safe characters (alphanumeric, hyphen, underscore, dot, tilde)"
        );
        Self(value)
    }

    /// Creates a new correlation ID, returning `None` if validation fails.
    pub fn try_new(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        if value.len() <= Self::MAX_LENGTH && is_http_header_safe(&value) {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Returns the correlation ID string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// User agent suffix for request identification.
///
/// Appended to the user agent string to identify the source of requests.
/// Limited to 25 characters and must contain only HTTP header-safe characters
/// (alphanumeric, hyphen, underscore, dot, tilde).
///
/// If [`CorrelationId`] is not set, this suffix is used as the correlation
/// dimension for client-side metrics.
///
/// # Server-Side Enforcement
///
/// The Cosmos DB service enforces cardinality limits on user agent suffixes
/// more strictly than client-side correlation IDs. High-cardinality suffixes
/// may be rejected or normalized by the service.
///
/// # Examples
///
/// Good values:
/// - AKS cluster name: `"aks-prod-eastus"`
/// - Azure VM ID (if node count is limited): `"vm-worker-01"`
/// - App identifier with region: `"myapp-westus2"`
/// - Service name: `"order-service"`
///
/// Avoid:
/// - Instance-specific IDs with high cardinality
/// - Timestamps or request IDs
/// - Values that change frequently
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserAgentSuffix(String);

impl UserAgentSuffix {
    /// Maximum length for a user agent suffix.
    pub const MAX_LENGTH: usize = 25;

    /// Creates a new user agent suffix.
    ///
    /// # Panics
    ///
    /// Panics if the value exceeds 25 characters or contains invalid characters.
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(
            value.len() <= Self::MAX_LENGTH,
            "UserAgentSuffix must be at most {} characters, got {}",
            Self::MAX_LENGTH,
            value.len()
        );
        assert!(
            is_http_header_safe(&value),
            "UserAgentSuffix must contain only HTTP header-safe characters (alphanumeric, hyphen, underscore, dot, tilde)"
        );
        Self(value)
    }

    /// Creates a new user agent suffix, returning `None` if validation fails.
    pub fn try_new(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        if value.len() <= Self::MAX_LENGTH && is_http_header_safe(&value) {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Returns the user agent suffix string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UserAgentSuffix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_agent_default_has_base_prefix() {
        let ua = UserAgent::default();
        assert!(ua.as_str().starts_with("azsdk-rust-cosmos-driver/"));
        assert!(ua.suffix().is_none());
    }

    #[test]
    fn user_agent_with_suffix() {
        let ua = UserAgent::new(Some("my-app"));
        assert!(ua.as_str().contains("my-app"));
        assert_eq!(ua.suffix(), Some("my-app"));
    }

    #[test]
    fn user_agent_from_user_agent_suffix() {
        let suffix = UserAgentSuffix::new("myapp-westus2");
        let ua = UserAgent::from_suffix(&suffix);
        assert!(ua.as_str().contains("myapp-westus2"));
    }

    #[test]
    fn user_agent_from_workload_id() {
        let workload_id = WorkloadId::new(25);
        let ua = UserAgent::from_workload_id(workload_id);
        assert!(ua.as_str().contains("w25"));
    }

    #[test]
    fn user_agent_from_correlation_id() {
        let correlation_id = CorrelationId::new("aks-prod-eastus");
        let ua = UserAgent::from_correlation_id(&correlation_id);
        assert!(ua.as_str().contains("aks-prod-eastus"));
    }

    #[test]
    fn user_agent_strips_non_ascii() {
        // Non-ASCII characters should be replaced with underscores
        let input = "test café";
        let stripped = strip_non_ascii(input);
        assert!(stripped.is_ascii());
    }

    #[test]
    fn workload_id_valid_range() {
        assert!(WorkloadId::try_new(1).is_some());
        assert!(WorkloadId::try_new(25).is_some());
        assert!(WorkloadId::try_new(50).is_some());
    }

    #[test]
    fn workload_id_invalid_range() {
        assert!(WorkloadId::try_new(0).is_none());
        assert!(WorkloadId::try_new(51).is_none());
        assert!(WorkloadId::try_new(255).is_none());
    }

    #[test]
    #[should_panic(expected = "WorkloadId must be between 1 and 50")]
    fn workload_id_panics_on_zero() {
        WorkloadId::new(0);
    }

    #[test]
    fn correlation_id_valid() {
        let id = CorrelationId::new("aks-prod-eastus-001");
        assert_eq!(id.as_str(), "aks-prod-eastus-001");
    }

    #[test]
    fn correlation_id_max_length() {
        let long_id = "a".repeat(50);
        assert!(CorrelationId::try_new(&long_id).is_some());

        let too_long = "a".repeat(51);
        assert!(CorrelationId::try_new(&too_long).is_none());
    }

    #[test]
    fn correlation_id_invalid_chars() {
        assert!(CorrelationId::try_new("valid-id_123").is_some());
        assert!(CorrelationId::try_new("invalid id").is_none()); // space
        assert!(CorrelationId::try_new("invalid/id").is_none()); // slash
        assert!(CorrelationId::try_new("invalid:id").is_none()); // colon
    }

    #[test]
    fn user_agent_suffix_valid() {
        let suffix = UserAgentSuffix::new("myapp-westus2");
        assert_eq!(suffix.as_str(), "myapp-westus2");
    }

    #[test]
    fn user_agent_suffix_max_length() {
        let long_suffix = "a".repeat(25);
        assert!(UserAgentSuffix::try_new(&long_suffix).is_some());

        let too_long = "a".repeat(26);
        assert!(UserAgentSuffix::try_new(&too_long).is_none());
    }

    #[test]
    fn user_agent_suffix_invalid_chars() {
        assert!(UserAgentSuffix::try_new("valid-suffix").is_some());
        assert!(UserAgentSuffix::try_new("invalid suffix").is_none()); // space
    }
}
