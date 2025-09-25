// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

#![doc = include_str!("../README.md")]
// Docs.rs build is done with the nightly compiler, so we can enable nightly features in that build.
// In this case we enable two features:
// - `doc_auto_cfg`: Automatically scans `cfg` attributes and uses them to show those required configurations in the generated documentation.
// - `doc_cfg_hide`: Ignore the `doc` configuration for `doc_auto_cfg`.
// See https://doc.rust-lang.org/rustdoc/unstable-features.html#doc_auto_cfg-automatically-generate-doccfg for more details.
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(docsrs, feature(doc_cfg_hide))]

pub mod clients;
mod connection_string;
pub mod constants;
mod feed;
mod options;
mod partition_key;
pub(crate) mod pipeline;
pub mod query;
pub(crate) mod resource_context;
pub(crate) mod utils;

pub mod models;
mod session;

mod location_cache;

#[doc(inline)]
pub use clients::CosmosClient;

pub use connection_string::*;
pub use options::*;
pub use partition_key::*;
pub use query::Query;

pub use feed::{FeedPage, FeedPager};

/// A logical sequence number (LSN) used in Cosmos DB replication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Lsn(u64);

impl Lsn {
    /// Creates a new LSN from a u64 value.
    pub(crate) fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the inner u64 value.
    pub(crate) fn value(&self) -> u64 {
        self.0
    }
}

/// A region identifier used in Cosmos DB multi-region operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RegionId(u32);

impl RegionId {
    /// Creates a new RegionId from a u32 value.
    pub(crate) fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the inner u32 value.
    pub(crate) fn value(&self) -> u32 {
        self.0
    }
}
