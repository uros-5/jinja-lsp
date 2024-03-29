use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, InsertReplaceEdit, Position, Range,
    TextEdit,
};
use tree_sitter::{Point, Query, QueryCapture, QueryCursor, Tree};

use crate::to_input_edit::to_position2;

use super::Identifier;

#[derive(Default, Debug)]
pub struct Snippets {
    keyword: Identifier,
    block: (Identifier, usize, bool),
}

impl Snippets {
    pub fn check(&mut self, name: &str, capture: &QueryCapture<'_>, source: &str) -> Option<()> {
        let id = capture.node.id();
        let start = capture.node.start_position();
        let end = capture.node.end_position();

        if name == "block" || name == "error1" && self.block.1 != id {
            self.block.0.start = start;
            self.block.0.end = end;
            self.block.1 = id;
            self.block.2 = true;
        } else if name == "longer_keyword" {
            let keyword = capture.node.utf8_text(source.as_bytes()).ok()?;
            let keyword = Identifier::new(keyword, start, end);
            self.keyword = keyword;
        }
        None
    }

    pub fn at_keyword(&self, trigger_point: Point) -> Option<String> {
        if trigger_point >= self.keyword.start
            && trigger_point == self.keyword.end
            && trigger_point >= self.block.0.start
            && trigger_point <= self.block.0.end
        {
            Some(self.keyword.name.to_owned())
        } else {
            None
        }
    }

    pub fn last_range(&self) -> Range {
        let start = to_position2(self.block.0.start);
        let end = to_position2(self.block.0.end);
        Range { start, end }
    }
}

pub fn snippets_query(
    query: &Query,
    tree: &Tree,
    trigger_point: Point,
    text: &str,
    all: bool,
) -> Snippets {
    let closest_node = tree.root_node();
    let mut snippets = Snippets::default();
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
        snippets.check(name, capture, text);
    }
    snippets
}

/// Range will be updated
pub fn snippets() -> Vec<CompletionItem> {
    // label detail text
    let all = [
        (
            "for1",
            "Basic for loop",
            r#"{% for ${1:i} in ${2:items} %}
{% endfor %}"#,
        ),
        (
            "for2",
            "For loop with key and value",
            r#"{% for (${1:key}, ${2:value}) in ${3:items} %}
{% endfor %}"#,
        ),
        (
            "with",
            "With block",
            r#"{% with $1 %}
{% endwith %}"#,
        ),
        ("set1", "Set variable", r#"{% set ${1:key} = ${2:value} %}"#),
        (
            "set2",
            "Set with scope",
            r#"{% set ${1:data} %}
{% endset %}"#,
        ),
        ("include", "Include template", r#"{% include "$1" %}"#),
        (
            "from",
            "Import from other template",
            r#"{% from "$1" import ${2:module} %}"#,
        ),
        (
            "import",
            "Import entire template as module",
            r#"{% import "$1" as ${2:module} %}"#,
        ),
        ("extends", "Extend parent template", r#"{% extends "$1" %}"#),
        (
            "if1",
            "If statement",
            r#"{% if $1 %}
{% endif %}"#,
        ),
        (
            "if2",
            "If statement",
            r#"{% if $1 %}
{% elif $2 %}
{% endif %}"#,
        ),
    ];

    let mut snippets = vec![];

    for snippet in all {
        let edit = TextEdit {
            new_text: snippet.2.to_owned(),
            ..Default::default()
        };
        let text_edit = CompletionTextEdit::Edit(edit);
        let item = CompletionItem {
            label: snippet.0.to_owned(),
            detail: Some(snippet.1.to_owned()),
            kind: Some(CompletionItemKind::SNIPPET),
            text_edit: Some(text_edit),
            ..Default::default()
        };
        snippets.push(item);
    }
    snippets
}
