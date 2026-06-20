(
  (subscript
    (attribute
      object: (identifier)* @object
      attribute: (identifier) @field
      (#match? @field "^(globals|filters)$")
      (#eq? @object "jinja_env")
    )
      (string
      	(string_content) @key_id
      )   
  )
)

(
  
    [
    	(call
        	function: (attribute
            	attribute: (identifier) @method
            )
            arguments: (argument_list
              (keyword_argument
                  name: (identifier) @key_id
              )
            )
        )
  
        (call
            function: (identifier) @method
            arguments: (argument_list
            	(keyword_argument
                	name: (identifier) @key_id
                )
            )
        )

        (call
            function: (identifier) @method
            arguments: (argument_list
            	(dictionary
                  (pair
                    key: (string
                      (string_content) @key_id
                    )
                  )
                )
            )
        )        
    ]
    (#match? @method "^(render_template|render|render_async)$")
  
)


(call
	function: (attribute
    	attribute: (identifier) @star_api
        (#eq? @star_api "TemplateResponse")
    )
  arguments: (argument_list
  	(keyword_argument
      	name: (identifier) @context_kw
        value: (dictionary
        	(pair
            	key: (string
                	(string_content) @key_id
                )
            )
        )
          (#eq? @context_kw "context")
      )
  )
)
  
  (ERROR) @error

