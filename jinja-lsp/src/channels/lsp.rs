use jinja_lsp_queries::search::objects::CompletionType;
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};
use tower_lsp::{
    lsp_types::{
        CodeActionParams, CodeActionProviderCapability, CodeActionResponse, CompletionItem,
        CompletionItemKind, CompletionOptions, CompletionParams, CompletionResponse,
        DidChangeTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
        Documentation, ExecuteCommandOptions, ExecuteCommandParams, GotoDefinitionParams,
        GotoDefinitionResponse, Hover, HoverContents, HoverParams, HoverProviderCapability,
        InitializeParams, InitializeResult, MarkupContent, MarkupKind, MessageType, OneOf,
        ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
        TextDocumentSyncOptions, TextDocumentSyncSaveOptions,
    },
    Client,
};

use crate::{
    backend::code_actions,
    config::{walkdir, JinjaConfig},
    filter::init_filter_completions,
    lsp_files2::LspFiles2,
};

use super::diagnostics::DiagnosticMessage;

pub fn lsp_task(
    client: Client,
    diagnostics_channel: mpsc::Sender<DiagnosticMessage>,
    lsp_channel: mpsc::Sender<LspMessage>,
    mut lsp_recv: mpsc::Receiver<LspMessage>,
) {
    // let mut documents = HashMap::new();
    let mut can_complete = false;
    let mut config = JinjaConfig::default();
    let mut lsp_data = LspFiles2::default();
    let filters = init_filter_completions();
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
                    let references_provider = None;
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
                            version: Some(String::from("0.1.61")),
                        }),
                        offset_encoding: None,
                    };
                    let _ = sender.send(msg);
                }
                LspMessage::Initialized(sender) => {
                    client.log_message(MessageType::INFO, "Initialized").await;
                    if !config.user_defined {
                        client
                            .log_message(MessageType::INFO, "Config doesn't exist.")
                            .await;
                        break;
                    }
                    if config.templates.is_empty() {
                        client
                            .log_message(MessageType::INFO, "Template directory not found")
                            .await;
                        break;
                    }
                    if config.lang != "rust" {
                        client
                            .log_message(MessageType::INFO, "Backend language not supported")
                            .await;
                        break;
                    } else {
                        match walkdir(&config) {
                            Ok(errors) => {
                                let _ = diagnostics_channel
                                    .send(DiagnosticMessage::Errors2(errors.0))
                                    .await;
                                lsp_data = errors.1;
                                lsp_data.config = config.clone();
                                lsp_data.main_channel = Some(lsp_channel.clone());
                                let _ = sender.send(true);
                            }
                            Err(err) => {
                                let msg = err.to_string();
                                client.log_message(MessageType::INFO, msg).await;
                                let _ = sender.send(false);
                                break;
                            }
                        }
                    }
                }
                LspMessage::DidChange(params) => {
                    lsp_data.did_change(params);
                }
                LspMessage::DidSave(params) => {
                    if let Some(errors) = lsp_data.did_save(params) {
                        let _ = diagnostics_channel.send(errors).await;
                    }
                }
                LspMessage::Completion(params, sender) => {
                    let position = params.text_document_position.position;
                    let uri = params.text_document_position.text_document.uri.clone();
                    let completion = lsp_data.completion(params, can_complete);
                    let mut items = None;

                    if let Some(completion) = completion {
                        match completion {
                            CompletionType::Filter => {
                                let completions = filters.clone();
                                let mut ret = Vec::with_capacity(completions.len());
                                for item in completions.into_iter() {
                                    ret.push(CompletionItem {
                                        label: item.name,
                                        kind: Some(CompletionItemKind::TEXT),
                                        documentation: Some(Documentation::MarkupContent(
                                            MarkupContent {
                                                kind: MarkupKind::Markdown,
                                                value: item.desc.to_string(),
                                            },
                                        )),
                                        ..Default::default()
                                    });
                                }
                                items = Some(CompletionResponse::Array(ret));
                            }
                            CompletionType::Identifier => {
                                if let Some(variables) = lsp_data.read_variables(&uri, position) {
                                    items = Some(CompletionResponse::Array(variables));
                                }
                            }
                            CompletionType::IncludedTemplate { name, range } => {
                                if let Some(templates) = lsp_data.read_templates(name, range) {
                                    items = Some(CompletionResponse::Array(templates));
                                }
                            }
                        };
                    }
                    let _ = sender.send(items);
                }
                LspMessage::Hover(params, sender) => {
                    let mut res = None;
                    if let Some(hover) = lsp_data.hover(params) {
                        let filter = filters
                            .iter()
                            .find(|name| name.name == hover.0.name && hover.1);
                        if let Some(filter) = filter {
                            let markup_content = MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: filter.desc.to_string(),
                            };
                            let hover_contents = HoverContents::Markup(markup_content);
                            let hover = Hover {
                                contents: hover_contents,
                                range: None,
                            };
                            res = Some(hover);
                        }
                    }
                    let _ = sender.send(res);
                }
                LspMessage::GoToDefinition(params, sender) => {
                    if let Some(definition) = lsp_data.goto_definition(params) {
                        let _ = sender.send(Some(definition));
                    }
                }
                LspMessage::CodeAction(params, sender) => {
                    if let Some(is_code_action) = lsp_data.code_action(params) {
                        if is_code_action {
                            let _ = sender.send(Some(code_actions()));
                        }
                    }
                }
                LspMessage::ExecuteCommand(params, sender) => {
                    let command = params.command;
                    if command == "reset_variables" {
                        let (sender2, _) = oneshot::channel();
                        let _ = lsp_channel.send(LspMessage::Initialized(sender2)).await;
                        let _ = sender.send(None);
                    }
                }
                LspMessage::DidOpen(params) => {
                    if let Some(errors) = lsp_data.did_open(params) {
                        let _ = diagnostics_channel.send(errors).await;
                    }
                }
            }
        }
    });
}

pub enum LspMessage {
    Initialize(Box<InitializeParams>, oneshot::Sender<InitializeResult>),
    Initialized(oneshot::Sender<bool>),
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
