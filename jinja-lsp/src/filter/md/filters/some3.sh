#!/bin/sh

echo 'safe\n Marks a value as safe.  This converts it into a string.
\n When a value is marked as safe, no further auto escaping will take place.
' > safe.md

echo 'escape\n Escapes a string.  By default to HTML.
\n By default this filter is also registered under the alias `e`.  Note that
\n this filter escapes with the format that is native to the format or HTML
\n otherwise.  This means that if the auto escape setting is set to
\n `Json` for instance then this filter will serialize to JSON instead.
' > escape.md


echo 'upper\n Converts a value to uppercase.
\n ```jinja
\n <h1>{{ chapter.title|upper }}</h1>
\n ```
' > upper.md

echo 'lower\n Converts a value to lowercase.
\n ```jinja
\n <h1>{{ chapter.title|lower }}</h1>
\n ```
' > lower.md

echo 'title\n Converts a value to title case.
\n ```jinja
\n <h1>{{ chapter.title|title }}</h1>
\n ```
' > title.md

echo 'capitalize\n Convert the string with all its characters lowercased
\n apart from the first char which is uppercased.
\n ```jinja
\n <h1>{{ chapter.title|capitalize }}</h1>
\n ```
' > capitalize.md

echo 'replace
\n Does a string replace.
\n It replaces all occurrences of the first parameter with the second.
\n ```jinja
\n {{ "Hello World"|replace("Hello", "Goodbye") }}
\n   -> Goodbye World
\n ```
' > replace.md


echo 'length\n Returns the "length" of the value
\n By default this filter is also registered under the alias `count`.
\n ```jinja
\n <p>Search results: {{ results|length }}
\n ```
' > length.md

echo 'dictsort\n   \n Dict sorting functionality.
\n This filter works like `|items` but sorts the pairs by key first.
\n The filter accepts a few keyword arguments:
\n * `case_sensitive`: set to `true` to make the sorting of strings case sensitive.
\n * `by`: set to `"value"` to sort by value. Defaults to `"key"`.
\n * `reverse`: set to `true` to sort in reverse.
' > dictsort.md 

echo  'items\n Returns a list of pairs (items) from a mapping.
\n This can be used to iterate over keys and values of a mapping
\n at once.  Note that this will use the original order of the map
\n which is typically arbitrary unless the `preserve_order` feature
\n is used in which case the original order of the map is retained.
\n It s generally better to use `|dictsort` which sorts the map by
\n key before iterating.
\n ```jinja
\n <dl>
\n {% for key, value in my_dict|items %}
\n   <dt>{{ key }}
\n   <dd>{{ value }}
\n {% endfor %}
\n </dl>
\n ```"
    ' > items.md


echo 'reverse\nReverses a list or string
\n ```jinja
\n {% for user in users|reverse %}
\n   <li>{{ user.name }}
\n {% endfor %}
\n ``` ' > reverse.md

echo 'trim\n    Trims a value
' >trim.md 

echo 'join\n Joins a sequence by a character
' >join.md 

echo 'abs\n    \n Returns the absolute value of a number.
\n ```jinja
\n |a - b| = {{ (a - b)|abs }}
\n   -> |2 - 4| = 2
\n ```
' > abs.md 



echo 'int\n    \n Converts a value into an integer.
\n ```jinja
\n {{ "42"|int == 42 }} -> true
\n ```
' > int.md 



echo 'float\n    \n Converts a value into a float.
\n ```jinja
\n {{ "42.5"|float == 42.5 }} -> true
\n ```
' > float.md 

echo 'attr\n Looks up an attribute.
\n In MiniJinja this is the same as the `[]` operator.  In Jinja2 there is a
\n small difference which is why this filter is sometimes used in Jinja2
\n templates.  For compatibility it s provided here as well.
\n ```jinja
\n {{ value["key"] == value|attr("key") }} -> true
\n ```
' > attr.md

echo 'round\n    \n Round the number to a given precision.
\n Round the number to a given precision. The first parameter specifies the
\n precision (default is 0).
\n ```jinja
\n {{ 42.55|round }}
\n   -> 43.0
\n ```

' > round.md 

echo 'first  \n Returns the first item from a list.
\n If the list is empty `undefined` is returned.
\n ```jinja
\n <dl>
\n   <dt>primary email
\n   <dd>{{ user.email_addresses|first|default('no userecho 'first\n) }}
\n </dl>
\n ```
' > first.md 

echo 'last\nReturns the last item from a list.
\n If the list is empty `undefined` is returned.
\n ```jinja
\n <h2>Most Recent Update</h2>
\n {% with update = updates|last %}
\n   <dl>
\n     <dt>Location
\n     <dd>{{ update.location }}
\n     <dt>Status
\n     <dd>{{ update.status }}
\n   </dl>
\n {% endwith %}
\n ```

