use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use ropey::Rope;

use crate::lsp_files::LspFiles;

pub type Doc = DashMap<String, Rope>;
pub type Lsp = Arc<Mutex<LspFiles>>;
// pub type LspFiles = Arc<Mutex<LspFiles>>;
