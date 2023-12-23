**select**

Creates a new sequence of values that pass a test.

Filters a sequence of objects by applying a test to each object.
Only values that pass the test are included.
If no test is specified, each object will be evaluated as a boolean.

```jinja
{{ [1, 2, 3, 4]|select("odd") }} -> [1, 3]
{{ [false, null, 42]|select }} -> [42]
```

