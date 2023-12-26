use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::read_to_string,
    path::Path,
    sync::{Arc, Mutex, MutexGuard, RwLock},
};

use anyhow::Error;
use dashmap::DashMap;
use ropey::Rope;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use walkdir::WalkDir;

use crate::{
    lsp_files::{JinjaVariable, LspFiles},
    query_helper::Queries,
};

/// Jinja configuration
/// `templates` can be absolute and relative path
#[derive(Serialize, Deserialize, Debug)]
pub struct JinjaConfig {
    templates: String,
    backend: Vec<String>,
    lang: String,
}

impl JinjaConfig {
    pub fn file_ext(&self, path: &&Path) -> Option<LangType> {
        match path.extension()?.to_str() {
            Some(e) => match e {
                "html" | "jinja" | "j2" => Some(LangType::Template),
                backend if is_backend(backend) => Some(LangType::Backend),
                _ => None,
            },
            None => None,
        }
    }
}

pub fn config_exist(config: Option<Value>) -> Option<JinjaConfig> {
    let config = config?;
    if let Ok(config) = serde_json::from_value::<JinjaConfig>(config) {
        return Some(config);
    }
    None
}

pub fn read_config(
    config: &RwLock<Option<JinjaConfig>>,
    lsp_files: &Arc<Mutex<LspFiles>>,
    queries: &Arc<Mutex<Queries>>,
    document_map: &DashMap<String, Rope>,
) -> anyhow::Result<HashMap<String, Vec<JinjaVariable>>> {
    if let Ok(config) = config.read() {
        if let Some(config) = config.as_ref() {
            if config.templates.is_empty() {
                return Err(Error::msg("Template directory not found"));
            }
            if !is_backend(&config.lang) {
                Err(Error::msg("Backend language not supported"))
            } else {
                walkdir(config, lsp_files, queries, document_map)
            }
        } else {
            Err(Error::msg("Config doesn't exist"))
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
) -> anyhow::Result<HashMap<String, Vec<JinjaVariable>>> {
    let templates = WalkDir::new(&config.templates);
    let mut diags = HashMap::new();
    for (index, entry) in templates.into_iter().enumerate() {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_file() {
            let path = &entry.path();
            let ext = config.file_ext(path);
            if let Some(ext) = ext {
                if ext == LangType::Backend {
                    continue;
                }
                let _ = lsp_files.lock().is_ok_and(|lsp_files| {
                    lsp_files.read_file(path, ext, queries, document_map, &mut diags);
                    true
                });
            }
        }
    }

    Ok(diags)
}

fn is_backend(lang: &str) -> bool {
    lang == "rust"
}

fn add_file(
    path: &&Path,
    lsp_files: &MutexGuard<LspFiles>,
    lang_type: LangType,
    queries: &Queries,
    _skip: bool,
    document_map: &DashMap<String, Rope>,
) -> Option<()> {
    if let Ok(name) = std::fs::canonicalize(path) {
        let name = name.to_str()?;
        let file = lsp_files.add_file(format!("file://{}", name))?;
        let _ = read_to_string(name).is_ok_and(|content| {
            let rope = ropey::Rope::from_str(&content);
            document_map.insert(format!("file://{}", name).to_string(), rope);
            lsp_files.add_tree(file, lang_type, &content, None);
            lsp_files.add_variables(file, lang_type, &content, queries);
            // let _ = lsp_files.add_tags_from_file(file, lang_type, &content, false, queries, diags);
            true
        });
    }
    None
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash)]
pub enum LangType {
    Template,
    Backend,
}
