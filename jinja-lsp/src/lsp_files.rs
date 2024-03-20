use jinja_lsp_queries::tree_builder::{DataType, JinjaDiagnostic, JinjaVariable, LangType};
use std::{collections::HashMap, fs::read_to_string, path::Path, time::Duration};
use tokio::{sync::mpsc, task::JoinHandle, time::sleep};
use tower_lsp::lsp_types::{
    CompletionItemKind, CompletionTextEdit, DidOpenTextDocumentParams, TextDocumentIdentifier,
    TextEdit,
};

use jinja_lsp_queries::{
    capturer::{
        included::IncludeCapturer,
        init::JinjaInitCapturer,
        object::{CompletionType, JinjaObjectCapturer},
        rust::RustCapturer,
    },
    lsp_helper::search_errors,
    parsers::Parsers,
    queries::{query_props, Queries},
    to_input_edit::{to_position, to_position2, ToInputEdit},
};
use ropey::Rope;

use tower_lsp::lsp_types::{
    CodeActionParams, CompletionContext, CompletionItem, CompletionParams, CompletionTriggerKind,
    DidChangeTextDocumentParams, DidSaveTextDocumentParams, GotoDefinitionParams,
    GotoDefinitionResponse, HoverParams, Location, Position, Range, Url,
};
use tree_sitter::{InputEdit, Point, Tree};

use crate::{
    channels::{diagnostics::DiagnosticMessage, lsp::LspMessage},
    config::JinjaConfig,
};

pub struct LspFiles {
    trees: HashMap<LangType, HashMap<String, Tree>>,
    documents: HashMap<String, Rope>,
    pub parsers: Parsers,
    pub variables: HashMap<String, Vec<JinjaVariable>>,
    pub queries: Queries,
    pub config: JinjaConfig,
    pub diagnostics_task: JoinHandle<()>,
    pub main_channel: Option<mpsc::Sender<LspMessage>>,
}

impl LspFiles {
    pub fn read_file(&mut self, path: &&Path, lang_type: LangType) -> Option<()> {
        if let Ok(name) = std::fs::canonicalize(path) {
            let name = name.to_str()?;
            let file_content = read_to_string(name).ok()?;
            let rope = Rope::from_str(&file_content);
            let name = format!("file://{}", name);
            let adding = name.clone();
            self.delete_variables(&name);
            self.documents.insert(name.to_string(), rope);
            self.add_tree(&name, lang_type, &file_content);
            self.add_variables(&adding, lang_type, &file_content);
        }
        None
    }

    pub fn add_tree(
        &mut self,
        file_name: &str,
        lang_type: LangType,
        file_content: &str,
    ) -> Option<()> {
        let trees = self.trees.get_mut(&lang_type)?;
        let old_tree = trees.get_mut(&file_name.to_string());
        match old_tree {
            Some(old_tree) => {
                let new_tree = self
                    .parsers
                    .parse(lang_type, file_content, Some(old_tree))?;
                trees.insert(file_name.to_string(), new_tree);
            }
            None => {
                // tree doesn't exist, first insertion
                let new_tree = self.parsers.parse(lang_type, file_content, None)?;
                trees.insert(file_name.to_string(), new_tree);
            }
        };
        None
    }

    fn delete_variables(&mut self, name: &str) -> Option<()> {
        self.variables.get_mut(name)?.clear();
        Some(())
    }

    fn add_variables(&mut self, name: &str, lang_type: LangType, file_content: &str) -> Option<()> {
        let trees = self.trees.get(&lang_type).unwrap();
        let tree = trees.get(name)?;
        let trigger_point = Point::new(0, 0);
        let closest_node = tree.root_node();
        match lang_type {
            LangType::Backend => {
                let query = &self.queries.rust_idents;
                let capturer = RustCapturer::default();
                let mut variables = vec![];
                let capturer = query_props(
                    closest_node,
                    file_content,
                    trigger_point,
                    query,
                    true,
                    capturer,
                );

                for variable in capturer.variables() {
                    variables.push(JinjaVariable::new(
                        &variable.0,
                        variable.1,
                        DataType::BackendVariable,
                    ));
                }
                for macros in capturer.macros() {
                    for variable in macros.1.variables() {
                        variables.push(JinjaVariable::new(
                            variable.0,
                            *variable.1,
                            DataType::BackendVariable,
                        ));
                    }
                }
                self.variables.insert(name.to_string(), variables);
            }
            LangType::Template => {
                let query = &self.queries.jinja_init;
                let capturer = JinjaInitCapturer::default();
                let capturer = query_props(
                    closest_node,
                    file_content,
                    trigger_point,
                    query,
                    true,
                    capturer,
                );
                self.variables.insert(name.to_string(), capturer.to_vec());
            }
        }
        Some(())
    }

