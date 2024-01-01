use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex, RwLock},
};

use anyhow::Error;
use dashmap::DashMap;
use ropey::Rope;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use walkdir::WalkDir;

use crate::{
    lsp_files::{JinjaDiagnostic, JinjaVariable, LspFiles},
    query_helper::Queries,
};

/// Jinja configuration
/// `templates` can be absolute and relative path
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct JinjaConfig {
    templates: String,
    backend: Vec<String>,
    lang: String,
    #[serde(skip)]
    pub user_defined: bool,
}

impl JinjaConfig {
    pub fn file_ext(&self, path: &&Path) -> Option<LangType> {
        match path.extension()?.to_str() {
            Some(e) => match e {
                "html" | "jinja" | "j2" => Some(LangType::Template),
                "rs" => Some(LangType::Backend),
                _ => None,
            },
            None => None,
        }
    }

    pub fn user_defined(&mut self, def: bool) -> Option<()> {
        self.user_defined = def;
        None
    }
}

pub fn config_exist(config: Option<Value>) -> Option<JinjaConfig> {
    let config = config?;
    if let Ok(mut config) = serde_json::from_value::<JinjaConfig>(config) {
        config.user_defined = true;
        return Some(config);
    }
    None
}

pub fn read_config(
    config: &RwLock<JinjaConfig>,
    lsp_files: &Arc<Mutex<LspFiles>>,
    queries: &Arc<Mutex<Queries>>,
    document_map: &DashMap<String, Rope>,
) -> anyhow::Result<HashMap<String, Vec<(JinjaVariable, JinjaDiagnostic)>>> {
    if let Ok(config) = config.read() {
        if !config.user_defined {
            return Err(Error::msg("Config doesn't exist"));
        }
        if config.templates.is_empty() {
            return Err(Error::msg("Template directory not found"));
        }
        if !is_backend(&config.lang) {
            Err(Error::msg("Backend language not supported"))
        } else {
            walkdir(&config, lsp_files, queries, document_map)
        }
    } else {
        Err(Error::msg("Config doesn't exist"))
    }
}

pub fn walkdir(
    config: &JinjaConfig,
    lsp_files: &Arc<Mutex<LspFiles>>,
    queries: &Arc<Mutex<Queries>>,
    document_map: &DashMap<String, Rope>,
) -> anyhow::Result<HashMap<String, Vec<(JinjaVariable, JinjaDiagnostic)>>> {
    let mut all = vec![config.templates.clone()];
    let mut backend = config.backend.clone();
    all.append(&mut backend);
    let mut diags = HashMap::new();
    for dir in all {
        let walk = WalkDir::new(dir);
        for (_index, entry) in walk.into_iter().enumerate() {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_file() {
                let path = &entry.path();
                let ext = config.file_ext(path);
                if let Some(ext) = ext {
                    // if ext == LangType::Backend {
                    //     continue;
                    // }
                    let _ = lsp_files.lock().is_ok_and(|lsp_files| {
                        lsp_files.read_file(path, ext, queries, document_map, &mut diags);
                        true
                    });
                }
            }
        }
    }
    let _ = lsp_files.lock().ok().and_then(|lsp_files| -> Option<()> {
        let trees = lsp_files.get_trees_vec(LangType::Template);
        for tree in trees {
            lsp_files.read_tree(tree, LangType::Template, queries, document_map, &mut diags);
        }
        None
    });

    Ok(diags)
}

fn is_backend(lang: &str) -> bool {
    lang == "rust"
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash)]
pub enum LangType {
    Template,
    Backend,
}
