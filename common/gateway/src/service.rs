use crate::{Gateway, GatewayConfig, RouteInfo, ROUTES};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use hyper::{server::conn::AddrStream, service::make_service_fn, service::service_fn, Request, Response, Server, Body, StatusCode, Method, header};
use kagi_node::services::{AbstractService, ServiceState, ServiceMetadata, RequestContext, ServiceRequest, ServiceResponse, ValueType};
use log::{debug, error, info};
use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::fmt::Debug;
use serde_json::Value;

/// A route entry in the registry
#[derive(Debug, Clone)]
pub struct RouteEntry {
    /// HTTP method
    pub method: String,
    /// Path pattern (e.g., "/users/:id")
    pub path_pattern: String,
    /// Service name to forward to
    pub service_name: String,
    /// Action name to call
    pub action_name: String,
    /// Path segments for efficient matching
    pub path_segments: Vec<String>,
    /// Whether each segment is a parameter
    pub is_parameter: Vec<bool>,
    /// Optional middleware to apply
    pub middleware: Option<Vec<String>>,
}

/// Gateway service that acts as a bridge between HTTP/WebSockets and internal services
pub struct GatewayService {
    /// Service name
    pub name: String,
    /// Service path in the registry
    pub path: String,
    /// Service description
    pub description: String,
    /// Service configuration
    pub config: GatewayConfig,
    /// Current service state
    pub state: ServiceState,
    /// Service context for request handling
    pub context: Option<Arc<RequestContext>>,
    /// Service running status
    pub running: bool,
    /// Route registry
    pub routes: Arc<Mutex<Vec<RouteEntry>>>,
    /// Service operations
    pub operations: Vec<String>,
    /// Service version
    pub version: String,
}

impl GatewayService {
    pub fn new(name: String, config: GatewayConfig) -> Self {
        Self {
            name: name.clone(),
            path: format!("gateway.{}", name),
            description: format!("HTTP/WebSocket Gateway Service: {}", name),
            config,
            state: ServiceState::Stopped,
            context: None,
            running: false,
            routes: Arc::new(Mutex::new(Vec::new())),
            operations: vec!["process".to_string()],
            version: "1.0.0".to_string(),
        }
    }

    /// Initialize routes from the registry
    pub async fn initialize_routes(&self) -> Result<()> {
        let mut routes = Vec::new();
        
        // Get the route information from the static registry
        unsafe {
            for route_info in ROUTES.iter() {
                let segments: Vec<String> = route_info.path
                    .split('/')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();
                
                let is_param: Vec<bool> = segments
                    .iter()
                    .map(|s| s.starts_with(':'))
                    .collect();
                
                // Parse the handler name to extract service and action
                let handler_parts: Vec<&str> = route_info.handler_name.split('.').collect();
                if handler_parts.len() != 2 {
                    error!("Invalid handler name format: {}", route_info.handler_name);
                    continue;
                }
                
                let entry = RouteEntry {
                    method: route_info.method.to_string(),
                    path_pattern: route_info.path.to_string(),
                    service_name: handler_parts[0].to_string(),
                    action_name: handler_parts[1].to_string(),
                    path_segments: segments,
                    is_parameter: is_param,
                    middleware: route_info.middleware.clone(),
                };
                
                let service_name = entry.service_name.clone();
                let action_name = entry.action_name.clone();
                
                routes.push(entry);
                info!("Registered route: {} {} -> {}.{}", 
                      route_info.method, route_info.path, 
                      service_name, action_name);
            }
        }
        
        // Update the routes registry
        let mut routes_lock = self.routes.lock().await;
        *routes_lock = routes;
        
        Ok(())
    }
    
    /// Extract parameters from a path based on the route entry
    pub fn extract_parameters(&self, route: &RouteEntry, path: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        
        for (i, segment) in path_segments.iter().enumerate() {
            if i < route.is_parameter.len() && route.is_parameter[i] {
                // This is a parameter segment, extract the parameter name from the pattern
                let param_name = route.path_segments[i].trim_start_matches(':');
                params.insert(param_name.to_string(), segment.to_string());
            }
        }
        
        params
    }
    
