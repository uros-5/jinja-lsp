pub static JINJA_DEF: &str = r#"

(
  [
    (
      (keyword) @key_name
      (identifier) @key_id
      (#not-match? @key_id "(^\\d+$)")
    )    
  ]
  (#match? @key_name "(set|macro|for|with)")
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
        (identifier) @key_id
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

pub static GOTO_DEF_JINJA: &str = r#"
( 
   [	
    (expression
        (expression_begin)
        identifier: (identifier) @key_id
        (operator) @pipe
        (identifier)? @filter
        (#not-eq? @filter @key_id) 
        (#match? @pipe "|")
    ) @expr_with_pipes

  	(
      	(statement
          	_
              (keyword) @key_name
              identifier: (identifier)? @key_id
              (#match? @key_name "(if|in|and|or|elif)")
              (_)
          ) 
    ) @just_statement

    (expression
        (identifier) @key_id
    ) @basic_expr
    (#not-match? @basic_expr "\\|")

    ]
  )
"#;

pub static TEMP: &str = r#"
(
    [
        (expression
            (
                (expression_begin)
                identifier: (identifier) @key_id
            )
            (operator)? @pipe
            (identifier)? @filter
            (#not-eq? @filter @key_id) 
            (#match? @pipe "|")
        ) @temp_expression

        (
          	(statement
            	_
                  (keyword) @key_name
                  identifier: (identifier) @key_id
                  (#match? @key_name "(if|in|and|or|elif)")
                  _
              ) 
        ) @just_statement

    ]
)
"#;
