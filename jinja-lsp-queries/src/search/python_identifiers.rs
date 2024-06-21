use std::collections::HashMap;

use tree_sitter::{Point, Query, QueryCursor, Tree};

pub struct PythonAttributes {
    pub attributes: HashMap<Point, Vec<PythonIdentifier>>,
}

impl PythonAttributes {
    pub fn merge(&self, line: u32) -> Vec<PythonIdentifier> {
        let mut identifiers = vec![];
        for i in &self.attributes {
            let mut start = i.0.to_owned();
            start.row += line as usize;
            let mut end = i.0;
            let mut name = String::new();
            let len = i.1.len();
            for (index, identifier) in i.1.iter().enumerate() {
                name.push_str(&identifier.field);
                end = &identifier.end;
                if index != len - 1 {
                    name.push('.');
                }
            }
            let mut end = end.to_owned();
            end.row += line as usize;
            let identifier = PythonIdentifier {
                id: 0,
                start,
                end,
                field: name,
            };
            identifiers.push(identifier);
        }
        identifiers
    }
}

#[derive(Default, Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct PythonIdentifier {
    pub id: usize,
    pub start: Point,
    pub end: Point,
    pub field: String,
}

pub fn python_identifiers(
    query: &Query,
    tree: &Tree,
    mut _trigger_point: Point,
    text: &str,
    line: u32,
) -> Vec<PythonIdentifier> {
    let closest_node = tree.root_node();
    let mut cursor_qry = QueryCursor::new();
    let _capture_names = query.capture_names();
    let mut attributes = PythonAttributes {
        attributes: HashMap::new(),
    };
    let matches = cursor_qry.matches(query, closest_node, text.as_bytes());
    for i in matches {
        for capture in i.captures {
            if let Some(parent) = capture.node.parent() {
                let attribute = attributes
                    .attributes
                    .entry(parent.start_position())
                    .or_default();
                let field = capture.node.utf8_text(text.as_bytes()).unwrap_or_default();
                let identifier = PythonIdentifier {
                    id: capture.node.id(),
                    start: capture.node.start_position(),
                    end: capture.node.end_position(),
                    field: field.to_string(),
                };
                attribute.push(identifier);
            }
        }
    }
    attributes.merge(line)
}
