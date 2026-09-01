# Hybrid Query Samples

These samples illustrate the proposed `azure_data_cosmos_hybrid` API. The API
is still a design and the examples are not expected to compile until the
corresponding prototype phases are implemented.

## Cross-partition aggregate in Fabric

An order-processing application stores individual orders in a Cosmos DB
container partitioned by `/customerId`. Each item includes scalar fields such
as `region`, `orderDate`, and `netAmount`.

The container is mirrored into Microsoft Fabric. The application attaches a
`FabricSqlQuerySource` mapped to the mirrored table's SQL analytics endpoint.
The query spans many Cosmos partitions but reads only `region` and `netAmount`,
allowing the analytical engine to use the mirrored columnar representation
instead of loading complete order documents.

```rust
use std::sync::Arc;

use azure_core::credentials::TokenCredential;
use azure_data_cosmos::{feed::FeedScope, CosmosClient};
use azure_data_cosmos_hybrid::{
    ConsistencyConstraint, CosmosContainerReference, FabricContainerMapping, FabricSqlQuerySource,
    FabricSqlSourceOptions, FabricTableReference, HybridQuery, HybridQueryClient,
};
use futures::StreamExt;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegionalRevenue {
    region: String,
    net_amount: f64,
}

async fn summarize_regional_revenue(
    cosmos: CosmosClient,
    fabric_credential: Arc<dyn TokenCredential>,
) -> Result<(), Box<dyn std::error::Error>> {
    let orders_mapping = FabricContainerMapping::new(
        CosmosContainerReference::new("commerce", "orders"),
        FabricTableReference::new("CommerceMirror", "dbo", "Orders"),
    )
    .with_column("/customerId", "customer_id")
    .with_column("/region", "region")
    .with_column("/orderDate", "order_date")
    .with_column("/netAmount", "net_amount");

    let fabric = FabricSqlQuerySource::new(
        "orders-fabric",
        FabricSqlSourceOptions::new(
            "tcp:example.fabric.microsoft.com,1433".parse()?,
            fabric_credential,
        )
        .with_mapping(orders_mapping),
    )?;

    let hybrid = HybridQueryClient::builder(cosmos)
        .with_query_source(fabric)
        .build()?;

    let orders = hybrid.container_client("commerce", "orders").await?;

    let query = HybridQuery::from(
        "SELECT c.region, SUM(c.netAmount) AS netAmount
         FROM c
         WHERE c.orderDate >= @start
         GROUP BY c.region
         ORDER BY SUM(c.netAmount) DESC",
    )
    .with_parameter("@start", "2026-01-01")?
    .with_consistency(ConsistencyConstraint::Analytical);

    let mut results = orders
        .query_items::<RegionalRevenue>(query, FeedScope::full_container())
        .await?;

    while let Some(result) = results.next().await {
        let result = result?;
        println!(
            "{}: {} ({:?})",
            result.item.region, result.item.net_amount, result.provenance,
        );
    }

    Ok(())
}
```

`Analytical` explicitly permits replication lag and prefers the attached
analytical source. If the query cannot be translated to the supported T-SQL
subset, fallback behavior depends on whether the source was marked optional or
required on the `HybridQuery`.

### Planner and execution flow

1. The `orders` client contributes its container properties and the
   `commerce/orders` to `CommerceMirror.dbo.Orders` mapping.
2. The Cosmos SQL parser identifies a full-container query with a filter,
   aggregate, grouping, and ordering. It also determines that only
   `orderDate`, `region`, and `netAmount` are required.
3. No parameter transformer or reranker is named, so those extension roles do
   not light up.
4. `ConsistencyConstraint::Analytical` permits an analytical source and prefers
   one over Cosmos. The planner considers `orders-fabric`, finds the matching
   container mapping, and asks it to translate the analyzed query.
5. The Fabric source accepts because every query feature and referenced path is
   covered by its supported T-SQL subset and column mappings. It becomes the
   single primary source.
6. The client executes the translated aggregate through `mssql-rs`, converts
   each row into `RegionalRevenue`, and emits `HybridQueryItem` values whose
   provenance identifies Fabric.

