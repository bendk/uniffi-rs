{%- if callable.has_receiver() || callable.has_ffi_buffer_arg() %}
let (
    {%- if callable.has_receiver() %}
    uniffi_self,
    {%- endif %}
    {%- for arg in callable.arguments %}
    {%- if arg.uses_buffer() %}
    {{ arg.name_rs() }},
    {%- endif %}
    {%- endfor %}
) = uniffi_buf.with_cursor(|uniffi_reader| Ok((
    {%- if let Some(receiver_type) = callable.receiver_type() %}
    {{ receiver_type.read_fn_rs() }}(uniffi_reader)?,
    {%- endif %}
    {%- for arg in callable.arguments %}
    {%- if arg.uses_buffer() %}
    {{ arg.ty.read_fn_rs() }}(uniffi_reader)?,
    {%- endif %}
    {%- endfor %}
)))?;
{%- endif %}

{%- for arg in callable.arguments %}
{%- if let ArgStrategy::Primitive(ffi_arg) = arg.strategy %}
let {{ arg.name_rs() }} = {{ arg.ty.lift_fn_rs() }}(uniffi_env, {{ ffi_arg.name_rs() }})?;
{%- endif %}
{%- endfor %}
