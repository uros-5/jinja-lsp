(
    [
        (
            (operator) @dot
            (#eq? @dot "\.")
        )

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
