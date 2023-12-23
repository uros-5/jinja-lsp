**first**  

Returns the first item from a list.

If the list is empty `undefined` is returned.
```jinja
<dl>
  <dt>primary email
  <dd>{{ user.email_addresses|first|default(no userecho first
) }}
 </dl>
 ```

