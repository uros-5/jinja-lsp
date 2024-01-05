**attr**

Looks up an attribute.

In MiniJinja this is the same as the `[]` operator.  In Jinja2 there is a
small difference which is why this filter is sometimes used in Jinja2
templates.  For compatibility it s provided here as well.

```jinja

{{ value["key"] == value|attr("key") }} -> true

```