    pub fn input_edit(
        &mut self,
        file: &String,
        code: String,
        input_edit: InputEdit,
        lang_type: Option<LangType>,
    ) -> Option<()> {
        let lang_type = lang_type?;
        let trees = self.trees.get_mut(&lang_type)?;
        let old_tree = trees.get_mut(file)?;
        old_tree.edit(&input_edit);
        let new_tree = self.parsers.parse(lang_type, &code, Some(old_tree))?;
        let trees = self.trees.get_mut(&lang_type)?;
        trees.insert(file.to_string(), new_tree);
        None
    }

    pub fn read_tree(&self, name: &str) -> Option<Vec<(JinjaVariable, JinjaDiagnostic)>> {
        let rope = self.documents.get(name)?;
        let mut writter = FileWriter::default();
        let _ = rope.write_to(&mut writter);
        let content = writter.content;
        let trees = self.trees.get(&LangType::Template)?;
        let tree = trees.get(name)?;
        let closest_node = tree.root_node();
        search_errors(
            closest_node,
            &content,
            &self.queries,
            &self.variables,
            &name.to_string(),
            &self.config.templates,
        )
    }

    pub fn did_change(&mut self, params: DidChangeTextDocumentParams) -> Option<()> {
        let uri = params.text_document.uri.to_string();
        let rope = self.documents.get_mut(&uri)?;
        let lang_type = self.config.file_ext(&Path::new(&uri));
        let mut changes = vec![];
        for change in params.content_changes {
            let range = change.range?;
            let input_edit = rope.to_input_edit(range, &change.text);
            if change.text.is_empty() {
                let start = rope.to_byte(range.start);
                let end = rope.to_byte(range.end);
                if start <= end {
                    rope.remove(start..end);
                } else {
                    rope.remove(end..start);
                }
            } else {
                let start = rope.to_byte(range.start);
                rope.insert(start, &change.text);
            }
            let mut w = FileWriter::default();
            let _ = rope.write_to(&mut w);
            changes.push((w.content, input_edit));
        }
        for change in changes {
            self.input_edit(&uri, change.0, change.1, lang_type);
        }
        let param = DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier::new(params.text_document.uri),
            text: None,
        };
        self.diagnostics_task.abort();
        let channel = self.main_channel.clone();
        self.diagnostics_task = tokio::spawn(async move {
            sleep(Duration::from_millis(200)).await;
            if let Some(channel) = channel {
                let _ = channel.send(LspMessage::DidSave(param)).await;
            }
        });
        None
    }

    pub fn did_save(&mut self, params: DidSaveTextDocumentParams) -> Option<DiagnosticMessage> {
        let uri = params.text_document.uri.as_str();
        let path = Path::new(&uri);
        let lang_type = self.config.file_ext(&path)?;
        let doc = self.documents.get(uri)?;
        let mut contents = FileWriter::default();
        let _ = doc.write_to(&mut contents);
        let content = contents.content;
        self.delete_variables(uri);
        self.add_variables(uri, lang_type, &content);
        let mut hm = HashMap::new();
        let v = self.read_tree(uri);
        if let Some(v) = v {
            hm.insert(uri.to_owned(), v);
        } else {
            hm.insert(uri.to_owned(), vec![]);
        }
        let message = DiagnosticMessage::Errors {
            diagnostics: hm,
            current_file: Some(uri.to_owned()),
        };
        Some(message)
    }

    pub fn completion(
        &self,
        params: CompletionParams,
        can_complete2: bool,
    ) -> Option<CompletionType> {
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
            let can_complete = can_complete2;
            if !can_complete {
                return None;
            }
        }

        let uri = params.text_document_position.text_document.uri.to_string();
        let row = params.text_document_position.position.line;
        let column = params.text_document_position.position.character;
        let point = Point::new(row as usize, column as usize);
        let ext = self.config.file_ext(&Path::new(&uri))?;
        if ext != LangType::Template {
            return None;
        }
        let trees = self.trees.get(&LangType::Template)?;
        let tree = trees.get(&uri)?;
        let closest_node = tree.root_node();
        let query = &self.queries.jinja_idents;
        let capturer = JinjaObjectCapturer::default();
        let doc = self.documents.get(&uri)?;
        let mut writter = FileWriter::default();
        let _ = doc.write_to(&mut writter);
        let props = query_props(
            closest_node,
            &writter.content,
            point,
            query,
            false,
            capturer,
        );
        if let Some(completion) = props.completion(point) {
            return Some(completion);
        }
        let query = &self.queries.jinja_imports;
        let capturer = IncludeCapturer::default();
        let props = query_props(
            closest_node,
            &writter.content,
            point,
            query,
            false,
            capturer,
        );
        props.completion(point)
    }

    pub fn hover(&self, params: HoverParams) -> Option<String> {
        let uri = &params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let lang_type = self.config.file_ext(&Path::new(uri.as_str()));
        let can_hover = lang_type.map_or(false, |lang_type| lang_type == LangType::Template);
        if !can_hover {
            return None;
        }

        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let row = params.text_document_position_params.position.line;
        let column = params.text_document_position_params.position.character;
        let point = Point::new(row as usize, column as usize);
        let trees = self.trees.get(&LangType::Template)?;
        let tree = trees.get(&uri)?;
        let closest_node = tree.root_node();
        let query = &self.queries.jinja_idents;
        let capturer = JinjaObjectCapturer::default();
        let doc = self.documents.get(&uri)?;
        let mut writter = FileWriter::default();
        let _ = doc.write_to(&mut writter);
        let props = query_props(
            closest_node,
            &writter.content,
            point,
            query,
            false,
            capturer,
        );
        if props.is_hover(point) {
            let id = props.get_last_id()?;
            return Some(id);
        }
        None
    }

    pub fn goto_definition(&self, params: GotoDefinitionParams) -> Option<GotoDefinitionResponse> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let uri2 = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let row = params.text_document_position_params.position.line;
        let column = params.text_document_position_params.position.character;
        let point = Point::new(row as usize, column as usize);
        let trees = self.trees.get(&LangType::Template)?;
        let tree = trees.get(&uri)?;
        let closest_node = tree.root_node();
        let mut current_ident = String::new();

        let query = &self.queries.jinja_idents;
        let capturer = JinjaObjectCapturer::default();
        let doc = self.documents.get(&uri)?;
        let mut writter = FileWriter::default();
        let _ = doc.write_to(&mut writter);
        let props = query_props(
            closest_node,
            &writter.content,
            point,
            query,
            false,
            capturer,
        );
        let mut res = props.is_ident(point).and_then(|ident| {
            current_ident = ident.to_string();
            let variables = self.variables.get(&uri)?;
            let max = variables
                .iter()
                .filter(|item| item.name == ident && item.location.0 <= point)
                .max()?;
            let (start, end) = to_position(max);
            let range = Range::new(start, end);
            Some(GotoDefinitionResponse::Scalar(Location {
                uri: uri2.clone(),
                range,
            }))
        });
        res.is_none().then(|| -> Option<()> {
            let query = &self.queries.jinja_imports;
            let capturer = IncludeCapturer::default();
            let doc = self.documents.get(&uri)?;
            let mut writter = FileWriter::default();
            let _ = doc.write_to(&mut writter);
            let props = query_props(
                closest_node,
                &writter.content,
                point,
                query,
                false,
                capturer,
            );
            if let Some(last) = props.in_template(point) {
                let uri = last.is_template(&self.config.templates)?;
                let start = to_position2(Point::new(0, 0));
                let end = to_position2(Point::new(0, 0));
                let range = Range::new(start, end);
                let location = Location { uri, range };
                res = Some(GotoDefinitionResponse::Scalar(location));
                None
            } else {
                let mut all: Vec<Location> = vec![];
                for i in &self.variables {
                    let idents = i.1.iter().filter(|item| item.name == current_ident);
                    for id in idents {
                        let uri = Url::parse(i.0).unwrap();
                        let (start, end) = to_position(id);
                        let range = Range::new(start, end);
                        let location = Location { uri, range };
                        all.push(location);
                    }
                }
                res = Some(GotoDefinitionResponse::Array(all));
                None
            }
        });
        res
    }

    pub fn code_action(&self, action_params: CodeActionParams) -> Option<bool> {
        let uri = action_params.text_document.uri.to_string();
        let lang_type = self.config.file_ext(&Path::new(&uri));
        let can_def = lang_type.map_or(false, |lang_type| lang_type == LangType::Template);
        if !can_def {
            return None;
        }
        let row = action_params.range.start.line;
        let column = action_params.range.start.character;
        let point = Point::new(row as usize, column as usize);
        let trees = self.trees.get(&LangType::Template)?;
        let tree = trees.get(&uri)?;
        let closest_node = tree.root_node();
        let _current_ident = String::new();
        let query = &self.queries.jinja_idents;
        let capturer = JinjaObjectCapturer::default();
        let doc = self.documents.get(&uri)?;
        let mut writter = FileWriter::default();
        let _ = doc.write_to(&mut writter);
        let props = query_props(
            closest_node,
            &writter.content,
            point,
            query,
            false,
            capturer,
        );
        Some(props.in_expr(point))
    }

    pub fn read_trees(&self, diags: &mut HashMap<String, Vec<(JinjaVariable, JinjaDiagnostic)>>) {
        for tree in self.trees.get(&LangType::Template).unwrap() {
            let errors = self.read_tree(tree.0);
            if let Some(errors) = errors {
                diags.insert(String::from(tree.0), errors);
            }
        }
    }

    pub fn read_variables(&self, uri: &Url, position: Position) -> Option<Vec<CompletionItem>> {
        let start = position.line as usize;
        let end = position.character as usize;
        let position = Point::new(start, end);
        let uri = &uri.to_string();
        let variables = self.variables.get(uri)?;
        let mut items = vec![];
        for variable in variables.iter() {
            if position < variable.location.1 {
                continue;
            }
            items.push(CompletionItem {
                label: variable.name.to_string(),
                detail: Some(completion_detail(variable.data_type).to_string()),
                kind: Some(completion_kind(variable.data_type)),
                ..Default::default()
            });
        }
        for file in self.variables.iter() {
            for variable in file.1 {
                if variable.data_type == DataType::BackendVariable {
                    items.push(CompletionItem {
                        label: variable.name.to_string(),
                        detail: Some(completion_detail(variable.data_type).to_string()),
                        kind: Some(completion_kind(variable.data_type)),
                        ..Default::default()
                    });
                }
            }
        }
        if !items.is_empty() {
            return Some(items);
        }
        None
    }

    pub fn read_templates(&self, mut prefix: String, range: Range) -> Option<Vec<CompletionItem>> {
        let all_templates = self.trees.get(&LangType::Template)?;
        if prefix.is_empty() {
            prefix = String::from("file:///");
        }
        let templates = all_templates
            .keys()
            .filter(|template| template.contains(&prefix));
        let mut abc = vec![];
        for template in templates {
            let c = &self.config.templates.replace('.', "");
            let mut parts = template.split(c);
            parts.next();
            let label = parts.next()?.replacen('/', "", 1);
            let new_text = format!("\"{label}\"");
            let text_edit = CompletionTextEdit::Edit(TextEdit::new(range, new_text));
            let item = CompletionItem {
                label,
                detail: Some("Jinja template".to_string()),
                kind: Some(CompletionItemKind::FILE),
                text_edit: Some(text_edit),
                ..Default::default()
            };
            abc.push(item);
        }

        Some(abc)
    }

    pub fn did_open(&mut self, params: DidOpenTextDocumentParams) -> Option<DiagnosticMessage> {
        let name = params.text_document.uri.as_str();
        let lang_type = self.config.file_ext(&Path::new(name))?;
        let file_content = params.text_document.text;
        let rope = Rope::from_str(&file_content);
        self.delete_variables(name);
        self.documents.insert(name.to_string(), rope);
        self.add_tree(name, lang_type, &file_content);
        self.add_variables(name, lang_type, &file_content);
        let diagnostics = self.read_tree(name)?;
        let mut hm = HashMap::new();
        hm.insert(name.to_owned(), diagnostics);
        let msg = DiagnosticMessage::Errors {
            diagnostics: hm,
            current_file: Some(name.to_owned()),
        };
        Some(msg)
    }
}

