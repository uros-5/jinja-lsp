use std::{
    collections::HashMap,
    fs::read_to_string,
    path::{Path, PathBuf},
};

use jinja_lsp_queries::{
    search::Identifier,
    tree_builder::{JinjaDiagnostic, LangType},
};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::lsp_files::LspFiles;
use clap::Parser;

/// Jinja configuration
/// `templates` can be absolute and relative path
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct JinjaConfig {
    pub templates: PathBuf,
    pub backend: Vec<String>,
    pub lang: String,
    #[serde(skip)]
    pub user_defined: bool,
    pub hide_undefined: Option<bool>,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct ExternalConfig {
    #[serde(rename(deserialize = "jinja-lsp"))]
    jinja_lsp: JinjaConfig,
}

pub fn search_config() -> Option<JinjaConfig> {
    let configs = ["pyproject.toml", "Cargo.toml", "jinja-lsp.toml"];
    for config in configs {
        let contents = read_to_string(config).unwrap_or_default();
        if contents.is_empty() {
            continue;
        }
        let config = toml::from_str::<ExternalConfig>(&contents);
        if let Ok(mut config) = config {
            config.jinja_lsp.user_defined = true;
            return Some(config.jinja_lsp);
        }
    }
    None
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
    lsp_files.ignore_globals = config.hide_undefined.unwrap_or(false);
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

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct JinjaLspArgs {
    /// Run language server.
    #[arg(long)]
    pub stdio: bool,
}
