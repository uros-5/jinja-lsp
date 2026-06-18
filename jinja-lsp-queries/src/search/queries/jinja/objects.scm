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
          (operator) @pipe
        )

        (expression) @expr

        (
          (keyword) @is
          (#eq? @is "is")
        )
        
        (ERROR) @error

    ]
) 
