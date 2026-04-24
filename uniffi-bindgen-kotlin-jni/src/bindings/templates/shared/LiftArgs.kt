{% for a in meth.callable.arguments %}
{%- if loop.first %}
val uniffiReader = FfiBufferCursor(uniffiBuffer)
{%- endif %}
{%- match a.strategy %}
{%- when ArgStrategy::Primitive(ffi_arg) %}
val {{ a.name_kt() }} = {{ a.ty.lift_fn_kt() }}({{ ffi_arg.name_kt() }})
{%- when ArgStrategy::FfiBuffer %}
val {{ a.name_kt() }} = {{ a.ty.read_fn_kt() }}(uniffiReader)
{%- endmatch %}
{%- endfor %}
