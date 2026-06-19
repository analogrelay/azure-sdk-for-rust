// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use std::borrow::Cow;

use azure_core::fmt::SafeDebug;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    models::PartitionKeyDefinition,
    models::{IndexingPolicy, SystemProperties},
};

/// Time-to-live settings for a container.
///
/// Azure Cosmos DB supports three TTL states:
/// - **Forever**: TTL is disabled and items do not expire. This is the default.
/// - **NoDefault**: TTL is enabled for the container, but items do not get a
///   default expiration. Individual items can still set their own `ttl` value.
///   On the wire, this is `-1`.
/// - **Seconds**: TTL is enabled with a default expiration in seconds. Items
///   expire after that many seconds unless they override it with their own
///   `ttl` value.
///
/// For more information, see <https://learn.microsoft.com/azure/cosmos-db/time-to-live#time-to-live-configurations>.
#[derive(Clone, Default, SafeDebug, PartialEq, Eq)]
#[safe(true)]
#[non_exhaustive]
pub enum TimeToLive {
    /// TTL is disabled; items never expire.
    #[default]
    Forever,

    /// TTL is enabled, but items have no default expiration.
    ///
    /// Individual items can still define their own TTL.
    NoDefault,

    /// TTL is enabled with a default expiration of the given number of seconds.
    Seconds(u32),
}

impl TimeToLive {
    /// Returns `true` if TTL is [`Forever`](TimeToLive::Forever).
    pub fn is_forever(&self) -> bool {
        matches!(self, TimeToLive::Forever)
    }
}

impl From<u32> for TimeToLive {
    fn from(n: u32) -> Self {
        TimeToLive::Seconds(n)
    }
}

impl Serialize for TimeToLive {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            TimeToLive::Forever => serializer.serialize_none(),
            TimeToLive::NoDefault => serializer.serialize_i32(-1),
            TimeToLive::Seconds(n) => serializer.serialize_u32(*n),
        }
    }
}

impl<'de> Deserialize<'de> for TimeToLive {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Option::<i32>::deserialize(deserializer)? {
            None => Ok(TimeToLive::Forever),
            Some(-1) => Ok(TimeToLive::NoDefault),
            Some(n) if n > 0 => Ok(TimeToLive::Seconds(n as u32)),
            Some(n) => Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Signed(n as i64),
                &"a nonzero positive integer or -1",
            )),
        }
    }
}

/// Properties that define a container.
///
/// This includes the container ID, partition key definition, indexing policy,
/// uniqueness settings, conflict resolution, vector settings, and time-to-live
/// values.
#[derive(Clone, SafeDebug, Deserialize, Serialize, PartialEq, Eq)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ContainerProperties {
    /// The ID of the container.
    pub id: Cow<'static, str>,

    /// The definition of the partition key for the container.
    pub partition_key: PartitionKeyDefinition,

    /// The indexing policy for the container.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexing_policy: Option<IndexingPolicy>,

    /// The unique key policy for the container.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_key_policy: Option<UniqueKeyPolicy>,

    /// The conflict resolution policy for the container.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_resolution_policy: Option<ConflictResolutionPolicy>,

    /// The vector embedding policy for the container.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_embedding_policy: Option<VectorEmbeddingPolicy>,

    /// The time-to-live for items in the container.
    ///
    /// For more information see <https://learn.microsoft.com/azure/cosmos-db/time-to-live#time-to-live-configurations>
    #[serde(default)]
    #[serde(skip_serializing_if = "TimeToLive::is_forever")]
    pub default_ttl: TimeToLive,

    /// The time-to-live for the analytical store in the container.
    ///
    /// For more information see <https://learn.microsoft.com/azure/cosmos-db/analytical-store-introduction#analytical-ttl>
    #[serde(default)]
    #[serde(skip_serializing_if = "TimeToLive::is_forever")]
    pub analytical_storage_ttl: TimeToLive,

    /// A [`SystemProperties`] object containing common system properties for the container.
    #[serde(flatten)]
    pub system_properties: SystemProperties,
}

