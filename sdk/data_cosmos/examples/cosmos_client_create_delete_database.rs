use azure_data_cosmos::prelude::*;
use clap::Parser;
use futures::stream::StreamExt;
use crate::utils::CommonArgsRequireKey;

#[path="_cosmos_example_utils.rs"]
mod utils;

#[derive(Debug, Parser)]
struct Args {
    #[clap(flatten)]
    common: CommonArgsRequireKey,

    /// The name of the database to create (and then delete)
    database_name: String,
}

#[tokio::main]
async fn main() -> azure_core::Result<()> {
    let args = Args::parse();

    // Create/Delete database require using an Authentication Key rather than Entra ID
    // (You can use the Azure Resource Manager APIs to create/delete Cosmos Databases via Entra ID)
    //
    // Check out the "create_client" method in "_cosmos_example_utils" for more information
    // on creating a Cosmos DB Client for various authentication methods.
    let client = args.common.create_client()?;

    // The Cosmos' client exposes a lot of methods. This one lists the databases in the specified
    // account. Database do not implement Display but deref to &str so you can pass it to methods
    // both as struct or id.

    {
        let mut list_databases_stream = client.list_databases().into_stream();
        while let Some(list_databases_response) = list_databases_stream.next().await {
            println!("list_databases_response = {:#?}", list_databases_response?);
        }
    }

    let db = client.create_database(&args.database_name).await?;
    println!("created database = {db:#?}");

    // create collection!
    {
        let database = client.database_client(args.database_name.clone());
        let create_collection_response = database.create_collection("panzadoro", "/id").await?;

        println!("create_collection_response == {create_collection_response:#?}");

        let db_collection = database.collection_client("panzadoro");

        let get_collection_response = db_collection.get_collection().await?;
        println!("get_collection_response == {get_collection_response:#?}");

        let mut stream = database.list_collections().into_stream();
        while let Some(res) = stream.next().await {
            let res = res?;
            println!("res == {res:#?}");
        }

        let delete_response = db_collection.delete_collection().await?;
        println!("collection deleted: {delete_response:#?}");
    }

    let resp = client
        .database_client(args.database_name)
        .delete_database()
        .await?;
    println!("database deleted. resp == {resp:#?}");

    Ok(())
}
