use anyhow::Result;
use codex_mcp_rs::server::CodexServer;
use rmcp::{transport::stdio, ServiceExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Create an instance of our codex server
    let service = CodexServer::new().serve(stdio()).await.inspect_err(|e| {
        eprintln!("serving error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}
