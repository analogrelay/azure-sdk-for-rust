// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Container properties and related policy types.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{borrow::Cow, time::Duration};

// =============================================================================
// TimeToLive
// =============================================================================

/// Time-to-live (TTL) configuration for items in a container.
///
/// Controls automatic expiration of items. The semantic is clearer than raw integer values:
/// - `Off`: TTL is disabled, items never expire based on TTL
/// - `InheritFromItem`: Container enables TTL, but each item must specify its own `_ttl` field
/// - `Seconds(N)`: Items expire N seconds after last modification (can be overridden per-item)
///
/// # Wire Format
///
/// On the wire, TTL is represented as an integer:
/// - Absent/null: TTL disabled (`Off`)
/// - `-1`: Inherit from item (`InheritFromItem`)
/// - Positive integer: Seconds until expiration (`Seconds(N)`)
///
/// # Example
///
/// ```
/// use azure_data_cosmos_driver::models::TimeToLive;
/// use std::time::Duration;
///
/// // Disable TTL (items never expire)
/// let ttl = TimeToLive::off();
///
/// // Enable TTL but require each item to set _ttl
/// let ttl = TimeToLive::inherit_from_item();
///
/// // Items expire after 1 hour by default
/// let ttl = TimeToLive::from_duration(Duration::from_secs(3600));
///
/// // Items expire after 86400 seconds (1 day)
/// let ttl = TimeToLive::seconds(86400);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TimeToLive {
    /// TTL is disabled. Items never expire based on TTL.
    #[default]
    Off,

    /// Container enables TTL, but each item must specify its own `_ttl` field.
    /// Items without a `_ttl` field will not expire.
    InheritFromItem,

    /// Items expire after the specified number of seconds since last modification.
    /// Individual items can override this by setting their own `_ttl` field.
    Seconds(u32),
}

impl TimeToLive {
    /// Creates a TTL configuration that disables expiration.
    pub const fn off() -> Self {
        Self::Off
    }

    /// Creates a TTL configuration where items inherit TTL from their `_ttl` field.
    pub const fn inherit_from_item() -> Self {
        Self::InheritFromItem
    }

    /// Creates a TTL configuration with a specific duration.
    ///
    /// # Panics
    ///
    /// Panics if the duration exceeds `u32::MAX` seconds (~136 years).
    pub fn from_duration(duration: Duration) -> Self {
        let secs = duration.as_secs();
        assert!(
            secs <= u32::MAX as u64,
            "TTL duration exceeds maximum of {} seconds",
            u32::MAX
        );
        Self::Seconds(secs as u32)
    }

    /// Creates a TTL configuration with a specific number of seconds.
    pub const fn seconds(secs: u32) -> Self {
        Self::Seconds(secs)
    }

    /// Returns the TTL value in seconds, if applicable.
    pub const fn as_seconds(&self) -> Option<u32> {
        match self {
            Self::Off => None,
            Self::InheritFromItem => None,
            Self::Seconds(s) => Some(*s),
        }
    }
}

impl Serialize for TimeToLive {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Off => serializer.serialize_none(),
            Self::InheritFromItem => serializer.serialize_i32(-1),
            Self::Seconds(s) => serializer.serialize_u32(*s),
        }
    }
}

impl<'de> Deserialize<'de> for TimeToLive {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: Option<i64> = Option::deserialize(deserializer)?;
        match value {
            None => Ok(Self::Off),
            Some(-1) => Ok(Self::InheritFromItem),
            Some(v) if v > 0 && v <= i64::from(u32::MAX) => Ok(Self::Seconds(v as u32)),
            Some(v) => Err(serde::de::Error::custom(format!(
                "invalid TTL value: {}",
                v
            ))),
        }
    }
}

// =============================================================================
// ChangeFeedPolicy
// =============================================================================

/// Change feed policy for a container.
///
/// Controls how change feed captures changes:
/// - `LatestVersion`: Only the latest version of each item is available (default)
/// - `AllVersionsAndDeletes`: Intermediate changes and deletes are captured for a retention period
///
/// # Example
///
/// ```
/// use azure_data_cosmos_driver::models::ChangeFeedPolicy;
/// use std::time::Duration;
///
/// // Default: only latest version of items
/// let policy = ChangeFeedPolicy::latest_version();
///
/// // Track all changes and deletes for 24 hours
/// let policy = ChangeFeedPolicy::all_versions_and_deletes(Duration::from_secs(24 * 60 * 60));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ChangeFeedPolicy {
    /// Only the latest version of each item is available in change feed.
    /// This is the default policy.
    #[default]
    LatestVersion,

    /// All versions and deletes are tracked for the specified retention period.
    /// Retention is specified in minutes (must be a positive multiple of minutes).
    AllVersionsAndDeletes {
        /// Retention duration in minutes.
        retention_minutes: u32,
    },
}

