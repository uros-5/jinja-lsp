(call
    [
      (attribute
      	(identifier) @method_name
      )
      (identifier) @method_name
      (#any-of? @method_name "render_jinja" "get_template")
    ]
    (argument_list
      (string)+ @template_name
    )
)

(call
	function: (attribute
    	attribute: (identifier) @method_name
        (#eq? @method_name "TemplateResponse")
    )
  arguments: (argument_list
  	(keyword_argument
      	name: (identifier) @name_kw
        value: (string) @template_name
          (#eq? @name_kw "name")
      )
  )
)


(call
	function: (attribute
    	attribute: (identifier) @star_api
        (#eq? @star_api "TemplateResponse")
    )
  arguments: (argument_list
  	(_)
    (string) @template_name
  )
)
