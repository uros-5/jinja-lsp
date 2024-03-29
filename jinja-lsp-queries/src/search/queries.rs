use tree_sitter::Query;

#[derive(Debug)]
pub struct Queries2 {
    pub jinja_definitions: Query,
    pub jinja_objects: Query,
    pub jinja_imports: Query,
    pub rust_definitions: Query,
    pub rust_templates: Query,
    pub jinja_snippets: Query,
}

impl Clone for Queries2 {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl Default for Queries2 {
    fn default() -> Self {
        Self {
            jinja_definitions: Query::new(tree_sitter_jinja2::language(), DEFINITIONS).unwrap(),
            jinja_objects: Query::new(tree_sitter_jinja2::language(), OBJECTS).unwrap(),
            rust_definitions: Query::new(tree_sitter_rust::language(), RUST_DEFINITIONS).unwrap(),
            jinja_imports: Query::new(tree_sitter_jinja2::language(), JINJA_IMPORTS).unwrap(),
            rust_templates: Query::new(tree_sitter_rust::language(), RUST_TEMPLATES).unwrap(),
            jinja_snippets: Query::new(tree_sitter_jinja2::language(), JINJA_SNIPPETS).unwrap(),
        }
    }
}

const DEFINITIONS: &str = r#"


(
  [
    (statement
      (statement_begin)
      (keyword) @for_keyword
      [
        (
          (operator)? @open_par
          (identifier)? @for_key
          .
          (operator)? @comma
          .
          (identifier)? @for_value
          .
          (operator)? @close_par
          (_).
        ) @for2

        (
          (identifier) @for_key
        ) @for1
      ]


      (#eq? @open_par "\(")
      (#match-eq? @comma ",")
      (#eq? @close_par "\)")
      (#not-match? @for_key "(^\\d+$)")
      (#not-match? @for_value "(^\\d+$)")
  
      (keyword) @in
      (#eq @in "in")
      (#eq? @for_keyword "for")
      (identifier) @for_items
      (_)? @other
      (statement_end) @range_start
    ) @for        

    (
      (statement
        (statement_begin)
        (keyword) @set_keyword
        (identifier) @set_identifier
        (operator)? @equals
        (_)? @others
        (statement_end) @range_start

        (#eq? @set_keyword "set")
        (#not-match? @set_identifier "(^\\d+$)")
        (#eq? @equals "= ")
      )
    ) @set
    
    (statement
      (statement_begin)
      (keyword) @with_keyword
      (identifier) @with_identifier
      (#eq? @with_keyword "with")
      (#not-match? @with_identifier "(^\\d+$)")
      (statement_end) @range_start
    ) @with

    (statement
      (statement_begin)
      (keyword) @macro_keyword
      (identifier) @macro_identifier
      (#eq? @macro_keyword "macro")
      (#not-match? @macro_identifier "(^\\d+$)")
      (statement_end) @range_start
    ) @macro

    (statement
      (statement_begin)
      (keyword) @block_keyword
      (identifier) @block_identifier
      (#eq? @block_keyword "block")
      (#not-match? @block_identifier "(^\\d+$)")
      (statement_end) @range_start
    ) @block
    
    
    (statement
    	(statement_begin)
        (keyword) @ifkeyword
        (#eq? @ifkeyword "if")
        (statement_end) @range_start
    ) @if
    
    (statement
    	(statement_begin)
        (keyword) @elifkeyword
        (#eq? @elifkeyword "elif")
        (statement_end) @range_start
    ) @elif

    (statement
    	(statement_begin)
        (keyword) @elsekeyword
        (#eq? @elsekeyword "else")
        (statement_end) @range_start
    ) @else
    
    
    (statement
        (statement_begin)
        (keyword) @filterkeyword
        (#eq? @filterkeyword "filter")
        (statement_end) @range_start
    ) @filter
    
    (statement
        (statement_begin)
        (keyword) @autokeyword
        (#eq? @autokeyword "autoescape")
        (statement_end) @range_start
    ) @autoescape
    
    (statement
        (statement_begin)
        (keyword) @rawkeyword
        (#eq? @rawkeyword "raw")
        (statement_end) @range_start
    ) @raw
  ]
)
[
	(statement
      (statement_begin) @range_end
      (keyword) @endkeyword
      (#match? @endkeyword "^end")
      (statement_end)
    ) @ended
]

"#;

const OBJECTS: &str = r#"
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
        )

        (expression) @expr

    ]
) 
"#;

pub static RUST_DEFINITIONS: &str = r#"
([
	(macro_invocation
    	(identifier) @context
        (#eq? @context "context")
    ) @macro
    
    (token_tree
    	(identifier) @key_id
        (#not-eq? @key_id "context")
    )
    
    (
    	(field_expression
        	(identifier) @jinja
            (field_identifier) @method
        )
        (arguments
        	(string_literal)+ @name
        )
    
        (#eq? @jinja "jinja")
        (#match? @method "(add_global|add_filter|add_function)")
    
    ) @function
])
"#;

const JINJA_IMPORTS: &str = r#"
  
(
  [

    (statement
      (statement_begin)
      (keyword) @extends_keyword
      (string) @template_name
      (statement_end)
      (#eq? @extends_keyword "extends")
    ) @extends


    (statement
      (statement_begin)
      (keyword) @include_keyword
      (string) @template_name
      (statement_end)
      (#eq? @include_keyword "include")
    ) @include

    (statement
      (statement_begin)
      (keyword) @from_keyword
      (string) @template_name
      (keyword)? @import_keyword
      (identifier)? @import_identifier
      (#not-match? @import_identifier "(^\\d)")
      (statement_end)
      (#eq? @from_keyword "from")
      (#eq? @import_keyword "import")
    ) @from


    (statement
      (statement_begin)
      (keyword) @import_keyword
      (string) @template_name
      (identifier)? @as_keyword
      (identifier)? @import_identifier
      (#not-match? @import_identifier "(^\\d)")
      (#eq? @import_keyword "import")
      (#eq? @as_keyword "as")
      (statement_end)
    ) @import
  ]
)
"#;

const RUST_TEMPLATES: &str = r#"
(call_expression
  	[
    	(field_expression
        	(field_identifier) @method_name
        )
        (identifier) @method_name
        (#any-of? @method_name "render_jinja" "get_template")
      ;;(#match? @method_name "(render_jinja|get_template)")
    ]
    (arguments
      (string_literal)+ @template_name
    )
)
"#;

const JINJA_SNIPPETS: &str = r#"
[
	(statement) @block
    (ERROR
    	(ERROR)
    ) @error1
	
    (
      (keyword) @missing
      (#eq? @missing "")
    )
    
    (
    	(keyword) @longer_keyword
    )
]
"#;
