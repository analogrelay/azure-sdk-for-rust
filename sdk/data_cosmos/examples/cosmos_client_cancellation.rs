use azure_data_cosmos::prelude::*;
use clap::Parser;
use stop_token::prelude::*;
use stop_token::StopSource;
use tokio::time::{Duration, Instant};
use azure_core::error::ErrorKind;
use crate::utils::CommonArgsRequireKey;

#[path="_cosmos_example_utils.rs"]
mod utils;

#[derive(Debug, Parser)]
struct Args {
    #[clap(flatten)]
    common: CommonArgsRequireKey,

    /// A prefix to use on the names of the databases created by this example.
    #[clap(default_value = "azure_sdk_cancellation_")]
    database_prefix: String,
}

#[tokio::main]
async fn main() -> azure_core::Result<()> {
    tracing_subscriber::fmt().init();
    // First we retrieve the account name and access key from environment variables, and
    // create an authorization token.
    let args = Args::parse();
    let client = args.common.create_client()?;

    let database_names: Vec<_> = (0..10).map(|i| format!("{}{}", args.database_prefix, i)).collect();

    // Create a new database, and time out if it takes more than 1 second.
    let future = client.create_database(database_names[0].clone()).into_future();
    let deadline = Instant::now() + Duration::from_secs(1);
    match future.timeout_at(deadline).await {
        Ok(Ok(r)) => println!("successful response: {r:?}"),
        Ok(Err(e)) => println!("request was made but failed: {e:?}"),
        Err(_) => println!("request timed out!"),
    };

    // Create multiple new databases, and cancel them if they don't complete before
    // they're sent a stop signal.
    let source = StopSource::new();
    for i in 1..10 {
        let client = client.clone();
        // Clone the stop token for each request.
        let deadline = source.token();
        let database_name = database_names[i].clone();
        tokio::spawn(async move {
            let future = client.create_database(database_name).into_future();
            match future.timeout_at(deadline).await {
                Ok(Ok(r)) => println!("successful response: {r:?}"),
                Ok(Err(e)) => println!("request was made but failed: {e:?}"),
                Err(_) => println!("request was cancelled!"),
            };
        });
    }

    tokio::time::sleep(Duration::from_secs(5)).await;
    // This causes all cancel tokens to fire. Any request tied to a stop token created
    // from this source will be canceled.
    println!("cancelling all requests");
    drop(source);
    // Any request that has not yet completed will be canceled at this point

    // Keep the program alive for a bit longer so the tasks get a chance to
    // print before exiting.
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Now clean up any databases that were created.
    for database_name in database_names {
        match client.database_client(database_name.clone()).delete_database().await {
            Ok(_) => println!("Cleaned up database {database_name}"),
            Err(e) => {
                if let ErrorKind::HttpResponse { status, .. } = e.kind() {
                    if *status == 404 {
                        println!("Database {database_name} not found, that's fine.");
                    } else {
                        println!("Error cleaning up database {database_name}: {e}");
                    }
                } else {
                    println!("Error cleaning up database {database_name}: {e}");
                }
            }
        };
    }
    Ok(())
}