impl ChangeFeedPolicy {
    /// Creates a policy that only tracks the latest version of items.
    pub const fn latest_version() -> Self {
        Self::LatestVersion
    }

    /// Creates a policy that tracks all versions and deletes.
    ///
    /// # Arguments
    ///
    /// * `retention` - How long to retain intermediate changes and deletes.
    ///   Must be a positive duration with minute granularity.
    ///
    /// # Panics
    ///
    /// Panics if `retention` is zero, negative, or not a whole number of minutes.
    pub fn all_versions_and_deletes(retention: Duration) -> Self {
        let secs = retention.as_secs();
        assert!(secs > 0, "Retention duration must be positive");
        assert!(
            secs.is_multiple_of(60),
            "Retention duration must be a whole number of minutes"
        );
        let minutes = (secs / 60) as u32;
        Self::AllVersionsAndDeletes {
            retention_minutes: minutes,
        }
    }

    /// Creates a policy that tracks all versions and deletes for the given minutes.
    pub const fn all_versions_and_deletes_minutes(retention_minutes: u32) -> Self {
        Self::AllVersionsAndDeletes { retention_minutes }
    }
}

impl Serialize for ChangeFeedPolicy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;

        match self {
            Self::LatestVersion => {
                let mut s = serializer.serialize_struct("ChangeFeedPolicy", 1)?;
                s.serialize_field("retentionDurationMinutes", &Option::<u32>::None)?;
                s.end()
            }
            Self::AllVersionsAndDeletes { retention_minutes } => {
                let mut s = serializer.serialize_struct("ChangeFeedPolicy", 1)?;
                s.serialize_field("retentionDurationMinutes", retention_minutes)?;
                s.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for ChangeFeedPolicy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Inner {
            retention_duration_minutes: Option<u32>,
        }

        let inner = Inner::deserialize(deserializer)?;
        match inner.retention_duration_minutes {
            None | Some(0) => Ok(Self::LatestVersion),
            Some(minutes) => Ok(Self::AllVersionsAndDeletes {
                retention_minutes: minutes,
            }),
        }
    }
}

// =============================================================================
// UniqueKeyPolicy
// =============================================================================

/// Unique key policy for enforcing uniqueness constraints in a container.
///
/// # Example
///
/// ```
/// use azure_data_cosmos_driver::models::{UniqueKeyPolicy, UniqueKey};
///
/// let policy = UniqueKeyPolicy::new(vec![
///     UniqueKey::new(vec!["/email"]),
///     UniqueKey::new(vec!["/tenantId", "/username"]),
/// ]);
/// ```
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct UniqueKeyPolicy {
    /// The unique keys to enforce.
    pub unique_keys: Vec<UniqueKey>,
}

impl UniqueKeyPolicy {
    /// Creates a new unique key policy with the given keys.
    pub fn new(unique_keys: Vec<UniqueKey>) -> Self {
        Self { unique_keys }
    }
}

/// A unique key constraint within a container.
///
/// Specifies one or more paths that must be unique together.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct UniqueKey {
    /// Paths that must be unique together (e.g., `["/email"]` or `["/tenantId", "/username"]`).
    pub paths: Vec<Cow<'static, str>>,
}

impl UniqueKey {
    /// Creates a new unique key with the given paths.
    pub fn new(paths: impl IntoIterator<Item = impl Into<Cow<'static, str>>>) -> Self {
        Self {
            paths: paths.into_iter().map(Into::into).collect(),
        }
    }
}

// =============================================================================
// ConflictResolutionPolicy
// =============================================================================

/// Conflict resolution policy for multi-region writes.
///
/// Controls how conflicts are resolved when the same item is written in multiple regions.
///
/// # Example
///
/// ```
/// use azure_data_cosmos_driver::models::{ConflictResolutionPolicy, ConflictResolutionMode};
///
/// // Last writer wins based on _ts (default)
/// let policy = ConflictResolutionPolicy::last_writer_wins();
///
/// // Last writer wins based on a custom path
/// let policy = ConflictResolutionPolicy::last_writer_wins_with_path("/modifiedTime");
///
/// // Custom conflict resolution via stored procedure
/// let policy = ConflictResolutionPolicy::custom("/dbs/mydb/colls/mycoll/sprocs/resolver");
/// ```
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ConflictResolutionPolicy {
    /// The conflict resolution mode.
    pub mode: ConflictResolutionMode,

    /// Path used for last-writer-wins resolution (e.g., `"/_ts"` or `"/modifiedTime"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_resolution_path: Option<Cow<'static, str>>,

    /// Full path to stored procedure for custom resolution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_resolution_procedure: Option<Cow<'static, str>>,
}

