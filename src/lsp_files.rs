use std::{
    cell::RefCell,
    collections::HashMap,
    fs::read_to_string,
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
};

use dashmap::{mapref::one::RefMut, DashMap};
use ropey::Rope;
use tree_sitter::{InputEdit, Point, Range, Tree};

use crate::{config::LangType, parsers::Parsers, query_helper::Queries};

#[derive(Clone)]
pub struct LspFiles {
    current: RefCell<usize>,
    indexes: DashMap<String, usize>,
    trees: DashMap<LangType, DashMap<usize, Tree>>,
    pub parsers: Arc<Mutex<Parsers>>,
    pub symbols: DashMap<String, SymbolData>,
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
            symbols: DashMap::new(),
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

    pub fn read_files(
        &self,
        path: &&Path,
        lang_type: LangType,
        queries: &Arc<Mutex<Queries>>,
        document_map: &DashMap<String, Rope>,
    ) -> Option<()> {
        if let Ok(name) = std::fs::canonicalize(path) {
            let name = name.to_str()?;
            let file = self.add_file(format!("file://{}", name))?;
            let _ = read_to_string(name).is_ok_and(|content| {
                let rope = ropey::Rope::from_str(&content);
                document_map.insert(format!("file://{}", name).to_string(), rope);
                self.add_tree(file, lang_type, &content, None);
                true
            });
            // let _ = queries.lock().is_ok_and(|queries| {
            //         let _ = lsp_files
            //             .add_tags_from_file(file, lang_type, &content, false, queries, diags);
            //         true
            //     });
            // }
            //     true
            // });
        }
        None
    }

    pub fn input_edit(&self, file: &String, code: String, input_edit: InputEdit) -> Option<()> {
        let file = self.get_index(file)?;
        let _ = self.parsers.lock().is_ok_and(|parsers| {
            self.edit_old_tree(file, LangType::Template, input_edit, parsers, code);
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
}

#[derive(Clone)]
pub struct SymbolData {
    file: usize,
    start: Point,
    end: Point,
    name: String,
}
