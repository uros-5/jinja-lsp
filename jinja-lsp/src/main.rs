mod backend;
pub mod channels;
mod config;
mod filter;
pub mod lsp_files;
mod template_tests;

use backend::Backend;
use clap::Parser;
use config::JinjaLspArgs;
use tower_lsp::LspService;
use tower_lsp::Server;

#[tokio::main]
async fn main() {
    let _ = JinjaLspArgs::parse();

    env_logger::init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(Backend::_new).finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}
