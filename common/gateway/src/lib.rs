use anyhow::{anyhow, Result};
use async_trait::async_trait;
use hyper::{
    header,
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode,
};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use futures::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    tungstenite::protocol::Message, WebSocketStream,
};
use std::time::Duration;
use tower;

// Re-exports
pub use hyper;

// Routes vector - replace distributed_slice with a simple static Vec
pub static mut ROUTES: Vec<RouteInfo> = Vec::new();

// Helper function to register routes safely
pub fn register_route(route: RouteInfo) {
    unsafe {
        ROUTES.push(route);
    }
}

/// Information about a route
pub struct RouteInfo {
    pub method: &'static str,
    pub path: &'static str, 
    pub handler_name: &'static str,
    pub middleware: Option<Vec<String>>,
}

/// Gateway trait for implementing an API gateway
#[async_trait]
pub trait Gateway: Send + Sync {
    async fn run(&self) -> Result<()>;
}

/// SSL configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslConfig {
    pub enabled: bool,
    pub cert_file: Option<String>,
    pub key_file: Option<String>,
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allow_credentials: bool,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub default_rate: u32,
    pub default_burst: u32,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub expiration: u32,
}

/// Gateway configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub host: String,
    pub port: u16,
    pub services: Vec<String>,
    pub ssl: SslConfig,
    pub cors: CorsConfig,
    pub rate_limit: RateLimitConfig,
    pub auth: AuthConfig,
    pub middleware: Vec<String>,
    pub config_file: Option<String>,
}

/// Middleware for processing HTTP requests
#[async_trait]
pub trait Middleware: Send + Sync {
    async fn process(&self, request: &Request<Body>, next: Next<'_>) -> Result<Response<Body>>;
}

/// Next middleware in the chain
pub struct Next<'a> {
    middlewares: &'a [Box<dyn Middleware>],
    handler: &'a (dyn Fn(&Request<Body>) -> Result<Response<Body>> + Send + Sync),
}

impl<'a> Next<'a> {
    pub fn new(
        middlewares: &'a [Box<dyn Middleware>],
        handler: &'a (dyn Fn(&Request<Body>) -> Result<Response<Body>> + Send + Sync),
    ) -> Self {
        Self {
            middlewares,
            handler,
        }
    }

    pub async fn run(self, req: &Request<Body>) -> Result<Response<Body>> {
        if let Some((current, rest)) = self.middlewares.split_first() {
            let next = Next {
                middlewares: rest,
                handler: self.handler,
            };
            current.process(req, next).await
        } else {
            (self.handler)(req)
        }
    }
}

/// WebSocket connection wrapper
pub struct WebSocketConnection {
    id: String,
    socket: WebSocketStream<TcpStream>,
}

impl WebSocketConnection {
    pub fn new(id: String, socket: WebSocketStream<TcpStream>) -> Self {
        Self { id, socket }
    }

    pub async fn send(&mut self, data: serde_json::Value) -> Result<()> {
        let message = Message::Text(data.to_string());
        self.socket.send(message).await.map_err(|e| anyhow!("WebSocket send error: {}", e))?;
        Ok(())
    }
}

/// Handler for WebSocket connections
pub struct WebSocketHandler {
    connections: Arc<RwLock<HashMap<String, WebSocketConnection>>>,
    heartbeat: Duration,
}

impl WebSocketHandler {
    pub fn new(heartbeat: Duration) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            heartbeat,
        }
    }

    pub async fn handle_connection(&self, socket: WebSocketStream<TcpStream>, id: String) {
        debug!("New WebSocket connection: {}", id);
        
        // Store connection
        {
            let conn = WebSocketConnection::new(id.clone(), socket);
            self.connections.write().await.insert(id.clone(), conn);
        }
        
        // Start a background task to handle the connection
        let connections = self.connections.clone();
        tokio::spawn(async move {
            loop {
                let conn_guard = connections.read().await;
                if let Some(_conn) = conn_guard.get(&id) {
                    // Handle messages
                    // This is a simplified example - in a real system you would
                    // have proper message handling here
                    tokio::time::sleep(Duration::from_secs(1)).await;
                } else {
                    break;
                }
            }
        });
    }

    async fn handle_message(&self, id: &str, text: String) -> Result<()> {
        debug!("Received WebSocket message from {}: {}", id, text);
        
        // Parse the message
        let message: serde_json::Value = serde_json::from_str(&text)?;
        
        // Get the message type
        let message_type = message.get("type").and_then(|v| v.as_str());
        
        match message_type {
            Some("ping") => {
                // Send a pong response
                if let Some(mut conn) = self.connections.write().await.get_mut(id) {
                    conn.send(serde_json::json!({ "type": "pong" })).await?;
                }
            },
            Some("action") => {
                // Handle an action call
                let action_id = message.get("id").and_then(|v| v.as_str());
                let action = message.get("action").and_then(|v| v.as_str());
                let params = message.get("params");
                
                if let (Some(action_id), Some(_action), Some(params)) = (action_id, action, params) {
                    // Here you would dispatch the action to the appropriate service
                    // For now, just echo back the parameters
                    if let Some(mut conn) = self.connections.write().await.get_mut(id) {
                        conn.send(serde_json::json!({
                            "id": action_id,
                            "success": true,
                            "data": params
                        })).await?;
                    }
                } else {
                    // Invalid action request
                    if let Some(mut conn) = self.connections.write().await.get_mut(id) {
                        conn.send(serde_json::json!({
                            "id": action_id.unwrap_or("unknown"),
                            "success": false,
                            "error": {
                                "message": "Invalid action request",
                                "code": 400
                            }
                        })).await?;
                    }
                }
            },
            _ => {
                warn!("Unknown WebSocket message type: {:?}", message_type);
            }
        }
        
        Ok(())
    }
}

