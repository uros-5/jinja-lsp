use std::{ffi::OsStr, path::Path};

use anyhow::Error;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use walkdir::WalkDir;

/// Jinja configuration
/// `templates` can be absolute and relative path
#[derive(Serialize, Deserialize, Debug)]
pub struct JinjaConfig {
    templates: String,
    backend: Vec<String>,
    lang: String,
}

impl JinjaConfig {
    fn file_ext(&self, path: &&Path) -> Option<LangType> {
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

pub fn read_config(config: &JinjaConfig) -> anyhow::Result<()> {
    if config.templates.is_empty() {
        return Err(Error::msg("Template directory not found"));
    }
    if !is_backend(&config.lang) {
        Err(Error::msg("Backend language not supported"))
    } else {
        walkdir(config)
    }
}

fn walkdir(config: &JinjaConfig) -> anyhow::Result<()> {
    let templates = WalkDir::new(&config.templates);
    for (index, entry) in templates.into_iter().enumerate() {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_file() {
            let path = &entry.path();
            let ext = config.file_ext(path);
            if let Some(ext) = ext {
                // match ext {
                //     LangType::Template => todo!(),
                //     LangType::Backend => todo!(),
                // }
            }
            // TODO create index for file
            // based on extension create tree for lang
            // add tree to LangType, (index, tree)
        }
    }

    Ok(())
}

fn is_backend(lang: &str) -> bool {
    lang == "rust"
}

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub enum LangType {
    Template,
    Backend,
}
