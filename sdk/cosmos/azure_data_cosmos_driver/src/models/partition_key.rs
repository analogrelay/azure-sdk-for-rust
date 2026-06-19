// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.
#![allow(dead_code)]

//! Partition key types for Cosmos DB operations.
//!
//! Every item in a Cosmos DB container belongs to a logical partition identified
//! by its partition key. The [`PartitionKey`] type represents one or more
//! partition key values used to target operations at a specific partition.
//!
//! # Creating partition keys
//!
//! `PartitionKey` implements [`From`] for common Rust types, so you can pass
//! values directly to any method that accepts `impl Into<PartitionKey>`:
//!
//! ```
//! use azure_data_cosmos_driver::models::PartitionKey;
//!
//! // From a string
//! let pk = PartitionKey::from("my-partition");
//!
//! // From a number
//! let pk = PartitionKey::from(42);
//!
//! // From a boolean
//! let pk = PartitionKey::from(true);
//!
//! // Null partition key (for items where the partition key property is JSON null)
//! let pk = PartitionKey::from(None::<String>);
//! ```
//!
//! # Hierarchical partition keys
//!
//! For containers with hierarchical (multi-level) partition keys, use tuples:
//!
//! ```
//! use azure_data_cosmos_driver::models::PartitionKey;
//!
//! // Two-level key
//! let pk = PartitionKey::from(("tenant-a", "user-123"));
//!
//! // Three-level key (maximum)
//! let pk = PartitionKey::from(("tenant-a", "user-123", 2024));
//! ```
//!
//! You can also build a partition key dynamically from a `Vec<PartitionKeyValue>`:
//!
//! ```
//! use azure_data_cosmos_driver::models::{PartitionKey, PartitionKeyValue};
//!
//! let values = vec![
//!     PartitionKeyValue::from("tenant-a"),
//!     PartitionKeyValue::from(42),
//! ];
//! let pk = PartitionKey::from(values);
//! ```

use crate::models::FiniteF64;
use azure_core::http::headers::{AsHeaders, HeaderName, HeaderValue};
use std::{borrow::Cow, hash::Hash};

/// Header name for partition key.
pub(crate) const PARTITION_KEY: HeaderName =
    HeaderName::from_static("x-ms-documentdb-partitionkey");

/// Header name to enable cross-partition queries.
pub(crate) const QUERY_ENABLE_CROSS_PARTITION: HeaderName =
    HeaderName::from_static("x-ms-documentdb-query-enablecrosspartition");

// =============================================================================
// PartitionKeyValue
// =============================================================================

/// A single component of a [`PartitionKey`].
///
/// You rarely need to construct `PartitionKeyValue` directly — most APIs accept
/// `impl Into<PartitionKey>`, and `PartitionKey` converts from primitives
/// automatically. Use `PartitionKeyValue` when building partition keys
/// dynamically (e.g., from a `Vec`).
///
/// Supported value types (via [`From`] impls):
/// - Strings: `&'static str`, [`String`], `&String`, [`Cow<'static, str>`](std::borrow::Cow)
/// - Numbers: all integer types (`i8`–`i64`, `u8`–`u64`, `isize`, `usize`) and `f32`/`f64`
/// - Booleans: `bool`
/// - Null: `Option<T>` where `None` maps to JSON `null`
///
/// The special [`PartitionKeyValue::NULL`] and [`PartitionKeyValue::UNDEFINED`]
/// constants handle items with explicit `null` or missing partition key properties.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct PartitionKeyValue(InnerPartitionKeyValue);

// We don't want to expose the implementation details of PartitionKeyValue, so we use
// this inner private enum to store the data.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum InnerPartitionKeyValue {
    Null,
    String(Cow<'static, str>),
    Number(FiniteF64),
    Bool(bool),
    /// Undefined sentinel — represents items with no partition key property.
    Undefined,
    /// Infinity sentinel, used only internally for EPK boundary calculations.
    Infinity,
}

/// Maximum number of string bytes to include when hashing (V1 truncation).
const MAX_STRING_BYTES_TO_APPEND: usize = 100;

/// Byte markers for partition key value encoding.
mod component {
    pub const UNDEFINED: u8 = 0x00;
    pub const NULL: u8 = 0x01;
    pub const BOOL_FALSE: u8 = 0x02;
    pub const BOOL_TRUE: u8 = 0x03;
    pub const NUMBER: u8 = 0x05;
    pub const STRING: u8 = 0x08;
    pub const INFINITY: u8 = 0xFF;
}

