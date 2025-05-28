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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JinjaConfig {
    pub templates: PathBuf,
    pub backend: Vec<String>,
    pub lang: String,
    #[serde(skip)]
    pub user_defined: bool,
    pub hide_undefined: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OptionalJinjaConfig {
    pub templates: Option<PathBuf>,
    pub backend: Option<Vec<String>>,
    #[serde(skip)]
    pub lang: Option<String>,
    #[serde(skip)]
    pub user_defined: Option<bool>,
    pub hide_undefined: Option<Option<bool>>,
}

impl Default for JinjaConfig {
    fn default() -> Self {
        Self {
            templates: PathBuf::from("./"),
            backend: vec![".".to_string()],
            lang: "python".to_string(),
            user_defined: false,
            hide_undefined: Some(false),
        }
    }
}

impl From<OptionalJinjaConfig> for JinjaConfig {
    fn from(value: OptionalJinjaConfig) -> Self {
        let mut config = Self::default();
        if let Some(templates) = value.templates {
            config.templates = templates;
        }
        if let Some(backend) = value.backend {
            config.backend = backend;
        }
        if let Some(lang) = value.lang {
            config.lang = lang;
        }
        if let Some(hide_undefined) = value.hide_undefined {
            config.hide_undefined = hide_undefined;
        }

        config
    }
}

pub fn search_config() -> Option<JinjaConfig> {
    let configs = [
        ("pyproject.toml", "tool", "python"),
        ("Cargo.toml", "metadata", "rust"),
        ("jinja-lsp.toml", "tool", "python"),
    ];
    for i in configs {
        let contents = read_to_string(i.0).unwrap_or_default();
        if contents.is_empty() {
            continue;
        }
        let config = get_config(&contents, i.1);
        if let Some(config) = config {
            let mut config = JinjaConfig::from(config);
            config.user_defined = true;
            config.lang = i.2.to_string();
            return Some(config);
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
    let mut all = vec![config
        .clone()
        .templates
        .to_str()
        .unwrap()
        .to_string()
        .clone()];
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

pub fn get_config(contents: &str, tools: &str) -> Option<OptionalJinjaConfig> {
    let toml_value: toml::Value = toml::from_str(contents).ok()?;
    let tools = toml_value.get(tools)?;
    let config = tools.get("jinja-lsp")?;
    let toml_value: OptionalJinjaConfig =
        toml::from_str(&toml::to_string_pretty(config).ok()?).ok()?;

    Some(toml_value)
}
