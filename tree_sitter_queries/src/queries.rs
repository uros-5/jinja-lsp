use std::{cell::RefCell, collections::HashMap};

use tree_sitter::{Node, Point, Query, QueryCursor, TreeCursor};

use crate::capturer::{CaptureDetails, Capturer};

#[derive(Debug)]
pub struct Queries {
    pub jinja_init: Query,
}

impl Clone for Queries {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl Default for Queries {
    fn default() -> Self {
        Self {
            jinja_init: Query::new(tree_sitter_jinja2::language(), INIT).unwrap(),
        }
    }
}

pub fn query_props<T: Capturer>(
    node: Node<'_>,
    source: &str,
    trigger_point: Point,
    query: &Query,
    all: bool,
    mut capturer: T,
) -> (HashMap<String, CaptureDetails>, T) {
    let mut cursor_qry = QueryCursor::new();
    let capture_names = query.capture_names();
    let matches = cursor_qry.matches(query, node, source.as_bytes());

    (
        matches
            .into_iter()
            .flat_map(|m| {
                m.captures
                    .iter()
                    .filter(|capture| all || capture.node.start_position() <= trigger_point)
            })
            .fold(HashMap::new(), |mut acc, capture| {
                capturer.save_by(capture, &mut acc, capture_names, source);
                acc
            }),
        capturer,
    )
}

pub static INIT: &str = r#"
(
	[
    	
        (statement
          (statement_begin)
          (keyword) @keyword
          (identifier) @variable
          (#not-match? @variable "\\d")
          _
        ) @start_statement
        
        (statement
          (statement_begin)
          (keyword) @end_keyword
          (#match? @end_keyword "^end")
          (statement_end)
        ) @end_statement
    ]
)
"#;
