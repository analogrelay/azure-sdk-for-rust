use clap::Parser;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
// Using the prelude module of the Cosmos crate makes easier to use the Rust Azure SDK for Cosmos DB.
use azure_core::prelude::*;

use azure_data_cosmos::prelude::*;
use time::OffsetDateTime;
use crate::utils::CommonArgs;

#[path="_cosmos_example_utils.rs"]
mod utils;

#[derive(Debug, Parser)]
struct Args {
    #[clap(flatten)]
    common: CommonArgs,

    /// The database to use for this example
    database: String,

    /// The collection to use for this example
    #[clap(default_value = "azure_sdk_example")]
    collection: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct MySampleStruct {
    id: String,
    a_string: String,
    a_number: u64,
    a_timestamp: i64,
}

impl azure_data_cosmos::CosmosEntity for MySampleStruct {
    type Entity = String;

    fn partition_key(&self) -> Self::Entity {
        self.id.clone()
    }
}

#[tokio::main]
async fn main() -> azure_core::Result<()> {
    let args = Args::parse();

    // Check out the "create_client" method in "_cosmos_example_utils" for more information
    // on creating a Cosmos DB Client for various authentication methods.
    let client = args.common.create_client()?;

    // list_databases will give us the databases available in our account. If there is
    // an error (for example, the given key is not valid) you will receive a
    // specific azure_data_cosmos::Error. In this example we will look for a specific database
    // so we chain a filter operation.
    let database = client
        .list_databases()
        .into_stream()
        .next()
        .await
        .unwrap()?
        .databases
        .into_iter()
        .find(|db| db.id == args.database)
        .expect(&format!("Could not find database {}", args.database));

    // If the requested database is not found we create it.
    println!("database == {database:?}");

    // Now we look for a specific collection. If is not already present
    // we will create it. The collection creation is more complex and
    // has many options (such as indexing and so on).
    let (we_created_collection, collection) = {
        let collections = client
            .database_client(database.id.clone())
            .list_collections()
            .into_stream()
            .next()
            .await
            .unwrap()?;

        if let Some(collection) = collections
            .collections
            .into_iter()
            .find(|coll| coll.id == args.collection)
        {
            (false, collection)
        } else {
            args.common.require_key_for("Create Collection");
            let c = client
                .clone()
                .database_client(database.id.clone())
                .create_collection(args.collection.clone(), "/id")
                .await?
                .collection;
            (true, c)
        }
    };

    println!("collection = {collection:?}");

    // Now that we have a database and a collection we can insert
    // data in them. Let's create a Document. The only constraint
    // is that we need an id and an arbitrary, Serializable type.
    let doc = MySampleStruct {
        id: "unique_id100".into(),
        a_string: "Something here".into(),
        a_number: 100,
        a_timestamp: OffsetDateTime::now_utc().unix_timestamp(),
    };

    // Now we store the struct in Azure Cosmos DB.
    // Notice how easy it is! :)
    // First we construct a "collection" specific client so we
    // do not need to specify it over and over.
    let collection = client
        .database_client(database.id.clone())
        .collection_client(collection.id);

    // The method create_document will return, upon success,
    // the document attributes.

    let create_document_response = collection.create_document(doc.clone()).await?;
    println!("create_document_response == {create_document_response:#?}");

    // Now we list all the documents in our collection. It
    // should show we have 1 document.
    println!("Listing documents...");
    let list_documents_response = collection
        .list_documents()
        .into_stream::<MySampleStruct>()
        .next()
        .await
        .unwrap()?;
    println!(
        "list_documents_response contains {} documents",
        list_documents_response.documents.len()
    );

    // Now we get the same document by id.
    println!("getting document by id {}", &doc.id);
    let get_document_response = collection
        .clone()
        .document_client(doc.id.clone(), &doc.id)?
        .get_document::<MySampleStruct>()
        .await?;
    println!("get_document_response == {get_document_response:#?}");

    // The document can be no longer there so the result is
    // an Option<Document<T>>
    if let GetDocumentResponse::Found(document) = get_document_response {
        // Now, for the sake of experimentation, we will update (replace) the
        // document created. We do this only if the original document has not been
        // modified in the meantime. This is called optimistic concurrency.
        // In order to do so, we pass to this replace_document call
        // the etag received in the previous get_document. The etag is an opaque value that
        // changes every time the document is updated. If the passed etag is different in
        // CosmosDB it means something else updated the document before us!
        let replace_document_response = collection
            .clone()
            .document_client(doc.id.clone(), &doc.id)?
            .replace_document(doc)
            .if_match_condition(IfMatchCondition::Match(document.etag))
            .await?;
        println!("replace_document_response == {replace_document_response:#?}");
    }

    // Clean up the collection if we created it.
    if we_created_collection {
        client
            .database_client(args.database)
            .collection_client(args.collection)
            .delete_collection()
            .await?;
        println!("collection deleted");
    } else {
        println!("collection existed before the test, not deleting it.");
    }

    Ok(())
}
