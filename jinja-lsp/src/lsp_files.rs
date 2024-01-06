use std::{
    collections::HashMap,
    fs::read_to_string,
    path::Path,
    sync::{Arc, Mutex, RwLock},
};

use dashmap::DashMap;
use ropey::Rope;
use tower_lsp::lsp_types::{
    CodeActionParams, CompletionItem, CompletionItemKind, CompletionParams, GotoDefinitionParams,
    GotoDefinitionResponse, HoverParams, Location, Position, Range, Url,
};
use tree_sitter::{InputEdit, Point, Tree};
use tree_sitter_queries::{
    capturer::{
        init::JinjaInitCapturer,
        object::{CompletionType, JinjaObjectCapturer},
        rust::RustCapturer,
    },
    lsp_helper::search_errors,
    parsers::Parsers,
    queries::{query_props, Queries},
    to_input_edit::to_position,
    tree_builder::{DataType, JinjaDiagnostic, JinjaVariable, LangType},
};

use crate::config::JinjaConfig;

#[derive(Clone)]
pub struct LspFiles {
    trees: DashMap<LangType, DashMap<String, Tree>>,
    pub parsers: Arc<Mutex<Parsers>>,
    pub variables: DashMap<String, Vec<JinjaVariable>>,
    pub queries: Arc<Mutex<Queries>>,
}

impl Default for LspFiles {
    fn default() -> Self {
        let trees = DashMap::new();
        trees.insert(LangType::Template, DashMap::new());
        trees.insert(LangType::Backend, DashMap::new());
        Self {
            trees,
            parsers: Arc::new(Mutex::new(Parsers::default())),
            variables: DashMap::new(),
            queries: Arc::new(Mutex::new(Queries::default())),
        }
    }
}

impl LspFiles {
    pub fn read_file(
        &self,
        path: &&Path,
        lang_type: LangType,
        document_map: &DashMap<String, Rope>,
        _diags: &mut HashMap<String, Vec<(JinjaVariable, JinjaDiagnostic)>>,
    ) -> Option<()> {
        let res = None;
        let mut errors = None;
        if let Ok(name) = std::fs::canonicalize(path) {
            let name = name.to_str()?;
            let mut file_content = String::new();
            read_to_string(name).ok().and_then(|content| -> Option<()> {
                file_content = content;
                let rope = ropey::Rope::from_str(&file_content);
                let name = format!("file://{}", name);
                document_map.insert(name.to_string(), rope);
                self.add_tree(&name, lang_type, &file_content);
                None
            });

            let _ = self.queries.lock().ok().and_then(|query| -> Option<()> {
                let name = format!("file://{}", name);
                self.delete_variables(&name);
                errors = self.add_variables(&name, lang_type, &file_content, &query);
                None
            });
        }
        res
    }

    pub fn read_tree(
        &self,
        document_map: &DashMap<String, Rope>,
        diags: &mut HashMap<String, Vec<(JinjaVariable, JinjaDiagnostic)>>,
        name: &str,
    ) -> Option<()> {
        let rope = document_map.get(name)?;
        let content = rope.value();
        let mut writter = FileWriter::default();
        let _ = content.write_to(&mut writter);
        let content = writter.content;
        self.queries.lock().ok().and_then(|query| -> Option<()> {
            let trees = self.trees.get(&LangType::Template)?;
            let tree = trees.get(name)?;
            let closest_node = tree.root_node();
            search_errors(
                closest_node,
                &content,
                &query,
                &self.variables,
                &name.to_string(),
                diags,
            )
        });
        None
    }

    pub fn get_trees_vec(&self, lang_type: LangType) -> Vec<String> {
        let trees = self.trees.get(&lang_type).unwrap();
        let mut all = vec![];
        for tree in trees.iter() {
            all.push(tree.key().to_string());
        }
        all
    }

    fn add_tree(&self, file_name: &str, lang_type: LangType, file_content: &str) {
        self.parsers
            .lock()
            .ok()
            .and_then(|mut parsers| -> Option<()> {
                let trees = self.trees.get_mut(&lang_type).unwrap();
                if let Some(old_tree) = trees.get_mut(&file_name.to_string()) {
                    if let Some(tree) = parsers.parse(lang_type, file_content, Some(&old_tree)) {
                        drop(old_tree);
                        trees.insert(file_name.to_string(), tree);
                    }
                } else {
                    // tree doesn't exist, first insertion
                    if let Some(tree) = parsers.parse(lang_type, file_content, None) {
                        trees.insert(file_name.to_string(), tree);
                    }
                }
                None
            });
    }

    fn delete_variables(&self, name: &str) {
        self.variables.remove(name);
    }