/// Start the gateway service
pub async fn start_gateway<G: Gateway + Send + Sync + 'static>(gateway: G, config: GatewayConfig) -> Result<()> {
    let routes = build_routes()?;
    let middlewares = build_middleware(&config)?;
    
    // Create shared state
    let state = Arc::new(GatewayState {
        routes: Arc::new(routes),
        middlewares: Arc::new(middlewares),
        config: config.clone(),
        gateway: Some(Arc::new(gateway)),
    });
    
    // Create WebSocket handler
    let ws_handler = Arc::new(WebSocketHandler::new(Duration::from_secs(30)));
    
    // Create address
    let addr = format!("{}:{}", config.host, config.port)
        .parse::<SocketAddr>()
        .map_err(|e| anyhow!("Invalid address: {}", e))?;
    
    info!("Starting gateway server on {}", addr);
    
    // Use tower Service-based approach
    let service = tower::service_fn(move |req: Request<Body>| {
        let state = state.clone();
        let ws_handler = ws_handler.clone();
        
        async move {
            if is_websocket_request(&req) {
                handle_websocket_request(req, ws_handler).await
            } else {
                handle_http_request(req, state).await
            }
        }
    });
    
    // Create the server with the tower service
    let server = hyper::Server::bind(&addr)
        .serve(tower::make::Shared::new(service));
    
    // Run the server
    server.await.map_err(|e| anyhow!("Server error: {}", e))?;
    
    Ok(())
}

/// Gateway state shared across HTTP handlers
struct GatewayState {
    routes: Arc<HashMap<(String, String), RouteHandler>>,
    middlewares: Arc<Vec<Box<dyn Middleware>>>,
    config: GatewayConfig,
    gateway: Option<Arc<dyn Gateway + Send + Sync>>,
}

/// Type alias for route handlers
type RouteHandler = Box<dyn Fn(&Request<Body>) -> Result<Response<Body>> + Send + Sync>;

/// Build routes from the ROUTES static vector
fn build_routes() -> Result<HashMap<(String, String), RouteHandler>> {
    let mut routes = HashMap::new();
    
    // Access the static vector safely
    let route_infos = unsafe { &ROUTES };
    
    for route_info in route_infos.iter() {
        let method = route_info.method.to_string();
        let path = route_info.path.to_string();
        
        // Create a handler function for this route
        let handler: RouteHandler = Box::new(move |_req: &Request<Body>| {
            // This is a placeholder - in a real implementation, you would:
            // 1. Extract parameters from the path
            // 2. Parse the request body
            // 3. Forward to the appropriate service/handler
            // 4. Return the response
            
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!("{{\"route\":\"{}\"}}", route_info.path)))
                .unwrap())
        });
        
        routes.insert((method, path), handler);
    }
    
    Ok(routes)
}

/// Build middleware chain from configuration
fn build_middleware(config: &GatewayConfig) -> Result<Vec<Box<dyn Middleware>>> {
    let mut middlewares = Vec::new();
    
    // Add CORS middleware
    let cors = CorsMiddleware::new(config.cors.clone());
    middlewares.push(Box::new(cors) as Box<dyn Middleware>);
    
    // Add other middleware here based on config.middleware
    
    Ok(middlewares)
}

/// Check if a request is a WebSocket upgrade request
fn is_websocket_request(req: &Request<Body>) -> bool {
    req.headers().contains_key(header::UPGRADE) &&
    req.headers().get(header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_lowercase().contains("websocket"))
        .unwrap_or(false)
}

/// Handle WebSocket connection
async fn handle_websocket_request(
    req: Request<Body>,
    ws_handler: Arc<WebSocketHandler>,
) -> Result<Response<Body>, Infallible> {
    let uri = req.uri();
    
    // Generate a unique ID for this connection
    let id = if let Some(query) = uri.query() {
        // Extract ID from query params if present
        query.to_string()
    } else {
        // Generate a random ID
        format!("ws-{}", uuid::Uuid::new_v4())
    };
    
    // Get remote address for logging
    if let Some(addr) = req.extensions().get::<SocketAddr>() {
        debug!("WebSocket connection from {}: {}", addr, id);
    }
    
    // For simplicity, we'll just return a response for now
    // WebSocket implementation would require more complex handling
    let response = Response::builder()
        .status(StatusCode::NOT_IMPLEMENTED)
        .body(Body::from("WebSocket support not fully implemented"))
        .unwrap();
        
    Ok(response)
}

