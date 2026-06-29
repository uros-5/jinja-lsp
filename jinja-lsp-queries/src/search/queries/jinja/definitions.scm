(statement
  (statement_begin)
  (keyword) @keyword
  (statement_end) @statement_start  
  (#not-match? @keyword "end")
  (#any-of? @keyword "if" "elif" "else" "raw" "call" "filter" "autoescape" "trans" "block")
) 

(
  statement
  (statement_begin) 
  (keyword) @keyword
  (statement_end) @statement_end 
  (#not-any-of? @keyword "and" "or" "extends" "include" "import" "from" "debug" "do" "is")
  (#match? @keyword "end")
) 


(statement
  	(statement_begin) 
    (keyword) @macro
    .
    (identifier) @macro_name
    (#not-match? @macro_name "^(\\d+)$")
    (
      (identifier) @macro_parameter
      (#not-match? @macro_parameter "^(\\d+)$")
    )?
    (#eq? @macro "macro")
    (statement_end)
) @macro_statement

(statement
  (statement_begin) 
  .
  (keyword) @for
  .
  (identifier) @for_key
  (
  	(operator) @comma
    .
    (identifier) @for_value
    (#not-match? @for_value "^(\\d+)$")
  )?
  .
  (#not-match? @for_key "^(\\d+)$")
  (#eq? @for "for")
) @for_statement

(statement
  (statement_begin)
  (keyword) @set
  .
  (identifier) @set_variable
  (#not-match? @set_variable "^(\\d+)$")
  .
  (equal_operator) @set_eq_sign
  (#eq? @set "set")
) @set_statement

(statement
	(statement_begin)
  (keyword) @set
  .
  (identifier) @set_variable
  (#not-match? @set_variable "^(\\d+)$")

  (#eq? @set "set")
) @set_statement


(statement
	(statement_begin)
  (keyword) @with
  .
  (identifier) @with_variable
  (#not-match? @with_variable "^(\\d+)$")
  (equal_operator) @with_eq_sign
  (#eq? @with "with")
) @with_definition

(statement
	(statement_begin)
  (keyword) @with
  (#eq? @with "with")
  .
  (statement_end)
) @with_definition 

(statement
	(statement_begin) @block_scope_start
  (keyword) @block
  (identifier) @block_variable
  (#eq? @block "block")
)

(ERROR) @error

