use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::{result::Result as StdResult, sync::RwLockWriteGuard};

use dashmap::mapref::one::RefMut;
use tower_lsp::lsp_types::{
    CodeActionProviderCapability, CompletionOptions, CompletionOptionsCompletionItem,
    DidChangeTextDocumentParams, HoverProviderCapability, InitializedParams, OneOf, Range,
    ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, TextDocumentSyncSaveOptions,
};
use tower_lsp::{
    jsonrpc::Result,
    lsp_types::{InitializeParams, InitializeResult, MessageType},
    Client, LanguageServer, LspService, Server,
};

use dashmap::DashMap;
use ropey::Rope;

use crate::config::{config_exist, read_config, JinjaConfig};
use crate::filters::init_filter_completions;
use crate::lsp_files::LspFiles;
use crate::query_helper::Queries;
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
                        lsp_files.input_edit(uri, w.content, input_edit);
                        true
                    });
                }
            }
        }
        // if let Some(text) = params.content_changes.first_mut() {
        // self.on_change(ServerTextDocumentItem {
        //     uri: params.text_document.uri,
        //     text: std::mem::take(&mut text.text),
        // })
        // .await
        // }
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