impl ConflictResolutionPolicy {
    /// Creates a last-writer-wins policy using the default `/_ts` path.
    pub fn last_writer_wins() -> Self {
        Self {
            mode: ConflictResolutionMode::LastWriterWins,
            conflict_resolution_path: None,
            conflict_resolution_procedure: None,
        }
    }

    /// Creates a last-writer-wins policy using a custom path.
    pub fn last_writer_wins_with_path(path: impl Into<Cow<'static, str>>) -> Self {
        Self {
            mode: ConflictResolutionMode::LastWriterWins,
            conflict_resolution_path: Some(path.into()),
            conflict_resolution_procedure: None,
        }
    }

    /// Creates a custom conflict resolution policy using a stored procedure.
    pub fn custom(procedure_path: impl Into<Cow<'static, str>>) -> Self {
        Self {
            mode: ConflictResolutionMode::Custom,
            conflict_resolution_path: None,
            conflict_resolution_procedure: Some(procedure_path.into()),
        }
    }
}

/// Conflict resolution mode.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ConflictResolutionMode {
    /// Last writer wins based on a timestamp path.
    #[default]
    LastWriterWins,
    /// Custom resolution via stored procedure.
    Custom,
}

// =============================================================================
// ComputedProperty
// =============================================================================

/// A computed property definition for a container.
///
/// Computed properties are calculated from other properties using a SQL query.
///
/// # Example
///
/// ```
/// use azure_data_cosmos_driver::models::ComputedProperty;
///
/// let prop = ComputedProperty::new(
///     "fullName",
///     "SELECT VALUE CONCAT(c.firstName, ' ', c.lastName) FROM c",
/// );
/// ```
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct ComputedProperty {
    /// Name of the computed property.
    pub name: Cow<'static, str>,
    /// SQL query that computes the property value.
    pub query: Cow<'static, str>,
}

impl ComputedProperty {
    /// Creates a new computed property.
    pub fn new(
        name: impl Into<Cow<'static, str>>,
        query: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            name: name.into(),
            query: query.into(),
        }
    }
}

// =============================================================================
// VectorEmbeddingPolicy
// =============================================================================

/// Vector embedding policy for vector search in a container.
///
/// # Example
///
/// ```
/// use azure_data_cosmos_driver::models::{
///     VectorEmbeddingPolicy, VectorEmbedding, VectorDataType, VectorDistanceFunction,
/// };
///
/// let policy = VectorEmbeddingPolicy::new(vec![
///     VectorEmbedding::new(
///         "/embedding",
///         VectorDataType::Float32,
///         1536,
///         VectorDistanceFunction::Cosine,
///     ),
/// ]);
/// ```
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct VectorEmbeddingPolicy {
    /// Vector embedding definitions.
    #[serde(rename = "vectorEmbeddings")]
    pub embeddings: Vec<VectorEmbedding>,
}

impl VectorEmbeddingPolicy {
    /// Creates a new vector embedding policy with the given embeddings.
    pub fn new(embeddings: Vec<VectorEmbedding>) -> Self {
        Self { embeddings }
    }
}

/// A vector embedding definition within a container.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct VectorEmbedding {
    /// Path to the vector field (e.g., `"/embedding"`).
    pub path: Cow<'static, str>,

    /// Data type of vector components.
    #[serde(rename = "dataType")]
    pub data_type: VectorDataType,

    /// Number of dimensions in the vector.
    pub dimensions: u32,

    /// Distance function for similarity comparisons.
    #[serde(rename = "distanceFunction")]
    pub distance_function: VectorDistanceFunction,
}

impl VectorEmbedding {
    /// Creates a new vector embedding definition.
    pub fn new(
        path: impl Into<Cow<'static, str>>,
        data_type: VectorDataType,
        dimensions: u32,
        distance_function: VectorDistanceFunction,
    ) -> Self {
        Self {
            path: path.into(),
            data_type,
            dimensions,
            distance_function,
        }
    }
}

