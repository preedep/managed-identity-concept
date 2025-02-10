use std::collections::HashMap;
use actix_web::{web, HttpRequest, HttpResponse, HttpServer, Responder};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use log::{debug, error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    aud: String, // Audience must match API_AUDIENCE
    iss: String, // Issuer must be Azure AD
    sub: String, // Subject (Service Principal or Managed Identity)
    exp: usize,  // Expiration time
    roles : Option<Vec<String>>, // Roles
}
#[derive(Debug,Clone, Serialize, Deserialize)]
struct AppState {
    jwks_url: String,
    api_audience: String,
    tenant_id: String,
}
// Global cache for JWKS keys
static JWKS_CACHE: OnceCell<HashMap<String, DecodingKey>> = OnceCell::const_new();


// Fetch JWKS keys from Azure AD
async fn fetch_jwks(jwks_url :&str) -> HashMap<String, DecodingKey> {
    let client = Client::new();
    let response = client.get(jwks_url).send().await.unwrap();
    let json: serde_json::Value = response.json().await.unwrap();

    debug!("JWKS: {:#?}", json);

    let mut keys = HashMap::new();
    for key in json["keys"].as_array().unwrap() {
        let kid = key["kid"].as_str().unwrap().to_string();
        let n = key["n"].as_str().unwrap();
        let e = key["e"].as_str().unwrap();
        let decoding_key = DecodingKey::from_rsa_components(n, e).unwrap();
        keys.insert(kid, decoding_key);
    }
    keys
}

// Validate JWT Token (for both Managed Identity & Service Principal)
async fn validate_token(token: &str,jwks_url:&str, api_audience: &str) -> Result<Claims, &'static str> {
    // Use a closure to call `fetch_jwks` with the `jwks_url` parameter
    let keys = JWKS_CACHE
        .get_or_init(|| async {
            fetch_jwks(jwks_url).await
        })
        .await;

    let header = jsonwebtoken::decode_header(token).map_err(|_| "Invalid token header")?;
    debug!("Header: {:#?}", header);
    let kid = header.kid.ok_or("No KID found")?;
    debug!("KID: {}", kid);
    let decoding_key = keys.get(&kid).ok_or("No matching JWK found")?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[api_audience]);
    let token_data = decode::<Claims>(token, decoding_key, &validation).map_err(|e| {
        error!("Error: {:#?}", e);
        "Invalid token"
    })?;
    debug!("Token: {:#?}", token_data);
    Ok(token_data.claims)
}

// Protected API Endpoint
async fn protected_endpoint(req: HttpRequest,app_state: web::Data<AppState>) -> impl Responder {
    let auth_header = req.headers().get("Authorization");
    if auth_header.is_none() {
        return HttpResponse::Unauthorized().body("Missing Authorization header");
    }

    let token = auth_header.unwrap().to_str().unwrap().replace("Bearer ", "");
    let api_audience = &app_state.api_audience;
    let jwks_url = &app_state.jwks_url;
    match validate_token(&token,jwks_url,api_audience).await {
        Ok(claims) => {
            if let Some(roles) = claims.roles {
                debug!("Roles: {:#?}", roles);
                // Check if the user has the required role
                // In this example, we are checking for the "Task.HelloWorld" role
                if !roles.contains(&"Task.HelloWorld".to_string()) {
                    return HttpResponse::Forbidden().body("Not authorized");
                }
                HttpResponse::Ok().json(format!("Welcome! Your ID is {}", claims.sub))
            } else {
                HttpResponse::Forbidden().body("Forbidden")
            }
    },
        Err(err) => HttpResponse::Unauthorized().body(err),
    }
}
#[actix_web::main]
async fn main() -> Result<(),Box<dyn std::error::Error>> {
    pretty_env_logger::init();
    info!("Starting server");

    dotenv::dotenv().ok();
    let tenant_id = std::env::var("TENANT_ID")?;
    let audience = std::env::var("API_AUDIENCE")?;
    let jwks_url = format!("https://login.microsoftonline.com/{}/discovery/v2.0/keys", tenant_id);

    debug!("Fetching JWKS from {}", jwks_url);

    let app_state = AppState {
        jwks_url,
        api_audience: audience,
        tenant_id,
    };

    debug!("App State: {:#?}", app_state);

    HttpServer::new(
        move || actix_web::App::new()
            .app_data(actix_web::web::Data::new(app_state.clone()))
            .wrap(actix_web::middleware::Logger::default())
            .route("/api_protected", actix_web::web::get().to(protected_endpoint))
    ).bind("0.0.0.0:8888")?
        .run().await?;

    Ok(())
}