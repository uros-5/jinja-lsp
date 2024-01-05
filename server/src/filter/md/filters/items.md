**items**

Returns a list of pairs (items) from a mapping.

This can be used to iterate over keys and values of a mapping
at once.

Note that this will use the original order of the map
which is typically arbitrary unless the `preserve_order` feature
is used in which case the original order of the map is retained.
It s generally better to use `|dictsort` which sorts the map by
key before iterating.

```jinja
<dl>
{% for key, value in my_dict|items %}
  <dt>{{ key }}
  <dd>{{ value }}
{% endfor %}
</dl>
```"
    
