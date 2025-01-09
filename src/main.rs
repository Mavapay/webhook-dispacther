use actix_cors::Cors;
use actix_files;
use actix_web::{web, App, HttpResponse, HttpServer};
use reqwest; // Using reqwest instead of awc for better thread safety
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WebhookEvent {
    #[serde(flatten)]
    payload: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WebhookEndpoint {
    #[serde(default)]
    id: String,
    url: String,
    name: String,
    #[serde(default)]
    is_active: bool,
}

// Add this new struct for the registration request
#[derive(Debug, Serialize, Deserialize)]
struct CreateWebhookRequest {
    url: String,
    name: String,
    #[serde(default)]
    is_active: bool,
}

struct AppState {
    endpoints: RwLock<Vec<WebhookEndpoint>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EndpointUpdate {
    is_active: bool,
}

// Endpoint to register new webhook endpoint
async fn register_endpoint(
    endpoint_req: web::Json<CreateWebhookRequest>,
    data: web::Data<AppState>,
) -> HttpResponse {
    // Validate URL
    if let Err(e) = url::Url::parse(&endpoint_req.url) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid URL format",
            "details": e.to_string()
        }));
    }

    // Validate name
    if endpoint_req.name.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Name cannot be empty"
        }));
    }

    let new_endpoint = WebhookEndpoint {
        id: uuid::Uuid::new_v4().to_string(),
        url: endpoint_req.url.clone(),
        name: endpoint_req.name.clone(),
        is_active: endpoint_req.is_active,
    };

    let mut endpoints = data.endpoints.write().unwrap();
    endpoints.push(new_endpoint.clone());

    HttpResponse::Ok().json(endpoints.clone())
}

// Endpoint to list all registered webhooks
async fn list_endpoints(data: web::Data<AppState>) -> HttpResponse {
    let endpoints = data.endpoints.read().unwrap();
    HttpResponse::Ok().json(endpoints.clone())
}

// Update endpoint status (active/inactive)
async fn update_endpoint(
    path: web::Path<String>,
    update: web::Json<EndpointUpdate>,
    data: web::Data<AppState>,
) -> HttpResponse {
    let id = path.into_inner();
    let mut endpoints = data.endpoints.write().unwrap();
    if let Some(endpoint) = endpoints.iter_mut().find(|e| e.id == id) {
        endpoint.is_active = update.is_active;
        HttpResponse::Ok().json(endpoint)
    } else {
        HttpResponse::NotFound().finish()
    }
}

// Endpoint to delete a webhook endpoint
async fn delete_endpoint(
    endpoint_id: web::Path<String>,
    data: web::Data<AppState>,
) -> HttpResponse {
    let id = endpoint_id.into_inner();
    let mut endpoints = data.endpoints.write().unwrap();
    if let Some(pos) = endpoints.iter().position(|e| e.id == id) {
        endpoints.remove(pos);
        HttpResponse::Ok().json(endpoints.clone())
    } else {
        HttpResponse::NotFound().finish()
    }
}

// Forward webhook to specific endpoint
async fn forward_webhook(
    client: &reqwest::Client,
    endpoint: &WebhookEndpoint,
    payload: &WebhookEvent,
) -> Result<(), String> {
    match client.post(&endpoint.url).json(&payload).send().await {
        Ok(response) if response.status().is_success() => Ok(()),
        Ok(response) => Err(format!(
            "Failed to forward to {}: HTTP {}",
            endpoint.name,
            response.status()
        )),
        Err(e) => Err(format!("Error forwarding to {}: {}", endpoint.name, e)),
    }
}

// Webhook receiver endpoint that forwards to active endpoints
async fn receive_webhook(
    payload: web::Json<WebhookEvent>,
    data: web::Data<AppState>,
) -> HttpResponse {
    let endpoints = data.endpoints.read().unwrap();
    let active_endpoints: Vec<_> = endpoints.iter().filter(|e| e.is_active).collect();

    if active_endpoints.is_empty() {
        return HttpResponse::Ok().json(serde_json::json!({
            "status": "no_active_endpoints",
            "message": "No active endpoints configured"
        }));
    }

    let client = reqwest::Client::new();
    let mut errors = Vec::new();

    for endpoint in active_endpoints {
        if let Err(error) = forward_webhook(&client, endpoint, &payload).await {
            errors.push(error);
        }
    }

    if errors.is_empty() {
        HttpResponse::Ok().json(serde_json::json!({
            "status": "success",
            "message": "Webhook forwarded to active endpoints"
        }))
    } else {
        HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "errors": errors
        }))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_address = format!("0.0.0.0:{}", port);

    println!("Starting webhook relay server on {}", bind_address);

    let app_state = web::Data::new(AppState {
        endpoints: RwLock::new(Vec::new()),
    });

    HttpServer::new(move || {
        let cors = Cors::permissive(); // For development only

        App::new()
            .wrap(cors)
            .app_data(app_state.clone())
            .route("/webhook", web::post().to(receive_webhook))
            .route("/endpoints", web::post().to(register_endpoint))
            .route("/endpoints", web::get().to(list_endpoints))
            .route("/endpoints/{id}", web::delete().to(delete_endpoint))
            .route("/endpoints/{id}/status", web::put().to(update_endpoint))
            .service(actix_files::Files::new("/", "./static").index_file("index.html"))
    })
    .bind(&bind_address)?
    .run()
    .await
}
