[
	(statement_begin) @start
	(statement_end) @end
    (ERROR
        (ERROR)? @error
    ) @error_block 
	
    (
        (keyword) @keyword
    )
]