impl ContainerProperties {
    /// Creates container properties with the required ID and partition key.
    pub fn new(id: impl Into<Cow<'static, str>>, partition_key: PartitionKeyDefinition) -> Self {
        Self {
            id: id.into(),
            partition_key,
            indexing_policy: None,
            unique_key_policy: None,
            conflict_resolution_policy: None,
            vector_embedding_policy: None,
            default_ttl: TimeToLive::Forever,
            analytical_storage_ttl: TimeToLive::Forever,
            system_properties: SystemProperties::default(),
        }
    }

    /// Sets the container's indexing policy.
    pub fn with_indexing_policy(mut self, indexing_policy: IndexingPolicy) -> Self {
        self.indexing_policy = Some(indexing_policy);
        self
    }

    /// Sets the container's unique key policy.
    pub fn with_unique_key_policy(mut self, unique_key_policy: UniqueKeyPolicy) -> Self {
        self.unique_key_policy = Some(unique_key_policy);
        self
    }

    /// Sets the container's conflict resolution policy.
    pub fn with_conflict_resolution_policy(
        mut self,
        conflict_resolution_policy: ConflictResolutionPolicy,
    ) -> Self {
        self.conflict_resolution_policy = Some(conflict_resolution_policy);
        self
    }

    /// Sets the container's vector embedding policy.
    pub fn with_vector_embedding_policy(
        mut self,
        vector_embedding_policy: VectorEmbeddingPolicy,
    ) -> Self {
        self.vector_embedding_policy = Some(vector_embedding_policy);
        self
    }

    /// Sets the default time to live for items in the container.
    pub fn with_default_ttl(mut self, default_ttl: impl Into<TimeToLive>) -> Self {
        self.default_ttl = default_ttl.into();
        self
    }

    /// Sets the time to live for the analytical store.
    pub fn with_analytical_storage_ttl(
        mut self,
        analytical_storage_ttl: impl Into<TimeToLive>,
    ) -> Self {
        self.analytical_storage_ttl = analytical_storage_ttl.into();
        self
    }
}

/// Vector embedding settings for a container.
#[derive(Clone, Default, SafeDebug, Deserialize, Serialize, PartialEq, Eq)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct VectorEmbeddingPolicy {
    /// The vector embeddings defined for items in the container.
    #[serde(rename = "vectorEmbeddings")]
    pub embeddings: Vec<VectorEmbedding>,
}

impl VectorEmbeddingPolicy {
    /// Adds a vector embedding to the policy.
    pub fn with_embedding(mut self, embedding: VectorEmbedding) -> Self {
        self.embeddings.push(embedding);
        self
    }
}

/// A vector embedding definition for items in a container.
#[derive(Clone, SafeDebug, Deserialize, Serialize, PartialEq, Eq)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct VectorEmbedding {
    /// The path to the property containing the vector.
    pub path: String,

    /// The data type of the elements stored in the vector.
    pub data_type: VectorDataType,

    /// The number of dimensions in the vector.
    pub dimensions: u32,

    /// The [`VectorDistanceFunction`] used to calculate the distance between vectors.
    pub distance_function: VectorDistanceFunction,
}

impl VectorEmbedding {
    /// Creates a vector embedding definition.
    pub fn new(
        path: impl Into<String>,
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

    /// Sets the property path for this embedding.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Sets the vector element type.
    pub fn with_data_type(mut self, data_type: VectorDataType) -> Self {
        self.data_type = data_type;
        self
    }

    /// Sets the number of vector dimensions.
    pub fn with_dimensions(mut self, dimensions: u32) -> Self {
        self.dimensions = dimensions;
        self
    }

    /// Sets the distance function for similarity comparisons.
    pub fn with_distance_function(mut self, distance_function: VectorDistanceFunction) -> Self {
        self.distance_function = distance_function;
        self
    }
}

/// Data types supported for vector elements.
#[derive(Clone, SafeDebug, Deserialize, Serialize, PartialEq, Eq)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum VectorDataType {
    /// 16-bit floating-point values.
    Float16,

    /// 32-bit floating-point values.
    Float32,

    /// Unsigned 8-bit integer values.
    Uint8,

    /// Signed 8-bit integer values.
    Int8,
}

