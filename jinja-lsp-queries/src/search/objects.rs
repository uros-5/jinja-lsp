use std::collections::HashSet;

use tower_lsp::lsp_types::Range;
use tree_sitter::{Point, Query, QueryCapture, QueryCursor, StreamingIterator, Tree};

use crate::search::{Identifier, completion_start, to_range};

#[derive(Default, Debug, Clone)]
pub struct JinjaObject {
    pub name: String,
    pub is_filter: bool,
    pub is_test: bool,
    pub fields: Vec<(String, (Point, Point))>,
    pub is_object: bool,
    pub location: (Point, Point),
}

impl JinjaObject {
    pub fn new(name: String, is_filter: bool, is_test: bool) -> Self {
        Self {
            name,
            fields: vec![],
            is_filter,
            is_test,
            is_object: false,
            location: Default::default(),
        }
    }

    pub fn add_field(&mut self, field: String, start: Point, end: Point) {
        self.fields.push((field, (start, end)));
    }

    pub fn last_field_end(&self) -> Point {
        let last = self
            .fields
            .last()
            .map(|item| item.1.1)
            .unwrap_or(self.location.1);
        last
    }

    pub fn location(&self) -> (Point, Point) {
        if self.is_object {
            return self.location;
        }
        let start = self
            .fields
            .first()
            .map(|item| item.1.0)
            .unwrap_or(self.location.0);
        let end = self.last_field_end();
        (start, end)
    }

    pub fn full_range(&self) -> Range {
        to_range(self.location())
    }
}

#[derive(Debug)]
pub enum CompletionMember {
    ExpresionStart,
    ExpressionEnd,
    StatementStart,
    StatementEnd,
    Ident(String),
    FilterOperator,
    Test,
}

#[derive(Default, Debug)]
pub struct JinjaObjects {
    pub objects: Vec<JinjaObject>,
    pub previous_node_id: HashSet<usize>,
    pub previous_nodes: Vec<(CompletionMember, (Point, Point))>,
    test: bool,
}

impl JinjaObjects {
    fn collect(
        &mut self,
        capture_name: &str,
        capture: &QueryCapture<'_>,
        source: &str,
    ) -> Option<()> {
        let start = capture.node.start_position();
        let end = capture.node.end_position();
        let value = capture.node.utf8_text(source.as_bytes()).unwrap();
        let id = capture.node.id();
        if self.previous_node_id.contains(&id) {
            return None;
        }
        self.previous_node_id.insert(id);

        match capture_name {
            "object" => {
                let mut object = JinjaObject::default();
                object.is_object = true;
                object.location = (start, end);
                self.objects.push(object);
            }
            "attribute" => {
                let last = self.objects.last_mut()?;
                last.fields.push((value.to_string(), (start, end)));
                if last.name == "" {
                    last.name = last.fields.first()?.0.to_string();
                    if last.fields.len() == 1 {
                        if VALID_IDENTIFIERS.contains(&value) {
                            return Some(());
                        }
                    }
                    self.previous_nodes
                        .push((CompletionMember::Ident(value.to_string()), (start, end)));
                }
            }
            "just_id" => {
                if VALID_IDENTIFIERS.contains(&value) {
                    return Some(());
                }
                let is_test = self.test;
                let is_filter = self.is_filter(None);
                let mut object = JinjaObject::new(String::from(value), is_filter, is_test);
                object.fields.push((String::from(value), (start, end)));
                self.objects.push(object);
                self.previous_nodes
                    .push((CompletionMember::Ident(value.to_string()), (start, end)));
            }
            "filter" => {
                self.previous_nodes
                    .push((CompletionMember::FilterOperator, (start, end)));
            }
            "expr_start" => {
                self.previous_nodes = vec![];
                self.previous_nodes
                    .push((CompletionMember::ExpresionStart, (start, end)));
            }
            "expr_end" => {
                self.previous_nodes
                    .push((CompletionMember::ExpressionEnd, (start, end)));
            }
            "statement_start" => {
                self.previous_nodes = vec![];
                self.previous_nodes
                    .push((CompletionMember::StatementStart, (start, end)));
            }
            "statement_end" => {
                self.previous_nodes
                    .push((CompletionMember::StatementEnd, (start, end)));
            }
            "is" => self
                .previous_nodes
                .push((CompletionMember::Test, (start, end))),
            "error" => return None,

            _ => {}
        }
        None
    }

