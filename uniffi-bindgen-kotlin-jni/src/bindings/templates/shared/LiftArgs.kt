{% for a in meth.callable.arguments %}
{%- if loop.first %}
val uniffiReader = FfiBufferCursor(uniffiBuffer)
{%- endif %}
val {{ a.name_kt() }} = {{ a.ty.read_fn_kt() }}(uniffiReader)
{%- endfor %}
