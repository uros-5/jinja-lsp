use tower_lsp::lsp_types::Range;
use tree_sitter::{Point, Query, QueryCapture, QueryCursor, Tree};

#[derive(Default, Debug, Clone)]
pub struct JinjaObject {
    pub name: String,
    pub location: (Point, Point),
    pub is_filter: bool,
    fields: Vec<(String, (Point, Point))>,
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
}

#[derive(Default, Debug)]
pub struct JinjaObjects {
    objects: Vec<JinjaObject>,
    dot: (Point, Point),
    pipe: (Point, Point),
    expr: (Point, Point),
    ident: (Point, Point),
}

impl JinjaObjects {
    fn check(&mut self, name: &str, capture: &QueryCapture<'_>, source: &str) -> Option<()> {
        let start = capture.node.start_position();
        let end = capture.node.end_position();
        match name {
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
                self.expr = (start, end);
            }
            _ => (),
        }
        None
    }

    pub fn build_object(&mut self, capture: &QueryCapture<'_>, source: &str) {
        let value = capture.node.utf8_text(source.as_bytes());
        let start = capture.node.start_position();
        let end = capture.node.end_position();
        if let Ok(value) = value {
            if start.row == self.dot.1.row && start.column == self.dot.1.column {
                match self
                    .objects
                    .last_mut()
                    .map(|last| {
                        last.fields.push((String::from(value), (start, end)));
                        self.ident = (start, end);
                    })
                    .is_none()
                {
                    true => {
                        // TODO: in future add those to main library
                        if VALID_IDENTIFIERS.contains(&value) {
                            return;
                        }
                        self.ident = (start, end);
                        let is_filter = self.is_hover(start) && self.is_filter(start);
                        self.objects.push(JinjaObject::new(
                            String::from(value),
                            start,
                            end,
                            is_filter,
                        ));
                    }
                    false => (),
                }
            } else {
                // TODO: in future add those to main library
                if VALID_IDENTIFIERS.contains(&value) {
                    return;
                }
                self.ident = (start, end);
                let is_filter = self.is_hover(start) && self.is_filter(start);
                self.objects
                    .push(JinjaObject::new(String::from(value), start, end, is_filter));
            }
        }
    }

    pub fn completion(&self, trigger_point: Point) -> Option<CompletionType> {
        if self.in_pipe(trigger_point) {
            return Some(CompletionType::Filter);
        } else if self.in_expr(trigger_point) {
            return Some(CompletionType::Identifier);
        }
        None
    }

    pub fn in_pipe(&self, trigger_point: Point) -> bool {
        trigger_point >= self.pipe.0 && trigger_point <= self.pipe.1
    }

    pub fn in_expr(&self, trigger_point: Point) -> bool {
        trigger_point >= self.expr.0 && trigger_point <= self.expr.1 && trigger_point > self.ident.1
    }

    pub fn is_ident(&self, trigger_point: Point) -> Option<String> {
        if trigger_point >= self.ident.0 && trigger_point <= self.ident.1 {
            self.objects.last().map(|last| last.name.to_string())
        } else {
            None
        }
    }

    pub fn is_hover(&self, trigger_point: Point) -> bool {
        let in_id = trigger_point >= self.ident.0 && trigger_point <= self.ident.1;
        in_id
    }

    pub fn is_filter(&self, trigger_point: Point) -> bool {
        self.pipe.1 == self.ident.0
    }

    pub fn get_last_id(&self) -> Option<&JinjaObject> {
        self.objects.last()
    }

    pub fn show(&self) -> Vec<JinjaObject> {
        self.objects.clone()
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
        objects.check(name, capture, text);
    }
    objects
}

#[derive(PartialEq, Debug)]
pub enum CompletionType {
    Filter,
    Identifier,
    IncludedTemplate { name: String, range: Range },
    Snippets { name: String, range: Range },
}

static VALID_IDENTIFIERS: [&str; 6] = ["loop", "true", "false", "not", "as", "module"];
