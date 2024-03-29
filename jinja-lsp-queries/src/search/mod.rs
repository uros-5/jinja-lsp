use tower_lsp::lsp_types::{CompletionItemKind, Position, Range, SymbolKind};
use tree_sitter::Point;

use self::objects::JinjaObject;

pub mod definition;
pub mod jinja_state;
pub mod objects;
pub mod queries;
pub mod rust_identifiers;
pub mod rust_state;
pub mod rust_template_completion;
pub mod snippets_completion;
pub mod templates;
pub mod test_queries;

#[derive(Default, Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct Identifier {
    pub start: Point,
    pub end: Point,
    pub name: String,
    pub scope_ends: (usize, Point),
    pub identifier_type: IdentifierType,
}

impl Identifier {
    pub fn new(name: &str, start: Point, end: Point) -> Self {
        Self {
            name: String::from(name),
            start,
            end,
            scope_ends: (0, Point::default()),
            identifier_type: IdentifierType::UndefinedVariable,
        }
    }
}

impl From<&JinjaObject> for Identifier {
    fn from(value: &JinjaObject) -> Self {
        Identifier::new(&value.name, value.location.0, value.location.1)
    }
}

pub fn completion_start(trigger_point: Point, identifier: &Identifier) -> Option<&str> {
    let len = identifier.name.len();
    let diff = identifier.end.column - 1 - trigger_point.column;
    if diff == 0 {
        return Some("");
    }
    if diff > len {
        return None;
    }
    let to = len - diff;
    let s = identifier.name.get(0..to + 1);
    s
}
pub fn to_range(points: (Point, Point)) -> Range {
    let start = Position::new(points.0.row as u32, points.0.column as u32);
    let end = Position::new(points.1.row as u32, points.1.column as u32);
    Range::new(start, end)
}

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum IdentifierType {
    ForLoopKey,
    ForLoopValue,
    ForLoopCount,
    SetVariable,
    WithVariable,
    MacroName,
    MacroParameter,
    TemplateBlock,
    BackendVariable,
    #[default]
    UndefinedVariable,
    JinjaTemplate,
}

impl IdentifierType {
    pub fn completion_detail(&self) -> &'static str {
        match self {
            IdentifierType::ForLoopKey => "For loop key",
            IdentifierType::ForLoopValue => "For loop value",
            IdentifierType::ForLoopCount => "For loop count",
            IdentifierType::SetVariable => "Set variable",
            IdentifierType::WithVariable => "With variable",
            IdentifierType::MacroName => "Macro",
            IdentifierType::MacroParameter => "Macro parameter",
            IdentifierType::TemplateBlock => "Template block",
            IdentifierType::BackendVariable => "Backend variable",
            IdentifierType::UndefinedVariable => "Undefined variable",
            IdentifierType::JinjaTemplate => "Jinja template",
        }
    }

    pub fn completion_kind(&self) -> CompletionItemKind {
        match self {
            IdentifierType::ForLoopKey => CompletionItemKind::VARIABLE,
            IdentifierType::ForLoopValue => CompletionItemKind::VARIABLE,
            IdentifierType::ForLoopCount => CompletionItemKind::FIELD,
            IdentifierType::SetVariable => CompletionItemKind::VARIABLE,
            IdentifierType::WithVariable => CompletionItemKind::VARIABLE,
            IdentifierType::MacroName => CompletionItemKind::FUNCTION,
            IdentifierType::MacroParameter => CompletionItemKind::FIELD,
            IdentifierType::TemplateBlock => CompletionItemKind::MODULE,
            IdentifierType::BackendVariable => CompletionItemKind::VARIABLE,
            IdentifierType::UndefinedVariable => CompletionItemKind::CONSTANT,
            IdentifierType::JinjaTemplate => CompletionItemKind::FILE,
        }
    }

    pub fn symbol_kind(&self) -> SymbolKind {
        match self {
            IdentifierType::ForLoopKey => SymbolKind::VARIABLE,
            IdentifierType::ForLoopValue => SymbolKind::VARIABLE,
            IdentifierType::ForLoopCount => SymbolKind::FIELD,
            IdentifierType::SetVariable => SymbolKind::VARIABLE,
            IdentifierType::WithVariable => SymbolKind::VARIABLE,
            IdentifierType::MacroName => SymbolKind::FUNCTION,
            IdentifierType::MacroParameter => SymbolKind::FIELD,
            IdentifierType::TemplateBlock => SymbolKind::MODULE,
            IdentifierType::BackendVariable => SymbolKind::VARIABLE,
            IdentifierType::UndefinedVariable => SymbolKind::CONSTANT,
            IdentifierType::JinjaTemplate => SymbolKind::FILE,
        }
    }
}
