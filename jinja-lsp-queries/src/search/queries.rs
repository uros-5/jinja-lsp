use tree_sitter::Query;

#[derive(Debug)]
pub struct Queries {
    pub jinja_definitions: Query,
    pub jinja_objects: Query,
    pub jinja_imports: Query,
    pub backend_definitions: Query,
    pub backend_templates: Query,
    pub jinja_snippets: Query,
    pub python_identifiers: Query,
}

impl Clone for Queries {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl Default for Queries {
    fn default() -> Self {
        Self {
            jinja_definitions: Query::new(
                &tree_sitter_jinja2::LANGUAGE.into(),
                include_str!("./queries/jinja/definitions.scm"),
            )
            .unwrap(),
            jinja_objects: Query::new(
                &tree_sitter_jinja2::LANGUAGE.into(),
                include_str!("./queries/jinja/objects.scm"),
            )
            .unwrap(),
            jinja_imports: Query::new(
                &tree_sitter_jinja2::LANGUAGE.into(),
                include_str!("./queries/jinja/imports.scm"),
            )
            .unwrap(),
            jinja_snippets: Query::new(
                &tree_sitter_jinja2::LANGUAGE.into(),
                include_str!("./queries/jinja/snippets.scm"),
            )
            .unwrap(),
            backend_definitions: Query::new(
                &tree_sitter_rust::LANGUAGE.into(),
                include_str!("./queries/rust/definitions.scm"),
            )
            .unwrap(),
            backend_templates: Query::new(
                &tree_sitter_rust::LANGUAGE.into(),
                include_str!("./queries/rust/paths.scm"),
            )
            .unwrap(),
            python_identifiers: Query::new(
                &tree_sitter_python::LANGUAGE.into(),
                include_str!("./queries/python/node_js.scm"),
            )
            .unwrap(),
        }
    }
}

impl Queries {
    pub fn update_backend(&mut self, lang: &str) {
        if lang == "python" {
            self.backend_templates = Query::new(
                &tree_sitter_python::LANGUAGE.into(),
                include_str!("./queries/python/paths.scm"),
            )
            .unwrap();
            self.backend_definitions = Query::new(
                &tree_sitter_python::LANGUAGE.into(),
                include_str!("./queries/python/definitions.scm"),
            )
            .unwrap();
            self.python_identifiers = Query::new(
                &tree_sitter_python::LANGUAGE.into(),
                include_str!("./queries/python/node_js.scm"),
            )
            .unwrap();
        }
    }
}
