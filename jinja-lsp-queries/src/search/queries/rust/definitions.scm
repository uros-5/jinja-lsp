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

    (ERROR) @error
])
