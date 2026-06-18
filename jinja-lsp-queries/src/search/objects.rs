use tower_lsp::lsp_types::Range;
use tree_sitter::{Point, Query, QueryCapture, QueryCursor, StreamingIterator, Tree};

use super::{completion_start, to_range, to_range2, Identifier};

#[derive(Default, Debug, Clone)]
pub struct JinjaObject {
    pub name: String,
    pub is_filter: bool,
    pub is_test: bool,
    pub fields: Vec<(String, (Point, Point))>,
    pub capture_first: bool,
    pub is_object: bool,
    pub location: (Point, Point),
}

impl JinjaObject {
    pub fn new(name: String, is_filter: bool, is_test: bool) -> Self {
        Self {
            name,
            fields: vec![],
            is_filter,
            capture_first: false,
            is_test,
            is_object: false,
            location: Default::default(),
        }
    }

    pub fn add_field(&mut self, field: String, start: Point, end: Point) {
        self.fields.push((field, (start, end)));
    }

    pub fn last_field_end(&self) -> Point {
        let last = self.fields.last().unwrap().1 .1;
        last
    }

    pub fn full_range(&self) -> Range {
        to_range(self.location())
    }

    pub fn location(&self) -> (Point, Point) {
        if self.is_object {
            return self.location;
        }
        let start = self.fields.first().unwrap().1 .0;
        let end = self.last_field_end();
        (start, end)
    }
}

#[derive(Default, Debug)]
pub struct JinjaObjects {
    objects: Vec<JinjaObject>,
    pipe: (Point, Point),
    test: (Point, Point, bool),
    expr: (Point, Point, ExpressionRange),
    ident: (Point, Point),
    previous_nodes: Vec<usize>,
}

