use std::collections::HashMap;

use dashmap::DashMap;
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
    variables: &DashMap<String, Vec<JinjaVariable>>,
    file_name: &String,
    diags: &mut HashMap<String, Vec<(JinjaVariable, JinjaDiagnostic)>>,
) -> Option<()> {
    let trigger_point = Point::new(0, 0);
    let query = &query.jinja_idents;
    let capturer = JinjaObjectCapturer::default();
    let props = query_props(root, source, trigger_point, query, true, capturer);
    let props = props.show();
    for object in props {
        let file = variables.get(file_name)?;
        let mut exist = false;
        let mut err_type = JinjaDiagnostic::Undefined;
        let mut to_warn = false;
        let temp = file
            .value()
            .iter()
            .filter(|variable| variable.name == object.name)
            .filter(|variable| {
                exist = true;
                object.location.0 >= variable.location.0
            });
        let empty = temp.count() == 0;
        if empty && exist {
            to_warn = true;
        } else if empty {
            to_warn = true;
            drop(file);
            for i in variables {
                let temp = i
                    .value()
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
            let variable = JinjaVariable::new(&object.name, object.location, DataType::Variable);
            if diags.get(file_name).is_none() {
                diags.insert(file_name.to_string(), vec![(variable, err_type)]);
            } else {
                diags.get_mut(file_name).unwrap().push((variable, err_type));
            }
        }
    }
    None
}
