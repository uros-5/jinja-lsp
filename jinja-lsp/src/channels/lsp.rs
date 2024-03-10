use std::collections::HashMap;

use jinja_lsp_queries::{
    parsers::Parsers,
    queries::Queries,
    tree_builder::{JinjaVariable, LangType},
};
use ropey::Rope;
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};
use tower_lsp::{
    lsp_types::{
        CodeActionParams, CodeActionProviderCapability, CodeActionResponse, CompletionOptions,
        CompletionParams, CompletionResponse, DidChangeTextDocumentParams,
        DidOpenTextDocumentParams, DidSaveTextDocumentParams, ExecuteCommandOptions,
        ExecuteCommandParams, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverParams,
        HoverProviderCapability, InitializeParams, InitializeResult, MessageType, OneOf,
        ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
        TextDocumentSyncOptions, TextDocumentSyncSaveOptions,
    },
    Client,
};
use tree_sitter::Tree;

use crate::{config::JinjaConfig, lsp_files::LspFiles};

pub async fn lsp_listen(
    lint_channel: mpsc::Sender<String>,
    lsp_channel: mpsc::Sender<LspMessage>,
    mut lsp_recv: mpsc::Receiver<LspMessage>,
    client: Client,
) {
    // let mut documents = HashMap::new();
    let mut can_complete = false;
    let mut config = JinjaConfig::default();
    let mut lsp_data = LspFiles::default();
    tokio::spawn(async move {
        while let Some(msg) = lsp_recv.recv().await {
            match msg {
                LspMessage::Initialize(params, sender) => {
                    if let Some(client_info) = params.client_info {
                        if client_info.name == "helix" {
                            can_complete = true;
                        }
                    }
                    params
                        .initialization_options
                        .map(serde_json::from_value)
                        .map(|res| res.ok())
                        .and_then(|c| -> Option<()> {
                            config = c?;
                            config.user_defined = true;
                            None
                        });

                    if !config.user_defined {
                        drop(sender);
                        continue;
                    }

                    let definition_provider = Some(OneOf::Left(true));
                    let references_provider = Some(OneOf::Left(true));
                    let code_action_provider = Some(CodeActionProviderCapability::Simple(true));
                    let hover_provider = Some(HoverProviderCapability::Simple(true));
                    let execute_command_provider = Some(ExecuteCommandOptions {
                        commands: vec!["reset_variables".to_string(), "warn".to_string()],
                        ..Default::default()
                    });

                    let msg = InitializeResult {
                        capabilities: ServerCapabilities {
                            text_document_sync: Some(TextDocumentSyncCapability::Options(
                                TextDocumentSyncOptions {
                                    change: Some(TextDocumentSyncKind::INCREMENTAL),
                                    will_save: Some(true),
                                    save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                                    ..Default::default()
                                },
                            )),
                            completion_provider: Some(CompletionOptions {
                                resolve_provider: Some(false),
                                trigger_characters: Some(vec![
                                    "-".to_string(),
                                    "\"".to_string(),
                                    " ".to_string(),
                                ]),
                                all_commit_characters: None,
                                work_done_progress_options: Default::default(),
                                completion_item: None,
                            }),
                            definition_provider,
                            references_provider,
                            code_action_provider,
                            execute_command_provider,
                            hover_provider,
                            ..ServerCapabilities::default()
                        },
                        server_info: Some(ServerInfo {
                            name: String::from("jinja-lsp"),
                            version: Some(String::from("0.1.5")),
                        }),
                        offset_encoding: None,
                    };
                    let _ = sender.send(msg);
                }
                LspMessage::Initialized => {
                    client.log_message(MessageType::INFO, "Initialized").await;
                    if !config.user_defined {
                        client
                            .log_message(MessageType::INFO, "Config doesn't exist.")
                            .await;
                    }
                    if config.templates.is_empty() {
                        client
                            .log_message(MessageType::INFO, "Template directory not found")
                            .await;
                    }
                    if config.lang == "rust" {
                        client
                            .log_message(MessageType::INFO, "Backend language not supported")
                            .await;
                    } else {
                        // walkdir(&config, lsp_files, document_map)
                    }
                }
                LspMessage::DidChange(_) => todo!(),
                LspMessage::DidSave(_) => todo!(),
                LspMessage::Completion(_, _) => todo!(),
                LspMessage::Hover(_, _) => todo!(),
                LspMessage::GoToDefinition(_, _) => todo!(),
                LspMessage::CodeAction(_, _) => todo!(),
                LspMessage::ExecuteCommand(_, _) => todo!(),
                LspMessage::DidOpen(_) => todo!(),
            }
        }
    });
}

pub enum LspMessage {
    Initialize(Box<InitializeParams>, oneshot::Sender<InitializeResult>),
    Initialized,
    DidOpen(DidOpenTextDocumentParams),
    DidChange(DidChangeTextDocumentParams),
    DidSave(DidSaveTextDocumentParams),
    Completion(
        CompletionParams,
        oneshot::Sender<Option<CompletionResponse>>,
    ),
    Hover(HoverParams, oneshot::Sender<Option<Hover>>),
    GoToDefinition(
        GotoDefinitionParams,
        oneshot::Sender<Option<GotoDefinitionResponse>>,
    ),
    CodeAction(
        CodeActionParams,
        oneshot::Sender<Option<CodeActionResponse>>,
    ),
    ExecuteCommand(ExecuteCommandParams, oneshot::Sender<Option<Value>>),
}
