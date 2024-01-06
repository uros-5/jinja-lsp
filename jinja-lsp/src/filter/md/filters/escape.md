**escape**

Escapes a string.  By default to HTML.

By default this filter is also registered under the alias `e`.  Note that
this filter escapes with the format that is native to the format or HTML
otherwise.  This means that if the auto escape setting is set to
`Json` for instance then this filter will serialize to JSON instead.

