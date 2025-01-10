#![cfg(feature = "unstable_query_engine")]
#![cfg(feature = "key_auth")]

mod framework;
mod mock_engine;

use std::error::Error;
use std::sync::Arc;

use azure_core::{error::HttpError, http::StatusCode};
use azure_core_test::{recorded, TestContext};
use azure_data_cosmos::{
    clients::ContainerClient,
    models::{ContainerProperties, ThroughputProperties},
    CosmosClient, CreateContainerOptions, QueryOptions, QueryPartitionStrategy,
};
use framework::TestAccount;
use futures::StreamExt;
use mock_engine::{MockItem, MockQueryEngine};

const PARTITION_KEYS: [&str; 3] = [
    // These keys have been tested to ensure they end up in separate PK ranges for a 40000 RU container with the test data inserted.
    // Conveniently, they also have descriptive names.
    "partition1",
    "partition2",
    "partition3",
];
const ITEMS_PER_PARTITION: usize = 10;

async fn create_test_items(container: &ContainerClient) -> azure_core::Result<()> {
    for (i, partition_key) in PARTITION_KEYS.iter().cloned().enumerate() {
        for j in 0..ITEMS_PER_PARTITION {
            let id = format!("{}", i * ITEMS_PER_PARTITION + j);
            let item = MockItem {
                id,
                partition_key: partition_key.to_string(),
                merge_order: i + j * PARTITION_KEYS.len(),
            };
            container.upsert_item(partition_key, item, None).await?;
        }
    }
    Ok(())
}

async fn create_container(
    account: &TestAccount,
    cosmos_client: &CosmosClient,
) -> azure_core::Result<ContainerClient> {
    let test_db_id = account.unique_db("CrossPartitionQueryEngine");

    // Create a database and a container
    cosmos_client.create_database(&test_db_id, None).await?;
    let db_client = cosmos_client.database_client(&test_db_id);
    let throughput = ThroughputProperties::manual(40_000);
    db_client
        .create_container(
            ContainerProperties {
                id: "Container".into(),
                partition_key: "/partitionKey".into(),
                ..Default::default()
            },
            Some(CreateContainerOptions {
                throughput: Some(throughput),
                ..Default::default()
            }),
        )
        .await?;

    // This should force the container to have multiple physical partitions.
    let container_client = db_client.container_client("Container");

    create_test_items(&container_client).await?;

    Ok(container_client)
}

#[recorded::test]
pub async fn cross_partition_order_by_without_query_engine_fails(
    context: TestContext,
) -> Result<(), Box<dyn Error>> {
    let account = TestAccount::from_env(context, None).await?;
    let cosmos_client = account.connect_with_key(None)?;
    let container_client = create_container(&account, &cosmos_client).await?;

    let mut items = container_client.query_items::<MockItem>(
        "SELECT * FROM c ORDER BY c.mergeOrder",
        QueryPartitionStrategy::CrossPartition,
        None,
    )?;

    let err = items
        .next()
        .await
        .transpose()
        .expect_err("should get an error");
    assert_eq!(Some(StatusCode::BadRequest), err.http_status());

    let http_err: HttpError = err.into_downcast()?;
    let msg = http_err.error_message().unwrap();
    assert!(msg.starts_with(
        "The provided cross partition query can not be directly served by the gateway."
    ));

    account.cleanup().await?;
    Ok(())
}

#[recorded::test]
pub async fn cross_partition_order_by_with_query_engine_succeeds(
    context: TestContext,
) -> Result<(), Box<dyn Error>> {
    use futures::TryStreamExt;

    let account = TestAccount::from_env(context, None).await?;
    let cosmos_client = account.connect_with_key(None)?;
    let container_client = create_container(&account, &cosmos_client).await?;

    let options = QueryOptions {
        query_engine: Some(Arc::new(MockQueryEngine::new())),
        ..Default::default()
    };
    let mut items = container_client.query_items::<MockItem>(
        "SELECT * FROM c ORDER BY c.mergeOrder",
        QueryPartitionStrategy::CrossPartition,
        Some(options),
    )?;

    let mut expected_partition_id = 0;
    let mut expected_merge_order = 0;
    let mut item_count = 0;
    while let Some(page) = items.try_next().await? {
        for item in page.into_items() {
            item_count += 1;
            assert_eq!(item.partition_key, PARTITION_KEYS[expected_partition_id]);
            assert_eq!(item.merge_order, expected_merge_order);

            expected_partition_id = (expected_partition_id + 1) % PARTITION_KEYS.len();
            expected_merge_order += 1;
        }
    }

    assert_eq!(item_count, PARTITION_KEYS.len() * ITEMS_PER_PARTITION);

    account.cleanup().await?;
    Ok(())
}