    /// Find a matching route for a request
    pub async fn find_route(&self, method: &str, path: &str) -> Option<(RouteEntry, HashMap<String, String>)> {
        let routes = self.routes.lock().await;
        let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        
        for route in routes.iter() {
            // First check method
            if route.method != method && route.method != "*" {
                continue;
            }
            
            // Then check path segment count
            if route.path_segments.len() != path_segments.len() {
                continue;
            }
            
            // Check each segment
            let mut matches = true;
            for (i, segment) in path_segments.iter().enumerate() {
                if !route.is_parameter[i] && route.path_segments[i] != *segment {
                    matches = false;
                    break;
                }
            }
            
            if matches {
                let params = self.extract_parameters(route, path);
                return Some((route.clone(), params));
            }
        }
        
        None
    }
    
    /// Handle an incoming service request
    async fn handle_service_request(&self, req: ServiceRequest) -> Result<ServiceResponse> {
        // Get the operation from the request path
        let parts: Vec<&str> = req.path.split('/').collect();
        if parts.is_empty() {
            return Ok(ServiceResponse::error("Invalid request path"));
        }
        
        let operation = parts[parts.len() - 1];
        
        match operation {
            "ping" => {
                Ok(ServiceResponse::success("pong".to_string(), Some("pong")))
            },
            "getRoutes" => {
                let routes = self.routes.lock().await;
                let route_data: Vec<Value> = routes.iter().map(|r| {
                    serde_json::json!({
                        "method": r.method,
                        "path": r.path_pattern,
                        "service": r.service_name,
                        "action": r.action_name
                    })
                }).collect();
                
                // Convert Vec<Value> to a ValueType
                let routes_json = serde_json::json!(route_data);
                Ok(ServiceResponse::success("Routes".to_string(), Some(routes_json)))
            },
            _ => {
                Err(anyhow!("Unsupported operation: {}", operation))
            }
        }
    }
}

#[async_trait]
impl Gateway for GatewayService {
    async fn run(&self) -> Result<()> {
        info!("Starting gateway service on {}:{}", self.config.host, self.config.port);
        
        // Initialize routes
        self.initialize_routes().await?;
        
        // Create the address to bind to
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let socket_addr = addr.parse::<SocketAddr>()?;
        
        // Create the service factory
        let routes = self.routes.clone();
        let _config = self.config.clone();
        
        let make_svc = make_service_fn(move |conn: &AddrStream| {
            let remote_addr = conn.remote_addr();
            let routes = routes.clone();
            
            async move {
                Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                    let routes = routes.clone();
                    handle_request(req, routes, remote_addr)
                }))
            }
        });
        
        // Create and start the server
        let server = Server::bind(&socket_addr).serve(make_svc);
        info!("Gateway service listening on {}", socket_addr);
        
        // Run the server
        if let Err(e) = server.await {
            error!("Gateway server error: {}", e);
            return Err(anyhow!("Server error: {}", e));
        }
        
        Ok(())
    }
}

#[async_trait]
impl AbstractService for GatewayService {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn path(&self) -> &str {
        &self.path
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    fn state(&self) -> ServiceState {
        self.state
    }
    
    fn metadata(&self) -> ServiceMetadata {
        ServiceMetadata {
            name: self.name.clone(),
            path: self.path.clone(),
            description: self.description.clone(),
            operations: self.operations.clone(),
            version: self.version.clone(),
            state: self.state,
        }
    }
    
    async fn init(&mut self, context: &RequestContext) -> Result<()> {
        info!("Initializing gateway service: {}", self.name);
        
        // Store context in Arc
        self.context = Some(Arc::new(context.clone()));
        
        // Initialize routes
        self.initialize_routes().await?;
        
        // Update state
        self.state = ServiceState::Initialized;
        
        Ok(())
    }
    
    async fn start(&mut self) -> Result<()> {
        info!("Starting gateway service: {}", self.name);
        
        // Update state
        self.state = ServiceState::Running;
        self.running = true;
        
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<()> {
        info!("Stopping gateway service: {}", self.name);
        
        // Update state
        self.state = ServiceState::Stopped;
        self.running = false;
        
        Ok(())
    }
    
    async fn handle_request(&self, request: ServiceRequest) -> Result<ServiceResponse> {
        debug!("Processing request: {:?}", request.path);
        
        // Handle the request
        self.handle_service_request(request).await
    }
}

// Handler for HTTP requests
async fn handle_request(
    req: Request<Body>,
    _routes: Arc<Mutex<Vec<RouteEntry>>>,
    _addr: SocketAddr
) -> Result<Response<Body>, Infallible> {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    
    debug!("Handling HTTP request: {} {}", method, path);
    
    // Simple 404 response for now
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not found"))
        .unwrap())
} 