impl InnerPartitionKeyValue {
    /// Common hashing writer core: writes type marker + payload.
    fn write_for_hashing_core(&self, string_suffix: u8, writer: &mut Vec<u8>, truncate: bool) {
        match self {
            InnerPartitionKeyValue::Bool(true) => writer.push(component::BOOL_TRUE),
            InnerPartitionKeyValue::Bool(false) => writer.push(component::BOOL_FALSE),
            InnerPartitionKeyValue::Null => writer.push(component::NULL),
            InnerPartitionKeyValue::Number(n) => {
                writer.push(component::NUMBER);
                writer.extend_from_slice(&n.value().to_le_bytes());
            }
            InnerPartitionKeyValue::String(s) => {
                writer.push(component::STRING);
                let bytes = s.as_bytes();
                if truncate && bytes.len() > MAX_STRING_BYTES_TO_APPEND {
                    writer.extend_from_slice(&bytes[..MAX_STRING_BYTES_TO_APPEND]);
                } else {
                    writer.extend_from_slice(bytes);
                }
                writer.push(string_suffix);
            }
            InnerPartitionKeyValue::Infinity => writer.push(component::INFINITY),
            InnerPartitionKeyValue::Undefined => writer.push(component::UNDEFINED),
        }
    }
    fn write_for_binary_encoding_v1(&self, writer: &mut Vec<u8>) {
        match self {
            InnerPartitionKeyValue::Bool(true) => writer.push(component::BOOL_TRUE),
            InnerPartitionKeyValue::Bool(false) => writer.push(component::BOOL_FALSE),
            InnerPartitionKeyValue::Infinity => writer.push(component::INFINITY),
            InnerPartitionKeyValue::Number(n) => {
                write_number_v1_binary(n.value(), writer);
            }
            InnerPartitionKeyValue::String(s) => {
                writer.push(component::STRING);
                let utf8 = s.as_bytes();
                let short = utf8.len() <= MAX_STRING_BYTES_TO_APPEND;
                let write_len = if short {
                    utf8.len()
                } else {
                    std::cmp::min(utf8.len(), MAX_STRING_BYTES_TO_APPEND + 1)
                };
                for item in utf8.iter().take(write_len) {
                    writer.push(item.wrapping_add(1));
                }
                if short {
                    writer.push(0x00);
                }
            }
            InnerPartitionKeyValue::Null => writer.push(component::NULL),
            InnerPartitionKeyValue::Undefined => writer.push(component::UNDEFINED),
        }
    }
}

pub(crate) fn encode_double_as_uint64(value: f64) -> u64 {
    let value_in_uint64 = u64::from_le_bytes(value.to_le_bytes());
    let mask: u64 = 0x8000_0000_0000_0000;
    if value_in_uint64 < mask {
        value_in_uint64 ^ mask
    } else {
        (!value_in_uint64).wrapping_add(1)
    }
}

/// Encode a number using V1 binary encoding (variable-length ordering-preserving).
///
/// Shared between [`InnerPartitionKeyValue::write_for_binary_encoding_v1`] and
/// the EPK V1 hash computation in [`effective_partition_key`](super::effective_partition_key).
pub(crate) fn write_number_v1_binary(value: f64, writer: &mut Vec<u8>) {
    writer.push(component::NUMBER);
    let mut payload = encode_double_as_uint64(value);
    writer.push((payload >> 56) as u8);
    payload <<= 8;
    let mut first = true;
    let mut byte_to_write: u8 = 0;
    while payload != 0 {
        if !first {
            writer.push(byte_to_write);
        } else {
            first = false;
        }
        byte_to_write = ((payload >> 56) as u8) | 0x01;
        payload <<= 7;
    }
    writer.push(byte_to_write & 0xFE);
}

impl From<InnerPartitionKeyValue> for PartitionKeyValue {
    fn from(value: InnerPartitionKeyValue) -> Self {
        PartitionKeyValue(value)
    }
}

impl PartitionKeyValue {
    /// The JSON `null` partition key value.
    pub const NULL: Self = Self(InnerPartitionKeyValue::Null);

    /// The partition key value used for items that do not have the partition key property.
    pub const UNDEFINED: Self = Self(InnerPartitionKeyValue::Undefined);