impl Default for LspFiles {
    fn default() -> Self {
        let mut trees = HashMap::new();
        trees.insert(LangType::Template, HashMap::new());
        trees.insert(LangType::Backend, HashMap::new());
        let diagnostics_task = tokio::spawn(async move {});
        let main_channel = None;
        Self {
            trees,
            parsers: Parsers::default(),
            variables: HashMap::new(),
            queries: Queries::default(),
            documents: HashMap::new(),
            config: JinjaConfig::default(),
            diagnostics_task,
            main_channel,
        }
    }
}

#[derive(Default, Debug)]
pub struct FileWriter {
    pub content: String,
}

impl std::io::Write for FileWriter {
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

pub fn completion_kind(variable_type: DataType) -> CompletionItemKind {
    match variable_type {
        DataType::Macro => CompletionItemKind::FUNCTION,
        DataType::MacroParameter => CompletionItemKind::FIELD,
        DataType::Variable => CompletionItemKind::VARIABLE,
        DataType::BackendVariable => CompletionItemKind::VARIABLE,
        DataType::WithVariable => CompletionItemKind::VARIABLE,
        DataType::Block => CompletionItemKind::UNIT,
        DataType::Template => CompletionItemKind::FILE,
    }
}

pub fn completion_detail(variable_type: DataType) -> &'static str {
    match variable_type {
        DataType::Macro => "Macro from this file",
        DataType::MacroParameter => "Macro parameter from this file",
        DataType::Variable => "Variable from this file",
        DataType::BackendVariable => "Backend variable.",
        DataType::WithVariable => "With variable from this file",
        DataType::Block => "Block from this file",
        DataType::Template => "Template",
    }
}
