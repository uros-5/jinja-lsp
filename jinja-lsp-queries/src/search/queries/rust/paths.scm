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
