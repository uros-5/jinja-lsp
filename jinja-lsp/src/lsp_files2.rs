use std::{collections::HashMap, fs::read_to_string, path::Path};

use jinja_lsp_queries::{
    parsers::Parsers,
    queries::Queries,
    tree_builder::{JinjaVariable, LangType},
};
use ropey::Rope;
use tree_sitter::Tree;

pub struct LspFiles2 {
    trees: HashMap<LangType, HashMap<String, Tree>>,
    documents: HashMap<String, Rope>,
    pub parsers: Parsers,
    pub variables: HashMap<String, Vec<JinjaVariable>>,
    pub queries: Queries,
}

impl LspFiles2 {
    pub fn read_file(&mut self, path: &&Path, lang_type: LangType) -> Option<()> {
        if let Ok(name) = std::fs::canonicalize(path) {
            let name = name.to_str()?;
            let file_content = read_to_string(name).ok()?;
            let rope = Rope::from_str(&file_content);
            let name = format!("file://{}", name);
            self.documents.insert(name.to_string(), rope);
            // self.add_tree(&name, lang_type, &file_content);

            // let _ = self.queries.lock().ok().and_then(|query| -> Option<()> {
            //     let name = format!("file://{}", name);
            //     self.delete_variables(&name);
            //     errors = self.add_variables(&name, lang_type, &file_content, &query);
            //     None
            // });
        }
        None
    }

    pub fn add_tree(
        &mut self,
        file_name: &str,
        lang_type: LangType,
        file_content: &str,
    ) -> Option<()> {
        let trees = self.trees.get_mut(&lang_type)?;
        let old_tree = trees.get_mut(&file_name.to_string());
        match old_tree {
            Some(old_tree) => {
                let new_tree = self
                    .parsers
                    .parse(lang_type, file_content, Some(old_tree))?;
                trees.insert(file_name.to_string(), new_tree);
            }
            None => {
                // tree doesn't exist, first insertion
                let new_tree = self.parsers.parse(lang_type, file_content, None)?;
                trees.insert(file_name.to_string(), new_tree);
            }
        };
        None
    }
}