    fn add_variables(
        &self,
        name: &str,
        lang_type: LangType,
        file_content: &str,
        query: &std::sync::MutexGuard<'_, Queries>,
    ) -> Option<Vec<(JinjaVariable, JinjaDiagnostic)>> {
        let trees = self.trees.get(&lang_type).unwrap();
        let tree = trees.get(name)?;
        let trigger_point = Point::new(0, 0);
        let closest_node = tree.root_node();
        let diags = vec![];
        match lang_type {
            LangType::Backend => {
                let query = &query.rust_idents;
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
                add_variable_from_rust(capturer, &mut variables);
                self.variables.insert(name.to_string(), variables);
            }
            LangType::Template => {
                let query = &query.jinja_init;
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
        Some(diags)
    }

    pub fn input_edit(
        &self,
        file: &String,
        code: String,
        input_edit: InputEdit,
        lang_type: Option<LangType>,
    ) -> Option<()> {
        let lang_type = lang_type?;
        self.parsers.lock().ok().and_then(|mut parsers| {
            let trees = self.trees.get_mut(&lang_type)?;
            let mut old_tree = trees.get_mut(file)?;
            old_tree.edit(&input_edit);
            let new_tree = parsers.parse(lang_type, &code, Some(&old_tree))?;
            drop(old_tree);
            drop(trees);
            let trees = self.trees.get_mut(&lang_type)?;
            trees.insert(file.to_string(), new_tree);
            None
        })
    }

    pub fn saved(
        &self,
        uri: &String,
        config: &RwLock<JinjaConfig>,
        document_map: &DashMap<String, Rope>,
        diagnostics: &mut HashMap<String, Vec<(JinjaVariable, JinjaDiagnostic)>>,
    ) -> Option<()> {
        let path = Path::new(&uri);
        let res = None;
        if let Ok(config) = config.read() {
            let lang_type = config.file_ext(&path)?;
            let doc = document_map.get(uri)?;
            let content = doc.value();
            let mut contents = FileWriter::default();
            let _ = content.write_to(&mut contents);
            let content = contents.content;
            let _ = self.queries.lock().is_ok_and(|queries| {
                self.delete_variables(uri);
                let errors = self.add_variables(uri, lang_type, &content, &queries);
                if let Some(errors) = errors {
                    diagnostics.insert(uri.to_string(), errors);
                }
                true
            });
            if lang_type == LangType::Template {
                self.read_tree(document_map, diagnostics, uri);
            }
        }
        res
    }

    pub fn completion(
        &self,
        params: CompletionParams,
        config: &RwLock<JinjaConfig>,
        document_map: &DashMap<String, Rope>,
    ) -> Option<CompletionType> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let row = params.text_document_position.position.line;
        let column = params.text_document_position.position.character;
        let point = Point::new(row as usize, column as usize);
        let can_complete = config
            .read()
            .ok()
            .and_then(|config| config.file_ext(&Path::new(&uri)))
            .map_or(false, |lang_type| lang_type == LangType::Template);
        if !can_complete {
            None
        } else {
            let trees = self.trees.get(&LangType::Template)?;
            let tree = trees.get(&uri)?;
            let closest_node = tree.root_node();
            self.queries.lock().ok().and_then(|queries| {
                let query = &queries.jinja_idents;
                let capturer = JinjaObjectCapturer::default();
                let doc = document_map.get(&uri)?;
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
                props.completion(point)
            })
        }
    }

    pub fn hover(
        &self,
        params: HoverParams,
        document_map: &DashMap<String, Rope>,
    ) -> Option<String> {
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

        let hover = self.queries.lock().ok().and_then(|queries| {
            let query = &queries.jinja_idents;
            let capturer = JinjaObjectCapturer::default();
            let doc = document_map.get(&uri)?;
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
        });

        hover
    }

    pub fn goto_definition(
        &self,
        params: GotoDefinitionParams,
        document_map: &DashMap<String, Rope>,
    ) -> Option<GotoDefinitionResponse> {
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

        let mut res = self
            .queries
            .lock()
            .ok()
            .and_then(|queries| {
                let query = &queries.jinja_idents;
                let capturer = JinjaObjectCapturer::default();
                let doc = document_map.get(&uri)?;
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
                props.is_ident(point)
            })
            .and_then(|ident| {
                current_ident = ident.to_string();
                let variables = self.variables.get(&uri)?;
                let max = variables
                    .value()
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
            if current_ident.is_empty() {
                return None;
            }
            let mut all: Vec<Location> = vec![];
            for i in &self.variables {
                let idents = i.value().iter().filter(|item| item.name == current_ident);
                for id in idents {
                    let uri = Url::parse(i.key()).unwrap();
                    let (start, end) = to_position(id);
                    let range = Range::new(start, end);
                    let location = Location { uri, range };
                    all.push(location);
                }
            }
            res = Some(GotoDefinitionResponse::Array(all));
            None
        });

        res
    }

    pub fn code_action(
        &self,
        action_params: CodeActionParams,
        document_map: &DashMap<String, Rope>,
    ) -> Option<bool> {
        let uri = action_params.text_document.uri.to_string();
        let row = action_params.range.start.line;
        let column = action_params.range.start.character;
        let point = Point::new(row as usize, column as usize);
        let trees = self.trees.get(&LangType::Template)?;
        let tree = trees.get(&uri)?;
        let closest_node = tree.root_node();
        let _current_ident = String::new();
        self.queries.lock().ok().and_then(|queries| {
            let query = &queries.jinja_idents;
            let capturer = JinjaObjectCapturer::default();
            let doc = document_map.get(&uri)?;
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
        })
    }

    pub fn get_variables(
        &self,
        uri: &Url,
        _document_map: &DashMap<String, Rope>,
        _lang_type: LangType,
        position: Position,
    ) -> Option<Vec<CompletionItem>> {
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
        drop(variables);
        for file in self.variables.iter() {
            for variable in file.value() {
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
}

pub fn add_variable_from_rust(rust: RustCapturer, variables: &mut Vec<JinjaVariable>) {
    for variable in rust.variables() {
        variables.push(JinjaVariable::new(
            &variable.0,
            variable.1,
            DataType::BackendVariable,
        ));
    }
    for macros in rust.macros() {
        for variable in macros.1.variables() {
            variables.push(JinjaVariable::new(
                variable.0,
                *variable.1,
                DataType::BackendVariable,
            ));
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
    }
}