If translation or Fabric execution fails before results are emitted, an
optional source can fall back to Cosmos with a warning. A required Fabric source
fails the query instead.

## Query embedding and semantic reranking

A product catalog is stored in a Cosmos DB container partitioned by
`/categoryId`. Product items contain an `embedding` vector covered by the
container's vector embedding policy, along with `name`, `description`, and
`categoryName` fields. The same container is mirrored into Fabric and has a
matching table mapping.

The application attaches a Fabric source and two Azure OpenAI extensions:

- a Fabric source that can handle ordinary analytical projections and
  aggregates over the mirrored product table;
- an embedding provider that converts the user's search text into the vector
  parameter used by Cosmos DB; and
- a reranker that reorders the Cosmos DB candidate set using the product name,
  description, and category.

The embedding directive names the transformer explicitly. The transformer can
see that `@queryVector` is paired with `c.embedding` in the `VectorDistance`
call, normalize that property to `/embedding`, and read the matching
`ContainerProperties` vector policy. It can obtain vector dimensions, element
type, distance function, and extension metadata without requiring those values
in the application code.

```rust
use std::sync::Arc;

use azure_core::credentials::TokenCredential;
use azure_data_cosmos::{feed::FeedScope, CosmosClient};
use azure_data_cosmos_hybrid::{
    AzureOpenAiEmbeddingProvider, AzureOpenAiOptions, AzureOpenAiReranker, ConsistencyConstraint,
    CosmosContainerReference, FabricContainerMapping, FabricSqlQuerySource, FabricSqlSourceOptions,
    FabricTableReference, HybridQuery, HybridQueryClient, ParameterTransformDirective,
    SemanticRerankingOptions,
};
use futures::StreamExt;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProductSearchResult {
    id: String,
    category_id: String,
    name: String,
    description: String,
}

async fn search_products(
    cosmos: CosmosClient,
    ai_credential: Arc<dyn TokenCredential>,
    fabric_credential: Arc<dyn TokenCredential>,
    search_text: &str,
) -> Result<Vec<ProductSearchResult>, Box<dyn std::error::Error>> {
    let products_mapping = FabricContainerMapping::new(
        CosmosContainerReference::new("catalog", "products"),
        FabricTableReference::new("CatalogMirror", "dbo", "Products"),
    )
    .with_column("/id", "id")
    .with_column("/categoryId", "category_id")
    .with_column("/name", "name")
    .with_column("/description", "description")
    .with_column("/embedding", "embedding");

    let fabric = FabricSqlQuerySource::new(
        "products-fabric",
        FabricSqlSourceOptions::new(
            "tcp:example.fabric.microsoft.com,1433".parse()?,
            fabric_credential,
        )
        .with_mapping(products_mapping),
    )?;

    let embedding_provider = AzureOpenAiEmbeddingProvider::new(
        "product-embedding",
        AzureOpenAiOptions {
            endpoint: "https://example.openai.azure.com/openai/v1/".parse()?,
            deployment: "text-embedding-3-small".into(),
            credential: ai_credential.clone(),
        },
    )?;

    let reranker = AzureOpenAiReranker::new(
        "product-reranker",
        AzureOpenAiOptions {
            endpoint: "https://example.openai.azure.com/openai/v1/".parse()?,
            deployment: "product-reranker".into(),
            credential: ai_credential,
        },
    )?;

    let hybrid = HybridQueryClient::builder(cosmos)
        .with_query_source(fabric)
        .with_parameter_transform(embedding_provider)
        .with_reranker(reranker)
        .build()?;

    let products = hybrid.container_client("catalog", "products").await?;

    let query = HybridQuery::from(
        "SELECT TOP 50
             c.id,
             c.categoryId,
             c.name,
             c.description
         FROM c
         ORDER BY VectorDistance(c.embedding, @queryVector)",
    )
    .with_transformed_parameter(
        "@queryVector",
        search_text,
        ParameterTransformDirective::new("product-embedding"),
    )?
    .with_consistency(ConsistencyConstraint::Analytical)
    .with_semantic_reranking(
        SemanticRerankingOptions::new("product-reranker")
            .with_query(search_text)
            .with_document_fields(["name", "description", "categoryId"])
            .with_candidate_count(50)
            .with_result_count(10),
    );

    let mut results = products
        .query_items::<ProductSearchResult>(query, FeedScope::full_container())
        .await?;
    let mut matched_products = Vec::new();

    while let Some(result) = results.next().await {
        let result = result?;

        println!(
            "{}: score={:?}, source={:?}",
            result.item.name, result.score, result.provenance,
        );

        if let Some(explanation) = result
            .extensions
            .get::<String>("product-reranker", "explanation")?
        {
            println!("  {explanation}");
        }

        matched_products.push(result.item);
    }

    Ok(matched_products)
}
```

