{%- if callable.uses_buffer() %}
val uniffiReader = FfiBufferCursor(uniffiBuffer)
{%- endif %}
{% for a in meth.callable.arguments %}
{%- match a.strategy %}
{%- when ArgStrategy::Primitive(ffi_arg) %}
val {{ a.name_kt() }} = {{ a.ty.lift_fn_kt() }}({{ ffi_arg.name_kt() }})
{%- when ArgStrategy::Deconstruct(ffi_args) %}
val {{ a.name_kt() }} = {{ a.ty.lift_fn_kt() }}(
    {%- for ffi_arg in ffi_args %}
    {{ ffi_arg.name_kt() }},
    {%- endfor %}
)
{%- when ArgStrategy::FfiBuffer %}
val {{ a.name_kt() }} = {{ a.ty.read_fn_kt() }}(uniffiReader)
{%- endmatch %}
{%- endfor %}
