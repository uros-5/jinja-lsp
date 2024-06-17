use std::fs;

use tower_lsp::lsp_types::Range;
use tree_sitter::{Node, Point, Query, QueryCapture, QueryCursor, Tree};

use super::{completion_start, to_range, Identifier};

#[derive(Default, Debug, Clone)]
pub struct JinjaObject {
    pub name: String,
    pub location: (Point, Point),
    pub is_filter: bool,
    pub fields: Vec<(String, (Point, Point))>,
}

impl JinjaObject {
    pub fn new(name: String, start: Point, end: Point, is_filter: bool) -> Self {
        Self {
            name,
            location: (start, end),
            fields: vec![],
            is_filter,
        }
    }

    pub fn add_field(&mut self, field: String, start: Point, end: Point) {
        self.fields.push((field, (start, end)));
    }

    pub fn last_field_end(&self) -> Point {
        let last = self.fields.last().map_or(self.location.1, |v| v.1 .1);
        last
    }

    pub fn full_range(&self) -> Range {
        let start = self.location.0;
        let end = self.last_field_end();
        to_range((start, end))
    }
}

#[derive(Default, Debug)]
pub struct JinjaObjects {
    objects: Vec<JinjaObject>,
    dot: (Point, Point),
    pipe: (Point, Point),
    expr: (Point, Point, ExpressionPoints),
    ident: (Point, Point),
}

impl JinjaObjects {
    fn check(&mut self, name: &str, capture: &QueryCapture<'_>, source: &str) -> Option<()> {
        let start = capture.node.start_position();
        let end = capture.node.end_position();
        match name {
            "error" => {
                return None;
            }
            "just_id" => {
                self.build_object(capture, source);
            }
            "dot" => {
                self.dot = (start, end);
            }
            "pipe" => {
                let content = capture.node.utf8_text(source.as_bytes()).ok()?;
                if content.starts_with('|') {
                    self.pipe = (start, end);
                }
            }
            "expr" => {
                let mut cursor = capture.node.walk();
                cursor.goto_first_child();
                let first = cursor.node();
                cursor.reset(capture.node);
                cursor.goto_last_child();
                let last = cursor.node();
                let expr = ExpressionPoints {
                    begin: (first.start_position(), first.end_position()),
                    end: (last.start_position(), last.end_position()),
                };
                self.expr = (start, end, expr);
            }
            _ => (),
        }
        Some(())
    }

    pub fn build_object(&mut self, capture: &QueryCapture<'_>, source: &str) {
        let value = capture.node.utf8_text(source.as_bytes());
        let start = capture.node.start_position();
        let end = capture.node.end_position();
        if let Ok(value) = value {
            if start.row == self.dot.1.row && start.column == self.dot.1.column {
                let last_object = self.objects.last_mut().map(|last| {
                    last.fields.push((String::from(value), (start, end)));
                    self.ident = (start, end);
                });
                match last_object {
                    Some(_) => {}
                    None => {
                        // TODO: in future add those to main library
                        if VALID_IDENTIFIERS.contains(&value) {
                            return;
                        }
                        self.ident = (start, end);
                        let is_filter = self.is_hover(start) && self.is_filter();
                        self.objects.push(JinjaObject::new(
                            String::from(value),
                            start,
                            end,
                            is_filter,
                        ));
                    }
                }
            } else {
                // TODO: in future add those to main library
                if VALID_IDENTIFIERS.contains(&value) {
                    return;
                }
                self.ident = (start, end);
                let is_filter = self.is_hover(start) && self.is_filter();
                self.objects
                    .push(JinjaObject::new(String::from(value), start, end, is_filter));
            }
        }
    }

    pub fn completion(&self, trigger_point: Point) -> Option<(CompletionType, bool)> {
        let autoclose = self.should_autoclose();
        if self.in_pipe(trigger_point) {
            return Some((CompletionType::Filter, autoclose));
        } else if self.in_expr(trigger_point) {
            if trigger_point == self.expr.2.begin.1 && trigger_point == self.expr.2.end.0 {
                return Some((CompletionType::Identifier, autoclose));
            }
            if trigger_point > self.ident.1 {
                return Some((CompletionType::Identifier, autoclose));
            }
            if let Some(ident_value) = self.is_ident(trigger_point) {
                // if let Some(ident2) = self.objects.last().map(|last| last) {
                let identifier = Identifier::new(&ident_value, self.ident.0, self.ident.1);
                let start = completion_start(trigger_point, &identifier);
                // let range = to_range((self.ident.0, self.ident.1));
                let range = self.full_range();
                return Some((
                    CompletionType::IncompleteIdentifier {
                        name: start?.to_string(),
                        range,
                    },
                    autoclose,
                ));
                // }
            }
            return Some((CompletionType::Identifier, autoclose));
        }
        None
    }

    pub fn in_pipe(&self, trigger_point: Point) -> bool {
        trigger_point >= self.pipe.0 && trigger_point <= self.pipe.1
    }

    pub fn in_expr(&self, trigger_point: Point) -> bool {
        trigger_point >= self.expr.0 && trigger_point <= self.expr.1 && trigger_point > self.ident.0
            || self.expr.2.begin.1 == self.expr.2.end.0
    }

    pub fn should_autoclose(&self) -> bool {
        self.expr.2.end.0 == self.expr.2.end.1
    }

    pub fn is_ident(&self, trigger_point: Point) -> Option<String> {
        if trigger_point >= self.ident.0 && trigger_point <= self.ident.1 {
            self.objects.last().map(|last| last.name.to_string())
        } else {
            None
        }
    }

    pub fn is_hover(&self, trigger_point: Point) -> bool {
        trigger_point >= self.ident.0 && trigger_point <= self.ident.1
    }

    pub fn is_filter(&self) -> bool {
        self.pipe.1 == self.ident.0
    }

    pub fn get_last_id(&self) -> Option<&JinjaObject> {
        self.objects.last()
    }

    pub fn show(&self) -> Vec<JinjaObject> {
        self.objects.clone()
    }

    pub fn full_range(&self) -> Range {
        self.objects
            .last()
            .map_or(Range::default(), |item| item.full_range())
    }
}

pub fn objects_query(
    query: &Query,
    tree: &Tree,
    trigger_point: Point,
    text: &str,
    all: bool,
) -> JinjaObjects {
    let closest_node = tree.root_node();
    let mut objects = JinjaObjects::default();
    let mut cursor_qry = QueryCursor::new();
    let capture_names = query.capture_names();
    let matches = cursor_qry.matches(query, closest_node, text.as_bytes());
    let captures = matches.into_iter().flat_map(|m| {
        m.captures
            .iter()
            .filter(|capture| all || capture.node.start_position() <= trigger_point)
    });
    for capture in captures {
        let name = &capture_names[capture.index as usize];
        let checked = objects.check(name, capture, text);
        if checked.is_none() {
            break;
        }
    }
    objects
}

#[derive(PartialEq, Debug)]
pub enum CompletionType {
    Filter,
    Identifier,
    IncludedTemplate { name: String, range: Range },
    Snippets { range: Range },
    IncompleteIdentifier { name: String, range: Range },
    IncompleteFilter { name: String, range: Range },
}

static VALID_IDENTIFIERS: [&str; 8] = [
    "loop", "true", "false", "not", "as", "module", "super", "url_for",
];

#[derive(Default, Debug)]
pub struct ExpressionPoints {
    begin: (Point, Point),
    end: (Point, Point),
}
