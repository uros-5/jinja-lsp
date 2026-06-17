use std::fmt::Display;

use tower_lsp::lsp_types::DiagnosticSeverity;

#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash)]
pub enum LangType {
    Template,
    Backend,
}

#[derive(PartialEq, Eq, Debug)]
pub enum JinjaDiagnostic {
    Undefined,
    TemplateNotFound,
}

impl JinjaDiagnostic {
    pub fn severity(&self) -> DiagnosticSeverity {
        match &self {
            JinjaDiagnostic::Undefined => DiagnosticSeverity::WARNING,
            JinjaDiagnostic::TemplateNotFound => DiagnosticSeverity::ERROR,
        }
    }
}

impl Display for JinjaDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JinjaDiagnostic::Undefined => f.write_str("Undefined variable"),
            JinjaDiagnostic::TemplateNotFound => f.write_str("Template not found"),
        }
    }
}
