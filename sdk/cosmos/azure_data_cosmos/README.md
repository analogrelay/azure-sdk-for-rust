# Azure Cosmos DB SDK for Rust

`azure_data_cosmos` is an async client library for Azure Cosmos DB for NoSQL. Use it to work with databases, containers, items, queries, and transactional batches from idiomatic Rust.

[Source code] | [Package (crates.io)] | [API reference documentation] | [Azure Cosmos DB for NoSQL documentation]

## Getting started

### Install the package

Install the Azure Cosmos DB SDK for Rust with cargo:

```sh
cargo add azure_data_cosmos
```

### Prerequisites

* An [Azure subscription] or free Azure Cosmos DB trial account.

If you don't have an Azure subscription, create a free account before you begin. You can also try Azure Cosmos DB for free without an Azure subscription, create an Azure Cosmos DB free tier account with the first 400 RU/s and 5 GB of storage for free, or use the Azure Cosmos DB Emulator at <https://localhost:8081>. For the emulator key, see [how to develop with the emulator](https://learn.microsoft.com/azure/cosmos-db/how-to-develop-emulator).

### Create an Azure Cosmos DB account

You can create an Azure Cosmos DB account using:

* [Azure Portal](https://portal.azure.com)
* [Azure CLI](https://learn.microsoft.com/cli/azure)
* [Azure ARM](https://learn.microsoft.com/azure/cosmos-db/quick-create-template)

### Authenticate the client

To work with Azure Cosmos DB, create a [`CosmosClient`] with your account endpoint and credentials.

## Key concepts

* **Client hierarchy**: Start with [`CosmosClient`], use [`CosmosClient::database_client`] to get a [`DatabaseClient`], then call [`DatabaseClient::container_client`] to get a [`ContainerClient`].
* **Partition keys**: Item operations, queries, and batches are scoped by partition key. Use [`PartitionKey`] when you need explicit partition key values or hierarchical partition keys.
* **Queries**: Use [`Query`] to build SQL queries, including parameters, and [`FeedScope`] to target a single partition or a broader range.

## Examples

Common scenarios include:

* [Create a client](#create-a-client)
* [CRUD operations on items](#crud-operations-on-items)
* [Querying items](#querying-items)
* [Transactional batch](#transactional-batch)

### Create a client

The following example uses `DeveloperToolsCredential`, which is appropriate for most local development environments. For production workloads, prefer a managed identity. For more information about available credential types, see [Azure Identity].

The `DeveloperToolsCredential` automatically picks up Azure CLI authentication. Ensure you are logged in:

```sh
az login
```

```rust
use azure_data_cosmos::{AccountEndpoint, AccountReference, CosmosClient, RoutingStrategy};
use azure_data_cosmos::options::Region;
use azure_identity::DeveloperToolsCredential;

async fn example() -> Result<(), Box<dyn std::error::Error>> {
    let credential: std::sync::Arc<dyn azure_core::credentials::TokenCredential> =
        DeveloperToolsCredential::new(None)?;
    let endpoint: AccountEndpoint = "https://myaccount.documents.azure.com/".parse()?;
    let account = AccountReference::with_credential(endpoint, credential);
    let cosmos_client = CosmosClient::builder()
        .build(account, RoutingStrategy::ProximityTo(Region::EAST_US))
        .await?;
    Ok(())
}
```

Cosmos DB also supports account keys. To use them, enable the `key_auth` feature:

```sh
cargo add azure_data_cosmos --features key_auth
```

For more information, see the [API reference documentation].

### CRUD operations on items

```rust
use azure_data_cosmos::CosmosClient;
use azure_data_cosmos::models::{PatchInstructions, PatchOperation};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Item {
    pub id: String,
    pub partition_key: String,
    pub value: String,
}

async fn example(cosmos_client: CosmosClient) -> Result<(), Box<dyn std::error::Error>> {
    let item = Item {
        id: "1".into(),
        partition_key: "partition1".into(),
        value: "2".into(),
    };

    let container = cosmos_client
        .database_client("myDatabase")
        .container_client("myContainer")
        .await?;

    container.create_item("partition1", "1", item, None).await?;

    let item_response = container.read_item("partition1", "1", None).await?;
    let mut item: Item = item_response.into_model()?;

    item.value = "3".into();
    container.replace_item("partition1", "1", item, None).await?;

    let patch = PatchInstructions::from(vec![
        PatchOperation::set("/value", serde_json::json!("4")),
    ]);
    let patched: Item = container
        .patch_item("partition1", "1", patch, None)
        .await?
        .into_model()?;
    println!("patched value = {}", patched.value);

    container.delete_item("partition1", "1", None).await?;
    Ok(())
}
```

### Querying items

```rust
use azure_data_cosmos::{CosmosClient, Query};
use azure_data_cosmos::feed::FeedScope;
use futures::StreamExt;

async fn example(cosmos_client: CosmosClient) -> Result<(), Box<dyn std::error::Error>> {
    let container = cosmos_client
        .database_client("mydb")
        .container_client("mycontainer")
        .await?;

    let query = Query::from("SELECT * FROM c WHERE c.category = @category")
        .with_parameter("@category", "electronics")?;

    let mut pages = container
        .query_items::<serde_json::Value>(query, FeedScope::partition("electronics"), None)
        .await?
        .into_pages();

    while let Some(page) = pages.next().await.transpose()? {
        for item in page.items() {
            println!("{item:?}");
        }
    }
    Ok(())
}
```

### Transactional batch

```rust
use azure_data_cosmos::{CosmosClient, TransactionalBatch};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Item {
    id: String,
    partition_key: String,
    value: String,
}

async fn example(cosmos_client: CosmosClient) -> Result<(), Box<dyn std::error::Error>> {
    let container = cosmos_client
        .database_client("mydb")
        .container_client("mycontainer")
        .await?;

    let item1 = Item {
        id: "1".into(),
        partition_key: "pk1".into(),
        value: "a".into(),
    };
    let item2 = Item {
        id: "2".into(),
        partition_key: "pk1".into(),
        value: "b".into(),
    };

    let batch = TransactionalBatch::new("pk1")
        .create_item(item1)?
        .create_item(item2)?;

    let response = container.execute_transactional_batch(batch, None).await?;
    println!("batch status = {:?}", response.status());
    Ok(())
}
```

## Next steps

* [Resource Model of Azure Cosmos DB Service](https://learn.microsoft.com/azure/cosmos-db/sql-api-resources)
* [Azure Cosmos DB Resource URI](https://learn.microsoft.com/rest/api/documentdb/documentdb-resource-uri-syntax-for-rest)
* [Partitioning](https://learn.microsoft.com/azure/cosmos-db/partition-data)
* [Using emulator](https://github.com/Azure/azure-documentdb-dotnet/blob/master/docs/documentdb-nosql-local-emulator.md)

### Provide feedback

If you encounter bugs or have suggestions, [open an issue](https://github.com/Azure/azure-sdk-for-rust/issues).

## Internal features

This crate exposes several feature flags prefixed with `__internal_`. Features behind these feature flags are internal APIs for testing within the Azure SDK for Rust and are not intended for public use. These APIs may change without warning, and using them may lead to broken code.

## Contributing

This project welcomes contributions and suggestions. Most contributions require you to agree to a Contributor License Agreement (CLA) declaring that you have the right to, and actually do, grant us the rights to use your contribution. For details, visit [https://cla.microsoft.com](https://cla.microsoft.com).

When you submit a pull request, a CLA-bot will automatically determine whether you need to provide a CLA and decorate the PR appropriately (e.g., label, comment). Simply follow the instructions provided by the bot. You'll only need to do this once across all repos using our CLA.

This project has adopted the [Microsoft Open Source Code of Conduct](https://opensource.microsoft.com/codeofconduct/). For more information, see the [Code of Conduct FAQ](https://opensource.microsoft.com/codeofconduct/faq/) or contact [opencode@microsoft.com](mailto:opencode@microsoft.com) with any additional questions or comments.

<!-- LINKS -->
[Azure subscription]: https://azure.microsoft.com/free/
[Azure Identity]: https://github.com/Azure/azure-sdk-for-rust/tree/main/sdk/identity/azure_identity
[API reference documentation]: https://docs.rs/azure_data_cosmos/latest/azure_data_cosmos/
[Azure Cosmos DB for NoSQL documentation]: https://learn.microsoft.com/azure/cosmos-db/nosql/
[Package (crates.io)]: https://crates.io/crates/azure_data_cosmos
[Source code]: https://github.com/Azure/azure-sdk-for-rust/tree/main/sdk/cosmos/azure_data_cosmos
