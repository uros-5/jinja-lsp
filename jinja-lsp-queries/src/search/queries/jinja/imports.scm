(
  [

    (statement
      (statement_begin)
      (keyword) @extends_keyword
      (string) @template_name
      (statement_end)
      (#eq? @extends_keyword "extends")
    ) @extends


    (statement
      (statement_begin)
      (keyword) @include_keyword
      (string) @template_name
      (statement_end)
      (#eq? @include_keyword "include")
    ) @include

    (statement
      (statement_begin)
      (keyword) @from_keyword
      (string) @template_name
      (keyword)? @import_keyword
      (identifier)? @import_identifier
      (#not-match? @import_identifier "(^\\d)")
      (statement_end)
      (#eq? @from_keyword "from")
      (#eq? @import_keyword "import")
    ) @from


    (statement
      (statement_begin)
      (keyword) @import_keyword
      (string) @template_name
      (identifier)? @as_keyword
      (identifier)? @import_identifier
      (#not-match? @import_identifier "(^\\d)")
      (#eq? @import_keyword "import")
      (#eq? @as_keyword "as")
      (statement_end)
    ) @import

    (ERROR) @error
  ]
)
