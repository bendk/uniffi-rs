{%- if let Some(return_ty) = callable.return_type() %}
let uniffi_return = uniffi_buf.with_cursor(|uniffi_reader| {
    {{ return_ty.read_fn_rs() }}(uniffi_reader)
})?;
{%- else %}
let uniffi_return = ();
{%- endif %}
{%- if callable.throws_type().is_some() %}
return Ok(Ok(uniffi_return));
{%- else %}
return Ok(uniffi_return);
{%- endif %}
