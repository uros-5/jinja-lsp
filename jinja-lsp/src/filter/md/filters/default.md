**default**

If the value is undefined it will return the passed default value,
otherwise the value of the variable:

```jinja
|a - b| = {{ (a - b)|abs }}
  -> |2 - 4| = 2
 ```
