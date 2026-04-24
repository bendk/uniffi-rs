{%- if callable.uses_buffer() %}
val uniffiWriter = uniffi.FfiBufferCursor(uniffiBuffer)
{%- endif %}
{%- if let Some(receiver_type) = callable.receiver_type() %}
uniffi.{{ receiver_type.write_fn_kt() }}(uniffiWriter, this)
{%- endif %}

{%- for arg in callable.arguments %}
uniffi.{{ arg.ty.write_fn_kt() }}(uniffiWriter, {{ arg.name_kt() }})
{%- endfor %}