The query permits analytical execution, but the Fabric source declines the
query because it cannot provide sufficiently equivalent `VectorDistance`
semantics. Cosmos DB therefore produces the initial candidate set. The reranker
receives only the configured document fields and cannot modify the source
documents. The final iterator still yields a uniform
`HybridQueryItem<ProductSearchResult>` with score, provenance, and
extension-provided metadata.

If the SQL shape does not expose a single property path, the application can
override the inferred configuration:

```rust
fn overridden_transform() -> ParameterTransformDirective {
    ParameterTransformDirective::new("product-embedding").with_options(
        EmbeddingTransformOptions::default()
            .with_vector_path("/alternateEmbedding")
            .with_model("alternate-embedding-model"),
    )
}
```

### Planner and execution flow

1. The parser identifies `@queryVector` as the second argument to
   `VectorDistance` and resolves the sibling expression `c.embedding` to the
   Cosmos property path `/embedding`.
2. The query names `product-embedding`, so only that registered parameter
   transformer is considered. Its `supports` call receives the parameter value,
   AST-derived function usage, and the product container properties.
3. The transformer matches `/embedding` to the container's vector embedding
   policy, derives its dimensions, element type, distance function, and model
   metadata, then replaces the string parameter with the generated vector.
4. `ConsistencyConstraint::Analytical` makes `products-fabric` eligible. The
   planner finds the `catalog/products` mapping and asks the source whether it
   can execute the parsed query.
5. The Fabric source recognizes `VectorDistance` but reports it as unsupported
   because its available translation cannot preserve Cosmos vector-distance
   behavior or ranking quality. Since the source is optional, the planner
   records a warning and falls back to Cosmos before emitting any results.
6. The client constructs a native Cosmos `Query` with the transformed parameter
   and requests 50 candidates.
7. The query names `product-reranker`, so the planner calls that reranker's
   `supports` method with the result shape, requested fields, candidate count,
   and result count.
8. After Cosmos returns the candidate window, the client sends only `name`,
   `description`, and `categoryId` to the reranker. It validates the returned
   identities and scores, emits the top 10 items, and attaches the score,
   Cosmos provenance, and reranker explanation to each `HybridQueryItem`.

The embedding transformer and reranker are independent stages. If either is
required, its failure fails the query. An optional reranker can preserve the
original Cosmos ordering and attach a warning, while an optional transform can
fall back only if executing with the original parameter remains valid. The
Fabric rejection is a planning-time capability decision rather than a failed
TDS request.

## Authoritative and analytical queries

An inventory service stores current stock levels in a Cosmos DB container
partitioned by `/warehouseId`. The same container is mirrored into Fabric for
historical reporting.

The application uses one `HybridContainerClient` for both workloads:

- operational queries use a standard Cosmos consistency constraint and
  explicitly select Cosmos as the authoritative source; and
- reporting queries use `Analytical`, which permits lag and prefers the Fabric
  source.

