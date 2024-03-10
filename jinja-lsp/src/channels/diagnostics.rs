use tokio::sync::mpsc::Receiver;
use tower_lsp::Client;

pub fn diagnostics_task(client: Client, mut receiver: Receiver<String>) {
    tokio::spawn(async move {
        while let Some(msg) = receiver.recv().await {
            //
        }
    });
}
