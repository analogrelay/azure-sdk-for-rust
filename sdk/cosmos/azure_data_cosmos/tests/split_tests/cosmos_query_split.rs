// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.
#![cfg(feature = "key_auth")]

use super::framework;

use std::collections::BTreeSet;
use std::error::Error;
use std::num::NonZeroU32;
use std::time::{Duration, Instant};

use azure_data_cosmos::{
    models::{ContainerProperties, ThroughputProperties},
    options::{MaxItemCountHint, QueryOptions},
    query::FeedScope,
    ContinuationToken, CreateContainerOptions, ReadFeedRangesOptions,
};
use framework::{MockItem, TestClient, TestOptions};
use futures::{StreamExt, TryStreamExt};

const PAGE_SIZE: u32 = 5;
const PARTITION_KEY_COUNT: usize = 5;
const ITEMS_PER_PARTITION_KEY: usize = 5;

// TODO: scale this timeout down before merging once we've measured how long
// the split actually takes on the live service. The poll loop prints the
// elapsed duration so we can pick a tighter value.
const SPLIT_POLL_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const SPLIT_POLL_INTERVAL: Duration = Duration::from_secs(15);

#[tokio::test]
#[cfg_attr(
    not(test_category = "split"),
    ignore = "requires test_category 'split'"
)]
pub async fn query_continuation_survives_partition_split() -> Result<(), Box<dyn Error>> {
    TestClient::run_with_unique_db(
        async |run_context, db_client| {
            // Create a container with a single physical partition by
            // pinning throughput to 1000 RU/s.
            let properties =
                ContainerProperties::new("QuerySplitContainer", "/partitionKey".into());
            let throughput = ThroughputProperties::manual(1000);
            let container_client = run_context
                .create_container(
                    db_client,
                    properties,
                    Some(CreateContainerOptions::default().with_throughput(throughput)),
                )
                .await?;

            println!("Container created with 1000 RU/s throughput to ensure single physical partition, inserting docs");

            // Seed enough items across multiple PK values that a
            // split can actually redistribute documents and that a page size
            // of PAGE_SIZE yields at least 3 pages.
            let mut expected_ids: BTreeSet<String> = BTreeSet::new();
            for p in 0..PARTITION_KEY_COUNT {
                let partition_key = format!("partition{p}");
                for i in 0..ITEMS_PER_PARTITION_KEY {
                    let item = MockItem {
                        id: format!("{p}-{i}"),
                        partition_key: partition_key.clone(),
                        merge_order: p * ITEMS_PER_PARTITION_KEY + i,
                    };
                    expected_ids.insert(item.id.clone());
                    container_client
                        .create_item(item.partition_key.clone(), &item.id.clone(), item, None)
                        .await?;
                }
            }
            assert!(
                expected_ids.len() >= (PAGE_SIZE as usize) * 3,
                "need at least 3 pages worth of items, have {}",
                expected_ids.len()
            );

            println!("Documents inserted, starting query with pagination to capture continuation token");

            // Confirm single physical partition.
            let ranges_before = container_client.read_feed_ranges(None).await?;
            assert!(
                ranges_before.len() == 1,
                "expected single physical partition before split, got {}",
                ranges_before.len()
            );

            // Fetch a single page and capture a continuation token.
            let mut collected: BTreeSet<String> = BTreeSet::new();
            let saved_token = {
                let initial_options = QueryOptions::default().with_max_item_count(
                    MaxItemCountHint::Limit(NonZeroU32::new(PAGE_SIZE).unwrap()),
                );
                let mut pages = container_client
                    .query_items::<MockItem>(
                        "SELECT * FROM c",
                        FeedScope::full_container(),
                        Some(initial_options),
                    )
                    .await?
                    .into_pages();

                let first_page = pages
                    .next()
                    .await
                    .expect("query should yield at least one page before split")?;
                for item in first_page.into_items() {
                    collected.insert(item.id);
                }

                // Round-trip through string form to mirror real usage (e.g.
                // persisting the token across processes).
                let token = pages.to_continuation_token()?;
                let serialized = token.as_str().to_owned();
                drop(pages);

                // Assert that we've just got the first five items:
                let expected_first_page: BTreeSet<String> = expected_ids
                    .iter()
                    .take(PAGE_SIZE as usize)
                    .cloned()
                    .collect();
                assert_eq!(
                    collected, expected_first_page,
                    "first page should contain the first {} items in id sort order",
                    PAGE_SIZE
                );
                ContinuationToken::from_string(serialized)
            };

            println!("Captured continuation token after fetching first page, now updating throughput to trigger split");

            // Force a split by raising throughput to 13000 RU/s
            // (>10k forces at least 2 physical partitions).
            let new_throughput = ThroughputProperties::manual(13000);
            let mut poller = container_client
                .begin_replace_throughput(new_throughput, None)
                .await?;
            println!("Throughput update initiated, polling for completion...");
            let mut last_throughput = None;
            let mut poll_count = 0;
            while let Some(status) = poller.try_next().await? {
                assert!(status.status().is_success());
                last_throughput = Some(status.into_model()?);
                if poll_count % 15 == 0 {
                    println!(
                        "Throughput update in progress... polled {} times, last observed throughput: {} RU/s",
                        poll_count,
                        last_throughput.as_ref().and_then(|t| t.throughput()).unwrap_or(0)
                    );
                }
                poll_count += 1;
            }
            let final_throughput = last_throughput
                .expect("throughput poller should have yielded at least one response");
            assert_eq!(Some(13000), final_throughput.throughput());
            println!("Throughput update completed, new throughput: {} RU/s", final_throughput.throughput().unwrap_or(0));

            // Poll read_feed_ranges until we observe >= 2 physical
            // partitions or the timeout elapses.
            let split_start = Instant::now();
            let mut iterations = 0;
            let observed_ranges = loop {
                let ranges = container_client
                    .read_feed_ranges(Some(ReadFeedRangesOptions::default().with_force_refresh(true)))
                    .await?;
                if ranges.len() >= 2 {
                    break ranges;
                }
                if split_start.elapsed() >= SPLIT_POLL_TIMEOUT {
                    panic!(
                        "split did not occur within {:?} (last range count: {})",
                        SPLIT_POLL_TIMEOUT,
                        ranges.len()
                    );
                }

                // Every minute, print a message to indicate we're still waiting and the test
                if iterations % 4 == 0 {
                    println!(
                        "Waiting for split to complete... elapsed: {:?}, last observed physical partition count: {}",
                        split_start.elapsed(),
                        ranges.len()
                    );
                }
                tokio::time::sleep(SPLIT_POLL_INTERVAL).await;
                iterations += 1;
            };
            println!(
                "Split observed after {:?}; physical partition count: {}",
                split_start.elapsed(),
                observed_ranges.len()
            );

            // Resume pagination using the saved continuation token.
            // Round-trip the token between every page so we keep exercising
            // the suspend/resume path now that the topology has changed.
            let mut continuation = Some(saved_token);
            loop {
                let mut resume_options = QueryOptions::default().with_max_item_count(
                    MaxItemCountHint::Limit(NonZeroU32::new(PAGE_SIZE).unwrap()),
                );
                if let Some(token) = continuation.take() {
                    resume_options = resume_options.with_continuation_token(token);
                }

                let mut pages = container_client
                    .query_items::<MockItem>(
                        "SELECT * FROM c",
                        FeedScope::full_container(),
                        Some(resume_options),
                    )
                    .await?
                    .into_pages();

                let Some(page) = pages.next().await else {
                    break;
                };
                let page = page?;
                for item in page.into_items() {
                    collected.insert(item.id);
                }

                let token = pages.to_continuation_token()?;
                let serialized = token.as_str().to_owned();
                drop(pages);
                continuation = Some(ContinuationToken::from_string(serialized));
            }

            // Validate the full result set. Using BTreeSet equality
            // catches both missing items and accidental duplicates.
            assert_eq!(
                collected,
                expected_ids,
                "items collected across split should match the seeded ground truth"
            );

            Ok(())
        },
        Some(TestOptions::new().with_timeout(Duration::from_secs(40 * 60))),
    )
    .await
}
