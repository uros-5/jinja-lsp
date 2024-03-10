use std::{collections::HashMap, fs::read_to_string, path::Path};

use jinja_lsp_queries::{
    capturer::{
        included::IncludeCapturer,
        init::JinjaInitCapturer,
        object::{CompletionType, JinjaObjectCapturer},
        rust::RustCapturer,
    },
    lsp_helper::{search_errors, search_errors2},
    parsers::Parsers,
    queries::{query_props, Queries},
    to_input_edit::{to_position, to_position2},
    tree_builder::{DataType, JinjaDiagnostic, JinjaVariable, LangType},
};
use ropey::Rope;
use tower_lsp::lsp_types::{
    CodeActionParams, CompletionParams, GotoDefinitionParams, GotoDefinitionResponse, HoverParams,
    Location, Range, Url,
};
use tree_sitter::{InputEdit, Point, Tree};

use crate::{
    config::JinjaConfig,
    lsp_files::{add_variable_from_rust, FileWriter},
};

pub struct LspFiles2 {
    trees: HashMap<LangType, HashMap<String, Tree>>,
    documents: HashMap<String, Rope>,
    pub parsers: Parsers,
    pub variables: HashMap<String, Vec<JinjaVariable>>,
    pub queries: Queries,
}

impl LspFiles2 {
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
            // self.add_tree(&name, lang_type, &file_content);

            // let _ = self.queries.lock().ok().and_then(|query| -> Option<()> {
            //     let name = format!("file://{}", name);
            //     self.delete_variables(&name);
            //     None
            // });
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

    pub fn read_tree(
        &self,
        name: &str,
    ) -> Option<HashMap<String, Vec<(JinjaVariable, JinjaDiagnostic)>>> {
        let rope = self.documents.get(name)?;
        let mut writter = FileWriter::default();
        let _ = rope.write_to(&mut writter);
        let content = writter.content;
        let trees = self.trees.get(&LangType::Template)?;
        let tree = trees.get(name)?;
        let closest_node = tree.root_node();
        let mut diags = HashMap::new();
        search_errors2(
            closest_node,
            &content,
            &self.queries,
            &self.variables,
            &name.to_string(),
            &mut diags,
        );
        Some(diags)
    }

    pub fn did_save(&mut self, uri: &String, config: &JinjaConfig) -> Option<()> {
        let path = Path::new(&uri);
        let lang_type = config.file_ext(&path)?;
        let doc = self.documents.get(uri)?;
        let mut contents = FileWriter::default();
        let _ = doc.write_to(&mut contents);
        let content = contents.content;
        self.delete_variables(uri);
        self.add_variables(uri, lang_type, &content);
        None
    }

    pub fn completion(
        &self,
        params: CompletionParams,
        config: &JinjaConfig,
    ) -> Option<CompletionType> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let row = params.text_document_position.position.line;
        let column = params.text_document_position.position.character;
        let point = Point::new(row as usize, column as usize);
        let ext = config.file_ext(&Path::new(&uri))?;
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
        // TODO check for include capturer
        props.completion(point)
    }

    pub fn hover(&self, params: HoverParams) -> Option<String> {
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

    pub fn goto_definition(
        &self,
        params: GotoDefinitionParams,
        config: &JinjaConfig,
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
                let uri = last.is_template(&config.templates)?;
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
}

impl Default for LspFiles2 {
    fn default() -> Self {
        let mut trees = HashMap::new();
        trees.insert(LangType::Template, HashMap::new());
        trees.insert(LangType::Backend, HashMap::new());
        Self {
            trees,
            parsers: Parsers::default(),
            variables: HashMap::new(),
            queries: Queries::default(),
            documents: HashMap::new(),
        }
    }
}
