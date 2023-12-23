**slice**

Slice an iterable and return a list of lists containing
those items.

Useful if you want to create a div containing three ul tags that
represent columns:

 ```jinja

<div class="columnwrapper">
 {% for column in items|slice(3) %}
   <ul class="column-{{ loop.index }}">
   {% for item in column %}
     <li>{{ item }}</li>
   {% endfor %}
   </ul>
 {% endfor %}
 </div>
 ```
 If you pass it a second argument it’s used to fill missing values on the

 last iteration.

