use std::{collections::HashMap, io::ErrorKind, path::PathBuf};

use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use tree_sitter::{Point, Tree};

use crate::{
    search::{
        objects::objects_query, queries::Queries, templates::templates_query, Identifier,
        IdentifierType,
    },
    tree_builder::{JinjaDiagnostic, LangType},
};

#[allow(clippy::too_many_arguments)]
pub fn search_errors(
    root: &Tree,
    source: &str,
    queries: &Queries,
    variables: &HashMap<String, Vec<Identifier>>,
    file_name: &String,
    templates: PathBuf,
    lang_type: LangType,
    ignore_globals: bool,
) -> Option<Vec<(JinjaDiagnostic, Identifier)>> {
    let mut diagnostics = vec![];
    match lang_type {
        LangType::Template => {
            let trigger_point = Point::new(0, 0);
            let query = &queries.jinja_objects;
            let objects = objects_query(query, root, trigger_point, source, true);
            let objects1 = objects.show();
            let this_file = variables.get(file_name)?;
            for object in objects1 {
                if object.is_filter || object.is_test {
                    continue;
                }
                let mut err_type = JinjaDiagnostic::Undefined;
                let located = this_file
                    .iter()
                    .filter(|variable| {
                        variable.name == object.name
                            && variable.identifier_type != IdentifierType::TemplateBlock
                    })
                    .filter(|file_variable| {
                        let object_location = object.location();
                        let can_be_used = object_location.1 >= file_variable.start;
                        let in_scope = object_location.0 < file_variable.scope_ends.1;
                        can_be_used && in_scope
                    });
                let empty = located.count() == 0;
                let mut to_warn = false;
                if empty {
                    if ignore_globals {
                        continue;
                    }
                    to_warn = true;
                    let mut count = 0;
                    for file in variables {
                        if file.0 == file_name {
                            continue;
                        }
                        let idents = file
                            .1
                            .iter()
                            .filter(|variable| variable.name == object.name);
                        count += idents.count();
                        if count > 1 {
                            err_type = JinjaDiagnostic::DefinedInMultiplePlaces;
                            to_warn = true;
                            break;
                        }
                    }
                    if count == 1 {
                        to_warn = false;
                    }
                }
                if to_warn {
                    if ignore_globals {
                        continue;
                    }
                    let diagnostic = (err_type, Identifier::from(&object));
                    diagnostics.push(diagnostic);
                }
            }

            let mut variables = vec![];
            let query_templates = &queries.jinja_imports;
            let jinja_imports = templates_query(query_templates, root, trigger_point, source, true);
            jinja_imports.collect(&mut variables);

            let id_templates = variables
                .iter()
                .filter(|identifier| identifier.identifier_type == IdentifierType::JinjaTemplate);
            for i in id_templates {
                let err_type = JinjaDiagnostic::TemplateNotFound;
                if i.name.is_empty() {
                    let diagnostic = (err_type, i.to_owned());
                    diagnostics.push(diagnostic);
                } else {
                    let mut templates = templates.clone();
                    templates.push(path_items(&i.name));
                    if let Err(err) = std::fs::canonicalize(templates) {
                        if err.kind() == ErrorKind::NotFound {
                            let diagnostic = (err_type, i.to_owned());
                            diagnostics.push(diagnostic);
                            diagnostics.push((JinjaDiagnostic::CreateNewTemplate, i.to_owned()))
                        }
                    }
                }
            }
            Some(diagnostics)
        }
        LangType::Backend => {
            let all_variables = variables.get(file_name)?;
            let templates2 = all_variables
                .iter()
                .filter(|id| id.identifier_type == IdentifierType::JinjaTemplate);
            for template in templates2 {
                let mut templates = templates.clone();
                templates.push(path_items(&template.name));
                if let Err(err) = std::fs::canonicalize(templates) {
                    if err.kind() == ErrorKind::NotFound {
                        let diagnostic = (JinjaDiagnostic::TemplateNotFound, template.to_owned());
                        diagnostics.push(diagnostic);
                    }
                }
            }
            Some(diagnostics)
        }
    }
}

pub fn create_diagnostic(
    template: &Identifier,
    severity: DiagnosticSeverity,
    message: String,
) -> Diagnostic {
    Diagnostic {
        range: Range::new(
            Position::new(template.start.row as u32, template.start.column as u32),
            Position::new(template.end.row as u32, template.end.column as u32),
        ),
        severity: Some(severity),
        message,
        source: Some(String::from("jinja-lsp")),
        ..Default::default()
    }
}

pub fn path_items(template: &str) -> PathBuf {
    template.split('/').collect()
}
