// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Partition key types for Cosmos DB operations.

use std::borrow::Cow;

/// Represents a value for a single partition key.
///
/// You shouldn't need to construct this type directly. The various implementations
/// of [`Into<PartitionKey>`] will handle it for you.
#[derive(Debug, Clone, PartialEq)]
pub struct PartitionKeyValue(InnerPartitionKeyValue);

// We don't want to expose the implementation details of PartitionKeyValue, so we use
// this inner private enum to store the data.
#[derive(Debug, Clone, PartialEq)]
enum InnerPartitionKeyValue {
    Null,
    String(Cow<'static, str>),
    Number(f64),
    Bool(bool),
}

impl From<InnerPartitionKeyValue> for PartitionKeyValue {
    fn from(value: InnerPartitionKeyValue) -> Self {
        PartitionKeyValue(value)
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
                InnerPartitionKeyValue::Number(value as f64).into()
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

/// A partition key used to identify the target partition for an operation.
///
/// Supports both single and hierarchical partition keys (HPK).
///
/// # Examples
///
/// Single partition key:
/// ```
/// use azure_data_cosmos_driver::models::PartitionKey;
///
/// let pk = PartitionKey::from("my-partition");
/// let pk_num = PartitionKey::from(42);
/// ```
///
/// Hierarchical partition key (tuple):
/// ```
/// use azure_data_cosmos_driver::models::PartitionKey;
///
/// let pk = PartitionKey::from(("tenant-1", "user-123"));
/// let pk3 = PartitionKey::from(("region", "tenant", 42));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct PartitionKey(Vec<PartitionKeyValue>);

impl PartitionKey {
    /// A null partition key value that can be used as a single partition key
    /// or as part of a hierarchical partition key.
    pub const NULL: PartitionKeyValue = PartitionKeyValue(InnerPartitionKeyValue::Null);

    /// An empty partition key, used to signal a cross-partition operation.
    pub const EMPTY: PartitionKey = PartitionKey(Vec::new());

    /// Creates a new partition key from a single value.
    pub fn new(value: impl Into<PartitionKeyValue>) -> Self {
        Self(vec![value.into()])
    }

    /// Creates a new partition key from multiple values (hierarchical partition key).
    pub fn from_values(values: Vec<PartitionKeyValue>) -> Self {
        Self(values)
    }

    /// Returns true if this partition key is empty (cross-partition).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of components in this partition key.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns an iterator over the partition key values.
    pub fn iter(&self) -> impl Iterator<Item = &PartitionKeyValue> {
        self.0.iter()
    }

    /// Returns the partition key values as a slice.
    pub fn values(&self) -> &[PartitionKeyValue] {
        &self.0
    }
}

// Single value conversions
impl<T: Into<PartitionKeyValue>> From<T> for PartitionKey {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl From<()> for PartitionKey {
    fn from(_: ()) -> Self {
        PartitionKey::EMPTY
    }
}

impl From<Vec<PartitionKeyValue>> for PartitionKey {
    /// Creates a [`PartitionKey`] from a vector of [`PartitionKeyValue`]s.
    ///
    /// This is useful when the partition key structure is determined at runtime,
    /// such as when working with multiple containers with different schemas or
    /// building partition keys from configuration.
    ///
    /// # Panics
    ///
    /// Panics if the vector contains more than 3 elements, as Cosmos DB supports
    /// a maximum of 3 hierarchical partition key levels.
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
