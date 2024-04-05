use crate::to_input_edit::to_position2;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, Range, TextEdit,
};
use tree_sitter::{Point, Query, QueryCapture, QueryCursor, Tree};

#[derive(Default, Debug)]
pub struct Snippets {
    start: Point,
    end: Point,
    keyword: Point,
    pub is_error: bool,
}

impl Snippets {
    pub fn check(&mut self, name: &str, capture: &QueryCapture<'_>) -> Option<()> {
        match name {
            "start" => {
                let start = capture.node.start_position();
                self.start = start;
            }
            "end" => {
                let end = capture.node.end_position();
                self.end = end;
            }
            "error_block" => {
                self.is_error = true;
                self.end = capture.node.end_position();
                return None;
            }
            "keyword" => {
                self.keyword = capture.node.start_position();
            }
            _ => (),
        }
        Some(())
    }

    pub fn to_complete(&self, trigger_point: Point) -> Option<Range> {
        if self.is_error && trigger_point <= self.end {
            return Some(Range::default());
        }
        if self.is_error && trigger_point >= self.start && trigger_point <= self.end {
            if self.keyword >= self.start && self.keyword <= self.end {
                return None;
            }
            let start_position = to_position2(trigger_point);
            let mut end_position = to_position2(trigger_point);
            end_position.character += 1;
            return Some(Range::new(start_position, end_position));
        }
        None
    }
}

pub fn snippets_query(
    query: &Query,
    tree: &Tree,
    mut trigger_point: Point,
    text: &str,
    all: bool,
) -> Snippets {
    let closest_node = tree.root_node();
    let mut snippets = Snippets::default();
    let mut cursor_qry = QueryCursor::new();
    let capture_names = query.capture_names();
    let matches = cursor_qry.matches(query, closest_node, text.as_bytes());
    trigger_point.column += 2;
    let captures = matches.into_iter().flat_map(|m| {
        m.captures
            .iter()
            .filter(|capture| all || capture.node.start_position() <= trigger_point)
    });
    for capture in captures {
        let name = &capture_names[capture.index as usize];
        let check = snippets.check(name, capture);
        if check.is_none() {
            break;
        }
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
            r#" for ${1:i} in ${2:items} %}
{% endfor %}
"#,
        ),
        (
            "for2",
            "For loop with key and value",
            r#" for (${1:key}, ${2:value}) in ${3:items} %}
{% endfor %}
"#,
        ),
        (
            "with",
            "With block",
            r#" with $1 %}
{% endwith %}
"#,
        ),
        (
            "set1",
            "Set variable that is current scope",
            r#" set ${1:key} = ${2:value} %}
            "#,
        ),
        (
            "set2",
            "Set with scope",
            r#" set ${1:data} %}
{% endset %}

"#,
        ),
        (
            "include",
            "Include template",
            r#" include "$1" %}
            "#,
        ),
        (
            "from",
            "Import from other template",
            r#" from "$1" import ${2:module} %}
            "#,
        ),
        (
            "import",
            "Import entire template as module",
            r#" import "$1" as ${2:module} %}
            "#,
        ),
        (
            "extends",
            "Extend parent template",
            r#" extends "$1" %}
            "#,
        ),
        (
            "if1",
            "If statement",
            r#" if $1 %}
{% endif %}
"#,
        ),
        (
            "if2",
            "If statement",
            r#" if $1 %}
{% elif $2 %}
{% endif %}
"#,
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
