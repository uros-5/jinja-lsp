use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex, RwLock},
};

use dashmap::{mapref::one::RefMut, DashMap};
use ropey::Rope;
use serde_json::Value;
use tokio::sync::mpsc;
use tower_lsp::{
    jsonrpc::Result,
    lsp_types::{InitializeParams, InitializeResult, MessageType},
    Client, LanguageServer,
};

use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams,
    CodeActionProviderCapability, CodeActionResponse, Command, CompletionContext, CompletionItem,
    CompletionItemKind, CompletionOptions, CompletionParams, CompletionResponse,
    CompletionTriggerKind, Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams,
    DidSaveTextDocumentParams, Documentation, ExecuteCommandOptions, ExecuteCommandParams,
    GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents, HoverParams,
    HoverProviderCapability, InitializedParams, MarkupContent, MarkupKind, OneOf, Position, Range,
    ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, TextDocumentSyncSaveOptions, Url,
};

use jinja_lsp_queries::{
    capturer::object::CompletionType,
    to_input_edit::ToInputEdit,
    tree_builder::{JinjaDiagnostic, JinjaVariable, LangType},
};

use crate::{
    channels::{diagnostics::diagnostics_task, lsp::lsp_task},
    config::{config_exist, read_config, walkdir, JinjaConfig},
    filter::{init_filter_completions, FilterCompletion},
    lsp_files::{FileWriter, LspFiles},
};

pub struct Backend {
    client: Client,
    document_map: DashMap<String, Rope>,
    can_complete: RwLock<bool>,
    config: RwLock<JinjaConfig>,
    filter_values: Vec<FilterCompletion>,
    pub lsp_files: Arc<Mutex<LspFiles>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let mut root = Url::parse("file://").unwrap();
        if let Some(folders) = params.workspace_folders {
            if let Some(dirs) = folders.first() {
                root = dirs.uri.to_owned();
            }
        } else if let Some(dir) = params.root_uri {
            root = dir.to_owned();
        }
        if let Ok(mut lsp_files) = self.lsp_files.lock() {
            lsp_files.root_path = root;
        }
        let mut definition_provider = None;
        let mut references_provider = None;
        let mut code_action_provider = None;
        let mut hover_provider = None;
        let mut execute_command_provider = None;

        if let Some(client_info) = params.client_info {
            if client_info.name == "helix" {
                if let Ok(mut can_complete) = self.can_complete.write() {
                    *can_complete = true;
                }
            }
        }

        match config_exist(params.initialization_options) {
            Some(config) => {
                self.config
                    .try_write()
                    .ok()
                    .and_then(|mut jinja_config| -> Option<()> {
                        definition_provider = Some(OneOf::Left(true));
                        references_provider = Some(OneOf::Left(true));
                        code_action_provider = Some(CodeActionProviderCapability::Simple(true));
                        hover_provider = Some(HoverProviderCapability::Simple(true));
                        execute_command_provider = Some(ExecuteCommandOptions {
                            commands: vec!["reset_variables".to_string(), "warn".to_string()],
                            ..Default::default()
                        });
                        *jinja_config = config;
                        None
                    });
            }
            None => {
                self.client
                    .log_message(MessageType::INFO, "Config not found")
                    .await;
                self.shutdown().await;
            }
        }

        Ok(InitializeResult {
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
                version: Some(String::from("0.1.3")),
            }),
            offset_encoding: None,
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Initialized")
            .await;

