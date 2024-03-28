use std::{collections::HashMap, path::Path};

use jinja_lsp_queries::tree_builder::LangType;
use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::Diagnostic;
use walkdir::WalkDir;

use crate::lsp_files2::LspFiles2;

/// Jinja configuration
/// `templates` can be absolute and relative path
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct JinjaConfig {
    pub templates: String,
    pub backend: Vec<String>,
    pub lang: String,
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

pub type InitLsp = (HashMap<String, Vec<Diagnostic>>, LspFiles2);

pub fn walkdir(config: &JinjaConfig) -> anyhow::Result<InitLsp> {
    let mut all = vec![config.templates.clone()];
    let mut backend = config.backend.clone();
    all.append(&mut backend);
    let mut lsp_files = LspFiles2::default();
    let mut diags = HashMap::new();
    for dir in all {
        let walk = WalkDir::new(dir);
        for entry in walk.into_iter() {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_file() {
                let path = &entry.path();
                let ext = config.file_ext(path);
                if let Some(ext) = ext {
                    lsp_files.read_file(path, ext);
                }
            }
        }
    }

    lsp_files.read_trees(&mut diags);
    Ok((diags, lsp_files))
}