```rust
use std::sync::Arc;

use azure_core::credentials::TokenCredential;
use azure_data_cosmos::{feed::FeedScope, CosmosClient};
use azure_data_cosmos_hybrid::{
    ConsistencyConstraint, CosmosContainerReference, FabricContainerMapping, FabricSqlQuerySource,
    FabricSqlSourceOptions, FabricTableReference, HybridContainerClient, HybridQuery,
    HybridQueryClient, SourceSelection,
};
use futures::StreamExt;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InventoryItem {
    id: String,
    warehouse_id: String,
    available: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WarehouseInventory {
    warehouse_id: String,
    available: i64,
}

async fn create_inventory_client(
    cosmos: CosmosClient,
    fabric_credential: Arc<dyn TokenCredential>,
) -> Result<HybridContainerClient, Box<dyn std::error::Error>> {
    let inventory_mapping = FabricContainerMapping::new(
        CosmosContainerReference::new("operations", "inventory"),
        FabricTableReference::new("InventoryMirror", "dbo", "Inventory"),
    )
    .with_column("/id", "id")
    .with_column("/warehouseId", "warehouse_id")
    .with_column("/available", "available");

    let fabric = FabricSqlQuerySource::new(
        "inventory-fabric",
        FabricSqlSourceOptions::new(
            "tcp:example.fabric.microsoft.com,1433".parse()?,
            fabric_credential,
        )
        .with_mapping(inventory_mapping),
    )?;

    let hybrid = HybridQueryClient::builder(cosmos)
        .with_query_source(fabric)
        .build()?;

    Ok(hybrid.container_client("operations", "inventory").await?)
}

async fn read_current_inventory(
    inventory: &HybridContainerClient,
    warehouse_id: &str,
) -> Result<Vec<InventoryItem>, Box<dyn std::error::Error>> {
    let query = HybridQuery::from(
        "SELECT *
         FROM c
         WHERE c.warehouseId = @warehouseId",
    )
    .with_parameter("@warehouseId", warehouse_id)?
    .with_consistency(ConsistencyConstraint::Session)
    .with_source(SourceSelection::Cosmos);

    let mut results = inventory
        .query_items::<InventoryItem>(query, FeedScope::partition(warehouse_id))
        .await?;
    let mut items = Vec::new();

    while let Some(result) = results.next().await {
        items.push(result?.item);
    }

    Ok(items)
}

async fn summarize_inventory(
    inventory: &HybridContainerClient,
) -> Result<Vec<WarehouseInventory>, Box<dyn std::error::Error>> {
    let query = HybridQuery::from(
        "SELECT c.warehouseId, SUM(c.available) AS available
         FROM c
         GROUP BY c.warehouseId
         ORDER BY c.warehouseId",
    )
    .with_consistency(ConsistencyConstraint::Analytical)
    .with_source(SourceSelection::PreferAnalytical);

    let mut results = inventory
        .query_items::<WarehouseInventory>(query, FeedScope::full_container())
        .await?;
    let mut summaries = Vec::new();

    while let Some(result) = results.next().await {
        let result = result?;
        println!(
            "warehouse {} came from {:?}",
            result.item.warehouse_id, result.provenance,
        );
        summaries.push(result.item);
    }

    Ok(summaries)
}
```

The authoritative query is partition-scoped and retains Cosmos session
semantics. The analytical query is cross-partition and can use the mirrored
columnar store. Both return the same enriched item shape, allowing shared
telemetry and provenance handling.

### Planner and execution flow

The same attached Fabric source is available to both calls, but availability
alone does not activate it.

For `read_current_inventory`:

1. The parser recognizes an equality filter on the partition key and the
   explicit partition scope.
2. `ConsistencyConstraint::Session` requires authoritative Cosmos semantics,
   and `SourceSelection::Cosmos` makes the source choice explicit.
3. The Fabric source is not asked to take over, even though a matching mapping
   exists. Cosmos executes the partition-scoped query and supplies session
   diagnostics and Cosmos provenance.

For `summarize_inventory`:

1. The parser recognizes a cross-partition aggregate grouped by `warehouseId`.
2. `ConsistencyConstraint::Analytical` and
   `SourceSelection::PreferAnalytical` make analytical execution eligible and
   preferred.
3. The planner finds the `operations/inventory` mapping, verifies the referenced
   paths against its Fabric columns, and asks `inventory-fabric` to translate
   the aggregate.
4. The Fabric source accepts, becomes the primary source, and executes the
   translated query through `mssql-rs`.
5. Fabric rows are converted into `WarehouseInventory` values and emitted with
   Fabric provenance.

Both calls use the same planner and return the same `HybridQueryItem<T>` shape.
The consistency and source-selection fields on each query determine whether the
attached analytical extension is eligible to take over.
