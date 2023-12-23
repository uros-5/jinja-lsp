use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use std::{result::Result as StdResult, sync::RwLockWriteGuard};

use dashmap::mapref::one::RefMut;
use tower_lsp::lsp_types::{
    CodeActionProviderCapability, CompletionContext, CompletionItem, CompletionItemKind,
    CompletionOptions, CompletionOptionsCompletionItem, CompletionParams, CompletionResponse,
    CompletionTriggerKind, DidChangeTextDocumentParams, Documentation, HoverProviderCapability,
    InitializedParams, MarkupContent, MarkupKind, OneOf, Range, ServerCapabilities, ServerInfo,
    TextDocumentPositionParams, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, TextDocumentSyncSaveOptions,
};
use tower_lsp::{
    jsonrpc::Result,
    lsp_types::{InitializeParams, InitializeResult, MessageType},
    Client, LanguageServer, LspService, Server,
};

use dashmap::DashMap;
use ropey::Rope;

use crate::config::{config_exist, read_config, JinjaConfig, LangType};
use crate::filters::init_filter_completions;
use crate::lsp_files::LspFiles;
use crate::query_helper::{CompletionType, Queries, QueryType};
use crate::to_input_edit::ToInputEdit;

pub struct Backend {
    client: Client,
    document_map: DashMap<String, Rope>,
    can_complete: RwLock<bool>,
    config: RwLock<Option<JinjaConfig>>,
    filter_values: HashMap<String, String>,
    pub lsp_files: Arc<Mutex<LspFiles>>,
    pub queries: Arc<Mutex<Queries>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let mut definition_provider = None;
        let mut references_provider = None;
        let mut code_action_provider = None;

        if let Some(client_info) = params.client_info {
            if client_info.name == "helix" {
                if let Ok(mut can_complete) = self.can_complete.write() {
                    *can_complete = true;
                }
            }
        }

        match config_exist(params.initialization_options) {
            Some(config) => {
                let _ = self.config.try_write().is_ok_and(|mut jinja_config| {
                    definition_provider = Some(OneOf::Left(true));
                    references_provider = Some(OneOf::Left(true));
                    code_action_provider = Some(CodeActionProviderCapability::Simple(true));
                    *jinja_config = Some(config);
                    true
                });
            }
            None => {
                self.client
                    .log_message(MessageType::INFO, "Config not found")
                    .await;
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
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: String::from("jinja-lsp"),
                version: Some(String::from("0.1.0")),
            }),
            offset_encoding: None,
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "initialized!")
            .await;

        match read_config(
            &self.config,
            &self.lsp_files,
            &self.queries,
            &self.document_map,
        ) {
            Ok(d) => {}
            Err(err) => {
                let _ = self.config.write().is_ok_and(|mut config| {
                    *config = None;
                    true
                });
                let msg = err.to_string();
                self.client.log_message(MessageType::INFO, msg).await;
            }
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = &params.text_document.uri.to_string();
        let rope = self.document_map.get_mut(uri);
        if let Some(mut rope) = rope {
            for change in params.content_changes {
                if let Some(range) = &change.range {
                    let input_edit = range.to_input_edit(&rope);
                    if change.text.is_empty() {
                        self.on_remove(range, &mut rope);
                    } else {
                        self.on_insert(range, &change.text, &mut rope);
                    }
                    let mut w = LocalWriter::default();
                    let _ = rope.write_to(&mut w);
                    let _ = self.lsp_files.lock().is_ok_and(|lsp_files| {
                        let lang_type = self.get_lang_type(uri);
                        lsp_files.input_edit(uri, w.content, input_edit, lang_type);
                        true
                    });
                }
            }
        }
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

        // TODO disable for backend and javascript
        if !can_complete {
            let can_complete = self.can_complete.read().is_ok_and(|d| *d);
            if !can_complete {
                return Ok(None);
            }
        }
        let uri = &params.text_document_position.text_document.uri;
        let mut lang_type = None;

        self.config.read().is_ok_and(|config| {
            if let Some(config) = config.as_ref() {
                lang_type = config.file_ext(&Path::new(&uri.as_str()));
            }
            false
        });
        let mut completion = None;
        let mut items = None;
        if let Some(lang_type) = lang_type {
            if lang_type == LangType::Template {
                completion = self.start_completion(&params.text_document_position, uri.to_string());
            }
        }
        if let Some(compl) = completion {
            match compl {
                CompletionType::Pipe => {
                    let completions = self.filter_values.clone();
                    let mut ret = Vec::with_capacity(completions.len());
                    for item in completions.into_iter() {
                        ret.push(CompletionItem {
                            label: item.0.to_string(),
                            kind: Some(CompletionItemKind::TEXT),
                            documentation: Some(Documentation::MarkupContent(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: item.1.to_string(),
                            })),
                            ..Default::default()
                        });
                    }
                    items = Some(CompletionResponse::Array(ret));
                    //
                }
                CompletionType::Identifier => {
                    //
                }
            }
        }

        Ok(items)
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let document_map = DashMap::new();
        let can_complete = RwLock::new(false);
        let config = RwLock::new(None);
        let filter_values = init_filter_completions();
        let lsp_files = Arc::new(Mutex::new(LspFiles::default()));
        let queries = Arc::new(Mutex::new(Queries::default()));
        Self {
            client,
            document_map,
            can_complete,
            config,
            filter_values,
            lsp_files,
            queries,
        }
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

    fn get_lang_type(&self, path: &String) -> Option<LangType> {
        let path = Path::new(path);
        let config = self.config.read();
        let mut lang_type = None;
        let _ = config.is_ok_and(|config| {
            if let Some(config) = config.as_ref() {
                lang_type = config.file_ext(&path);
            }
            true
        });
        lang_type
    }

    fn start_completion(
        &self,
        text_params: &TextDocumentPositionParams,
        uri: String,
    ) -> Option<CompletionType> {
        let text = self.document_map.get(&uri)?;
        let text = text.to_string();
        let pos = text_params.position;
        let mut res = None;
        let _ = self.queries.lock().is_ok_and(|queries| {
            if let Ok(lsp_files) = self.lsp_files.lock() {
                if let Some(index) = lsp_files.get_index(&uri) {
                    res = lsp_files.query_completion(
                        index,
                        &text,
                        QueryType::Completion,
                        pos,
                        &queries,
                    );
                } else if let Some(index) = lsp_files.add_file(String::from(&uri)) {
                    lsp_files.add_tree(index, LangType::Template, &text, None);
                    res = lsp_files.query_completion(
                        index,
                        &text,
                        QueryType::Completion,
                        pos,
                        &queries,
                    );
                } else {
                    res = None;
                }
            }
            true
        });
        res
    }
}

#[derive(Default, Debug)]
pub struct LocalWriter {
    pub content: String,
}

impl std::io::Write for LocalWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(b) = std::str::from_utf8(buf) {
            self.content.push_str(b);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
