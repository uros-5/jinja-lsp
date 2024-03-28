use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};
use tree_sitter::{Point, Query, Tree};

use crate::search::{
    rust_identifiers::rust_definition_query, rust_template_completion::rust_templates_query,
};

use super::{rust_identifiers::RustIdentifiers, rust_template_completion::RustTemplates};

#[derive(Default)]
pub struct RustState {
    rust_identifiers: RustIdentifiers,
    rust_templates: RustTemplates,
}

impl RustState {
    pub fn init(
        trigger_point: Point,
        query: (&Query, &Query),
        tree: &Tree,
        source: &str,
        all: bool,
    ) -> Self {
        let ids = rust_definition_query(query.0, tree, trigger_point, source, all);
        let templates = rust_templates_query(query.1, tree, trigger_point, source, all);
        RustState {
            rust_identifiers: ids,
            rust_templates: templates,
        }
    }

    pub fn reset(&mut self) {
        self.rust_identifiers = RustIdentifiers::default();
        self.rust_templates = RustTemplates::default();
    }

    pub fn template_errors(&self, root: &str) -> Option<Vec<Diagnostic>> {
        let mut diagnostics = vec![];
        for id in &self.rust_templates.templates {
            let name = &id.name;
            let template = format!("{}/{}", root, name);
            let template = std::fs::canonicalize(template);
            let mut is_error = false;
            if template.is_err() {
                is_error = true;
            } else {
                let buffer = template.ok()?;
                let url = format!("file://{}", buffer.to_str()?);
                let url = Url::parse(&url).ok();
                if url.is_none() {
                    is_error = true;
                }
            }
            if is_error {
                let diagnostic = Diagnostic {
                    range: Range::new(
                        Position::new(id.start.row as u32, id.start.column as u32),
                        Position::new(id.end.row as u32, id.end.column as u32),
                    ),
                    severity: Some(DiagnosticSeverity::WARNING),
                    message: "Template not found".to_owned(),
                    source: Some(String::from("jinja-lsp")),
                    ..Default::default()
                };
                diagnostics.push(diagnostic);
            }
        }
        Some(diagnostics)
    }
}
