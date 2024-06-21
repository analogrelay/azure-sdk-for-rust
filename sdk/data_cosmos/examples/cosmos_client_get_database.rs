use azure_core::headers::{HeaderName, HeaderValue, Headers};
use azure_core::prelude::*;
use azure_core::CustomHeaders;
use clap::Parser;

use azure_data_cosmos::prelude::*;use crate::utils::CommonArgs;

#[path="_cosmos_example_utils.rs"]
mod utils;

#[derive(Debug, Parser)]
struct Args {
    #[clap(flatten)]
    common: CommonArgs,

    /// The database to use for this example
    #[clap(default_value = "azure_sdk_example_db")]
    database: String,
}

#[tokio::main]
async fn main() -> azure_core::Result<()> {
    let args = Args::parse();
    let client = args.common.create_client()?;
    let database = client.database_client(args.database.clone());

    let mut context = Context::new();

    // Next we create a CustomHeaders type and insert it into the context allowing us to insert custom headers.
    let custom_headers: CustomHeaders = {
        let mut custom_headers = std::collections::HashMap::<HeaderName, HeaderValue>::new();
        custom_headers.insert("mycoolheader".into(), "CORS maybe?".into());
        let hs: Headers = custom_headers.into();
        hs.into()
    };

    context.insert(custom_headers);

    let response = database.get_database().context(context).await?;
    println!("response == {response:?}");

    Ok(())
}
