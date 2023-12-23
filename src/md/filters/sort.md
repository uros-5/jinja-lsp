**sort**
    
Returns the sorted version of the given list.

The filter accepts a few keyword arguments:
* `case_sensitive`: set to `true` to make the sorting of strings case sensitive.
* `attribute`: can be set to an attribute or dotted path to sort by that attribute
* `reverse`: set to `true` to sort in reverse.

```jinja
{{ [1, 3, 2, 4]|sort }} -> [4, 3, 2, 1]
{{ [1, 3, 2, 4]|sort(reverse=true) }} -> [1, 2, 3, 4]
# Sort users by age attribute in descending order.
{{ users|sort(attribute="age") }}
# Sort users by age attribute in ascending order.
{{ users|sort(attribute="age", reverse=true) }}
 ```