/// Distance functions used to compare vectors.
#[derive(Clone, SafeDebug, Deserialize, Serialize, PartialEq, Eq)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum VectorDistanceFunction {
    /// Euclidean distance.
    Euclidean,

    /// Cosine distance.
    Cosine,

    /// Dot product distance.
    #[serde(rename = "dotproduct")]
    DotProduct,
}

/// Unique key settings for a container.
///
/// For more information, see <https://learn.microsoft.com/azure/cosmos-db/unique-keys>.
#[derive(Clone, Default, SafeDebug, Deserialize, Serialize, PartialEq, Eq)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct UniqueKeyPolicy {
    /// The keys defined in this policy.
    pub unique_keys: Vec<UniqueKey>,
}

impl UniqueKeyPolicy {
    /// Adds a unique key to the policy.
    pub fn with_unique_key(mut self, unique_key: UniqueKey) -> Self {
        self.unique_keys.push(unique_key);
        self
    }
}

/// A unique key definition for a container.
#[derive(Clone, Default, SafeDebug, Deserialize, Serialize, PartialEq, Eq)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct UniqueKey {
    /// The set of paths which must be unique for each item.
    pub paths: Vec<String>,
}

impl UniqueKey {
    /// Adds a property path to the unique key.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.paths.push(path.into());
        self
    }
}

/// Conflict resolution settings for a container.
///
/// For more information, see <https://learn.microsoft.com/azure/cosmos-db/conflict-resolution-policies>.
#[derive(Clone, SafeDebug, Deserialize, Serialize, PartialEq, Eq)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ConflictResolutionPolicy {
    /// The conflict resolution mode.
    pub mode: ConflictResolutionMode,

    /// The item property used for [`ConflictResolutionMode::LastWriterWins`].
    #[serde(rename = "conflictResolutionPath")]
    pub resolution_path: String,

    /// The stored procedure path used for [`ConflictResolutionMode::Custom`].
    #[serde(rename = "conflictResolutionProcedure")]
    pub resolution_procedure: String,
}

impl ConflictResolutionPolicy {
    /// Creates a conflict resolution policy for the given mode.
    ///
    /// The resolution path and stored procedure path start out empty. Set the
    /// one that applies to the selected mode with
    /// [`with_resolution_path`](Self::with_resolution_path) or
    /// [`with_resolution_procedure`](Self::with_resolution_procedure).
    pub fn new(mode: ConflictResolutionMode) -> Self {
        Self {
            mode,
            resolution_path: String::new(),
            resolution_procedure: String::new(),
        }
    }

    /// Sets the item property used for [`ConflictResolutionMode::LastWriterWins`].
    pub fn with_resolution_path(mut self, resolution_path: impl Into<String>) -> Self {
        self.resolution_path = resolution_path.into();
        self
    }

    /// Sets the stored procedure path used for [`ConflictResolutionMode::Custom`].
    pub fn with_resolution_procedure(mut self, resolution_procedure: impl Into<String>) -> Self {
        self.resolution_procedure = resolution_procedure.into();
        self
    }
}

