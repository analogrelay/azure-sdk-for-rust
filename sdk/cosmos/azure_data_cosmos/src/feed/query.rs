// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Types for defining queries and choosing their scope.

use azure_data_cosmos_driver::models::{FeedRange, PartitionKey, PartitionKeyDefinition};
use serde::Serialize;

/// Defines which partition or range a query targets.
///
/// Use [`FeedScope::partition`] when you know the logical partition key.
/// Use [`FeedScope::range`] or [`FeedScope::full_container`] when you need to
/// query a broader range.
#[derive(Clone)]
#[non_exhaustive]
pub enum FeedScope {
    Partition(PartitionKey),
    Range(FeedRange),
}

impl FeedScope {
    /// Creates a scope for a single logical partition.
    ///
    /// The partition key must include every level of a hierarchical partition
    /// key. Use [`FeedScope::range`] to target anything broader than one
    /// logical partition.
    pub fn partition(pk: impl Into<PartitionKey>) -> Self {
        Self::Partition(pk.into())
    }

    /// Creates a scope from a [`FeedRange`].
    ///
    /// A range can cover one physical partition or many. Broader ranges can
    /// increase latency and request charges because the query may need to fan
    /// out across more data.
    pub fn range(fr: impl Into<FeedRange>) -> Self {
        Self::Range(fr.into())
    }

    /// Creates a scope for the full container.
    ///
    /// This is a cross-partition query scope and can be more expensive than
    /// targeting a single logical partition.
    pub fn full_container() -> Self {
        Self::Range(FeedRange::full())
    }

    /// Converts this [`FeedScope`] into a [`FeedRange`] that can be used for query execution, using the provided partition key definition to compute effective partition keys as needed.
    pub(crate) fn into_feed_range(
        self,
        partition_key_definition: &PartitionKeyDefinition,
    ) -> FeedRange {
        match self {
            FeedScope::Partition(pk) => FeedRange::for_partition(pk, partition_key_definition),
            FeedScope::Range(fr) => fr,
        }
    }
}

/// A Cosmos DB query and its parameters.
///
/// # Examples
///
/// Start with [`Query::from`] and add parameters with [`Query::with_parameter`].
///
/// ```rust
/// # use azure_data_cosmos::Query;
/// let query = Query::from("SELECT * FROM c WHERE c.id = @customer_id")
///     .with_parameter("@customer_id", 42).unwrap();
/// # assert_eq!(serde_json::to_string(&query).unwrap(), "{\"query\":\"SELECT * FROM c WHERE c.id = @customer_id\",\"parameters\":[{\"name\":\"@customer_id\",\"value\":42}]}");
/// ```
///
/// You can also replace the query text with [`Query::with_text`] or append to
/// it with [`Query::append_text`]:
///
/// ```rust
/// # use azure_data_cosmos::Query;
/// let query = Query::from("SELECT * FROM c")
///     .append_text(" WHERE c.time >= @low_time")
///     .with_parameter("@low_time", "2023-01-01").unwrap()
///     .append_text(" AND c.time <= @high_time")
///     .with_parameter("@high_time", "2023-12-31").unwrap();
/// # // We can't directly access the text field as it's private, but we can serialize to verify
/// # let serialized = serde_json::to_string(&query).unwrap();
/// # assert!(serialized.contains("WHERE c.time >= @low_time AND c.time <= @high_time"));
/// ```
///
/// # Specifying Parameters
///
/// Any JSON-serializable value can be used as a parameter. An empty tuple
/// (`()`) is serialized as `null`.
///
/// [`Query::with_parameter`] is fallible because the value must be serialized
/// before the query can be sent.
///
/// ```rust
/// # use azure_data_cosmos::Query;
/// let query = Query::from("
///     SELECT * FROM c
///     WHERE c.id = @customer_id
///     AND c.name = @customer_name
///     AND c.is_active = @is_active
///     AND c.offer_code = @offer_code")
///     .with_parameter("@customer_id", 42).unwrap()
///     .with_parameter("@customer_name", "Contoso").unwrap()
///     .with_parameter("@is_active", true).unwrap()
///     .with_parameter("@offer_code", ()).unwrap();
/// # assert_eq!(serde_json::to_string(&query).unwrap(), "{\"query\":\"\\n    SELECT * FROM c\\n    WHERE c.id = @customer_id\\n    AND c.name = @customer_name\\n    AND c.is_active = @is_active\\n    AND c.offer_code = @offer_code\",\"parameters\":[{\"name\":\"@customer_id\",\"value\":42},{\"name\":\"@customer_name\",\"value\":\"Contoso\"},{\"name\":\"@is_active\",\"value\":true},{\"name\":\"@offer_code\",\"value\":null}]}");
/// ```
///
/// This includes arrays and objects, if they implement [`serde::Serialize`]:
///
/// ```rust
/// # use azure_data_cosmos::Query;
/// #[derive(serde::Serialize)]
/// struct CustomerInfo {
///     id: u64,
///     name: String
/// }
/// let query = Query::from("
///     SELECT * FROM c
///     WHERE c.id = @customer_info.id
///     AND c.name = @customer_info.name")
///     .with_parameter("@customer_info", CustomerInfo { id: 42, name: "Contoso".into() }).unwrap();
/// # assert_eq!(serde_json::to_string(&query).unwrap(), "{\"query\":\"\\n    SELECT * FROM c\\n    WHERE c.id = @customer_info.id\\n    AND c.name = @customer_info.name\",\"parameters\":[{\"name\":\"@customer_info\",\"value\":{\"id\":42,\"name\":\"Contoso\"}}]}");
/// ```
#[derive(Clone, Debug, Serialize)]
pub struct Query {
    /// The query text itself.
    #[serde(rename = "query")]
    pub(crate) text: String,

