**rejectattr**

Creates a new sequence of values of which an attribute does not pass a test.
This functions like [`select`] but it will test an attribute of the
object itself:

```jinja
{{ users|rejectattr("is_active") }} -> all users where x.is_active is false
{{ users|rejectattr("id", "even") }} -> returns all users with an odd id
```

