use std::collections::HashMap;

use jinja_lsp_queries::tree_builder::{JinjaDiagnostic, JinjaVariable};
use tokio::sync::mpsc::Receiver;
use tower_lsp::{
    lsp_types::{Diagnostic, DiagnosticSeverity, MessageType, Position, Range, Url},
    Client,
};

pub fn diagnostics_task(client: Client, mut receiver: Receiver<DiagnosticMessage>) {
    tokio::spawn(async move {
        while let Some(msg) = receiver.recv().await {
            match msg {
                DiagnosticMessage::Errors {
                    diagnostics,
                    current_file,
                } => {
                    let mut hm: HashMap<String, Vec<Diagnostic>> = HashMap::new();
                    let mut added = false;
                    for (file, diags) in diagnostics {
                        for (variable, diag2) in diags {
                            let severity = {
                                match diag2 {
                                    JinjaDiagnostic::DefinedSomewhere => {
                                        DiagnosticSeverity::INFORMATION
                                    }
                                    JinjaDiagnostic::Undefined => DiagnosticSeverity::WARNING,
                                    JinjaDiagnostic::TemplateNotFound => {
                                        DiagnosticSeverity::WARNING
                                    }
                                }
                            };
                            added = true;

                            let diagnostic = Diagnostic {
                                range: Range::new(
                                    Position::new(
                                        variable.location.0.row as u32,
                                        variable.location.0.column as u32,
                                    ),
                                    Position::new(
                                        variable.location.1.row as u32,
                                        variable.location.1.column as u32,
                                    ),
                                ),
                                severity: Some(severity),
                                message: diag2.to_string(),
                                source: Some(String::from("jinja-lsp")),
                                ..Default::default()
                            };

                            if hm.contains_key(&file) {
                                let _ = hm.get_mut(&file).is_some_and(|d| {
                                    d.push(diagnostic);
                                    false
                                });
                            } else {
                                hm.insert(String::from(&file), vec![diagnostic]);
                            }
                        }
                    }

                    for (url, diagnostics) in hm {
                        if let Ok(uri) = Url::parse(&url) {
                            client.publish_diagnostics(uri, diagnostics, None).await;
                        }
                    }
                    if let Some(uri) = current_file {
                        if !added {
                            let uri = Url::parse(&uri).unwrap();
                            client.publish_diagnostics(uri, vec![], None).await;
                        }
                    }
                }
                DiagnosticMessage::Str(msg) => client.log_message(MessageType::INFO, msg).await,
            }
        }
    });
}

pub enum DiagnosticMessage {
    Errors {
        diagnostics: HashMap<String, Vec<(JinjaVariable, JinjaDiagnostic)>>,
        current_file: Option<String>,
    },
    Str(String),
}
