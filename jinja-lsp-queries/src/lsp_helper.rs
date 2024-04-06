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

pub fn search_errors(
    root: &Tree,
    source: &str,
    queries: &Queries,
    variables: &HashMap<String, Vec<Identifier>>,
    file_name: &String,
    templates: PathBuf,
    lang_type: LangType,
) -> Option<Vec<(JinjaDiagnostic, Identifier)>> {
    let mut diagnostics = vec![];
    match lang_type {
        LangType::Template => {
            let trigger_point = Point::new(0, 0);
            let query = &queries.jinja_objects;
            let objects = objects_query(query, root, trigger_point, source, true);
            let objects = objects.show();
            let this_file = variables.get(file_name)?;
            for object in objects {
                if object.is_filter {
                    continue;
                }
                let mut exist = false;
                let mut err_type = JinjaDiagnostic::Undefined;
                let mut to_warn = false;
                let located = this_file
                    .iter()
                    .filter(|variable| {
                        variable.name == object.name
                            && variable.identifier_type != IdentifierType::TemplateBlock
                    })
                    .filter(|variable| {
                        exist = true;
                        let bigger = object.location.1 >= variable.start;
                        let global = variable.scope_ends.1 == Point::default();
                        let in_scope = object.location.0 < variable.scope_ends.1;
                        if bigger && global {
                            true
                        } else {
                            bigger && in_scope
                        }
                    });
                let empty = located.count() == 0;
                if empty && exist {
                    to_warn = true;
                } else if empty {
                    to_warn = true;
                    for file in variables {
                        let temp = file
                            .1
                            .iter()
                            .filter(|variable| variable.name == object.name);
                        if temp.count() != 0 {
                            err_type = JinjaDiagnostic::DefinedSomewhere;
                            to_warn = true;
                            break;
                        }
                    }
                }
                if to_warn {
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
