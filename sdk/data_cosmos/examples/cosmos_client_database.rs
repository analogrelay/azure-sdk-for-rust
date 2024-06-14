use azure_data_cosmos::prelude::*;
use clap::Parser;
use futures::stream::StreamExt;
use serde_json::Value;
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

    let dbs = client
        .list_databases()
        .into_stream()
        .next()
        .await
        .unwrap()?;

    for db in dbs.databases {
        println!("Database: {db:?}");
        let database = client.database_client(db.name().to_owned());

        // List all the collections
        let collections = database
            .list_collections()
            .into_stream()
            .next()
            .await
            .unwrap()?;
        for collection in collections.collections {
            println!("Collection: {collection:?}");
            let mut indexing_policy_new = collection.indexing_policy.clone();
            let collection = database.collection_client(collection.id);

            if collection.collection_name() == "democ" {
                println!("Found democ collection!");

                let data = r#"
                {
                    "id": "my_id",
                    "name": "John Tonno7",
                    "age": 43,
                    "phones": [
                        "+44 1234567",
                        "+44 2345678"
                    ]
                }"#;
                let document: Value = serde_json::from_str(data)?;

                let create_document = collection
                    .create_document(document)
                    .is_upsert(true)
                    .partition_key(&43u32)?
                    .await?;

                println!("`create_document` response: {create_document:?}");

                // Alternatively, we can just fetch a specific collection directly
                let _ = database.collection_client("cnt").get_collection().await?;

                // Replace the collection's indexing policy
                indexing_policy_new
                    .excluded_paths
                    .push("/\"collo2\"/?".to_owned().into());

                println!("\nReplacing collection");
                let replace_collection_response = collection
                    .replace_collection("/age")
                    .indexing_policy(indexing_policy_new)
                    .await?;
                println!("`replace_collection` response: {replace_collection_response:#?}");
            }

            // Fetch all the documents as json
            let documents = collection
                .list_documents()
                .into_stream::<Value>()
                .next()
                .await
                .unwrap()?;
            println!("\n`list_documents` as json: {documents:?}");
        }
    }

    Ok(())
}
