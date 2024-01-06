**indent**

indents Value with spaces

The first optional parameter to the filter can be set to `true` to
indent the first line. The parameter defaults to false.
the second optional parameter to the filter can be set to `true`
to indent blank lines. The parameter defaults to false.
This filter is useful, if you want to template yaml-files

```jinja
example:
  config:
{{ global_conifg|indent(2) }}          # does not indent first line
{{ global_config|indent(2,true) }}     # indent whole Value with two spaces
{{ global_config|indent(2,true,true)}} # indent whole Value and all blank lines
``` 
