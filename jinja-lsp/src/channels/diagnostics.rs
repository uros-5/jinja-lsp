use std::collections::HashMap;

use jinja_lsp_queries::{
    lsp_helper::create_diagnostic, search::Identifier, tree_builder::JinjaDiagnostic,
};
use tokio::sync::mpsc::{Receiver, Sender};
use tower_lsp::{
    lsp_types::{MessageType, Url},
    Client,
};

use super::lsp::LspMessage;

pub fn diagnostics_task(
    client: Client,
    mut receiver: Receiver<DiagnosticMessage>,
    lsp_channel: Sender<LspMessage>,
) {
    tokio::spawn(async move {
        while let Some(msg) = receiver.recv().await {
            match msg {
                DiagnosticMessage::Str(msg) => client.log_message(MessageType::INFO, msg).await,
                DiagnosticMessage::Errors(all_errors) => {
                    let mut code_actions = HashMap::new();
                    for (uri, errors) in all_errors.into_iter() {
                        let template_errors = errors
                            .iter()
                            .filter(|err| err.0 == JinjaDiagnostic::TemplateNotFound);
                        let mut v = vec![];
                        for err in template_errors {
                            v.push(err.1.to_owned());
                        }
                        code_actions.insert(uri.to_owned(), v);
                        let mut v = vec![];
                        for error in errors {
                            let diagnostic = create_diagnostic(
                                &error.1,
                                error.0.severity(),
                                error.0.to_string(),
                            );
                            v.push(diagnostic);
                        }
                        let uri = Url::parse(&uri).unwrap();
                        client.publish_diagnostics(uri, v, None).await;
                    }
                    let _ = lsp_channel
                        .send(LspMessage::CodeActions(code_actions))
                        .await;
                }
            }
        }
    });
}

#[derive(Debug)]
pub enum DiagnosticMessage {
    Errors(HashMap<String, Vec<(JinjaDiagnostic, Identifier)>>),
    Str(String),
}