    /// A list of parameters used in the query and their associated value.
    #[serde(skip_serializing_if = "Vec::is_empty")] // Don't serialize an empty array.
    parameters: Vec<QueryParameter>,
}

impl Query {
    /// Adds a parameter and returns the updated query.
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be serialized as JSON.
    pub fn with_parameter(
        mut self,
        name: impl Into<String>,
        value: impl Serialize,
    ) -> crate::Result<Self> {
        let parameter = QueryParameter {
            name: name.into(),
            value: serde_json::to_value(value)?,
        };
        self.parameters.push(parameter);

        Ok(self)
    }

    /// Replaces the query text and returns the updated query.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    /// Appends text to the query and returns the updated query.
    pub fn append_text(mut self, text: &str) -> Self {
        self.text.push_str(text);
        self
    }
}

impl<T: Into<String>> From<T> for Query {
    fn from(value: T) -> Self {
        let query = value.into();
        Self {
            text: query,
            parameters: vec![],
        }
    }
}

/// Represents a single parameter in a Cosmos DB query.
#[derive(Clone, Debug, Serialize)]
struct QueryParameter {
    name: String,
    value: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde::Serialize;

    use crate::Query;

    #[test]
    pub fn serialize_query_without_parameters() -> Result<(), Box<dyn Error>> {
        let query: Query = "SELECT * FROM c".into();
        let serialized = serde_json::to_string(&query)?;
        assert_eq!(serialized, r#"{"query":"SELECT * FROM c"}"#);
        Ok(())
    }

    #[test]
    pub fn serialize_query_with_string_parameters() -> Result<(), Box<dyn Error>> {
        let query = Query::from("SELECT * FROM c")
            .with_parameter("name1", "value1")?
            .with_parameter("name2", "value2")?;
        let serialized = serde_json::to_string(&query).unwrap();
        assert_eq!(
            serialized,
            r#"{"query":"SELECT * FROM c","parameters":[{"name":"name1","value":"value1"},{"name":"name2","value":"value2"}]}"#
        );
        Ok(())
    }

    #[test]
    pub fn serialize_query_with_various_parameter_types() -> Result<(), Box<dyn Error>> {
        #[derive(Serialize)]
        struct ObjectParameter {
            name: String,
            value: String,
        }
        let obj_param = ObjectParameter {
            name: "foo".into(),
            value: "bar".into(),
        };
        let null_option: Option<&str> = None;

        let query = Query::from("SELECT * FROM c")
            .with_parameter("string_param", "value1")?
            .with_parameter("int_param", 42)?
            .with_parameter("float_param", 4.2)?
            .with_parameter("bool_param", true)?
            .with_parameter("obj_param", obj_param)?
            .with_parameter("arr_param", ["a", "b", "c"])?
            .with_parameter("null_option", null_option)?
            .with_parameter("null_value", ())?;
        let serialized = serde_json::to_string(&query).unwrap();
        assert_eq!(
            serialized,
            r#"{"query":"SELECT * FROM c","parameters":[{"name":"string_param","value":"value1"},{"name":"int_param","value":42},{"name":"float_param","value":4.2},{"name":"bool_param","value":true},{"name":"obj_param","value":{"name":"foo","value":"bar"}},{"name":"arr_param","value":["a","b","c"]},{"name":"null_option","value":null},{"name":"null_value","value":null}]}"#
        );
        Ok(())
    }

    #[test]
    pub fn with_text_replaces_query_text() {
        let query = Query::from("SELECT * FROM c").with_text("SELECT c.id FROM c".to_string());
        assert_eq!(query.text, "SELECT c.id FROM c");
    }

    #[test]
    pub fn with_text_preserves_parameters() -> Result<(), Box<dyn Error>> {
        let query = Query::from("SELECT * FROM c")
            .with_parameter("@id", 42)?
            .with_text("SELECT c.name FROM c WHERE c.id = @id".to_string());

        assert_eq!(query.text, "SELECT c.name FROM c WHERE c.id = @id");
        assert_eq!(query.parameters.len(), 1);
        assert_eq!(query.parameters[0].name, "@id");
        Ok(())
    }

    #[test]
    pub fn append_text_adds_to_existing_text() {
        let query = Query::from("SELECT * FROM c").append_text(" WHERE c.id = @id");
        assert_eq!(query.text, "SELECT * FROM c WHERE c.id = @id");
    }

    #[test]
    pub fn append_text_preserves_parameters() -> Result<(), Box<dyn Error>> {
        let query = Query::from("SELECT * FROM c")
            .with_parameter("@id", 42)?
            .append_text(" WHERE c.id = @id");

        assert_eq!(query.text, "SELECT * FROM c WHERE c.id = @id");
        assert_eq!(query.parameters.len(), 1);
        assert_eq!(query.parameters[0].name, "@id");
        Ok(())
    }

    #[test]
    pub fn method_chaining_works_with_new_methods() -> Result<(), Box<dyn Error>> {
        let query = Query::from("SELECT * FROM c")
            .append_text(" WHERE c.time >= @low_time")
            .with_parameter("@low_time", "2023-01-01")?
            .append_text(" AND c.time <= @high_time")
            .with_parameter("@high_time", "2023-12-31")?;

        assert_eq!(
            query.text,
            "SELECT * FROM c WHERE c.time >= @low_time AND c.time <= @high_time"
        );
        assert_eq!(query.parameters.len(), 2);
        Ok(())
    }
}
