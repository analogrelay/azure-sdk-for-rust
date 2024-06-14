use azure_data_cosmos::prelude::*;
use clap::Parser;
use futures::stream::StreamExt;

use crate::utils::CommonArgs;

#[path="_cosmos_example_utils.rs"]
mod utils;

#[derive(Debug, Parser)]
struct Args {
    #[clap(flatten)]
    common: CommonArgs,
}

#[tokio::main]
async fn main() -> azure_core::Result<()> {
    let args = Args::parse();

    // Check out the "create_client" method in "_cosmos_example_utils" for more information
    // on creating a Cosmos DB Client for various authentication methods.
    let client = args.common.create_client()?;

    // The Cosmos' client exposes a lot of methods. This one lists the databases in the specified account.
    let databases = client
        .list_databases()
        .into_stream()
        .next()
        .await
        .unwrap()?;

    println!(
        "Account {} has {} database(s)",
        args.common.account.clone(),
        databases.databases.len()
    );

    // try get on the first database (if any)
    if let Some(db) = databases.databases.first() {
        println!("getting info of database {}", &db.id);
        let db = client.database_client(db.id.clone()).get_database().await?;
        println!("db {} found == {:?}", &db.database.id, &db);
    }

    // Each Cosmos' database contains one or more collections. We can enumerate them using the
    // list_collection method.

    for db in databases.databases {
        let database = client.database_client(db.id.clone());
        let collections = database
            .list_collections()
            .into_stream()
            .next()
            .await
            .unwrap()?;
        println!(
            "database {} has {} collection(s)",
            db.id,
            collections.collections.len()
        );

        for collection in collections.collections {
            println!("\tcollection {}", collection.id);

            let collection_response = database
                .collection_client(collection.id)
                .get_collection()
                .await?;

            println!("\tcollection_response {collection_response:?}");
        }
    }

    Ok(())
}
