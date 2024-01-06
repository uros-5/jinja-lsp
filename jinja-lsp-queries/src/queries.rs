

use tree_sitter::{Node, Point, Query, QueryCursor};

use crate::capturer::{Capturer};

#[derive(Debug)]
pub struct Queries {
    pub jinja_init: Query,
    pub jinja_idents: Query,
    pub rust_idents: Query,
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
            jinja_idents: Query::new(tree_sitter_jinja2::language(), OBJECTS).unwrap(),
            rust_idents: Query::new(tree_sitter_rust::language(), RUST).unwrap(),
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
) -> T {
    let mut cursor_qry = QueryCursor::new();
    let capture_names = query.capture_names();
    let matches = cursor_qry.matches(query, node, source.as_bytes());

    matches
        .into_iter()
        .flat_map(|m| {
            m.captures
                .iter()
                .filter(|capture| all || capture.node.start_position() <= trigger_point)
        })
        .for_each(|capture| {
            capturer.save_by(capture, capture_names, source);
        });
    capturer
}

pub static INIT: &str = r#"
(
	[
    	
        (statement
          (statement_begin)
          (keyword)
          (identifier) @variable
          (#not-match? @variable "^(\\d)")
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

pub static OBJECTS: &str = r#"
(
  [
      (
          (operator) @dot
          (#eq? @dot "\.")
      )

      (
        (identifier) @just_id
        (#not-match? @just_id "(^\\d+$)")
      )

      (
        (operator) @pipe
        (#match? @pipe "\\|")
      )

      (expression) @expr

  ]
)
"#;

pub static RUST: &str = r#"
([

    (macro_invocation
    	(identifier) @context
        (token_tree
        	(identifier) @key_id
        )
        (#eq? @context "context")
    ) @context_macro 

    (
    	(field_expression
        	(identifier) @jinja
            (field_identifier) @method
        )
        (arguments
        	(string_literal) @name
        )
    
        (#eq? @jinja "jinja")
        (#match? @method "^(add_global|add_filter|add_function)$")
    
    ) @function
])
"#;
