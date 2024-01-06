**batch**

Batch items.

This filter works pretty much like `slice` just the other way round. It

returns a list of lists with the given number of items. If you provide a

second parameter this is used to fill up missing items.

```jinja

<table>

{% for row in items|batch(3, "&nbsp;") %}
<tr>
{% for column in row %}
   <td>{{ column }}</td>
 {% endfor %}
 </tr>
 {% endfor %}
</table>
```

