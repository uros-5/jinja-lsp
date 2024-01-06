**selectattr**

Creates a new sequence of values of which an attribute passes a test.

This functions like [`select`] but it will test an attribute of the
object itself:

```jinja
{{ users|selectattr("is_active") }} -> all users where x.is_active is true
{{ users|selectattr("id", "even") }} -> returns all users with an even id
```

