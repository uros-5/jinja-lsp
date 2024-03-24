use std::collections::HashMap;

use tree_sitter::{Point, Query, QueryCapture, QueryCursor, Tree};

use super::Identifier;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum Current {
    InMacro(Point),
    #[default]
    Free,
}

#[derive(Default, Debug, Clone)]
pub struct RustIdentifiers {
    variables: Vec<Identifier>,
    current: Current,
}

impl RustIdentifiers {
    pub fn show(&self) -> &Vec<Identifier> {
        &self.variables
    }

    pub fn check(&mut self, name: &str, capture: &QueryCapture<'_>, text: &str) -> Option<()> {
        match name {
            "macro" => {
                let end = capture.node.end_position();
                self.current = Current::InMacro(end);
            }
            "key_id" => {
                if let Current::InMacro(end_macro) = self.current {
                    let start = capture.node.start_position();
                    let end = capture.node.end_position();
                    if start > end_macro {
                        self.current = Current::Free;
                        return None;
                    }
                    let name = capture.node.utf8_text(text.as_bytes()).ok()?;
                    let identifier = Identifier::new(name, start, end);
                    self.variables.push(identifier);
                }
            }
            "name" => {
                let start = capture.node.start_position();
                let end = capture.node.end_position();
                let name = capture.node.utf8_text(text.as_bytes()).ok()?;
                let identifier = Identifier::new(name, start, end);
                self.variables.push(identifier);
            }
            _ => (),
        }
        None
    }
}

pub fn rust_definition_query(
    query: &Query,
    tree: Tree,
    trigger_point: Point,
    text: &str,
    all: bool,
) -> RustIdentifiers {
    let closest_node = tree.root_node();
    let mut cursor_qry = QueryCursor::new();
    let mut rust = RustIdentifiers::default();
    let capture_names = query.capture_names();
    let matches = cursor_qry.matches(&query, closest_node, text.as_bytes());
    let captures = matches.into_iter().flat_map(|m| {
        m.captures
            .iter()
            .filter(|capture| all || capture.node.start_position() <= trigger_point)
    });
    for capture in captures {
        let name = &capture_names[capture.index as usize];
        rust.check(name, capture, text);
    }
    rust
}
