
echo 'batch
\n Batch items.
\n This filter works pretty much like `slice` just the other way round. It
\n returns a list of lists with the given number of items. If you provide a
\n second parameter this is used to fill up missing items.
\n ```jinja
\n <table>
\n   {% for row in items|batch(3, "&nbsp;") %}
\n   <tr>
\n   {% for column in row %}
\n     <td>{{ column }}</td>
\n   {% endfor %}
\n   </tr>
\n   {% endfor %}
\n </table>
\n ```
' > batch.md



echo 'indent
\n indents Value with spaces
\n The first optional parameter to the filter can be set to `true` to
\n indent the first line. The parameter defaults to false.
\n the second optional parameter to the filter can be set to `true`
\n to indent blank lines. The parameter defaults to false.
\n This filter is useful, if you want to template yaml-files
\n ```jinja
\n example:
\n   config:
\n {{ global_conifg|indent(2) }}          # does not indent first line
\n {{ global_config|indent(2,true) }}     # indent whole Value with two spaces
\n {{ global_config|indent(2,true,true)}} # indent whole Value and all blank lines
\n ``` ' > indent.md



echo 'select\n
\n Creates a new sequence of values that pass a test.
\n Filters a sequence of objects by applying a test to each object.
\n Only values that pass the test are included.
\n If no test is specified, each object will be evaluated as a boolean.
\n ```jinja
\n {{ [1, 2, 3, 4]|select("odd") }} -> [1, 3]
\n {{ [false, null, 42]|select }} -> [42]
\n ```
' > select.md


echo 'tojson\n
\n Dumps a value to JSON.
\n This filter is only available if the `json` feature is enabled.  The resulting
\n value is safe to use in HTML as well as it will not contain any special HTML
\n characters.  The optional parameter to the filter can be set to `true` to enable
\n pretty printing.  Not that the `"` character is left unchanged as it s the
\n JSON string delimiter.  If you want to pass JSON serialized this way into an
\n HTTP attribute use single quoted HTML attributes:
\n ```jinja
\n <script>
\n   const GLOBAL_CONFIG = {{ global_config|tojson }};
\n </script>
\n <a href="#" data-info="{{ json_object|tojson }}">...</a>
\n ```
' > tojson.md



echo 'default\n\n If the value is undefined it will return the passed default value,
\n otherwise the value of the variable:
\n ```jinja
|a - b| = {{ (a - b)|abs }}
  -> |2 - 4| = 2
\n ```' > default.md 