/// Conflict resolution modes supported by Azure Cosmos DB.
#[derive(Clone, SafeDebug, Deserialize, Serialize, PartialEq, Eq)]
#[safe(true)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub enum ConflictResolutionMode {
    /// Resolves conflicts by choosing the highest value from the property named by [`ConflictResolutionPolicy::resolution_path`].
    LastWriterWins,

    /// Resolves conflicts by running the stored procedure named by [`ConflictResolutionPolicy::resolution_procedure`].
    Custom,
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::TimeToLive;
    use crate::models::ContainerProperties;

    #[derive(Debug, Deserialize, Serialize)]
    struct TtlHolder {
        #[serde(default)]
        #[serde(skip_serializing_if = "TimeToLive::is_forever")]
        pub ttl: TimeToLive,
    }

    #[test]
    fn serialize_ttl_seconds() {
        let value = TtlHolder {
            ttl: TimeToLive::Seconds(4200),
        };
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(r#"{"ttl":4200}"#, json);
    }

    #[test]
    fn serialize_ttl_forever() {
        let value = TtlHolder {
            ttl: TimeToLive::Forever,
        };
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(r#"{}"#, json);
    }

    #[test]
    fn serialize_ttl_no_default() {
        let value = TtlHolder {
            ttl: TimeToLive::NoDefault,
        };
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(r#"{"ttl":-1}"#, json);
    }

    #[test]
    fn deserialize_ttl_seconds() {
        let value: TtlHolder = serde_json::from_str(r#"{"ttl":4200}"#).unwrap();
        assert_eq!(TimeToLive::Seconds(4200), value.ttl);
    }

    #[test]
    fn deserialize_ttl_missing() {
        let value: TtlHolder = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(TimeToLive::Forever, value.ttl);
    }

    #[test]
    fn deserialize_ttl_null() {
        let value: TtlHolder = serde_json::from_str(r#"{"ttl":null}"#).unwrap();
        assert_eq!(TimeToLive::Forever, value.ttl);
    }

    #[test]
    fn deserialize_ttl_negative_one() {
        let value: TtlHolder = serde_json::from_str(r#"{"ttl":-1}"#).unwrap();
        assert_eq!(TimeToLive::NoDefault, value.ttl);
    }

    #[test]
    fn deserialize_ttl_zero() {
        let result = serde_json::from_str::<TtlHolder>(r#"{"ttl":0}"#);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_ttl_invalid_negative() {
        let result = serde_json::from_str::<TtlHolder>(r#"{"ttl":-2}"#);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_ttl_overflow() {
        let result = serde_json::from_str::<TtlHolder>(r#"{"ttl":2147483648}"#);
        assert!(result.is_err());
    }

    #[test]
    fn serialize_ttl_seconds_value() {
        let json = serde_json::to_string(&TimeToLive::Seconds(86400)).unwrap();
        assert_eq!("86400", json);
    }

    #[test]
    fn serialize_ttl_no_default_value() {
        let json = serde_json::to_string(&TimeToLive::NoDefault).unwrap();
        assert_eq!("-1", json);
    }

    #[test]
    fn serialize_ttl_forever_value() {
        let json = serde_json::to_string(&TimeToLive::Forever).unwrap();
        assert_eq!("null", json);
    }

    #[test]
    fn deserialize_container_properties_with_ttl_negative_one() {
        let json = r#"{
            "id": "MyContainer",
            "partitionKey": {"paths": ["/pk"], "kind": "Hash", "version": 2},
            "defaultTtl": -1
        }"#;
        let props: ContainerProperties = serde_json::from_str(json).unwrap();
        assert_eq!(TimeToLive::NoDefault, props.default_ttl);
        assert_eq!(TimeToLive::Forever, props.analytical_storage_ttl);
    }

    #[test]
    fn deserialize_container_properties_with_ttl_seconds() {
        let json = r#"{
            "id": "MyContainer",
            "partitionKey": {"paths": ["/pk"], "kind": "Hash", "version": 2},
            "defaultTtl": 3600,
            "analyticalStorageTtl": -1
        }"#;
        let props: ContainerProperties = serde_json::from_str(json).unwrap();
        assert_eq!(TimeToLive::Seconds(3600), props.default_ttl);
        assert_eq!(TimeToLive::NoDefault, props.analytical_storage_ttl);
    }

    #[test]
    pub fn container_properties_default_serialization() {
        // This test asserts that the default value serializes the same way across SDK versions.
        // When new properties are added to ContainerProperties, this test should not break.
        // If it does, users may start sending an unexpected payload to the server.
        // In rare cases, it's reasonable to update this test, if the new generated JSON is considered _equivalent_ to the original by the server.
        // But in general, a failure in this test means that the same user code will send an unexpected value in a new version of the SDK.
        let properties = ContainerProperties::new("MyContainer", "/partitionKey".into());
        let json = serde_json::to_string(&properties).unwrap();

        assert_eq!(
            "{\"id\":\"MyContainer\",\"partitionKey\":{\"paths\":[\"/partitionKey\"],\"kind\":\"Hash\",\"version\":2}}",
            json
        );
    }
}
