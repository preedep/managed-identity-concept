use reqwest::Client;
use serde_json::Value;
use std::error::Error;
use log::{debug, info};



#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    dotenv::dotenv().ok();

    let api_url = std::env::var("API_URL").expect("API_URL is not set");
    let resource = std::env::var("RESOURCE_NAME").expect("RESOURCE is not set");

    let client = Client::new();

    // Request a token from Azure IMDS
    let identity_endpoint = "http://169.254.169.254/metadata/identity/oauth2/token";
    let token_response = client
        .get(identity_endpoint)
        .query(&[
            ("api-version", "2019-08-01"),
            ("resource", resource.as_str()),
        ])
        .header("Metadata", "true")
        .send()
        .await?;

    let token_body: Value = token_response.json().await?;
    let access_token = token_body["access_token"].as_str().unwrap();

    debug!("Access Token: {}", access_token);

    // Call the protected API with the token
    let api_response = client
        .get(api_url)
        .bearer_auth(access_token)
        .send()
        .await?;

    let result = api_response.text().await?;
    info!("API Response: {}", result);

    Ok(())
}