use tower_lsp::lsp_types::DiagnosticSeverity;

#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash)]
pub enum LangType {
    Template,
    Backend,
}

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
            JinjaDiagnostic::TemplateNotFound => DiagnosticSeverity::WARNING,
        }
    }
}

impl ToString for JinjaDiagnostic {
    fn to_string(&self) -> String {
        match self {
            JinjaDiagnostic::Undefined => String::from("Undefined variable"),
            JinjaDiagnostic::DefinedSomewhere => String::from("Variable is defined in other file."),
            JinjaDiagnostic::TemplateNotFound => String::from("Template not found"),
        }
    }
}
