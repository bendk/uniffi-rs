{%- if callable.has_ffi_buffer_arg() %}
uniffi_buf.with_cursor(|uniffi_writer| {
    {%- for a in callable.arguments %}
    {%- if a.uses_buffer() %}
    {{ a.ty.write_fn_rs() }}(uniffi_writer, {{ a.name_rs() }})?;
    {%- endif %}
    {%- endfor %}
    Ok(())
})?;
{%- endif %}

{%- for arg in callable.arguments %}
{%- match arg.strategy %}
{%- when ArgStrategy::Primitive(ffi_arg) %}
let {{ ffi_arg.name_rs() }} = {{ arg.ty.lower_fn_rs() }}(uniffi_env, {{ arg.name_rs() }})?;
{%- else %}
{%- endmatch %}
{%- endfor %}

