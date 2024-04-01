use tree_sitter::Query;

#[derive(Debug)]
pub struct Queries {
    pub jinja_definitions: Query,
    pub jinja_objects: Query,
    pub jinja_imports: Query,
    pub rust_definitions: Query,
    pub rust_templates: Query,
    pub jinja_snippets: Query,
}

impl Clone for Queries {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl Default for Queries {
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

        (ERROR) @error

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

    (ERROR) @error
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
	(statement_begin) @start
	(statement_end) @end
    (ERROR
        (ERROR) @error
    ) @error_block 
	
    (
        (keyword) @keyword
    )
]
"#;

const DEFINITIONS: &str = r#"
(statement
  (statement_begin) @scope_end

  (statement_end) @scope_start
)

(
  (identifier) @id
  (#not-match? @id "^(\\d+)$")
)
    (
    	(keyword) @definition
        (#match? @definition "^(for|set|with|macro|block)$")
    )
    
    (
    	(keyword) @scope
        (#match? @scope "^(if|elif|else|filter|autoescape|raw)$")
    )

    (
        (keyword) @endblock
        (#match? @endblock "^end")
    )

(
	(operator) @equals
    (#match? @equals "=")
)

(ERROR) @error
"#;