/// Data type for vector components.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VectorDataType {
    /// 8-bit signed integer.
    #[serde(rename = "int8")]
    Int8,
    /// 8-bit unsigned integer.
    #[serde(rename = "uint8")]
    Uint8,
    /// 16-bit floating point.
    #[serde(rename = "float16")]
    Float16,
    /// 32-bit floating point (default).
    #[default]
    #[serde(rename = "float32")]
    Float32,
}

/// Distance function for vector similarity.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VectorDistanceFunction {
    /// Euclidean (L2) distance.
    #[serde(rename = "euclidean")]
    Euclidean,
    /// Cosine similarity (default for normalized vectors).
    #[default]
    #[serde(rename = "cosine")]
    Cosine,
    /// Dot product.
    #[serde(rename = "dotproduct")]
    DotProduct,
}

// =============================================================================
// FullTextPolicy
// =============================================================================

/// Full-text search policy for a container.
///
/// # Example
///
/// ```
/// use azure_data_cosmos_driver::models::{FullTextPolicy, FullTextPath};
///
/// let policy = FullTextPolicy::new(
///     Some("en-US"),
///     vec![FullTextPath::new("/description", "en-US")],
/// );
/// ```
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct FullTextPolicy {
    /// Default language for full-text search (e.g., `"en-US"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_language: Option<Cow<'static, str>>,

    /// Paths configured for full-text search.
    #[serde(rename = "fullTextPaths")]
    pub paths: Vec<FullTextPath>,
}

impl FullTextPolicy {
    /// Creates a new full-text search policy.
    pub fn new(
        default_language: Option<impl Into<Cow<'static, str>>>,
        paths: Vec<FullTextPath>,
    ) -> Self {
        Self {
            default_language: default_language.map(Into::into),
            paths,
        }
    }
}

/// A path configured for full-text search.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct FullTextPath {
    /// Path to the text field (e.g., `"/description"`).
    pub path: Cow<'static, str>,

    /// Language for text analysis (e.g., `"en-US"`).
    pub language: Cow<'static, str>,
}

impl FullTextPath {
    /// Creates a new full-text path.
    pub fn new(
        path: impl Into<Cow<'static, str>>,
        language: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            path: path.into(),
            language: language.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_to_live_off_serializes_to_none() {
        let ttl = TimeToLive::Off;
        let json = serde_json::to_string(&ttl).unwrap();
        assert_eq!(json, "null");
    }

    #[test]
    fn time_to_live_inherit_serializes_to_minus_one() {
        let ttl = TimeToLive::InheritFromItem;
        let json = serde_json::to_string(&ttl).unwrap();
        assert_eq!(json, "-1");
    }

    #[test]
    fn time_to_live_seconds_serializes_to_number() {
        let ttl = TimeToLive::seconds(3600);
        let json = serde_json::to_string(&ttl).unwrap();
        assert_eq!(json, "3600");
    }

    #[test]
    fn time_to_live_deserializes_from_null() {
        let ttl: TimeToLive = serde_json::from_str("null").unwrap();
        assert_eq!(ttl, TimeToLive::Off);
    }

    #[test]
    fn time_to_live_deserializes_from_minus_one() {
        let ttl: TimeToLive = serde_json::from_str("-1").unwrap();
        assert_eq!(ttl, TimeToLive::InheritFromItem);
    }

    #[test]
    fn time_to_live_deserializes_from_positive() {
        let ttl: TimeToLive = serde_json::from_str("7200").unwrap();
        assert_eq!(ttl, TimeToLive::Seconds(7200));
    }

    #[test]
    fn change_feed_policy_latest_version() {
        let policy = ChangeFeedPolicy::latest_version();
        let json = serde_json::to_string(&policy).unwrap();
        assert!(json.contains("null") || !json.contains("retentionDurationMinutes"));
    }

    #[test]
    fn change_feed_policy_all_versions() {
        let policy = ChangeFeedPolicy::all_versions_and_deletes_minutes(60);
        let json = serde_json::to_string(&policy).unwrap();
        assert!(json.contains("60"));
    }

    #[test]
    fn vector_data_type_serializes_lowercase() {
        let dt = VectorDataType::Float32;
        let json = serde_json::to_string(&dt).unwrap();
        assert_eq!(json, "\"float32\"");
    }

    #[test]
    fn vector_distance_function_serializes_lowercase() {
        let df = VectorDistanceFunction::DotProduct;
        let json = serde_json::to_string(&df).unwrap();
        assert_eq!(json, "\"dotproduct\"");
    }
}