' > last.md 

echo 'min\n    \n Returns the smallest item from the list.
' > min.md 

echo 'max\n    \n Returns the largest item from the list.
' > max.md 

echo 'sort\n    \n Returns the sorted version of the given list.
\n The filter accepts a few keyword arguments:
\n * `case_sensitive`: set to `true` to make the sorting of strings case sensitive.
\n * `attribute`: can be set to an attribute or dotted path to sort by that attribute
\n * `reverse`: set to `true` to sort in reverse.
\n ```jinja
\n {{ [1, 3, 2, 4]|sort }} -> [4, 3, 2, 1]
\n {{ [1, 3, 2, 4]|sort(reverse=true) }} -> [1, 2, 3, 4]
\n # Sort users by age attribute in descending order.
\n {{ users|sort(attribute="age") }}
\n # Sort users by age attribute in ascending order.
\n {{ users|sort(attribute="age", reverse=true) }}
\n ```

' > sort.md 

echo 'list\n Converts the input value into a list.
\n If the value is already a list, then it s returned unchanged.
\n Applied to a map this returns the list of keys, applied to a
\n string this returns the characters.  If the value is undefined
\n an empty list is returned.
' > list.md 

echo 'bool\n Converts the value into a boolean value.
\n This behaves the same as the if statement does with regards to
\n handling of boolean values.
' > bool.md

echo 'slice\n Slice an iterable and return a list of lists containing
\n those items.
\n Useful if you want to create a div containing three ul tags that
\n represent columns:
\n ```jinja
\n <div class="columnwrapper">
\n {% for column in items|slice(3) %}
\n   <ul class="column-{{ loop.index }}">
\n   {% for item in column %}
\n     <li>{{ item }}</li>
\n   {% endfor %}
\n   </ul>
\n {% endfor %}
\n </div>
\n ```
\n If you pass it a second argument it’s used to fill missing values on the
\n last iteration.
' > slice.md

echo 'urlencode
\n URL encodes a value.
\n If given a map it encodes the parameters into a query set, otherwise it
\n encodes the stringified value.  If the value is none or undefined, an
\n empty string is returned.
\n ```jinja
\n <a href="/search?{{ {"q": "my search", "lang": "fr"}|urlencode }}">Search</a>
\n ```
' > urlencode.md

echo 'selectattr\nCreates a new sequence of values of which an attribute passes a test.
\n This functions like [`select`] but it will test an attribute of the
\n object itself:
\n ```jinja
\n {{ users|selectattr("is_active") }} -> all users where x.is_active is true
\n {{ users|selectattr("id", "even") }} -> returns all users with an even id
\n ```
' > selectattr.md

echo 'reject\n Creates a new sequence of values that don t pass a test.
\n This is the inverse of [`select`].
' > reject.md

echo 'rejectattr
\n Creates a new sequence of values of which an attribute does not pass a test.
\n This functions like [`select`] but it will test an attribute of the
\n object itself:
\n ```jinja
\n {{ users|rejectattr("is_active") }} -> all users where x.is_active is false
\n {{ users|rejectattr("id", "even") }} -> returns all users with an odd id
\n ```
' > rejectattr.md
echo 'map
\n Applies a filter to a sequence of objects or looks up an attribute.
\n This is useful when dealing with lists of objects but you are really
\n only interested in a certain value of it.
\n The basic usage is mapping on an attribute. Given a list of users
\n you can for instance quickly select the username and join on it:
\n ```jinja
\n {{ users|map(attribute="username")|join(', ') }}
\n ```
\n You can specify a `default` value to use if an object in the list does
\n not have the given attribute.
\n ```jinja
\n {{ users|map(attribute="username", default="Anonymous")|join(", ") }}
\n ```
\n Alternatively you can have `map` invoke a filter by passing the name of the
\n filter and the arguments afterwards. A good example would be applying a
\n text conversion filter on a sequence:
\n ```jinja
\n Users on this page: {{ titles|map("lower")|join(', ') }}
\n ```
' > map.md

echo 'unique\n Returns a list of unique items from the given iterable.
\n ```jinja
\n {{ ["foo", "bar", "foobar", "foobar"]|unique|list }}
\n   -> ["foo", "bar", "foobar"]
\n ```
\n The unique items are yielded in the same order as their first occurrence
\n in the iterable passed to the filter.  The filter will not detect
\n duplicate objects or arrays, only primitives such as strings or numbers.
' > unique.md

echo 'pprint
    \n Pretty print a variable.
        \n This is useful for debugging as it better shows what s inside an object.
' > pprint.md
