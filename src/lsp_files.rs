use std::{
    cell::RefCell,
    collections::HashMap,
    fs::read_to_string,
    path::Path,
    sync::{Arc, Mutex, MutexGuard, RwLock},
};

use dashmap::DashMap;
use ropey::Rope;
use tower_lsp::lsp_types::Position;
use tree_sitter::{InputEdit, Point, Range, Tree};

use crate::{
    capturer::{JinjaCapturer, RustCapturer},
    config::{JinjaConfig, LangType},
    parsers::Parsers,
    query_helper::{
        query_action, query_completion, query_definition, query_hover, query_identifiers,
        query_props, CaptureDetails, CompletionType, Queries, QueryType,
    },
    server::FileWriter,
};

#[derive(Clone)]
pub struct LspFiles {
    current: RefCell<usize>,
    indexes: DashMap<String, usize>,
    trees: DashMap<LangType, DashMap<usize, Tree>>,
    pub parsers: Arc<Mutex<Parsers>>,
    pub variables: DashMap<usize, Vec<JinjaVariable>>,
}

impl Default for LspFiles {
    fn default() -> Self {
        let trees = DashMap::new();
        trees.insert(LangType::Template, DashMap::new());
        trees.insert(LangType::Backend, DashMap::new());
        Self {
            current: RefCell::new(0),
            indexes: DashMap::new(),
            trees,
            parsers: Arc::new(Mutex::new(Parsers::default())),
            variables: DashMap::new(),
        }
    }
}

impl LspFiles {
    pub fn reset(&self) {
        self.indexes.clear();
        self.trees.clear();
    }

    pub fn add_file(&self, key: String) -> Option<usize> {
        if self.get_index(&key).is_none() {
            let old = self.current.replace_with(|&mut old| old + 1);
            self.indexes.insert(key, old);
            return Some(old);
        }
        None
    }

    pub fn get_index(&self, key: &String) -> Option<usize> {
        if let Some(d) = self.indexes.get(key) {
            let a = *d;
            return Some(a);
        }
        None
    }

    pub fn get_uri(&self, index: usize) -> Option<String> {
        self.indexes.iter().find_map(|item| {
            if item.value() == &index {
                Some(String::from(item.key()))
            } else {
                None
            }
        })
    }

    pub fn add_tree(&self, index: usize, lang_type: LangType, text: &str, _range: Option<Range>) {
        self.parsers
            .lock()
            .ok()
            .and_then(|mut parsers| -> Option<()> {
                let old_trees = self.trees.get_mut(&lang_type).unwrap();
                if let Some(old_tree) = old_trees.get_mut(&index) {
                    if let Some(tree) = parsers.parse(lang_type, text, Some(&old_tree)) {
                        drop(old_tree);
                        old_trees.insert(index, tree);
                    }
                } else {
                    // tree doesn't exist, first insertion
                    if let Some(tree) = parsers.parse(lang_type, text, None) {
                        old_trees.insert(index, tree);
                    }
                }
                None
            });
    }

    pub fn get_trees_vec(&self, lang_type: LangType) -> Vec<usize> {
        let trees = self.trees.get(&lang_type).unwrap();
        let mut all = vec![];
        for tree in trees.iter().enumerate() {
            all.push(tree.0);
        }
        all
    }

    pub fn read_tree(
        &self,
        index: usize,
        _lang_type: LangType,
        queries: &Arc<Mutex<Queries>>,
        document_map: &DashMap<String, Rope>,
        diags: &mut HashMap<String, Vec<(JinjaVariable, JinjaDiagnostic)>>,
    ) -> Option<()> {
        let uri = self.get_uri(index)?;
        let rope = document_map.get(&uri)?;
        let content = rope.value();
        let mut writter = FileWriter::default();
        let _ = content.write_to(&mut writter);
        let content = writter.content;
        queries.lock().ok().and_then(|queries| -> Option<()> {
            let trees = self.trees.get(&LangType::Template)?;
            let tree = trees.get(&index)?;
            let closest_node = tree.root_node();
            let trigger_point = Point::new(0, 0);
            query_identifiers(
                closest_node,
                &content,
                trigger_point,
                &queries,
                &self.variables,
                (index, uri),
                diags,
            );
            None
        });
        None
    }

