use anyhow::Result;
use runar_macros::main;
use runar_node::node::{Node, NodeConfig};
use common::services::auth::AuthService;
use common::services::profile::ProfileService;
use crate::services::invoice::InvoiceService;

mod services;

#[main]
async fn main() -> Result<()> {
    // Initialize node configuration
    let config = NodeConfig::default()
        .with_name("invoice-demo")
        .with_description("Invoice Demo Application")
        .with_port(3000);

    // Create and initialize node
    let mut node = Node::new(config);

    // Register services using the proper add_service method
    node.add_service(AuthService::new()).await?;
    node.add_service(ProfileService::new()).await?;
    node.add_service(InvoiceService::new()).await?;

    // Start the node
    node.start().await?;

    // Keep the application running
    tokio::signal::ctrl_c().await?;
    println!("Shutting down...");

    Ok(())
} 