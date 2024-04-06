use serde_json::Value;
use tokio::sync::{
    mpsc::{self, Sender},
    oneshot,
};
use tower_lsp::{
    jsonrpc::Result,
    lsp_types::{
        CompletionParams, CompletionResponse, DidChangeConfigurationParams,
        DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentSymbolParams,
        DocumentSymbolResponse, InitializeParams, InitializeResult,
    },
    Client, LanguageServer,
};

use tower_lsp::lsp_types::{
    CodeActionParams, CodeActionResponse, DidChangeTextDocumentParams, DidSaveTextDocumentParams,
    ExecuteCommandParams, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverParams,
    InitializedParams,
};

use crate::channels::{
    diagnostics::diagnostics_task,
    lsp::{lsp_task, LspMessage},
};

pub struct Backend {
    main_channel: Sender<LspMessage>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let (sender, rx) = oneshot::channel();
        let _ = self
            .main_channel
            .send(LspMessage::Initialize(Box::new(params), sender))
            .await;
        if let Ok(msg) = rx.await {
            Ok(msg)
        } else {
            Ok(InitializeResult::default())
        }
    }

    async fn initialized(&self, _params: InitializedParams) {
        let (sender, _) = oneshot::channel();
        let _ = self
            .main_channel
            .send(LspMessage::Initialized(sender))
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let _ = self.main_channel.send(LspMessage::DidOpen(params)).await;
    }

    async fn did_close(&self, _params: DidCloseTextDocumentParams) {}

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let _ = self.main_channel.send(LspMessage::DidChange(params)).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let _ = self.main_channel.send(LspMessage::DidSave(params)).await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let (sender, tx) = oneshot::channel();
        let _ = self
            .main_channel
            .send(LspMessage::Completion(params, sender))
            .await;
        if let Ok(completion) = tx.await {
            return Ok(completion);
        }
        Ok(None)
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let (sender, tx) = oneshot::channel();
        let _ = self
            .main_channel
            .send(LspMessage::Hover(params, sender))
            .await;
        if let Ok(hover) = tx.await {
            return Ok(hover);
        }
        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let (sender, tx) = oneshot::channel();
        let _ = self
            .main_channel
            .send(LspMessage::GoToDefinition(params, sender))
            .await;
        if let Ok(definition) = tx.await {
            return Ok(definition);
        }
        Ok(None)
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let (sender, tx) = oneshot::channel();
        let _ = self
            .main_channel
            .send(LspMessage::CodeAction(params, sender))
            .await;
        if let Ok(code_action) = tx.await {
            return Ok(code_action);
        }
        Ok(None)
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        let (sender, _) = oneshot::channel();
        let _ = self
            .main_channel
            .send(LspMessage::ExecuteCommand(params, sender))
            .await;
        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let (sender, tx) = oneshot::channel();
        let _ = self
            .main_channel
            .send(LspMessage::DocumentSymbol(params, sender))
            .await;
        if let Ok(symbols) = tx.await {
            return Ok(symbols);
        }

        Ok(None)
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        let _ = self
            .main_channel
            .send(LspMessage::DidChangeConfiguration(params))
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let (lsp_sender, lsp_recv) = mpsc::channel(50);
        let (diagnostic_sender, diagnostic_recv) = mpsc::channel(20);
        lsp_task(
            client.clone(),
            diagnostic_sender,
            lsp_sender.clone(),
            lsp_recv,
        );
        diagnostics_task(client.clone(), diagnostic_recv, lsp_sender.clone());
        Self {
            main_channel: lsp_sender,
        }
    }
}
