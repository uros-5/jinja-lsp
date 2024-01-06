**urlencode**

URL encodes a value.

If given a map it encodes the parameters into a query set, otherwise it
encodes the stringified value.  If the value is none or undefined, an
empty string is returned.

```jinja
<a href="/search?{{ {"q": "my search", "lang": "fr"}|urlencode }}">Search</a>
```

