**unique**

Returns a list of unique items from the given iterable.

```jinja
{{ ["foo", "bar", "foobar", "foobar"]|unique|list }}
  -> ["foo", "bar", "foobar"]
```
The unique items are yielded in the same order as their first occurrence
in the iterable passed to the filter.  The filter will not detect
duplicate objects or arrays, only primitives such as strings or numbers.

