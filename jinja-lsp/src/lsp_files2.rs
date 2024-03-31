use jinja_lsp_queries::{
    lsp_helper::search_errors,
    search::{
        completion_start, definition::definition_query, objects::objects_query, queries::Queries,
        rust_identifiers::rust_definition_query, rust_template_completion::rust_templates_query,
        snippets_completion::snippets_query, templates::templates_query, to_range, Identifier,
        IdentifierType,
    },
    tree_builder::LangType,
};
use std::{
    collections::{HashMap, HashSet},
    fs::read_to_string,
    path::Path,
    time::Duration,
};
use tokio::{sync::mpsc, task::JoinHandle, time::sleep};
use tower_lsp::lsp_types::{
    CompletionItemKind, CompletionTextEdit, Diagnostic, DidOpenTextDocumentParams, DocumentSymbol,
    DocumentSymbolResponse, TextDocumentIdentifier, TextEdit,
};

use jinja_lsp_queries::{
    parsers::Parsers,
    search::objects::CompletionType,
    to_input_edit::{to_position2, ToInputEdit},
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
    pub queries: Queries,
    pub config: JinjaConfig,
    pub diagnostics_task: JoinHandle<()>,
    pub main_channel: Option<mpsc::Sender<LspMessage>>,
    pub variables: HashMap<String, Vec<Identifier>>,
    pub is_vscode: bool,
}

impl LspFiles {
    pub fn read_file(&mut self, path: &&Path, lang_type: LangType) -> Option<()> {
        if let Ok(name) = std::fs::canonicalize(path) {
            let name = name.to_str()?;
            let file_content = read_to_string(name).ok()?;
            let rope = Rope::from_str(&file_content);
            let name = format!("file://{}", name);
            let adding = name.clone();
            self.documents.insert(name.to_string(), rope);
            self.add_tree(&name, lang_type, &file_content);
            self.add_variables(&adding, lang_type, &file_content);
        }
        None
    }

