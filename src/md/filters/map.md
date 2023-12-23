**map**

Applies a filter to a sequence of objects or looks up an attribute.

This is useful when dealing with lists of objects but you are really
only interested in a certain value of it.
The basic usage is mapping on an attribute. Given a list of users
you can for instance quickly select the username and join on it:

```jinja
{{ users|map(attribute="username")|join(, ) }}
```
You can specify a `default` value to use if an object in the list does
not have the given attribute.
```jinja
{{ users|map(attribute="username", default="Anonymous")|join(", ") }}
```

Alternatively you can have `map` invoke a filter by passing the name of the
filter and the arguments afterwards. A good example would be applying a
text conversion filter on a sequence:
```jinja
Users on this page: {{ titles|map("lower")|join(, ) }}
```

