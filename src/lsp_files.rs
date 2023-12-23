use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

use dashmap::DashMap;
use tree_sitter::{Point, Range, Tree};

use crate::{config::LangType, parsers::Parsers};

#[derive(Clone)]
pub struct LspFiles {
    current: RefCell<usize>,
    indexes: DashMap<String, usize>,
    trees: DashMap<LangType, Tree>,
    pub parsers: Arc<Mutex<Parsers>>,
    pub symbols: DashMap<String, SymbolData>,
}

impl Default for LspFiles {
    fn default() -> Self {
        Self {
            current: RefCell::new(0),
            indexes: DashMap::new(),
            trees: DashMap::new(),
            parsers: Arc::new(Mutex::new(Parsers::default())),
            symbols: DashMap::new(),
        }
    }
}

#[derive(Clone)]
pub struct SymbolData {
    file: usize,
    start: Point,
    end: Point,
    name: String,
}
