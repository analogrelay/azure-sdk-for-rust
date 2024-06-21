use azure_data_cosmos::prelude::*;
use clap::Parser;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use crate::utils::CommonArgs;

#[path="_cosmos_example_utils.rs"]
mod utils;

#[derive(Debug, Parser)]
struct Args {
    #[clap(flatten)]
    common: CommonArgs,

    /// The database to use for this example
    #[clap(default_value = "azure_sdk_example_db")]
    database: String,

    /// The collection to use for this example
    #[clap(default_value = "azure_sdk_examples")]
    collection: String,
}

// Now we create a sample struct.
#[derive(Serialize, Deserialize, Clone, Debug)]
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

// NOTE: Attachments are a legacy feature and are not enabled for new accounts.
// Use this functionality only if you have an existing account that has this feature enabled.
// See https://aka.ms/cosmosdb-attachments for more information.

// This example expects you to have created a collection
// with partitionKey on "id".
#[tokio::main]
async fn main() -> azure_core::Result<()> {
    let args = Args::parse();
    let client = args.common.create_client()?
        .database_client(args.database)
        .collection_client(args.collection);

    let doc = MySampleStruct {
        id: format!("unique_id{}", 100),
        a_string: "Something here".into(),
        a_number: 100,
        a_timestamp: OffsetDateTime::now_utc().unix_timestamp(),
    };

    // let's add an entity.
    match client.create_document(doc.clone()).await {
        Ok(_) => {
            println!("document created");
        }
        Err(err) => {
            println!("already exists? ==> {err:?}");
        }
    };

    let document = client.document_client(doc.id.clone(), &doc.id)?;

    // list attachments
    let ret = document
        .list_attachments()
        .into_stream()
        .next()
        .await
        .unwrap()?;
    println!("list attachments == {ret:#?}");

    // reference attachment
    println!("creating");
    let attachment = document.attachment_client("myref06");
    let resp = attachment
        .create_attachment(
            "https://cdn.pixabay.com/photo/2020/01/11/09/30/abstract-background-4756987__340.jpg",
            "image/jpeg",
        )
        .consistency_level(ret)
        .await?;
    println!("create reference == {resp:#?}");

    // we pass the consistency level to make
    // sure to find the just created attachment
    let session_token: ConsistencyLevel = resp.into();

    let resp = attachment.get().consistency_level(session_token).await?;

    println!("get attachment == {resp:#?}");
    let session_token: ConsistencyLevel = resp.into();

    println!("replacing");
    let attachment = document.attachment_client("myref06");
    let resp = attachment
        .replace_attachment(
            "https://Adn.pixabay.com/photo/2020/01/11/09/30/abstract-background-4756987__340.jpg",
            "image/jpeg",
        )
        .consistency_level(session_token)
        .await?;
    println!("replace reference == {resp:#?}");

    println!("deleting");
    let resp_delete = attachment.delete().consistency_level(&resp).await?;
    println!("delete attachment == {resp_delete:#?}");

    // slug attachment
    println!("creating slug attachment");
    let attachment = document.attachment_client("slug00".to_owned());
    let resp = attachment
        .create_slug("FFFFF".into())
        .consistency_level(&resp_delete)
        .content_type("text/plain")
        .await?;

    println!("create slug == {resp:#?}");

    println!("deleting");
    let resp_delete = attachment.delete().consistency_level(&resp).await?;
    println!("delete attachment == {resp_delete:#?}");

    Ok(())
}
