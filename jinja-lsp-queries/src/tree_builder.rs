use std::fmt::Display;

use tower_lsp::lsp_types::DiagnosticSeverity;

#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash)]
pub enum LangType {
    Template,
    Backend,
}

#[derive(PartialEq, Eq, Debug)]
pub enum JinjaDiagnostic {
    DefinedSomewhere,
    Undefined,
    TemplateNotFound,
}

impl JinjaDiagnostic {
    pub fn severity(&self) -> DiagnosticSeverity {
        match &self {
            JinjaDiagnostic::DefinedSomewhere => DiagnosticSeverity::INFORMATION,
            JinjaDiagnostic::Undefined => DiagnosticSeverity::WARNING,
            JinjaDiagnostic::TemplateNotFound => DiagnosticSeverity::ERROR,
        }
    }
}

impl Display for JinjaDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JinjaDiagnostic::Undefined => f.write_str("Undefined variable"),
            JinjaDiagnostic::DefinedSomewhere => f.write_str("Variable is defined in other file."),
            JinjaDiagnostic::TemplateNotFound => f.write_str("Template not found"),
        }
    }
}