    pub fn warn_undefined(
        &self,
        uri: &String,
        _config: &RwLock<JinjaConfig>,
        document_map: &DashMap<String, Rope>,
        queries: &Arc<Mutex<Queries>>,
        diags: &mut HashMap<String, Vec<(JinjaVariable, JinjaDiagnostic)>>,
    ) -> Option<()> {
        let index = self.get_index(uri)?;
        self.read_tree(index, LangType::Template, queries, document_map, diags);
        None
    }

    pub fn read_file(
        &self,
        path: &&Path,
        lang_type: LangType,
        queries: &Arc<Mutex<Queries>>,
        document_map: &DashMap<String, Rope>,
        diags: &mut HashMap<String, Vec<(JinjaVariable, JinjaDiagnostic)>>,
    ) -> Option<()> {
        let res = None;
        let mut errors = None;
        let mut index = String::new();
        if let Ok(name) = std::fs::canonicalize(path) {
            let name = name.to_str()?;
            let file = self.add_file(format!("file://{}", name))?;
            let mut file_content = String::new();
            read_to_string(name).ok().and_then(|content| -> Option<()> {
                file_content = content;
                let rope = ropey::Rope::from_str(&file_content);
                document_map.insert(format!("file://{}", name).to_string(), rope);
                self.add_tree(file, lang_type, &file_content, None);
                None
            });
            let _ = queries.lock().ok().and_then(|query| -> Option<()> {
                self.delete_variables(file);
                errors = self.add_variables(file, lang_type, &file_content, &query);
                index.push_str("file://");
                index.push_str(name);
                None
            });
        }
        let errors = errors?;
        diags.insert(index, errors);
        res
    }

    pub fn input_edit(
        &self,
        file: &String,
        code: String,
        input_edit: InputEdit,
        lang_type: Option<LangType>,
    ) -> Option<()> {
        let lang_type = lang_type?;
        let file = self.get_index(file)?;
        self.parsers
            .lock()
            .ok()
            .and_then(|parsers| self.edit_old_tree(file, lang_type, input_edit, parsers, code))
    }
    pub fn edit_old_tree(
        &self,
        index: usize,
        lang_type: LangType,
        input_edit: InputEdit,
        mut parsers: MutexGuard<Parsers>,
        code: String,
    ) -> Option<()> {
        let trees = self.trees.get_mut(&lang_type)?;
        let mut old_tree = trees.get_mut(&index)?;
        old_tree.edit(&input_edit);
        let new_tree = parsers.parse(lang_type, &code, Some(&old_tree))?;
        drop(old_tree);
        drop(trees);
        let trees = self.trees.get_mut(&lang_type)?;
        trees.insert(index, new_tree);
        None
    }

    pub fn query_completion(
        &self,
        index: usize,
        text: &str,
        _query_type: QueryType,
        pos: Position,
        query: &Queries,
    ) -> Option<CompletionType> {
        let trees = self.trees.get(&LangType::Template)?;
        let old_tree = trees.get(&index)?;
        let root_node = old_tree.root_node();
        let trigger_point = Point::new(pos.line as usize, pos.character as usize);
        query_completion(root_node, text, trigger_point, query)
    }

    pub fn query_something(
        &self,
        index: usize,
        text: &str,
        query_type: QueryType,
        pos: Position,
        query: &Queries,
    ) -> Option<String> {
        let trees = self.trees.get(&LangType::Template)?;
        let old_tree = trees.get(&index)?;
        let root_node = old_tree.root_node();
        let trigger_point = Point::new(pos.line as usize, pos.character as usize);
        match query_type {
            QueryType::Definition => query_definition(root_node, text, trigger_point, query),
            QueryType::CodeAction => query_action(root_node, text, trigger_point, query),
            QueryType::Hover => query_hover(root_node, text, trigger_point, query),
            QueryType::Completion => None,
        }
    }

    pub fn delete_variables(&self, index: usize) {
        self.variables.remove(&index);
    }

