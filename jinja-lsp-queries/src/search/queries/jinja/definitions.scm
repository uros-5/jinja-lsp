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