/// Handle HTTP request
async fn handle_http_request(
    req: Request<Body>,
    state: Arc<GatewayState>,
) -> Result<Response<Body>, Infallible> {
    // Create a response for when things go wrong
    let error_response = |status: StatusCode, message: &str| {
        let body = format!("{{\"error\":\"{}\"}}", message);
        Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap()
    };
    
    // Check if we have a route for this request
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    
    let response = match state.routes.get(&(method.clone(), path.clone())) {
        Some(handler) => {
            // Apply middleware chain
            let next = Next::new(&state.middlewares, handler);
            
            match next.run(&req).await {
                Ok(response) => response,
                Err(e) => {
                    error!("Error processing request: {}", e);
                    error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
                }
            }
        },
        None => {
            // If the route wasn't found, check if we have a pattern match
            // This would be for routes with parameters like /users/:id
            // For simplicity, we're not implementing this here
            
            warn!("Route not found: {} {}", method, path);
            error_response(StatusCode::NOT_FOUND, "Route not found")
        }
    };
    
    Ok(response)
}

/// Forward a request to a gateway service via the Gateway trait
async fn forward_to_gateway<G: Gateway + Send + Sync + 'static>(
    gateway: &G,
    req_path: String,
    body_params: Option<serde_json::Value>,
) -> Result<Response<Body>> {
    // Get the handler for the path
    match gateway.handle_request(req_path, body_params).await {
        Ok(json_response) => {
            // Convert to HTTP response
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json_response.to_string()))
                .unwrap())
        },
        Err(e) => {
            // Return error response
            error!("Gateway error: {}", e);
            let error_body = format!("{{\"error\":\"{}\"}}", e);
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(error_body))
                .unwrap())
        }
    }
}

/// CORS middleware implementation
struct CorsMiddleware {
    config: CorsConfig,
}

impl CorsMiddleware {
    fn new(config: CorsConfig) -> Self {
        Self { config }
    }
    
    fn is_origin_allowed(&self, origin: &str) -> bool {
        // Check if origin matches any allowed origins
        self.config.allowed_origins.iter()
            .any(|allowed| {
                if allowed == "*" {
                    return true;
                }
                
                // Simple exact match
                if allowed == origin {
                    return true;
                }
                
                // Wildcard match (e.g., https://*.example.com)
                if allowed.starts_with("*.") {
                    let domain = allowed.trim_start_matches("*.");
                    origin.ends_with(domain)
                } else {
                    false
                }
            })
    }
}

#[async_trait]
impl Middleware for CorsMiddleware {
    async fn process(&self, req: &Request<Body>, next: Next<'_>) -> Result<Response<Body>> {
        // Create a clone of the request that can be moved across threads
        let origin = req.headers()
            .get(header::ORIGIN)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        
        // For preflight requests (OPTIONS)
        if req.method() == Method::OPTIONS {
            let mut response = Response::builder()
                .status(StatusCode::OK);
            
            // Add CORS headers if origin is allowed
            if !origin.is_empty() && self.is_origin_allowed(origin) {
                response = response
                    .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin)
                    .header(header::ACCESS_CONTROL_ALLOW_METHODS, "GET, POST, PUT, DELETE, OPTIONS")
                    .header(header::ACCESS_CONTROL_ALLOW_HEADERS, "Content-Type, Authorization")
                    .header(header::ACCESS_CONTROL_MAX_AGE, "86400"); // 24 hours
                
                if self.config.allow_credentials {
                    response = response.header(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true");
                }
            }
            
            return Ok(response.body(Body::empty())?);
        }
        
        // For regular requests
        let mut response = next.run(req).await?;
        
        // Add CORS headers to the response if origin is allowed
        if !origin.is_empty() && self.is_origin_allowed(origin) {
            let headers = response.headers_mut();
            headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, header::HeaderValue::from_str(origin)?);
            
            if self.config.allow_credentials {
                headers.insert(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, header::HeaderValue::from_static("true"));
            }
        }
        
        Ok(response)
    }
}

/// Gateway request handler trait
#[async_trait]
pub trait GatewayRequestHandler {
    async fn handle_request(&self, path: String, params: Option<serde_json::Value>) -> Result<serde_json::Value>;
}

#[async_trait]
impl<G: Gateway + Send + Sync + 'static> GatewayRequestHandler for G {
    async fn handle_request(&self, path: String, params: Option<serde_json::Value>) -> Result<serde_json::Value> {
        // Convert path to method and endpoint
        let parts: Vec<&str> = path.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid path format, expected 'METHOD:PATH'"));
        }
        
        let method = parts[0].to_string();
        let endpoint = parts[1].to_string();
        
        // In a real implementation, you would:
        // 1. Parse the endpoint to extract path parameters
        // 2. Call the appropriate service method
        // 3. Return the response
        
        // Simple echo implementation for testing
        let mut result = serde_json::json!({
            "method": method,
            "endpoint": endpoint
        });
        
        if let Some(params) = params {
            result["params"] = params;
        }
        
        Ok(result)
    }
}

// Re-export the service module
pub mod service; 