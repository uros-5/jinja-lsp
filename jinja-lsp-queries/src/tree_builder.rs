use std::fmt::Display;

use tower_lsp::lsp_types::DiagnosticSeverity;

use crate::search::definition::ScopeError;

#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash)]
pub enum LangType {
    Template,
    Backend,
}

#[derive(PartialEq, Eq, Debug)]
pub enum JinjaDiagnostic {
    Undefined,
    DefinedInMultiplePlaces,
    TemplateNotFound,
    CreateNewTemplate,
    ScopeError(ScopeError),
}

impl JinjaDiagnostic {
    pub fn severity(&self) -> DiagnosticSeverity {
        match &self {
            JinjaDiagnostic::Undefined => DiagnosticSeverity::WARNING,
            JinjaDiagnostic::TemplateNotFound => DiagnosticSeverity::ERROR,
            JinjaDiagnostic::DefinedInMultiplePlaces => DiagnosticSeverity::INFORMATION,
            JinjaDiagnostic::CreateNewTemplate => DiagnosticSeverity::HINT,
            JinjaDiagnostic::ScopeError(_) => DiagnosticSeverity::HINT,
        }
    }
}

impl Display for JinjaDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JinjaDiagnostic::Undefined => f.write_str("Undefined variable"),
            JinjaDiagnostic::TemplateNotFound => f.write_str("Template not found"),
            JinjaDiagnostic::DefinedInMultiplePlaces => f.write_str("Defined in multiple places"),
            JinjaDiagnostic::CreateNewTemplate => {
                f.write_str("Create new template with code actions.")
            }
            JinjaDiagnostic::ScopeError(scope_error) => match scope_error {
                ScopeError::WrongEndScopeKeyword(scope) => {
                    f.write_str("Expected `end")?;
                    f.write_str(&scope.keyword)?;
                    f.write_str("` keyword")
                }
                ScopeError::ElifStatement(_) => f.write_str("Elif statement called before if"),
                ScopeError::ElseStatement(_) => {
                    f.write_str("Else statement called before if or elif")
                }
            },
        }
    }
}
