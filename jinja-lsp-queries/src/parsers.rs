use tree_sitter::{Parser, Tree};

use crate::tree_builder::LangType;

pub struct Parsers {
    jinja: Parser,
    backend: Parser,
}

impl Parsers {
    pub fn parse(
        &mut self,
        lang_type: LangType,
        text: &str,
        old_tree: Option<&Tree>,
    ) -> Option<Tree> {
        match lang_type {
            LangType::Template => self.jinja.parse(text, old_tree),
            LangType::Backend => self.backend.parse(text, old_tree),
        }
    }

    pub fn update_backend(&mut self, lang: &str) {
        if lang == "python" {
            self.backend = Parser::new();
            let _ = self.backend.set_language(tree_sitter_python::language());
        }
    }
}

impl Default for Parsers {
    fn default() -> Self {
        let mut jinja = Parser::new();
        let _ = jinja.set_language(tree_sitter_jinja2::language());
        let mut backend = Parser::new();
        let _ = backend.set_language(tree_sitter_rust::language());
        Self { jinja, backend }
    }
}

impl Clone for Parsers {
    fn clone(&self) -> Self {
        Self::default()
    }
}
