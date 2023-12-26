use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::HashMap,
    fs::read_to_string,
    path::Path,
    sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock},
};

use dashmap::{mapref::one::RefMut, DashMap};
use ropey::Rope;
use tower_lsp::lsp_types::Position;
use tree_sitter::{InputEdit, Point, Range, Tree};

use crate::{
    capturer::JinjaCapturer,
    config::{JinjaConfig, LangType},
    parsers::Parsers,
    query_helper::{
        query_action, query_completion, query_definition, query_hover, query_ident, query_props,
        CaptureDetails, CompletionType, Queries, QueryType,
    },
    server::LocalWriter,
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
        let _ = self.parsers.lock().is_ok_and(|mut parsers| {
            let mut old_trees = self.trees.get_mut(&lang_type).unwrap();
            if let Some(old_tree) = old_trees.get_mut(&index) {
                if let Some(tree) = parsers.parse(lang_type, text, Some(&old_tree)) {
                    let lang = lang_type;
                    drop(old_tree);
                    old_trees.insert(index, tree);
                }
            } else {
                // tree doesn't exist, first insertion
                if let Some(tree) = parsers.parse(lang_type, text, None) {
                    old_trees.insert(index, tree);
                }
            }

            true
        });
    }

    pub fn read_file(
        &self,
        path: &&Path,
        lang_type: LangType,
        queries: &Arc<Mutex<Queries>>,
        document_map: &DashMap<String, Rope>,
        diags: &mut HashMap<String, Vec<JinjaVariable>>,
    ) -> Option<()> {
        let res = None;
        let mut errors = None;
        let mut index = String::new();
        if let Ok(name) = std::fs::canonicalize(path) {
            let name = name.to_str()?;
            let file = self.add_file(format!("file://{}", name))?;
            let _ = read_to_string(name).is_ok_and(|content| {
                let rope = ropey::Rope::from_str(&content);
                document_map.insert(format!("file://{}", name).to_string(), rope);
                self.add_tree(file, lang_type, &content, None);
                let _ = queries.lock().is_ok_and(|query| {
                    self.delete_variables(file);
                    errors = self.add_variables(file, lang_type, &content, &query);
                    index.push_str("file://");
                    index.push_str(name);
                    true
                });
                true
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
        let _ = self.parsers.lock().is_ok_and(|parsers| {
            self.edit_old_tree(file, lang_type, input_edit, parsers, code);
            true
        });

        None
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
        query_type: QueryType,
        pos: Position,
        query: &Queries,
    ) -> Option<CompletionType> {
        let trees = self.trees.get(&LangType::Template)?;
        let old_tree = trees.get(&index)?;
        let root_node = old_tree.root_node();
        let trigger_point = Point::new(pos.line as usize, pos.character as usize);
        query_completion(root_node, text, trigger_point, query)
    }

    pub fn query_hover(
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
        query_hover(root_node, text, trigger_point, query)
    }

    pub fn code_action(
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
        query_action(root_node, text, trigger_point, query)
    }

    pub fn query_definition(
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
        query_definition(root_node, text, trigger_point, query)
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
    ) -> Option<Vec<JinjaVariable>> {
        if lang_type == LangType::Backend {
            return None;
        }
        let trees = self.trees.get(&lang_type).unwrap();
        let tree = trees.get(&index)?;
        let trigger_point = Point::new(0, 0);
        let closest_node = tree.root_node();
        let query = &query.jinja_ident_query;
        let capturer = JinjaCapturer::default();
        let mut diags = vec![];
        let mut variables = vec![];
        let props = query_props(closest_node, text, trigger_point, query, true, capturer);
        let mut props: Vec<&CaptureDetails> = props.values().collect();
        props.sort();
        for capture in props {
            let variable = JinjaVariable::from(capture);
            if !ident_exist(&capture.value, &variables) {
                variables.push(variable);
            } else {
                diags.push(variable);
            }
        }
        self.variables.insert(index, variables);
        Some(diags)
    }

    pub fn saved(
        &self,
        uri: &String,
        config: &RwLock<Option<JinjaConfig>>,
        document_map: &DashMap<String, Rope>,
        queries: &Arc<Mutex<Queries>>,
        diags: &mut HashMap<String, Vec<JinjaVariable>>,
    ) -> Option<()> {
        let path = Path::new(&uri);
        let file = self.get_index(uri)?;
        let mut res = None;
        if let Ok(config) = config.read() {
            let config = config.as_ref()?;
            let lang_type = config.file_ext(&path)?;
            if lang_type == LangType::Backend {
                return None;
            }
            let content = document_map.get(uri)?;
            let content = content.value();
            let mut a = LocalWriter::default();
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

pub fn get_jinja_variable(
    name: &str,
    same: bool,
    variables: &Vec<JinjaVariable>,
) -> Option<Vec<JinjaVariable>> {
    let mut new = vec![];
    let mut res = None;
    for variable in variables {
        if variable.name == name {
            new.push(variable.clone());
            if !same {
                break;
            }
        }
    }
    if !variables.is_empty() {
        res = Some(new);
    }
    res
}

/// Used only by jinja variables
pub fn ident_exist(name: &str, variables: &Vec<JinjaVariable>) -> bool {
    let ident = variables.iter().find(|item| item.name == name);
    ident.is_some()
}