    fn add_variables(&mut self, name: &str, lang_type: LangType, file_content: &str) -> Option<()> {
        let trees = self.trees.get(&lang_type).unwrap();
        let tree = trees.get(name)?;
        let trigger_point = Point::new(0, 0);
        match lang_type {
            LangType::Backend => {
                let mut variables = vec![];
                let query_defs = &self.queries.rust_definitions;
                let query_templates = &self.queries.rust_templates;
                let mut ids =
                    rust_definition_query(query_defs, tree, trigger_point, file_content, true)
                        .show();
                let mut templates =
                    rust_templates_query(query_templates, tree, trigger_point, file_content, true)
                        .collect();
                variables.append(&mut ids);
                variables.append(&mut templates);
                self.variables.insert(String::from(name), variables);
            }
            LangType::Template => {
                let mut variables = vec![];
                let query_snippets = &self.queries.jinja_snippets;
                let snippets =
                    snippets_query(query_snippets, tree, trigger_point, file_content, true);
                if snippets.is_error {
                    return None;
                }
                let query_defs = &self.queries.jinja_definitions;
                let mut definitions =
                    definition_query(query_defs, tree, trigger_point, file_content, true)
                        .identifiers();
                variables.append(&mut definitions);
                self.variables.insert(String::from(name), variables);
            }
        }
        Some(())
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

    pub fn did_change(&mut self, params: DidChangeTextDocumentParams) -> Option<()> {
        let uri = params.text_document.uri.to_string();
        let rope = self.documents.get_mut(&uri)?;
        let lang_type = self.config.file_ext(&Path::new(&uri));
        let mut changes = vec![];
        for change in params.content_changes {
            let range = change.range?;
            let input_edit = rope.to_input_edit(range, &change.text);
            let start = rope.to_byte(range.start);
            let end = rope.to_byte(range.end);
            if start <= end {
                rope.remove(start..end);
            } else {
                rope.remove(end..start);
            }
            if !change.text.is_empty() {
                rope.insert(start, &change.text);
            }
            let mut w = FileContent::default();
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

    pub fn read_tree(&self, name: &str) -> Option<Vec<Diagnostic>> {
        let rope = self.documents.get(name)?;
        let mut writter = FileContent::default();
        let _ = rope.write_to(&mut writter);
        let content = writter.content;
        let lang_type = self.config.file_ext(&Path::new(name))?;
        let trees = self.trees.get(&lang_type)?;
        let tree = trees.get(name)?;
        search_errors(
            tree,
            &content,
            &self.queries,
            &self.variables,
            &name.to_string(),
            &self.config.templates,
            lang_type,
        )
    }

    pub fn did_save(&mut self, params: DidSaveTextDocumentParams) -> Option<DiagnosticMessage> {
        let uri = params.text_document.uri.as_str();
        let path = Path::new(&uri);
        let lang_type = self.config.file_ext(&path)?;
        let doc = self.documents.get(uri)?;
        let mut contents = FileContent::default();
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
        let message = DiagnosticMessage::Errors(hm);
        Some(message)
    }

    pub fn completion(&self, params: CompletionParams) -> Option<CompletionType> {
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
            return None;
        }

        let uri = params.text_document_position.text_document.uri.to_string();
        let row = params.text_document_position.position.line;
        let column = params.text_document_position.position.character;
        let point = Point::new(row as usize, column as usize);
        let ext = self.config.file_ext(&Path::new(&uri))?;
        let trees = self.trees.get(&ext)?;
        let tree = trees.get(&uri)?;
        let doc = self.documents.get(&uri)?;
        let mut writter = FileContent::default();
        let _ = doc.write_to(&mut writter);
        match ext {
            LangType::Template => {
                let query = &self.queries.jinja_snippets;
                let snippets = snippets_query(query, tree, point, &writter.content, false);
                if snippets.to_complete(point).is_some() {
                    let start = to_position2(point);
                    let mut end = to_position2(point);
                    end.character += 1;
                    let range = Range::new(start, end);
                    return Some(CompletionType::Snippets { range });
                }
                let query = &self.queries.jinja_objects;
                let objects = objects_query(query, tree, point, &writter.content, false);
                if let Some(completion) = objects.completion(point) {
                    return Some(completion);
                }
                let query = &self.queries.jinja_imports;
                let query = templates_query(query, tree, point, &writter.content, false);
                let identifier = query.in_template(point)?.get_identifier(point)?;
                let start = completion_start(point, identifier)?;
                let range = to_range((identifier.start, identifier.end));
                Some(CompletionType::IncludedTemplate {
                    name: start.to_owned(),
                    range,
                })
            }
            LangType::Backend => {
                let rust_templates = rust_templates_query(
                    &self.queries.rust_templates,
                    tree,
                    point,
                    &writter.content,
                    false,
                );
                let identifier = rust_templates.in_template(point)?;
                let start = completion_start(point, identifier)?;
                let range = to_range((identifier.start, identifier.end));
                Some(CompletionType::IncludedTemplate {
                    name: start.to_owned(),
                    range,
                })
            }
        }
    }

    fn delete_variables(&mut self, uri: &str) -> Option<()> {
        self.variables.get_mut(uri)?.clear();
        Some(())
    }

    pub fn hover(&self, params: HoverParams) -> Option<(Identifier, bool)> {
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
        let trigger_point = Point::new(row as usize, column as usize);
        let trees = self.trees.get(&LangType::Template)?;
        let tree = trees.get(&uri)?;
        let query = &self.queries.jinja_objects;
        let doc = self.documents.get(&uri)?;
        let mut writter = FileWriter::default();
        let _ = doc.write_to(&mut writter);
        let objects = objects_query(query, tree, trigger_point, &writter.content, false);
        if objects.is_hover(trigger_point) {
            let object = objects.get_last_id()?;
            if object.is_filter {
                return Some((Identifier::from(object), true));
            } else {
                return Some((Identifier::from(object), false));
            }
        }
        // else if objects.is_ident(point) {

        // }
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
        let lang_type = self.config.file_ext(&Path::new(&uri))?;
        let trees = self.trees.get(&lang_type)?;
        let tree = trees.get(&uri)?;
        let row = params.text_document_position_params.position.line;
        let column = params.text_document_position_params.position.character;
        let point = Point::new(row as usize, column as usize);
        let doc = self.documents.get(&uri)?;
        let mut writter = FileWriter::default();
        let _ = doc.write_to(&mut writter);

        let mut current_ident = String::new();

        match lang_type {
            LangType::Template => {
                let query = &self.queries.jinja_objects;
                let objects = objects_query(query, tree, point, &writter.content, false);
                let mut res = objects.is_ident(point).and_then(|ident| {
                    current_ident = ident.to_owned();
                    let variables = self.variables.get(&uri)?;
                    let max = variables
                        .iter()
                        .filter(|item| {
                            item.name == ident && item.start <= point && point <= item.scope_ends.1
                        })
                        .max()?;
                    let (start, end) = (to_position2(max.start), to_position2(max.end));
                    let range = Range::new(start, end);
                    Some(GotoDefinitionResponse::Scalar(Location {
                        uri: uri2.clone(),
                        range,
                    }))
                });
                res.is_none().then(|| -> Option<()> {
                    let query = &self.queries.jinja_imports;
                    let query = templates_query(query, tree, point, &writter.content, false);
                    let identifier = query.in_template(point)?.get_identifier(point)?;
                    let dir = &self.config.templates;
                    let path = format!("{dir}/{}", identifier.name);
                    let buffer = std::fs::canonicalize(path).ok()?;
                    let url = format!("file://{}", buffer.to_str()?);
                    let url = Url::parse(&url).ok()?;
                    let start = to_position2(identifier.start);
                    let end = to_position2(identifier.end);
                    let range = Range::new(start, end);
                    let location = Location::new(url, range);
                    res = Some(GotoDefinitionResponse::Scalar(location));
                    None
                });
                res.is_none().then(|| -> Option<()> {
                    let mut all: Vec<Location> = vec![];
                    for file in &self.variables {
                        if file.0 == &uri {
                            continue;
                        }
                        let variables = file.1.iter().filter(|item| item.name == current_ident);
                        for variable in variables {
                            let uri = Url::parse(file.0).unwrap();
                            let start = to_position2(variable.start);
                            let end = to_position2(variable.end);
                            let range = Range::new(start, end);
                            let location = Location::new(uri, range);
                            all.push(location);
                        }
                    }
                    res = Some(GotoDefinitionResponse::Array(all));
                    None
                });
                res
            }

            LangType::Backend => {
                let query = &self.queries.rust_templates;
                let templates = rust_templates_query(query, tree, point, &writter.content, false);
                let template = templates.in_template(point)?;
                let dir = &self.config.templates;
                let path = format!("{dir}/{}", template.name);
                let buffer = std::fs::canonicalize(path).ok()?;
                let url = format!("file://{}", buffer.to_str()?);
                let url = Url::parse(&url).ok()?;
                let start = to_position2(template.start);
                let end = to_position2(template.end);
                let range = Range::new(start, end);
                let location = Location::new(url, range);
                Some(GotoDefinitionResponse::Scalar(location))
            }
        }
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
        let query = &self.queries.jinja_objects;
        let doc = self.documents.get(&uri)?;
        let mut writter = FileWriter::default();
        let _ = doc.write_to(&mut writter);
        let objects = objects_query(query, tree, point, &writter.content, false);
        Some(objects.in_expr(point))
    }

    pub fn read_trees(&self, diags: &mut HashMap<String, Vec<Diagnostic>>) {
        for tree in self.trees.get(&LangType::Template).unwrap() {
            let errors = self.read_tree(tree.0);
            if let Some(errors) = errors {
                diags.insert(String::from(tree.0), errors);
            }
        }
    }

    pub fn read_variables(&self, uri: &Url, position: Position) -> Option<Vec<CompletionItem>> {
        let mut items = vec![];
        let start = position.line as usize;
        let end = position.character as usize;
        let position = Point::new(start, end);
        let uri = &uri.to_string();
        let mut names = HashSet::new();
        let this_file = self.variables.get(uri)?;
        let this_file = this_file
            .iter()
            .filter(|variable| {
                variable.identifier_type != IdentifierType::TemplateBlock
                    && variable.identifier_type != IdentifierType::JinjaTemplate
            })
            .filter(|variable| {
                let bigger = position >= variable.end;
                let in_scope = position <= variable.scope_ends.1;
                bigger && in_scope
            });
        for identifier in this_file {
            if !names.contains(&identifier.name) {
                names.insert(&identifier.name);
                items.push(CompletionItem {
                    label: identifier.name.to_string(),
                    detail: Some(identifier.identifier_type.completion_detail().to_owned()),
                    kind: Some(identifier.identifier_type.completion_kind()),
                    ..Default::default() // detail: Some()
                });
            }
        }
        for file in self.variables.iter() {
            for variable in file.1 {
                if variable.identifier_type == IdentifierType::BackendVariable {
                    items.push(CompletionItem {
                        label: variable.name.to_string(),
                        detail: Some(variable.identifier_type.completion_detail().to_owned()),
                        kind: Some(variable.identifier_type.completion_kind()),
                        ..Default::default() // detail: Some()
                    });
                }
            }
        }
        Some(items)
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
            let text_edit = {
                if self.is_vscode {
                    None
                } else {
                    Some(CompletionTextEdit::Edit(TextEdit::new(range, new_text)))
                }
            };
            let item = CompletionItem {
                label,
                detail: Some("Jinja template".to_string()),
                kind: Some(CompletionItemKind::FILE),
                text_edit,
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
        let msg = DiagnosticMessage::Errors(hm);
        Some(msg)
    }

    pub fn data_type(&self, uri: Url, hover: Identifier) -> Option<IdentifierType> {
        let this_file = self.variables.get(&uri.as_str().to_string())?;
        let this_file = this_file
            .iter()
            .filter(|variable| variable.identifier_type != IdentifierType::TemplateBlock)
            .filter(|variable| {
                let bigger = hover.start >= variable.end;
                let in_scope = hover.start <= variable.scope_ends.1;
                let same_name = hover.name == variable.name;
                bigger && in_scope && same_name
            })
            .max()?;
        Some(this_file.identifier_type.clone())
    }

    pub fn document_symbols(
        &self,
        params: tower_lsp::lsp_types::DocumentSymbolParams,
    ) -> Option<DocumentSymbolResponse> {
        let mut symbols = vec![];
        let variables = self.variables.get(params.text_document.uri.as_str())?;
        for variable in variables {
            #[allow(deprecated)]
            let symbol = DocumentSymbol {
                name: variable.name.to_owned(),
                detail: None,
                kind: variable.identifier_type.symbol_kind(),
                range: to_range((variable.start, variable.end)),
                selection_range: to_range((variable.start, variable.end)),
                children: None,
                deprecated: None,
                tags: None,
            };
            symbols.push(symbol);
        }
        Some(DocumentSymbolResponse::Nested(symbols))
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
            queries: Queries::default(),
            documents: HashMap::new(),
            config: JinjaConfig::default(),
            diagnostics_task,
            main_channel,
            variables: HashMap::default(),
            is_vscode: false,
        }
    }
}

#[derive(Default, Debug)]
pub struct FileContent {
    pub content: String,
}

impl std::io::Write for FileContent {
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