    pub fn add_variables(
        &self,
        index: usize,
        lang_type: LangType,
        text: &str,
        query: &Queries,
    ) -> Option<Vec<(JinjaVariable, JinjaDiagnostic)>> {
        let trees = self.trees.get(&lang_type).unwrap();
        let tree = trees.get(&index)?;
        let trigger_point = Point::new(0, 0);
        let closest_node = tree.root_node();
        let mut diags = vec![];
        match lang_type {
            LangType::Backend => {
                let query = &query.rust_ident_query;
                let mut capturer = RustCapturer::default();
                capturer.force();
                let mut variables = vec![];
                let props = query_props(closest_node, text, trigger_point, query, true, capturer);
                let mut props: Vec<&CaptureDetails> = props.values().collect();
                props.sort();
                for capture in props {
                    variables.push(JinjaVariable::from(capture));
                }
                self.variables.insert(index, variables);
            }
            LangType::Template => {
                let query = &query.jinja_ident_query;
                let capturer = JinjaCapturer::default();
                let mut variables = vec![];
                let props = query_props(closest_node, text, trigger_point, query, true, capturer);
                let mut props: Vec<&CaptureDetails> = props.values().collect();
                props.sort();
                for capture in props {
                    let variable = JinjaVariable::from(capture);
                    if !ident_exist(&capture.value, &variables) {
                        variables.push(variable);
                    } else {
                        diags.push((variable, JinjaDiagnostic::AlreadyDefined));
                    }
                }
                self.variables.insert(index, variables);
            }
        }
        Some(diags)
    }

    pub fn saved(
        &self,
        uri: &String,
        config: &RwLock<JinjaConfig>,
        document_map: &DashMap<String, Rope>,
        queries: &Arc<Mutex<Queries>>,
        diags: &mut HashMap<String, Vec<(JinjaVariable, JinjaDiagnostic)>>,
    ) -> Option<()> {
        let path = Path::new(&uri);
        let file = self.get_index(uri)?;
        let res = None;
        if let Ok(config) = config.read() {
            let lang_type = config.file_ext(&path)?;
            let content = document_map.get(uri)?;
            let content = content.value();
            let mut a = FileWriter::default();
            let _ = content.write_to(&mut a);
            let content = a.content;
            let _ = queries.lock().is_ok_and(|queries| {
                self.delete_variables(file);
                let errors = self.add_variables(file, lang_type, &content, &queries);
                if let Some(errors) = errors {
                    diags.insert(uri.to_string(), errors);
                }
                true
            });
        }
        res
    }

    pub fn get_jinja_variables(
        &self,
        name: &str,
    ) -> Option<HashMap<String, std::vec::Vec<JinjaVariable>>> {
        let mut all = HashMap::new();
        let mut added = false;
        for i in self.variables.iter() {
            let variables = get_jinja_variables(name, true, &i);
            if !variables.is_empty() {
                added = true;
            }
            let index = self.get_uri(i.key().to_owned())?;
            all.insert(index, variables);
        }
        if added {
            Some(all)
        } else {
            None
        }
    }

    pub fn get_all_variables(&self, current_file: &String) -> Option<Vec<(String, String)>> {
        let mut all = vec![];
        let index = self.get_index(current_file)?;
        let variables = self.variables.get(&index)?;
        if variables.is_empty() {
            return None;
        }
        for i in variables.value() {
            all.push(("this file".to_string(), i.name.to_string()));
        }
        for i in self.variables.iter() {
            for variable in i.value() {
                all.push(("other file".to_string(), variable.name.to_string()));
            }
        }
        Some(all)
    }
}

#[derive(Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub struct JinjaVariable {
    pub start: Point,
    pub end: Point,
    pub name: String,
}

impl From<&CaptureDetails> for JinjaVariable {
    fn from(value: &CaptureDetails) -> Self {
        Self {
            name: String::from(&value.value),
            start: value.start_position,
            end: value.end_position,
        }
    }
}

pub fn get_jinja_variables(
    name: &str,
    same: bool,
    variables: &Vec<JinjaVariable>,
) -> Vec<JinjaVariable> {
    let mut new = vec![];
    for variable in variables {
        if variable.name == name {
            new.push(variable.clone());
            if !same {
                break;
            }
        }
    }
    new
}

/// Used only by jinja variables
pub fn ident_exist(name: &str, variables: &[JinjaVariable]) -> bool {
    let ident = variables.iter().find(|item| item.name == name);
    ident.is_some()
}

pub enum JinjaDiagnostic {
    AlreadyDefined,
    DefinedSomewhere,
    Undefined,
}

impl ToString for JinjaDiagnostic {
    fn to_string(&self) -> String {
        match self {
            JinjaDiagnostic::AlreadyDefined => String::from("This variable is already defined"),
            JinjaDiagnostic::Undefined => String::from("Undefined variable"),
            JinjaDiagnostic::DefinedSomewhere => String::from("Variable is defined in other file."),
        }
    }
}