        match read_config(&self.config, &self.lsp_files, &self.document_map) {
            Ok(d) => {
                self.publish_tag_diagnostics(d, None).await;
            }
            Err(err) => {
                self.config
                    .write()
                    .ok()
                    .and_then(|mut config| config.user_defined(false));
                let msg = err.to_string();
                self.client.log_message(MessageType::INFO, msg).await;
            }
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let rope = self.document_map.get_mut(&uri);
        rope.and_then(|mut rope| self.on_change(&mut rope, params, uri));
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let mut diags = HashMap::new();
        if let Ok(lsp_files) = self.lsp_files.lock() {
            lsp_files.saved(&uri, &self.config, &self.document_map, &mut diags);
        }
        self.publish_tag_diagnostics(diags, Some(uri)).await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let can_complete = {
            matches!(
                params.context,
                Some(CompletionContext {
                    trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                    ..
                }) | Some(CompletionContext {
                    trigger_kind: CompletionTriggerKind::INVOKED,
                    ..
                })
            )
        };
        if !can_complete {
            let can_complete = self.can_complete.read().is_ok_and(|d| *d);
            if !can_complete {
                return Ok(None);
            }
        }
        let uri = &params.text_document_position.text_document.uri.clone();
        let position = params.text_document_position.position;
        let lang_type = self.get_lang_type(uri.as_str());

        let mut completion = None;
        let mut items = None;
        if let Some(lang_type) = lang_type {
            if lang_type == LangType::Template {
                completion = self.start_completion(params);
            }
        }
        if let Some(completion) = completion {
            match completion {
                CompletionType::Filter => {
                    let completions = self.filter_values.clone();
                    let mut ret = Vec::with_capacity(completions.len());
                    for item in completions.into_iter() {
                        ret.push(CompletionItem {
                            label: item.name,
                            kind: Some(CompletionItemKind::TEXT),
                            documentation: Some(Documentation::MarkupContent(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: item.desc.to_string(),
                            })),
                            ..Default::default()
                        });
                    }
                    items = Some(CompletionResponse::Array(ret));
                }
                CompletionType::Identifier => {
                    if let Some(variables) = self.lsp_files.lock().ok().and_then(|lsp_files| {
                        lsp_files.get_variables(
                            uri,
                            &self.document_map,
                            LangType::Template,
                            position,
                        )
                    }) {
                        items = Some(CompletionResponse::Array(variables));
                    }
                }
            }
        }
        Ok(items)
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let lang_type = self.get_lang_type(uri.as_str());
        let can_hover = lang_type.map_or(false, |lang_type| lang_type == LangType::Template);
        if !can_hover {
            return Ok(None);
        }
        let mut res = None;

        if let Some(filter) = self
            .start_hover(params)
            .and_then(|hover| self.filter_values.iter().find(|name| name.name == hover))
        {
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
        Ok(res)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let lang_type = self.get_lang_type(uri.as_str());
        let can_def = lang_type.map_or(false, |lang_type| lang_type == LangType::Template);
        if !can_def {
            return Ok(None);
        }
        let res = self.start_goto_definition(params);
        Ok(res)
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = &params.text_document.uri.clone();
        let lang_type = self.get_lang_type(uri.as_str());
        let can_def = lang_type.map_or(false, |lang_type| lang_type == LangType::Template);
        if !can_def {
            return Ok(None);
        }
        let mut res = None;
        let code_action = self.start_code_action(params);
        if let Some(code_action) = code_action {
            if code_action {
                res = Some(code_actions());
            }
        }
        Ok(res)
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        Ok(self.start_command(params).await)
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

pub fn code_actions() -> Vec<CodeActionOrCommand> {
    let mut commands = vec![];
    for command in [
        ("Reset variables", "reset_variables"),
        ("Warn about unused", "warn"),
    ] {
        commands.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: command.0.to_string(),
            kind: Some(CodeActionKind::EMPTY),
            command: Some(Command::new(
                command.1.to_string(),
                command.1.to_string(),
                None,
            )),
            ..Default::default()
        }));
    }
    commands
}
impl Backend {
    pub fn new(client: Client) -> Self {
        let (lsp_sender, lsp_recv) = mpsc::channel(20);
        let (diagnostic_sender, diagnostic_recv) = mpsc::channel(20);
        lsp_task(client.clone(), diagnostic_sender, lsp_sender, lsp_recv);
        diagnostics_task(client.clone(), diagnostic_recv);
        let document_map = DashMap::new();
        let can_complete = RwLock::new(false);
        let config = RwLock::new(JinjaConfig::default());
        let filter_values = init_filter_completions();
        let lsp_files = Arc::new(Mutex::new(LspFiles::default()));
        Self {
            client,
            document_map,
            can_complete,
            config,
            filter_values,
            lsp_files,
        }
    }

