use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use jinja_lsp_queries::{
    search::Identifier,
    tree_builder::{JinjaDiagnostic, LangType},
};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::lsp_files::LspFiles;

/// Jinja configuration
/// `templates` can be absolute and relative path
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct JinjaConfig {
    pub templates: PathBuf,
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
                "rs" | "py" => Some(LangType::Backend),
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

pub type InitLsp = (
    HashMap<String, Vec<(JinjaDiagnostic, Identifier)>>,
    LspFiles,
);

pub fn walkdir(config: &JinjaConfig) -> anyhow::Result<InitLsp> {
    let mut all = vec![config.templates.to_str().unwrap().to_string().clone()];
    let mut backend = config.backend.clone();
    all.append(&mut backend);
    let mut lsp_files = LspFiles::default();
    lsp_files.config = config.clone();
    if config.lang == "python" {
        lsp_files.queries.update_backend(&config.lang);
        lsp_files.parsers.update_backend(&config.lang);
    }
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
