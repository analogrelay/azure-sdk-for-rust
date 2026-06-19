// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use azure_core::fmt::SafeDebug;
use serde::{Deserialize, Serialize};

/// Indexing settings for a container.
///
/// For more information, see <https://learn.microsoft.com/azure/cosmos-db/index-policy>.
#[derive(Clone, Default, SafeDebug, Deserialize, Serialize, PartialEq, Eq)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct IndexingPolicy {
    /// Indicates that the indexing policy is automatic.
    #[serde(default)]
    pub automatic: bool,

    /// The indexing mode in use.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexing_mode: Option<IndexingMode>,

    /// The paths to be indexed.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub included_paths: Vec<PropertyPath>,

    /// The paths to be excluded.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub excluded_paths: Vec<PropertyPath>,

    /// A list of spatial indexes in the container.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub spatial_indexes: Vec<SpatialIndex>,

    /// The composite indexes defined for the container.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub composite_indexes: Vec<CompositeIndex>,

    /// The vector indexes defined for the container.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub vector_indexes: Vec<VectorIndex>,
}

impl IndexingPolicy {
    /// Sets the indexing mode.
    pub fn with_indexing_mode(mut self, indexing_mode: IndexingMode) -> Self {
        self.indexing_mode = Some(indexing_mode);
        self
    }

    /// Adds an included property path.
    pub fn with_included_path(mut self, included_path: impl Into<PropertyPath>) -> Self {
        self.included_paths.push(included_path.into());
        self
    }

    /// Adds an excluded property path.
    pub fn with_excluded_path(mut self, excluded_path: impl Into<PropertyPath>) -> Self {
        self.excluded_paths.push(excluded_path.into());
        self
    }

    /// Adds a spatial index.
    pub fn with_spatial_index(mut self, spatial_index: SpatialIndex) -> Self {
        self.spatial_indexes.push(spatial_index);
        self
    }

    /// Adds a composite index.
    pub fn with_composite_index(mut self, composite_index: CompositeIndex) -> Self {
        self.composite_indexes.push(composite_index);
        self
    }

    /// Adds a vector index.
    pub fn with_vector_index(mut self, vector_index: VectorIndex) -> Self {
        self.vector_indexes.push(vector_index);
        self
    }
}

/// Defines the indexing modes supported by Azure Cosmos DB.
#[derive(Clone, SafeDebug, Deserialize, Serialize, PartialEq, Eq)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum IndexingMode {
    /// Updates indexes synchronously as items are written.
    Consistent,
    /// Disables indexing.
    None,
}

/// A property path used in an indexing policy.
#[derive(Clone, Default, SafeDebug, Deserialize, Serialize, PartialEq, Eq)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PropertyPath {
    /// The path to the indexed property.
    pub path: String,
}

impl PropertyPath {
    /// Sets the property path.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }
}

impl<T: Into<String>> From<T> for PropertyPath {
    fn from(value: T) -> Self {
        PropertyPath { path: value.into() }
    }
}

/// A spatial index definition.
#[derive(Clone, SafeDebug, Deserialize, Serialize, PartialEq, Eq)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SpatialIndex {
    /// The path to the property referenced in this index.
    pub path: String,

    /// The spatial types indexed at this path.
    pub types: Vec<SpatialType>,
}

impl SpatialIndex {
    /// Creates a spatial index for the given path.
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            types: Vec::new(),
        }
    }

    /// Adds a spatial type to the index.
    pub fn with_type(mut self, spatial_type: SpatialType) -> Self {
        self.types.push(spatial_type);
        self
    }
}

/// Defines the types of spatial data that can be indexed.
#[derive(Clone, SafeDebug, Deserialize, Serialize, PartialEq, Eq)]
#[safe(true)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub enum SpatialType {
    /// A point value.
    Point,
    /// A polygon value.
    Polygon,
    /// A line string value.
    LineString,
    /// A multi-polygon value.
    MultiPolygon,
}

/// A composite index definition.
#[derive(Clone, Default, SafeDebug, Deserialize, Serialize, PartialEq, Eq)]
#[safe(true)]
#[serde(transparent)]
#[non_exhaustive]
pub struct CompositeIndex {
    /// The indexed properties, in order.
    pub properties: Vec<CompositeIndexProperty>,
}