    /// A sentinel value used for advanced effective partition key range calculations.
    ///
    /// This value is not valid in request partition keys.
    pub const INFINITY: Self = Self(InnerPartitionKeyValue::Infinity);

    /// Writes this value into a byte buffer using the V2 hashing encoding.
    ///
    /// Used by the effective partition key computation for MurmurHash3-128.
    pub(crate) fn write_for_hashing_v2(&self, writer: &mut Vec<u8>) {
        self.0.write_for_hashing_core(0xFFu8, writer, false)
    }

    /// Writes this value into a byte buffer using the V1 hashing encoding.
    ///
    /// Used by the effective partition key computation for MurmurHash3-32.
    pub(crate) fn write_for_hashing_v1(&self, writer: &mut Vec<u8>) {
        self.0.write_for_hashing_core(0x00u8, writer, true)
    }

    /// Writes this value using V1 binary encoding for the EPK output string.
    pub(crate) fn write_for_binary_encoding_v1(&self, writer: &mut Vec<u8>) {
        self.0.write_for_binary_encoding_v1(writer)
    }

    /// Returns `true` if this value is the special Infinity sentinel.
    pub(crate) fn is_infinity(&self) -> bool {
        matches!(self.0, InnerPartitionKeyValue::Infinity)
    }

    /// Returns a truncated copy of this value for V1 binary encoding.
    ///
    /// String values longer than [`MAX_STRING_BYTES_TO_APPEND`] bytes are truncated
    /// so that `write_for_binary_encoding_v1` sees them as "short" and appends the
    /// `0x00` terminator, matching how the hashing step truncates strings.
    pub(crate) fn truncated_for_v1_encoding(&self) -> PartitionKeyValue {
        match &self.0 {
            InnerPartitionKeyValue::String(s) if s.len() > MAX_STRING_BYTES_TO_APPEND => {
                InnerPartitionKeyValue::String(Cow::Owned(
                    s[..MAX_STRING_BYTES_TO_APPEND].to_string(),
                ))
                .into()
            }
            _ => self.clone(),
        }
    }
}

impl From<&'static str> for PartitionKeyValue {
    fn from(value: &'static str) -> Self {
        InnerPartitionKeyValue::String(Cow::Borrowed(value)).into()
    }
}

impl From<String> for PartitionKeyValue {
    fn from(value: String) -> Self {
        InnerPartitionKeyValue::String(Cow::Owned(value)).into()
    }
}

impl From<&String> for PartitionKeyValue {
    fn from(value: &String) -> Self {
        InnerPartitionKeyValue::String(Cow::Owned(value.clone())).into()
    }
}

impl From<Cow<'static, str>> for PartitionKeyValue {
    fn from(value: Cow<'static, str>) -> Self {
        InnerPartitionKeyValue::String(value).into()
    }
}

macro_rules! impl_from_number {
    ($source_type:ty) => {
        impl From<$source_type> for PartitionKeyValue {
            fn from(value: $source_type) -> Self {
                InnerPartitionKeyValue::Number(FiniteF64::new_strict(value as f64)).into()
            }
        }
    };
}

impl_from_number!(i8);
impl_from_number!(i16);
impl_from_number!(i32);
impl_from_number!(i64);
impl_from_number!(isize);
impl_from_number!(u8);
impl_from_number!(u16);
impl_from_number!(u32);
impl_from_number!(u64);
impl_from_number!(usize);
impl_from_number!(f32);
impl_from_number!(f64);

impl From<bool> for PartitionKeyValue {
    fn from(value: bool) -> Self {
        InnerPartitionKeyValue::Bool(value).into()
    }
}

impl<T: Into<PartitionKeyValue>> From<Option<T>> for PartitionKeyValue {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(v) => v.into(),
            None => InnerPartitionKeyValue::Null.into(),
        }
    }
}

