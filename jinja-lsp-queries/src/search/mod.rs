use tree_sitter::Point;

pub mod definition;
pub mod jinja_completion;
pub mod jinja_state;
pub mod objects;
pub mod parsers;
pub mod queries;
pub mod rust_identifiers;
pub mod rust_state;
pub mod rust_template_completion;
pub mod templates;
pub mod test_queries;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Identifier {
    pub start: Point,
    pub end: Point,
    pub name: String,
}

impl Identifier {
    pub fn new(name: &str, start: Point, end: Point) -> Self {
        Self {
            name: String::from(name),
            start,
            end,
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum CompletionType {
    Filter,
    Identifier,
    IncludedTemplate { name: String, range: (Point, Point) },
}

pub fn completion_start(trigger_point: Point, identifier: &Identifier) -> Option<&str> {
    let len = identifier.name.len();
    let diff = identifier.end.column - trigger_point.column;
    let to = len - diff;
    let s = identifier.name.get(0..to);
    s
}