impl CompositeIndex {
    /// Adds a property to the composite index.
    pub fn with_property(mut self, property: CompositeIndexProperty) -> Self {
        self.properties.push(property);
        self
    }
}

/// Describes a single property in a composite index.
#[derive(Clone, SafeDebug, Deserialize, Serialize, PartialEq, Eq)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CompositeIndexProperty {
    /// The path to the property referenced in this index.
    pub path: String,

    /// The sort order for this property in the composite index.
    pub order: CompositeIndexOrder,
}

impl CompositeIndexProperty {
    /// Creates a composite index property.
    pub fn new(path: impl Into<String>, order: CompositeIndexOrder) -> Self {
        Self {
            path: path.into(),
            order,
        }
    }

    /// Sets the property path.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Sets the sort order.
    pub fn with_order(mut self, order: CompositeIndexOrder) -> Self {
        self.order = order;
        self
    }
}

/// Ordering values available for composite indexes.
#[derive(Clone, SafeDebug, Deserialize, Serialize, PartialEq, Eq)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum CompositeIndexOrder {
    /// Sorts this property in ascending order.
    Ascending,
    /// Sorts this property in descending order.
    Descending,
}

/// A vector index definition.
///
/// For more information, see <https://learn.microsoft.com/azure/cosmos-db/index-policy#vector-indexes>.
#[derive(Clone, SafeDebug, Deserialize, Serialize, PartialEq, Eq)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct VectorIndex {
    /// The path to the property referenced in this index.
    pub path: String,

    /// The type of the vector index.
    #[serde(rename = "type")] // "type" is a reserved word in Rust.
    pub index_type: VectorIndexType,
}

impl VectorIndex {
    /// Creates a vector index definition.
    pub fn new(path: impl Into<String>, index_type: VectorIndexType) -> Self {
        Self {
            path: path.into(),
            index_type,
        }
    }

    /// Sets the property path.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Sets the vector index type.
    pub fn with_index_type(mut self, index_type: VectorIndexType) -> Self {
        self.index_type = index_type;
        self
    }
}

/// Vector index types supported by Azure Cosmos DB.
#[derive(Clone, SafeDebug, Deserialize, Serialize, PartialEq, Eq)]
#[safe(true)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum VectorIndexType {
    /// A flat vector index.
    Flat,

    /// A quantized flat vector index.
    QuantizedFlat,

    /// A DiskANN vector index.
    DiskANN,
}

#[cfg(test)]
mod tests {
    use crate::models::{
        CompositeIndex, CompositeIndexOrder, CompositeIndexProperty, IndexingMode, IndexingPolicy,
        PropertyPath, SpatialIndex, SpatialType, VectorIndex, VectorIndexType,
    };

