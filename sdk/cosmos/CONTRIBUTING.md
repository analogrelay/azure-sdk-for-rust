<!-- cSpell:ignore WL6WTRCUBG -->

# Contributing to the Azure Cosmos DB SDK for Rust.

First, review the Azure Core SDK Contributing guide: [CONTRIBUTING.md](https://github.com/Azure/azure-sdk-for-rust/blob/main/CONTRIBUTING.md)

## Running Integration Tests

Our integration tests are designed to run against either the Cosmos DB Emulator, or an actual Azure Cosmos DB account.
They use the [Azure SDK Test Proxy](https://github.com/Azure/azure-sdk-tools/blob/main/tools/test-proxy/Azure.Sdk.Tools.TestProxy/README.md) to record and playback HTTP interactions.

**By default** when you run the integration tests, they work against recorded HTTP interactions (playback mode).
This means that if you're adding a new test, it will likely fail with this error:

```
Error: Error { context: SimpleMessage(DataConversion, "header not found x-recording-id") }
```

This indicates that the test proxy failed to find a recorded interaction for the test you are trying to run.

### `AZURE_TEST_MODE` Environment Variable

The test proxy behavior is controlled by the `AZURE_TEST_MODE` environment variable.
It can have one of three values:

* `playback` (this is the default if the variable is empty or unset): The test proxy will only use recorded HTTP interactions.
* `record`: The test proxy will forward HTTP requests to the actual service and record the interactions for future playback.
* `live`: The test proxy will forward HTTP requests to the actual service, but will not record the interactions.

When adding a new test, you should set `AZURE_TEST_MODE=record` to have the test proxy record the HTTP interactions.
The HTTP recordings will be saved to the `.assets/WL6WTRCUBG` folder (this folder name is used by all tests in `sdk/cosmos`).
You can review the JSON recordings to ensure they look correct.

### Specifying the Cosmos DB Endpoint

When running tests in `record` or `live` mode, the test proxy needs to know which Cosmos DB account to use.
You can specify this using the `AZURE_COSMOS_CONNECTION_STRING` environment variable.
This can contain a valid Cosmos DB connection string, **or** the special string `emulator` to use the Cosmos DB Emulator.
The `emulator` marker is highly recommended when running against a local emulator because it ALSO automatically disables TLS certificate validation, which is often necessary when testing against the local emulator.

If you need to disable TLS certificate validation when using a different connection string, you can also set the `AZURE_COSMOS_ALLOW_INVALID_CERT` environment variable to `true`.

### Running Live Tests

1. Create an account (and optionally a database). When using the emulator, the test framework automatically creates a default database (`emulator-test-db`) if one does not already exist. For cloud accounts you can create a database using:
   - [Azure Portal](https://portal.azure.com)
   - [Azure CLI](https://learn.microsoft.com/cli/azure)
   - [Azure ARM](https://learn.microsoft.com/azure/cosmos-db/quick-create-template)
1. Optionally set the `DATABASE_NAME` environment variable to the name of the database to use. If not set, it defaults to `emulator-test-db`.
1. Set the `RUSTFLAGS` environment variable to `"--cfg=test_category=\"emulator\" --cfg=test_category=\"multi_write\""`.
1. Set the `RUST_TEST_THREADS` environment variable to `1` to ensure tests run serially.
1. Set the `AZURE_COSMOS_CONNECTION_STRING` environment variable to your Cosmos DB connection string (see [Specifying the Cosmos DB Endpoint](#specifying-the-cosmos-db-endpoint) above).
1. Set the `ACCOUNT_HOST` environment variable to your Cosmos DB account host.
1. Run the tests using the following command:

   ```bash
   cargo +nightly test --lib --bins --tests --examples --package azure_data_cosmos --all-features --no-fail-fast -- --format json -Z unstable-options --nocapture
   ```

### Saving Recordings

Before opening a PR, if you've added any integration tests, or changed the HTTP interactions of existing tests, you must save the recordings.
The recordings are saved in the https://github.com/Azure/azure-sdk-assets repository.
Whenever you "push" new recordings, a new commit is made to that repository and given a unique Git tag (for example [`azure_data_cosmos_69ad1e4995`](https://github.com/Azure/azure-sdk-assets/commit/69ad1e49952a7dd831d80e19eebbbee3454f404a)).
Then, the `sdk/cosmos/assets.json` file is updated to specify the name of that Git tag.
The Test Proxy uses this tag to find the correct recordings when running tests in playback mode.
The CI pipeline runs tests in playback mode, so it's important that the `assets.json` file is updated correctly before submitting a PR.

Setting `AZURE_TEST_MODE=record` and running the tests will update your LOCAL copy of the recordings, in `.assets`, but will not publish them to the `azure-sdk-assets` repository, or update the `assets.json` file.
To do that, install the [Azure SDK Test Proxy](https://github.com/Azure/azure-sdk-tools/blob/main/tools/test-proxy/Azure.Sdk.Tools.TestProxy/README.md#installation) tools, then run the following command from the repo root:

```bash
test-proxy push --assets-json-path ./sdk/cosmos/assets.json
```

This will publish the recordings to the `azure-sdk-assets` repository, and update the `assets.json` file accordingly.
It's fine to run this command multiple times if you realize the recordings need to be updated again before submitting your PR, or while reviewing feedback from code reviewers.
Each time the command is run, the `assets.json` file will be updated to point to the latest Git tag in the `azure-sdk-assets` repository.
Make sure you commit this change to your PR branch, so that the test proxy can find the correct recordings when running tests in playback mode.

## Common Issues

Using Windows, you might run into OpenSSL errors like:

```text
error: failed to run custom build command for `openssl-sys v0.9.x`
  Could not find directory of OpenSSL installation, and this `-sys` crate cannot
  proceed without this knowledge.
```

Ensure you have a proper installation of OpenSSL and that the relevant environment variables are set:

| Variable | Example Value |
| --- | --- |
| `OPENSSL_DIR` | `C:\Program Files\OpenSSL-Win64` |
| `OPENSSL_LIB_DIR` | `C:\Program Files\OpenSSL-Win64\lib` |
| `OPENSSL_INCLUDE_DIR` | `C:\Program Files\OpenSSL-Win64\include` |

## Useful Commands

- `cargo fmt --all`: Format the code according to Rust's standard formatting guidelines.
- `cargo clippy --workspace --all-features --all-targets --keep-going --no-deps`: Run Clippy to catch common mistakes and improve code quality.
