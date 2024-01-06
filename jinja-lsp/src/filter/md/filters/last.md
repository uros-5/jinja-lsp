**last**

Returns the last item from a list.

If the list is empty `undefined` is returned.

```jinja
<h2>Most Recent Update</h2>
{% with update = updates|last %}
<dl>
  <dt>Location
  <dd>{{ update.location }}
  <dt>Status
  <dd>{{ update.status }}
 </dl>
{% endwith %}
 ```


