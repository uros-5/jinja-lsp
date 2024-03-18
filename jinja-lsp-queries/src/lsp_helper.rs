use std::{collections::HashMap, io::ErrorKind};

use tree_sitter::{Node, Point};

use crate::{
    capturer::object::JinjaObjectCapturer,
    queries::{query_props, Queries},
    tree_builder::{DataType, JinjaDiagnostic, JinjaVariable},
};

pub fn search_errors(
    root: Node<'_>,
    source: &str,
    query: &Queries,
    variables: &HashMap<String, Vec<JinjaVariable>>,
    file_name: &String,
    templates: &String,
) -> Option<Vec<(JinjaVariable, JinjaDiagnostic)>> {
    let trigger_point = Point::new(0, 0);
    let query = &query.jinja_idents;
    let capturer = JinjaObjectCapturer::default();
    let props = query_props(root, source, trigger_point, query, true, capturer);
    let props = props.show();
    let mut diags = vec![];
    for object in props {
        if object.is_filter {
            continue;
        }
        let jinja_variables = variables.get(file_name)?;
        let mut exist = false;
        let mut err_type = JinjaDiagnostic::Undefined;
        let mut to_warn = false;
        // variable definition is in this file
        let located = jinja_variables
            .iter()
            .filter(|variable| variable.name == object.name)
            .filter(|variable| {
                exist = true;
                object.location.0 >= variable.location.0
            });
        let empty = located.count() == 0;
        if empty && exist {
            to_warn = true;
        } else if empty {
            to_warn = true;
            for i in variables {
                let temp = i.1.iter().filter(|variable| variable.name == object.name);

                if temp.count() != 0 {
                    err_type = JinjaDiagnostic::DefinedSomewhere;
                    to_warn = true;
                    break;
                }
            }
        }
        if to_warn {
            let variable = JinjaVariable::new(&object.name, object.location, DataType::Variable);
            diags.push((variable, err_type));
        }
    }
    let jinja_variables = variables.get(file_name)?;
    let abc = jinja_variables
        .iter()
        .filter(|variable| variable.data_type == DataType::Template);
    for i in abc {
        if i.name.is_empty() {
            diags.push((i.clone(), JinjaDiagnostic::TemplateNotFound));
        } else {
            let path = format!("{templates}/{}", i.name);
            if let Err(err) = std::fs::canonicalize(path) {
                if err.kind() == ErrorKind::NotFound {
                    diags.push((i.clone(), JinjaDiagnostic::TemplateNotFound));
                }
            }
        }
    }

    if diags.is_empty() {
        None
    } else {
        Some(diags)
    }
}
