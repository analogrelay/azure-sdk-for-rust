use azure_data_cosmos::prelude::*;
use clap::Parser;
use futures::stream::StreamExt;

#[derive(Debug, Parser)]
struct Args {
    /// The cosmos account you're using
    #[clap(env = "AZURE_COSMOS_ACCOUNT")]
    account: String,

    /// The key to use to authenticate with the account. If omitted, Entra ID auth will be used.
    #[clap(short, long, env = "AZURE_COSMOS_KEY")]
    key: Option<String>,
}

#[tokio::main]
async fn main() -> azure_core::Result<()> {
    // First we retrieve the account name and access key from environment variables.
    // We expect access keys (ie, not resource constrained)
    let args = Args::parse();

    let authorization_token = if let Some(k) = args.key {
        // Connect using a key.
        AuthorizationToken::primary_key(k)?
    } else {
        // Connect using an Entra ID token.
        let cred = azure_identity::create_credential()?;
        AuthorizationToken::from_token_credential(cred)
    };

    // Once we have an authorization token you can create a client instance. You can change the
    // authorization token at later time if you need, for example, to escalate the privileges for a
    // single operation.
    // Here we are using reqwest but other clients are supported (check the documentation).
    let client = CosmosClient::new(&args.account, authorization_token);

    // The Cosmos' client exposes a lot of methods. This one lists the databases in the specified account.
    let databases = client
        .list_databases()
        .into_stream()
        .next()
        .await
        .unwrap()?;

    println!(
        "Account {} has {} database(s)",
        args.account,
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
