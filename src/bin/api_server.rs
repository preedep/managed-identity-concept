use std::collections::HashMap;
use actix_web::{web, HttpRequest, HttpResponse, HttpServer, Responder};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use log::{debug, error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;

/// Represents the claims contained in a JWT token.
    ///
    /// # Fields
    ///
    /// * `aud` - A string that holds the audience of the token. Must match `API_AUDIENCE`.
    /// * `iss` - A string that holds the issuer of the token. Must be Azure AD.
    /// * `sub` - A string that holds the subject of the token (Service Principal or Managed Identity).
    /// * `exp` - A usize that holds the expiration time of the token.
    /// * `roles` - An optional vector of strings that holds the roles associated with the token.
    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        aud: String, // Audience must match API_AUDIENCE
        iss: String, // Issuer must be Azure AD
        sub: String, // Subject (Service Principal or Managed Identity)
        exp: usize,  // Expiration time
        roles: Option<Vec<String>>, // Roles
    }

/// Represents the application state containing configuration details.
///
/// # Fields
///
/// * `jwks_url` - A string that holds the URL to fetch the JSON Web Key Sets (JWKS).
/// * `api_audience` - A string that holds the expected audience for the API.
/// * `tenant_id` - A string that holds the tenant ID for the Azure Active Directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppState {
    jwks_url: String,
    api_audience: String,
    tenant_id: String,
}
// Global cache for JWKS keys
static JWKS_CACHE: OnceCell<HashMap<String, DecodingKey>> = OnceCell::const_new();



/// Fetches JSON Web Key Sets (JWKS) from the specified URL and returns a HashMap of decoding keys.
    ///
    /// # Arguments
    ///
    /// * `jwks_url` - A string slice that holds the URL to fetch the JWKS from.
    ///
    /// # Returns
    ///
    /// A `HashMap` where the keys are the Key IDs (KID) and the values are the corresponding `DecodingKey` objects.
    ///
    /// # Errors
    ///
    /// This function will panic if the HTTP request fails or if the response cannot be parsed as JSON.
    ///
    /// # Example
    ///
    /// ```
    /// let jwks_url = "https://example.com/jwks";
    /// let keys = fetch_jwks(jwks_url).await;
    /// ```
    ///
    /// # Remarks
    ///
    /// This function uses the `reqwest` crate to perform the HTTP request and the `serde_json` crate to parse the JSON response.
    async fn fetch_jwks(jwks_url: &str) -> HashMap<String, DecodingKey> {
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


/// Validates a JWT token using the JWKS fetched from the specified URL and the provided API audience.
        ///
        /// # Arguments
        ///
        /// * `token` - A string slice that holds the JWT token to be validated.
        /// * `jwks_url` - A string slice that holds the URL to fetch the JWKS from.
        /// * `api_audience` - A string slice that holds the expected audience for the token.
        ///
        /// # Returns
        ///
        /// A `Result` which is:
        /// * `Ok(Claims)` if the token is valid and contains the expected claims.
        /// * `Err(&'static str)` if the token is invalid or any error occurs during validation.
        ///
        /// # Errors
        ///
        /// This function will return an error if:
        /// * The token header is invalid.
        /// * The KID (Key ID) is not found in the token header.
        /// * There is no matching JWK (JSON Web Key) for the KID.
        /// * The token is invalid according to the provided validation criteria.
        ///
        /// # Example
        ///
        /// ```
        /// let token = "your.jwt.token";
        /// let jwks_url = "https://example.com/jwks";
        /// let api_audience = "your_api_audience";
        /// let claims = validate_token(token, jwks_url, api_audience).await;
        /// ```
        async fn validate_token(token: &str, jwks_url: &str, api_audience: &str) -> Result<Claims, &'static str> {
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