/// A partition key identifying a logical partition in a Cosmos DB container.
///
/// Most Cosmos DB containers use a single partition key path (e.g., `/tenantId`),
/// but hierarchical partition keys allow up to three levels. `PartitionKey`
/// handles both cases.
///
/// # Creating partition keys
///
/// For single-value keys, just pass the value — any type that implements
/// `Into<PartitionKeyValue>` works:
///
/// ```
/// use azure_data_cosmos_driver::models::PartitionKey;
///
/// let pk = PartitionKey::from("my-tenant");
/// let pk = PartitionKey::from(42);
/// let pk = PartitionKey::from(None::<String>); // null
/// ```
///
/// For hierarchical keys, use tuples:
///
/// ```
/// use azure_data_cosmos_driver::models::PartitionKey;
///
/// let pk = PartitionKey::from(("tenant", "user", 2024));
/// ```
///
/// In most SDK methods, you don't need to call `from` explicitly — the method
/// accepts `impl Into<PartitionKey>` so you can pass the value directly:
///
/// ```rust,no_run
/// # mod azure_data_cosmos {
/// #     pub mod clients {
/// #         #[derive(Clone)]
/// #         pub struct ContainerClient;
/// #
/// #         impl ContainerClient {
/// #             pub async fn read_item(
/// #                 &self,
/// #                 _partition_key: impl Into<azure_data_cosmos_driver::models::PartitionKey>,
/// #                 _item_id: &str,
/// #                 _options: Option<()>,
/// #             ) -> Result<(), Box<dyn std::error::Error>> {
/// #                 Ok(())
/// #             }
/// #         }
/// #     }
/// # }
/// # let container_client: azure_data_cosmos::clients::ContainerClient = panic!("example");
/// # async fn doc(container_client: azure_data_cosmos::clients::ContainerClient) -> Result<(), Box<dyn std::error::Error>> {
/// // Just pass a string directly — no need for PartitionKey::from()
/// let response = container_client.read_item("my-partition-value", "item-id", None).await?;
/// # let _ = response;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct PartitionKey(Vec<PartitionKeyValue>);

impl PartitionKey {
    /// A single null partition key value.
    pub const NULL: PartitionKeyValue = PartitionKeyValue::NULL;

    /// A single undefined partition key value.
    pub const UNDEFINED: PartitionKeyValue = PartitionKeyValue::UNDEFINED;

    /// An empty partition key. Used internally by the routing/range-cache
    /// layer to mean "no specific partition" — *not* a public way to issue
    /// cross-partition operations. Public callers express cross-partition
    /// intent through the query/feed APIs (e.g. `FeedScope`), not through
    /// this constant.
    pub(crate) const EMPTY: PartitionKey = PartitionKey(Vec::new());

    /// Creates a new partition key from a single value.
    pub(crate) fn new(value: impl Into<PartitionKeyValue>) -> Self {
        Self(vec![value.into()])
    }

    /// Returns `true` if this partition key has no components.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of components in this partition key.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns the partition key components.
    pub fn values(&self) -> &[PartitionKeyValue] {
        &self.0
    }
}

impl AsHeaders for PartitionKey {
    type Error = crate::error::CosmosError;
    type Iter = std::iter::Once<(HeaderName, HeaderValue)>;

