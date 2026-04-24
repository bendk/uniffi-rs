{%- if !callable.arguments.is_empty() %}
uniffi_buf.with_cursor(|uniffi_writer| {
    {%- for a in callable.arguments %}
    {{ a.ty.write_fn_rs() }}(uniffi_writer, {{ a.name_rs() }})?;
    {%- endfor %}
    Ok(())
})?;
{%- endif %}

