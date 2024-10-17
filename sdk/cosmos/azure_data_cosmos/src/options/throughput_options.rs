// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

#[cfg(doc)]
use crate::clients::ContainerClientMethods;

/// Options to be passed to operations related to Throughput offers.
#[derive(Clone, Debug, Default)]
pub struct ThroughputOptions {}

impl ThroughputOptions {
    /// Creates a new [`ThroughputOptionsBuilder`](ThroughputOptionsBuilder) that can be used to construct a [`ThroughputOptions`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// let options = azure_data_cosmos::ThroughputOptions::builder().build();
    /// ```
    pub fn builder() -> ThroughputOptionsBuilder {
        ThroughputOptionsBuilder::default()
    }
}

/// Builder used to construct a [`ThroughputOptions`].
///
/// Obtain a [`ThroughputOptionsBuilder`] by calling [`ThroughputOptions::builder()`]
#[derive(Default)]
pub struct ThroughputOptionsBuilder(ThroughputOptions);

impl ThroughputOptionsBuilder {
    /// Builds a [`ThroughputOptions`] from the builder.
    ///
    /// This does not consume the builder, and can be called multiple times.
    pub fn build(&self) -> ThroughputOptions {
        self.0.clone()
    }
}
