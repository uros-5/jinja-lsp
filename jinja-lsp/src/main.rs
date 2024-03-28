mod backend;
pub mod channels;
mod config;
mod filter;
pub mod lsp_files2;

use backend::Backend;
use tower_lsp::LspService;
use tower_lsp::Server;

#[tokio::main]
async fn main() {
    env_logger::init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(Backend::new).finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}
