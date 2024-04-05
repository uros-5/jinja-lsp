use tree_sitter::{Point, Query, QueryCapture, QueryCursor, Tree};

use super::{Identifier, IdentifierType};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum Current {
    InMacro(Point),
    #[default]
    Free,
}

#[derive(Default, Debug, Clone)]
pub struct BackendIdentifiers {
    variables: Vec<Identifier>,
}

impl BackendIdentifiers {
    pub fn show(self) -> Vec<Identifier> {
        self.variables
    }

    pub fn check(&mut self, name: &str, capture: &QueryCapture<'_>, text: &str) -> Option<()> {
        match name {
            "key_id" => {
                let start = capture.node.start_position();
                let end = capture.node.end_position();
                let name = capture.node.utf8_text(text.as_bytes()).ok()?;
                let mut identifier = Identifier::new(name, start, end);
                identifier.identifier_type = IdentifierType::BackendVariable;
                self.variables.push(identifier);
            }
            "name" => {
                let start = capture.node.start_position();
                let end = capture.node.end_position();
                let name = capture.node.utf8_text(text.as_bytes()).ok()?;
                let name = name.replace(['\"', '\''], "");
                let mut identifier = Identifier::new(&name, start, end);
                identifier.identifier_type = IdentifierType::BackendVariable;
                self.variables.push(identifier);
            }
            "error" => {
                return None;
            }
            _ => (),
        }
        Some(())
    }
}

pub fn backend_definition_query(
    query: &Query,
    tree: &Tree,
    trigger_point: Point,
    text: &str,
    all: bool,
) -> BackendIdentifiers {
    let closest_node = tree.root_node();
    let mut cursor_qry = QueryCursor::new();
    let mut rust = BackendIdentifiers::default();
    let capture_names = query.capture_names();
    let matches = cursor_qry.matches(query, closest_node, text.as_bytes());
    let captures = matches.into_iter().flat_map(|m| {
        m.captures
            .iter()
            .filter(|capture| all || capture.node.start_position() <= trigger_point)
    });
    for capture in captures {
        let name = &capture_names[capture.index as usize];
        if rust.check(name, capture, text).is_none() {
            break;
        }
    }
    rust
}
