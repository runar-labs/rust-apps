use anyhow::Result;
use kagi_macros::main;
use kagi_node::node::{Node, NodeConfig};
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

    // Register services
    node.register_service(AuthService::new()).await?;
    node.register_service(ProfileService::new()).await?;
    node.register_service(InvoiceService::new()).await?;

    // Start the node
    node.start().await?;

    // Keep the application running
    tokio::signal::ctrl_c().await?;
    println!("Shutting down...");

    Ok(())
} 