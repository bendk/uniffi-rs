{%- if !callable.arguments.is_empty() || callable.has_receiver() %}
let (
    {%- if callable.has_receiver() %}
    uniffi_self,
    {%- endif %}
    {%- for arg in callable.arguments %}
    {{ arg.name_rs() }},
    {%- endfor %}
) = uniffi_buf.with_cursor(|uniffi_reader| Ok((
    {%- if let Some(receiver_type) = callable.receiver_type() %}
    {{ receiver_type.read_fn_rs() }}(uniffi_reader)?,
    {%- endif %}
    {%- for arg in callable.arguments %}
    {{ arg.ty.read_fn_rs() }}(uniffi_reader)?,
    {%- endfor %}
)))?;
{%- endif %}
