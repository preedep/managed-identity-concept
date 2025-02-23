use azure_core::auth::TokenCredential;
use azure_identity::{DefaultAzureCredential, TokenCredentialOptions};
use dotenv::dotenv;
use log::{debug, info};
use reqwest::Client;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    dotenv().ok();

    let api_url = std::env::var("API_URL").expect("API_URL is not set");
    let resource = std::env::var("RESOURCE_NAME").expect("RESOURCE_NAME is not set");

    let client = Client::new();

    // Use Managed Identity with DefaultAzureCredential
    let credential = DefaultAzureCredential::create(TokenCredentialOptions::default())?;
    // Get a token for the resource
    // Example resource > "https://management.azure.com/" or api://<resource-id>
    let token_response = credential.get_token(&[resource.as_str()]).await?;
    let access_token = token_response.token.secret();

    debug!("Access Token: {}", access_token);

    // Call the protected API with the token
    let api_response = client
        .get(&api_url)
        .bearer_auth(access_token)
        .send()
        .await?;

    let result = api_response.text().await?;
    info!("API Response: {}", result);

    Ok(())
}
