use serde_json::Value;
use tokio::sync::{
    mpsc::{self, Sender},
    oneshot,
};
use tower_lsp::{
    jsonrpc::Result,
    lsp_types::{
        CompletionParams, CompletionResponse, DidCloseTextDocumentParams,
        DidOpenTextDocumentParams, InitializeParams, InitializeResult,
    },
    Client, LanguageServer,
};

use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, CodeActionResponse, Command,
    DidChangeTextDocumentParams, DidSaveTextDocumentParams, ExecuteCommandParams,
    GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverParams, InitializedParams,
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
        let (sender, rx) = oneshot::channel();
        let _ = self
            .main_channel
            .send(LspMessage::Initialized(sender))
            .await;
        if let Ok(msg) = rx.await {
            if !msg {
                let _ = self.shutdown().await;
            }
        } else {
            let _ = self.shutdown().await;
        }
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
        lsp_task(
            client.clone(),
            diagnostic_sender,
            lsp_sender.clone(),
            lsp_recv,
        );
        diagnostics_task(client.clone(), diagnostic_recv);
        Self {
            main_channel: lsp_sender,
        }
    }
}