    fn as_headers(&self) -> Result<Self::Iter, Self::Error> {
        // We have to do some manual JSON serialization here.
        // The partition key is sent in an HTTP header, when used to set the partition key for a query.
        // It's not safe to use non-ASCII characters in HTTP headers, and serde_json will not escape non-ASCII characters if they are otherwise valid as UTF-8.
        // So, we do some conversion by hand, with the help of Rust's own `encode_utf16` method which gives us the necessary code points for non-ASCII values, and produces surrogate pairs as needed.

        // Quick shortcut for empty partition keys list, which also prevents a bug when we pop the trailing comma for an empty list.
        if self.0.is_empty() {
            // An empty partition key means a cross partition query
            return Ok(std::iter::once((
                QUERY_ENABLE_CROSS_PARTITION,
                HeaderValue::from_static("True"),
            )));
        }

        let mut json = String::new();
        let mut utf_buf = [0; 2]; // A buffer for encoding UTF-16 characters.
        json.push('[');
        for key in &self.0 {
            match &key.0 {
                InnerPartitionKeyValue::Null => json.push_str("null"),
                InnerPartitionKeyValue::String(ref string_key) => {
                    json.push('"');
                    for char in string_key.chars() {
                        match char {
                            '\x08' => json.push_str(r#"\b"#),
                            '\x0c' => json.push_str(r#"\f"#),
                            '\n' => json.push_str(r#"\n"#),
                            '\r' => json.push_str(r#"\r"#),
                            '\t' => json.push_str(r#"\t"#),
                            '"' => json.push_str(r#"\""#),
                            '\\' => json.push_str(r#"\\"#),
                            c if c.is_ascii() && !c.is_control() => json.push(c),
                            c if c.is_ascii() => {
                                // Remaining ASCII control characters (< 0x20) must be \uXXXX-escaped.
                                json.push_str(&format!("\\u{:04x}", c as u32));
                            }
                            c => {
                                let encoded = c.encode_utf16(&mut utf_buf);
                                for code_unit in encoded {
                                    json.push_str(&format!(r#"\u{:04x}"#, code_unit));
                                }
                            }
                        }
                    }
                    json.push('"');
                }
                InnerPartitionKeyValue::Number(num) => {
                    // Format number - integers without decimal, floats with decimal
                    let val = num.value();
                    if val.fract() == 0.0 && val.abs() < (i64::MAX as f64) {
                        json.push_str(&format!("{}", val as i64));
                    } else {
                        json.push_str(&format!("{}", val));
                    }
                }
                InnerPartitionKeyValue::Bool(b) => {
                    json.push_str(if *b { "true" } else { "false" });
                }
                InnerPartitionKeyValue::Infinity => {
                    // Internal sentinel — should never appear in a user-facing partition key.
                    return Err(crate::error::CosmosError::builder()
                        .with_status(crate::error::CosmosStatus::new(
                            azure_core::http::StatusCode::BadRequest,
                        ))
                        .with_message(
                            "Infinity is not a valid partition key value for serialization",
                        )
                        .build());
                }
                InnerPartitionKeyValue::Undefined => {
                    // Items with no partition key property.
                    json.push_str("{}");
                }
            }

            json.push(',');
        }

        // Pop the trailing ','
        json.pop();
        json.push(']');

        Ok(std::iter::once((
            PARTITION_KEY,
            HeaderValue::from_cow(json),
        )))
    }
}

// Single value conversions
impl<T: Into<PartitionKeyValue>> From<T> for PartitionKey {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl From<Vec<PartitionKeyValue>> for PartitionKey {
    /// Creates a partition key from a vector of components.
    ///
    /// Use this when the partition key structure is determined at runtime.
    ///
    /// # Panics
    ///
    /// Panics if the vector contains more than 3 elements (Cosmos DB supports
    /// at most 3 hierarchical partition key levels).
    fn from(values: Vec<PartitionKeyValue>) -> Self {
        assert!(
            values.len() <= 3,
            "Partition keys can have at most 3 levels, got {}",
            values.len()
        );
        PartitionKey(values)
    }
}

// Tuple conversions for hierarchical partition keys
impl<T1, T2> From<(T1, T2)> for PartitionKey
where
    T1: Into<PartitionKeyValue>,
    T2: Into<PartitionKeyValue>,
{
    fn from((v1, v2): (T1, T2)) -> Self {
        Self(vec![v1.into(), v2.into()])
    }
}

impl<T1, T2, T3> From<(T1, T2, T3)> for PartitionKey
where
    T1: Into<PartitionKeyValue>,
    T2: Into<PartitionKeyValue>,
    T3: Into<PartitionKeyValue>,
{
    fn from((v1, v2, v3): (T1, T2, T3)) -> Self {
        Self(vec![v1.into(), v2.into(), v3.into()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_partition_key() {
        let pk = PartitionKey::from("test");
        assert_eq!(pk.len(), 1);
        assert!(!pk.is_empty());
    }

    #[test]
    fn numeric_partition_key() {
        let pk1 = PartitionKey::from(42);
        let pk2 = PartitionKey::from(42i64);
        let pk3 = PartitionKey::from(1.5f64);
        assert_eq!(pk1.len(), 1);
        assert_eq!(pk2.len(), 1);
        assert_eq!(pk3.len(), 1);
    }

    #[test]
    fn hierarchical_partition_key() {
        let pk = PartitionKey::from(("tenant", "user", 42));
        assert_eq!(pk.len(), 3);
    }

    #[test]
    fn empty_partition_key() {
        let pk = PartitionKey::EMPTY;
        assert!(pk.is_empty());
        assert_eq!(pk.len(), 0);
    }

    #[test]
    fn null_partition_key_value() {
        let pk = PartitionKey::from(None::<String>);
        assert_eq!(pk.len(), 1);
    }

    #[test]
    #[should_panic(expected = "at most 3 levels")]
    fn too_many_levels() {
        let values = vec![
            PartitionKeyValue::from("a"),
            PartitionKeyValue::from("b"),
            PartitionKeyValue::from("c"),
            PartitionKeyValue::from("d"),
        ];
        let _pk = PartitionKey::from(values);
    }
}
