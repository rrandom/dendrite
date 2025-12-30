//! Dendrite LSP Server Binary Entry Point

use dendrite_lsp::create_lsp_service;
use tower_lsp::Server;

#[tokio::main]
async fn main() {
    env_logger::init();

    eprintln!("🚀 Dendrite LSP Server starting...");
    eprintln!("📝 Listening on stdin/stdout...");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = create_lsp_service();
    eprintln!("✅ LSP service created, waiting for client connection...");
    eprintln!("✅ Engine Started");
    
    Server::new(stdin, stdout, socket).serve(service).await;
}