    pub fn full_range(&self) -> (Point, Point) {
        self.objects
            .last()
            .map_or((Point::default(), Point::default()), |item| item.location())
    }

    pub fn completion(&self, trigger_point: Point) -> Option<CompletionType> {
        let mut is_filter = false;
        let mut is_id = false;
        let mut is_expr_start = false;
        let mut after_expr = false;
        let mut incomplete = None;
        let mut is_error = false;
        let mut is_test = false;
        for (member, (start, end)) in &self.previous_nodes {
            if &trigger_point < start {
                break;
            }
            match member {
                CompletionMember::ExpresionStart => {
                    if &trigger_point > start && &trigger_point < end {
                        is_error = true;
                    }
                    if &trigger_point >= end {
                        is_expr_start = true;
                    }
                }
                CompletionMember::ExpressionEnd => {
                    if &trigger_point > start && &trigger_point < end {
                        is_error = true;
                    }
                    if &trigger_point >= end {
                        after_expr = true;
                    }
                }
                CompletionMember::Ident(name) => {
                    if &trigger_point >= start {
                        is_id = true;
                    }
                    if &trigger_point >= start && &trigger_point <= end {
                        let range = self.full_range();
                        let identifier = Identifier::new(&name, *start, *end);
                        incomplete = completion_start(trigger_point, &identifier)
                            .map(|item| (item.to_string(), range));
                    }
                }
                CompletionMember::FilterOperator => {
                    if &trigger_point >= start {
                        is_filter = true;
                    }
                }
                CompletionMember::Test => {
                    if &trigger_point >= start {
                        is_test = true;
                    }
                }
                _ => {}
            }
        }
        match (
            is_expr_start,
            is_id,
            incomplete,
            is_filter,
            after_expr,
            is_error,
            is_test,
        ) {
            (_, _, _, _, _, _, true) => Some(CompletionType::Test),
            (_, _, _, _, _, true, _) => None,
            (_, _, _, _, true, _, _) => None,
            (true, true, Some((name, (start, end))), true, _, _, _) => {
                return Some(CompletionType::IncompleteFilter {
                    name,
                    range: (start, end),
                });
            }
            (true, true, _, true, _, _, _) => {
                return Some(CompletionType::Filter);
            }
            (true, true, Some((n, range)), _, _, _, _) => {
                return Some(CompletionType::IncompleteIdentifier { name: n, range });
            }

            (true, false, _, _, _, _, _) => return Some(CompletionType::Identifier),
            _ => None,
        }
    }

    pub fn is_hover(&self, trigger_point: Point) -> bool {
        let full_range = self.full_range();
        trigger_point >= full_range.0 && trigger_point <= full_range.1
    }

    pub fn is_filter(&self, trigger_point: Option<Point>) -> bool {
        for (member, (start, end)) in &self.previous_nodes {
            if let CompletionMember::FilterOperator = member {
                if let Some(trigger_point) = trigger_point {
                    if &trigger_point >= start && &trigger_point <= end {
                        return true;
                    }
                }
                return true;
            }
        }
        return false;
    }

    pub fn is_ident(&self, trigger_point: Point) -> Option<String> {
        let last = self.objects.last()?;
        let location = last.location();
        if trigger_point >= location.0 && trigger_point <= location.1 {
            return Some(last.name.to_string());
        }
        None
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
    let mut matches = cursor_qry.matches(query, closest_node, text.as_bytes());
    while let Some(m) = matches.next() {
        for capture in m.captures {
            let _smaller = trigger_point <= capture.node.start_position();
            if all || trigger_point >= capture.node.start_position() {
                let name = &capture_names[capture.index as usize];
                let _checked = objects.collect(name, capture, text);
                // if checked.is_none() {
                //     break;
                // }
            }
        }
    }

    return objects;
}

#[derive(PartialEq, Debug)]
pub enum CompletionType {
    Filter,
    Test,
    Identifier,
    IncludedTemplate { name: String, range: (Point, Point) },
    Snippets { range: (Point, Point) },
    IncompleteIdentifier { name: String, range: (Point, Point) },
    IncompleteFilter { name: String, range: (Point, Point) },
}

static VALID_IDENTIFIERS: [&str; 8] = [
    "loop", "true", "false", "not", "as", "module", "super", "url_for",
];