impl JinjaObjects {
    fn collect(
        &mut self,
        capture_name: &str,
        capture: &QueryCapture<'_>,
        source: &str,
    ) -> Option<ObjectState> {
        let start = capture.node.start_position();
        let end = capture.node.end_position();
        let value = capture.node.utf8_text(source.as_bytes());
        match capture_name {
            "expr" => {
                let mut cursor = capture.node.walk();
                cursor.goto_first_child();
                let first = cursor.node();
                cursor.reset(capture.node);
                cursor.goto_last_child();
                let last = cursor.node();
                let expr = ExpressionRange {
                    begin: (first.start_position(), first.end_position()),
                    end: (last.start_position(), last.end_position()),
                };
                self.expr = (start, end, expr);
                return Some(ObjectState::Expression);
            }
            "object" => {
                if self.previous_nodes.contains(&capture.node.id()) {
                    return Some(ObjectState::NewObject);
                }
                let mut object = JinjaObject::default();
                object.is_object = true;
                object.location = (start, end);
                self.objects.push(object);
                self.previous_nodes.push(capture.node.id());
                return Some(ObjectState::NewObject);
            }
            "attribute" => {
                if self.previous_nodes.contains(&capture.node.id()) {
                    return Some(ObjectState::Attribute);
                }
                self.previous_nodes.push(capture.node.id());
                if let Ok(value) = value {
                    if VALID_IDENTIFIERS.contains(&value) {
                        return Some(ObjectState::Invalid);
                    }
                    let last = self.objects.last_mut()?;
                    last.fields.push((value.to_string(), (start, end)));
                    if last.name == "" {
                        last.name = last.fields.first().unwrap().0.to_string();
                    }
                    self.ident = (start, end);
                    self.test.2 = false;
                }
                return Some(ObjectState::Attribute);
            }
            "just_id" => {
                if self.previous_nodes.contains(&capture.node.id()) {
                    return Some(ObjectState::NewObject);
                }
                self.previous_nodes.push(capture.node.id());
                if let Ok(value) = value {
                    if VALID_IDENTIFIERS.contains(&value) {
                        return Some(ObjectState::Invalid);
                    }
                    self.ident = (start, end);
                    let is_test = self.test.2;
                    let is_filter = self.is_hover(start) && self.is_filter();
                    let mut object = JinjaObject::new(String::from(value), is_filter, is_test);
                    object.fields.push((String::from(value), (start, end)));
                    self.objects.push(object);
                    self.test.2 = false;
                }
                return Some(ObjectState::NewObject);
            }
            "pipe" => {
                let content = capture.node.utf8_text(source.as_bytes()).ok()?;
                if content.starts_with('|') {
                    self.pipe = (start, end);
                }
                return Some(ObjectState::NewObject);
            }
            "is" => {
                self.test = (start, end, true);
                return Some(ObjectState::NewTest);
            }
            "error" => {
                return None;
            }
            _ => (),
        }
        Some(ObjectState::Invalid)
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
                let range = self.full_range();
                let identifier = Identifier::new(&ident_value, self.ident.0, self.ident.1);
                let start = completion_start(trigger_point, &identifier);
                return Some((
                    CompletionType::IncompleteIdentifier {
                        name: start?.to_string(),
                        range,
                    },
                    autoclose,
                ));
            }
            return Some((CompletionType::Identifier, autoclose));
        } else if self.is_test(trigger_point) {
            return Some((CompletionType::Test, false));
        }
        None
    }

    pub fn in_pipe(&self, trigger_point: Point) -> bool {
        trigger_point >= self.pipe.0 && trigger_point <= self.pipe.1
    }

    pub fn in_expr(&self, trigger_point: Point) -> bool {
        let in_expr = trigger_point >= self.expr.0 && trigger_point < self.expr.1;
        let after_ident = trigger_point > self.ident.0;
        let no_ident = self.expr.2.begin.1 == self.expr.2.end.0;
        if !in_expr {
            return false;
        }
        in_expr && after_ident || no_ident
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
        let full_range = self.full_range();
        trigger_point >= self.ident.0 && trigger_point <= self.ident.1
            || to_range2(full_range, trigger_point)
    }

    pub fn is_filter(&self) -> bool {
        self.pipe.1 == self.ident.0 && !self.objects.is_empty()
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

    pub fn points(&self) -> Range {
        self.objects
            .last()
            .map_or(Range::default(), |item| item.full_range())
    }

    pub fn is_test(&self, trigger_point: Point) -> bool {
        self.test.2 && trigger_point >= self.test.1 && trigger_point.row == self.test.1.row
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
    let mut continued = false;
    let mut my_id = 0;
    let mut my_expr = (
        Point::default(),
        Point::default(),
        ExpressionRange::default(),
    );
    let mut matches = cursor_qry.matches(query, closest_node, text.as_bytes());
    while let Some(m) = matches.next() {
        for capture in m.captures {
            let smaller = trigger_point <= capture.node.start_position();
            if all || trigger_point >= capture.node.start_position() {
                let name = &capture_names[capture.index as usize];
                let checked = objects.collect(name, capture, text);
                if checked.is_none() {
                    break;
                }
            } else if smaller {
                let name = capture_names[capture.index as usize];
                if objects.is_filter() || name == "expr" {
                    break;
                } else if !continued {
                    if objects.is_hover(trigger_point) {
                        continued = true;
                        my_id = objects.objects.len() - 1;
                        my_expr = objects.expr;
                        continue;
                    } else {
                        break;
                    }
                } else if continued {
                    let name = &capture_names[capture.index as usize];
                    let checked = objects.collect(name, capture, text);
                    if checked.is_none() {
                        break;
                    } else if checked.is_some_and(|item| {
                        matches!(
                            item,
                            ObjectState::Expression | ObjectState::NewObject | ObjectState::Invalid // | ObjectState::Attribute
                        )
                    }) {
                        objects
                            .objects
                            .get_mut(my_id)
                            .and_then(|obj| -> Option<()> {
                                objects.ident = obj.location();
                                obj.capture_first = true;
                                None
                            });
                        if my_id != objects.objects.len() - 1 {
                            objects.objects.pop();
                            objects.expr = my_expr;
                        }
                    }
                }
            }
        }
    }
    objects
}

#[derive(PartialEq, Debug)]
pub enum CompletionType {
    Filter,
    Test,
    Identifier,
    IncludedTemplate { name: String, range: Range },
    Snippets { range: Range },
    IncompleteIdentifier { name: String, range: Range },
    IncompleteFilter { name: String, range: Range },
}

static VALID_IDENTIFIERS: [&str; 8] = [
    "loop", "true", "false", "not", "as", "module", "super", "url_for",
];

#[derive(Default, Debug, Clone, Copy)]
pub struct ExpressionRange {
    begin: (Point, Point),
    end: (Point, Point),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ObjectState {
    Expression,
    Invalid,
    NewField,
    NewFilter,
    NewTest,
    NewObject,
    Attribute,
}
