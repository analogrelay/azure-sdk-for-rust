// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Resource reference types for databases and containers.

use std::borrow::Cow;

/// A reference to a Cosmos DB database.
///
/// Contains either the name or resource identifier (RID) of the database, or both.
/// Provides methods to generate name-based or RID-based relative paths.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DatabaseReference {
    /// The database name.
    name: Option<Cow<'static, str>>,
    /// The database resource identifier (RID).
    rid: Option<Cow<'static, str>>,
}

impl DatabaseReference {
    /// Creates a new database reference from a name.
    pub fn from_name(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: Some(name.into()),
            rid: None,
        }
    }

    /// Creates a new database reference from a resource identifier (RID).
    pub fn from_rid(rid: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: None,
            rid: Some(rid.into()),
        }
    }

    /// Creates a new database reference with both name and RID.
    pub fn new(name: impl Into<Cow<'static, str>>, rid: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: Some(name.into()),
            rid: Some(rid.into()),
        }
    }

    /// Returns the database name, if available.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the database resource identifier (RID), if available.
    pub fn rid(&self) -> Option<&str> {
        self.rid.as_deref()
    }

    /// Sets the database name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the database resource identifier (RID).
    #[must_use]
    pub fn with_rid(mut self, rid: impl Into<Cow<'static, str>>) -> Self {
        self.rid = Some(rid.into());
        self
    }

    /// Returns the name-based relative path: `/dbs/{name}`
    ///
    /// Returns `None` if the name is not set.
    pub fn name_based_path(&self) -> Option<String> {
        self.name.as_ref().map(|n| format!("/dbs/{}", n))
    }

    /// Returns the RID-based relative path: `/dbs/{rid}`
    ///
    /// Returns `None` if the RID is not set.
    pub fn rid_based_path(&self) -> Option<String> {
        self.rid.as_ref().map(|r| format!("/dbs/{}", r))
    }
}

/// A reference to a Cosmos DB container.
///
/// Contains either the name or resource identifier (RID) of the container, or both,
/// along with a reference to its parent database.
/// Provides methods to generate name-based or RID-based relative paths.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ContainerReference {
    /// Reference to the parent database.
    database: DatabaseReference,
    /// The container name.
    name: Option<Cow<'static, str>>,
    /// The container resource identifier (RID).
    rid: Option<Cow<'static, str>>,
}

impl ContainerReference {
    /// Creates a new container reference from a name.
    pub fn from_name(database: DatabaseReference, name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            database,
            name: Some(name.into()),
            rid: None,
        }
    }

    /// Creates a new container reference from a resource identifier (RID).
    pub fn from_rid(database: DatabaseReference, rid: impl Into<Cow<'static, str>>) -> Self {
        Self {
            database,
            name: None,
            rid: Some(rid.into()),
        }
    }

    /// Creates a new container reference with both name and RID.
    pub fn new(
        database: DatabaseReference,
        name: impl Into<Cow<'static, str>>,
        rid: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            database,
            name: Some(name.into()),
            rid: Some(rid.into()),
        }
    }

    /// Returns a reference to the parent database.
    pub fn database(&self) -> &DatabaseReference {
        &self.database
    }

    /// Returns the container name, if available.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the container resource identifier (RID), if available.
    pub fn rid(&self) -> Option<&str> {
        self.rid.as_deref()
    }

    /// Sets the container name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the container resource identifier (RID).
    #[must_use]
    pub fn with_rid(mut self, rid: impl Into<Cow<'static, str>>) -> Self {
        self.rid = Some(rid.into());
        self
    }

    /// Returns the name-based relative path: `/dbs/{db_name}/colls/{container_name}`
    ///
    /// Returns `None` if either the database name or container name is not set.
    pub fn name_based_path(&self) -> Option<String> {
        match (self.database.name(), self.name.as_deref()) {
            (Some(db_name), Some(container_name)) => {
                Some(format!("/dbs/{}/colls/{}", db_name, container_name))
            }
            _ => None,
        }
    }

    /// Returns the RID-based relative path: `/dbs/{db_rid}/colls/{container_rid}`
    ///
    /// Returns `None` if either the database RID or container RID is not set.
    pub fn rid_based_path(&self) -> Option<String> {
        match (self.database.rid(), self.rid.as_deref()) {
            (Some(db_rid), Some(container_rid)) => {
                Some(format!("/dbs/{}/colls/{}", db_rid, container_rid))
            }
            _ => None,
        }
    }
}
