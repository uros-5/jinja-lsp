(
    [
        (dotted_identifier
          attribute: (identifier) @attribute
        ) @object

        (
          (identifier) @just_id
          (#not-match? @just_id "(^\\d+$)")
        )

        (
          (filter_operator) @filter
        )

        (expression_begin) @expr_start
        (expression_end) @expr_end

        (statement_begin) @statement_start
        (statement_end) @statement_end


        (
          (keyword) @is
          (#eq? @is "is")
        )
        
        (ERROR) @error

    ]
) 
