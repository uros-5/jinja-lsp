**tojson**

Dumps a value to JSON.

This filter is only available if the `json` feature is enabled.  The resulting
value is safe to use in HTML as well as it will not contain any special HTML
characters.  The optional parameter to the filter can be set to `true` to enable
pretty printing.  Not that the `"` character is left unchanged as it s the
JSON string delimiter.  If you want to pass JSON serialized this way into an
HTTP attribute use single quoted HTML attributes:

```jinja
<script>
  const GLOBAL_CONFIG = {{ global_config|tojson }};
</script>
<a href="#" data-info="{{ json_object|tojson }}">...</a>
```