    #[test]
    pub fn deserialize_indexing_policy() {
        // A fairly complete deserialization test that covers most of the indexing policies described in our docs.
        let policy = r#"
            {
                "indexingMode": "consistent",
                "includedPaths": [
                    {
                        "path": "/*"
                    }
                ],
                "excludedPaths": [
                    {
                        "path": "/path/to/single/excluded/property/?"
                    },
                    {
                        "path": "/path/to/root/of/multiple/excluded/properties/*"
                    }
                ],
                "spatialIndexes": [
                    {
                        "path": "/path/to/geojson/property/?",
                        "types": [
                            "Point",
                            "Polygon",
                            "MultiPolygon",
                            "LineString"
                        ]
                    }
                ],
                "vectorIndexes": [
                    {
                        "path": "/vector1",
                        "type": "quantizedFlat"
                    },
                    {
                        "path": "/vector2",
                        "type": "diskANN"
                    }
                ],
                "compositeIndexes":[
                    [
                        {
                            "path":"/name",
                            "order":"ascending"
                        },
                        {
                            "path":"/age",
                            "order":"descending"
                        }
                    ],
                    [
                        {
                            "path":"/name2",
                            "order":"descending"
                        },
                        {
                            "path":"/age2",
                            "order":"ascending"
                        }
                    ]
                ],
                "extraValueNotCurrentlyPresentInModel": {
                    "this": "should not fail"
                }
            }
        "#;

        let policy: IndexingPolicy = serde_json::from_str(policy).unwrap();

        assert_eq!(
            IndexingPolicy {
                automatic: false,
                indexing_mode: Some(IndexingMode::Consistent),
                included_paths: vec![PropertyPath {
                    path: "/*".to_string(),
                }],
                excluded_paths: vec![
                    PropertyPath {
                        path: "/path/to/single/excluded/property/?".to_string()
                    },
                    PropertyPath {
                        path: "/path/to/root/of/multiple/excluded/properties/*".to_string()
                    },
                ],
                spatial_indexes: vec![SpatialIndex {
                    path: "/path/to/geojson/property/?".to_string(),
                    types: vec![
                        SpatialType::Point,
                        SpatialType::Polygon,
                        SpatialType::MultiPolygon,
                        SpatialType::LineString,
                    ]
                }],
                composite_indexes: vec![
                    CompositeIndex {
                        properties: vec![
                            CompositeIndexProperty {
                                path: "/name".to_string(),
                                order: CompositeIndexOrder::Ascending,
                            },
                            CompositeIndexProperty {
                                path: "/age".to_string(),
                                order: CompositeIndexOrder::Descending,
                            },
                        ]
                    },
                    CompositeIndex {
                        properties: vec![
                            CompositeIndexProperty {
                                path: "/name2".to_string(),
                                order: CompositeIndexOrder::Descending,
                            },
                            CompositeIndexProperty {
                                path: "/age2".to_string(),
                                order: CompositeIndexOrder::Ascending,
                            },
                        ]
                    },
                ],
                vector_indexes: vec![
                    VectorIndex {
                        path: "/vector1".to_string(),
                        index_type: VectorIndexType::QuantizedFlat,
                    },
                    VectorIndex {
                        path: "/vector2".to_string(),
                        index_type: VectorIndexType::DiskANN,
                    }
                ]
            },
            policy
        );
    }

    #[test]
    pub fn serialize_indexing_policy() {
        let policy = IndexingPolicy {
            automatic: true,
            indexing_mode: None,
            included_paths: vec![PropertyPath {
                path: "/*".to_string(),
            }],
            excluded_paths: vec![
                PropertyPath {
                    path: "/path/to/single/excluded/property/?".to_string(),
                },
                PropertyPath {
                    path: "/path/to/root/of/multiple/excluded/properties/*".to_string(),
                },
            ],
            spatial_indexes: vec![
                SpatialIndex {
                    path: "/path/to/geojson/property/?".to_string(),
                    types: vec![
                        SpatialType::Point,
                        SpatialType::Polygon,
                        SpatialType::MultiPolygon,
                        SpatialType::LineString,
                    ],
                },
                SpatialIndex {
                    path: "/path/to/geojson/property2/?".to_string(),
                    types: vec![],
                },
            ],
            composite_indexes: vec![
                CompositeIndex {
                    properties: vec![
                        CompositeIndexProperty {
                            path: "/name".to_string(),
                            order: CompositeIndexOrder::Ascending,
                        },
                        CompositeIndexProperty {
                            path: "/age".to_string(),
                            order: CompositeIndexOrder::Descending,
                        },
                    ],
                },
                CompositeIndex { properties: vec![] },
            ],
            vector_indexes: vec![
                VectorIndex {
                    path: "/vector1".to_string(),
                    index_type: VectorIndexType::QuantizedFlat,
                },
                VectorIndex {
                    path: "/vector2".to_string(),
                    index_type: VectorIndexType::DiskANN,
                },
            ],
        };
        let json = serde_json::to_string(&policy).unwrap();

        assert_eq!(
            "{\"automatic\":true,\"includedPaths\":[{\"path\":\"/*\"}],\"excludedPaths\":[{\"path\":\"/path/to/single/excluded/property/?\"},{\"path\":\"/path/to/root/of/multiple/excluded/properties/*\"}],\"spatialIndexes\":[{\"path\":\"/path/to/geojson/property/?\",\"types\":[\"Point\",\"Polygon\",\"MultiPolygon\",\"LineString\"]},{\"path\":\"/path/to/geojson/property2/?\",\"types\":[]}],\"compositeIndexes\":[[{\"path\":\"/name\",\"order\":\"ascending\"},{\"path\":\"/age\",\"order\":\"descending\"}],[]],\"vectorIndexes\":[{\"path\":\"/vector1\",\"type\":\"quantizedFlat\"},{\"path\":\"/vector2\",\"type\":\"diskANN\"}]}",
            json
        );
    }
}