    fn on_change(
        &self,
        rope: &mut RefMut<'_, String, Rope>,
        params: DidChangeTextDocumentParams,
        uri: String,
    ) -> Option<()> {
        for change in params.content_changes {
            let range = &change.range?;
            let input_edit = range.to_input_edit(rope);
            if change.text.is_empty() {
                self.on_remove(range, rope);
            } else {
                self.on_insert(range, &change.text, rope);
            }
            let mut w = FileWriter::default();
            let _ = rope.write_to(&mut w);
            self.lsp_files.lock().ok().and_then(|lsp_files| {
                let lang_type = self.get_lang_type(&uri);
                lsp_files.input_edit(&uri, w.content, input_edit, lang_type)
            });
        }

        None
    }

    fn on_remove(&self, range: &Range, rope: &mut RefMut<'_, String, Rope>) -> Option<()> {
        let (start, end) = range.to_byte(rope);
        rope.remove(start..end);
        None
    }

    fn on_insert(
        &self,
        range: &Range,
        text: &str,
        rope: &mut RefMut<'_, String, Rope>,
    ) -> Option<()> {
        let (start, _) = range.to_byte(rope);
        rope.insert(start, text);
        None
    }

    fn get_lang_type(&self, path: &str) -> Option<LangType> {
        let path = Path::new(path);
        let config = self.config.read();
        config.ok().and_then(|config| config.file_ext(&path))
    }

    pub async fn publish_tag_diagnostics(
        &self,
        diagnostics: HashMap<String, Vec<(JinjaVariable, JinjaDiagnostic)>>,
        current_file: Option<String>,
    ) {
        let mut hm: HashMap<String, Vec<Diagnostic>> = HashMap::new();
        let mut added = false;

        for (file, diags) in diagnostics {
            for (variable, diag2) in diags {
                let severity = {
                    match diag2 {
                        JinjaDiagnostic::DefinedSomewhere => DiagnosticSeverity::INFORMATION,
                        JinjaDiagnostic::Undefined => DiagnosticSeverity::WARNING,
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
                self.client
                    .publish_diagnostics(uri, diagnostics, None)
                    .await;
            }
        }
        if let Some(uri) = current_file {
            if !added {
                let uri = Url::parse(&uri).unwrap();
                self.client.publish_diagnostics(uri, vec![], None).await;
            }
        }
    }

    pub fn start_completion(&self, params: CompletionParams) -> Option<CompletionType> {
        self.lsp_files
            .lock()
            .ok()
            .and_then(|lsp_files| lsp_files.completion(params, &self.config, &self.document_map))
    }

    pub fn start_hover(&self, params: HoverParams) -> Option<String> {
        self.lsp_files
            .lock()
            .ok()
            .and_then(|lsp_files| lsp_files.hover(params, &self.document_map))
    }

    pub fn start_goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Option<GotoDefinitionResponse> {
        self.lsp_files.lock().ok().and_then(|lsp_files| {
            lsp_files.goto_definition(params, &self.document_map, &self.config)
        })
    }

    pub fn start_code_action(&self, params: CodeActionParams) -> Option<bool> {
        self.lsp_files
            .lock()
            .ok()
            .and_then(|lsp_files| lsp_files.code_action(params, &self.document_map))
    }

    pub async fn start_command(&self, params: ExecuteCommandParams) -> Option<Value> {
        let mut diagnostics = HashMap::new();
        let command = params.command;
        if command == "reset_variables" {
            self.config.read().ok().and_then(|config| -> Option<()> {
                let diagnostics2 = walkdir(&config, &self.lsp_files, &self.document_map);
                if let Ok(all) = diagnostics2 {
                    diagnostics = all;
                }
                None
            });
            if diagnostics.is_empty() {
                return None;
            }
            self.publish_tag_diagnostics(diagnostics, None).await;
            None
        } else {
            None
        }
    }
}
