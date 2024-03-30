use std::collections::HashMap;

use tokio::sync::mpsc::Receiver;
use tower_lsp::{
    lsp_types::{Diagnostic, MessageType, Url},
    Client,
};

pub fn diagnostics_task(client: Client, mut receiver: Receiver<DiagnosticMessage>) {
    tokio::spawn(async move {
        while let Some(msg) = receiver.recv().await {
            match msg {
                DiagnosticMessage::Str(msg) => client.log_message(MessageType::INFO, msg).await,
                DiagnosticMessage::Errors(all_errors) => {
                    for (uri, errors) in all_errors.into_iter() {
                        let uri = Url::parse(&uri).unwrap();
                        client.publish_diagnostics(uri, errors, None).await;
                    }
                }
            }
        }
    });
}

pub enum DiagnosticMessage {
    Errors(HashMap<String, Vec<Diagnostic>>),
    Str(String),
}
