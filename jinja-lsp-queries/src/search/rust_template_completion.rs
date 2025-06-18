use tree_sitter::{Point, Query, QueryCapture, QueryCursor, StreamingIterator, Tree};

use super::{Identifier, IdentifierType};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct BackendTemplates {
    pub templates: Vec<Identifier>,
    in_method: bool,
}

impl BackendTemplates {
    pub fn in_template(&self, trigger_point: Point) -> Option<&Identifier> {
        let last = self.templates.last()?;
        if trigger_point >= last.start && trigger_point <= last.end {
            Some(last)
        } else {
            None
        }
    }

    pub fn check(&mut self, name: &str, capture: &QueryCapture<'_>, text: &str) -> Option<()> {
        if name == "template_name" && self.in_method {
            let template = capture.node.utf8_text(text.as_bytes()).ok()?;
            let template = template.replace(['\"', '\''], "");
            let start = capture.node.start_position();
            let end = capture.node.end_position();
            let mut identifer = Identifier::new(&template, start, end);
            identifer.identifier_type = IdentifierType::JinjaTemplate;
            self.templates.push(identifer);
            self.in_method = false;
        } else if name == "method_name" {
            let content = capture.node.utf8_text(text.as_bytes()).ok()?;
            if !METHODS.contains(&content) {
                return None;
            }
            self.in_method = true;
        }
        None
    }

    pub fn collect(self) -> Vec<Identifier> {
        self.templates
    }
}

pub fn backend_templates_query(
    query: &Query,
    tree: &Tree,
    trigger_point: Point,
    text: &str,
    all: bool,
) -> BackendTemplates {
    let mut templates = BackendTemplates::default();
    let closest_node = tree.root_node();
    let mut cursor_qry = QueryCursor::new();
    let capture_names = query.capture_names();
    let mut matches = cursor_qry.matches(query, closest_node, text.as_bytes());
    while let Some(m) = matches.next() {
        for capture in m.captures {
            if all || capture.node.start_position() <= trigger_point {
                let name = &capture_names[capture.index as usize];
                templates.check(name, capture, text);
            }
        }
    }
    templates
}

static METHODS: [&str; 2] = ["render_jinja", "get_template"];
