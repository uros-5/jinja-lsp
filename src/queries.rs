pub static JINJA_DEF: &str = r#"

(
  [
    (
      (keyword) @key_name
      (identifier) @key_id
    )    
  ]

      (#any-of? @key_name "set" "macro" "for" "with")
)
"#;

pub static JINJA_REF: &str = r#"
(
  [     
    (expression
      (identifier) @key_id
    ) @temp_expression

    (statement
      (keyword) @key_name
      (identifier) @key_id
      (#any-of? @key_name "set" "macro" "for" "with")
    ) @temp_statement
  ]
)
"#;

pub static RUST_DEF: &str = r#"
(macro_invocation
	(identifier) @context
    (token_tree
    	(identifier) @key_id
    )
    (#eq? @context "context")
) @temp_expression  
"#;

pub static JINJA_COMPLETION: &str = r#"
 ( 
   [	
    (expression
        (identifier)
        (operator) @pipe
        (identifier)? @filter
        (#match? @pipe "|")
    ) @pipe_waiting

  	(
      	(statement
          	_
              (keyword) @key_name
              (identifier)? @key_id
              (#match? @key_name "(if|in|and|or|elif)")
              (_)
          ) 
    ) @ident_waiting

    (expression
    	(expression_begin) @start
        (expression_end) @end
    ) @empty_expression
    (#not-match? @empty_expression "\\|")

    ]
  )
"#;
