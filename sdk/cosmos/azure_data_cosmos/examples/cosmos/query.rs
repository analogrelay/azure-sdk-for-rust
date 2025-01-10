use std::error::Error;

use azure_data_cosmos::{CosmosClient, PartitionKey, QueryPartitionStrategy};
use clap::{Args, Subcommand};
use futures::TryStreamExt;

/// Run a single-partition query against a container.
#[derive(Clone, Args)]
pub struct QueryCommand {
    #[command(subcommand)]
    subcommand: Subcommands,
}

#[derive(Clone, Subcommand)]
enum Subcommands {
    Items {
        /// The database to query.
        database: String,

        /// The container to query.
        container: String,

        /// The query to execute.
        query: String,

        /// The partition key to use when querying the container. This can be ommitted to attempt a cross-partition query using the Gateway.
        #[arg(long, short)]
        partition_key: Option<String>,
    },
    Databases {
        /// The query to execute.
        query: String,
    },
    Containers {
        /// The database to query.
        database: String,

        /// The query to execute.
        query: String,
    },
}

impl QueryCommand {
    pub async fn run(self, client: CosmosClient) -> Result<(), Box<dyn Error>> {
        match self.subcommand {
            Subcommands::Items {
                database,
                container,
                query,
                partition_key,
            } => {
                let db_client = client.database_client(&database);
                let container_client = db_client.container_client(&container);

                let strategy = match partition_key {
                    Some(pk) => QueryPartitionStrategy::SinglePartition(PartitionKey::from(pk)),
                    None => QueryPartitionStrategy::CrossPartition,
                };
                let mut items =
                    container_client.query_items::<serde_json::Value>(&query, strategy, None)?;
                while let Some(page) = items.try_next().await? {
                    println!("Results Page");
                    println!("  Items:");
                    for item in page.into_items() {
                        println!("    * {:#?}", item);
                    }
                }
                Ok(())
            }
            Subcommands::Databases { query } => {
                let mut dbs = client.query_databases(query, None)?;

                while let Some(page) = dbs.try_next().await? {
                    println!("Results Page");
                    println!("  Databases:");
                    for item in page.into_items() {
                        println!("    * {:#?}", item);
                    }
                }
                Ok(())
            }
            Subcommands::Containers { database, query } => {
                let db_client = client.database_client(&database);
                let mut dbs = db_client.query_containers(query, None)?;

                while let Some(page) = dbs.try_next().await? {
                    println!("Results Page");
                    println!("  Containers:");
                    for item in page.into_items() {
                        println!("    * {:#?}", item);
                    }
                }
                Ok(())
            }
        }
    }
}
