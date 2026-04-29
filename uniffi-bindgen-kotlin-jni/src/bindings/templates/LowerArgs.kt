{%- if callable.uses_buffer() %}
val uniffiWriter = uniffi.FfiBufferCursor(uniffiBuffer)
{%- endif %}
{%- if let Some(receiver_type) = callable.receiver_type() %}
uniffi.{{ receiver_type.write_fn_kt() }}(uniffiWriter, this)
{%- endif %}

{%- for arg in callable.arguments %}
{%- match arg.strategy %}
{%- when ArgStrategy::Primitive(ffi_arg) %}
val {{ ffi_arg.name_kt() }} = uniffi.{{ arg.ty.lower_fn_kt() }}({{ arg.name_kt() }})
{%- when ArgStrategy::Deconstruct(ffi_args) %}
{%- let deconstructed_arg_name = "uniffiDeconstructed{}"|format(loop.index0) %}
val {{ deconstructed_arg_name }} = uniffi.{{ arg.ty.lower_fn_kt() }}({{ arg.name_kt() }})
{%- for ffi_arg in ffi_args %}
val {{ ffi_arg.name_kt() }} = {{ deconstructed_arg_name }}.v{{ loop.index0 }}
{%- endfor %}
{%- when ArgStrategy::FfiBuffer %}
uniffi.{{ arg.ty.write_fn_kt() }}(uniffiWriter, {{ arg.name_kt() }})
{%- endmatch %}
{%- endfor %}
