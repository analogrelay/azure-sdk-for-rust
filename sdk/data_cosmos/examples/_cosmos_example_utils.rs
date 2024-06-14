use clap::Parser;
use azure_data_cosmos::clients::CosmosClient;
use azure_data_cosmos::prelude::AuthorizationToken;

fn main() {
    panic!("This is not an example. It's shared code for other examples to use. Try running one of the other cosmos examples!");
}

#[derive(Debug, Parser)]
pub struct CommonArgs {
    /// The cosmos account you're using
    #[clap(env = "AZURE_COSMOS_ACCOUNT")]
    pub account: String,

    /// The key to use to authenticate with the account. If omitted, Entra ID auth will be used.
    #[clap(short, long, env = "AZURE_COSMOS_KEY")]
    pub key: Option<String>,
}

impl CommonArgs {
    pub fn create_client(&self) -> azure_core::Result<CosmosClient> {
        let token = if let Some(ref k) = self.key {
            // Connect using a key.
            AuthorizationToken::primary_key(k)?
        } else {
            // Connect using an Entra ID token.
            let cred = azure_identity::create_credential()?;
            AuthorizationToken::from_token_credential(cred)
        };

        Ok(CosmosClient::new(self.account.clone(), token))
    }
}
