// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Test framework for azure_data_cosmos_driver emulator tests.

mod env;
mod test_client;

pub use env::{
    CosmosTestMode, CONNECTION_STRING_ENV_VAR, DATABASE_NAME_ENV_VAR, EMULATOR_CONNECTION_STRING,
    TEST_MODE_ENV_VAR,
};
pub use test_client::{DriverTestClient, DriverTestRunContext};
