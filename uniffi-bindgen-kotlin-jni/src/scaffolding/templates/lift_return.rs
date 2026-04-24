{%- match return_strategy %}
{%- when ReturnStrategy::FfiBuffer(return_type) %}
let uniffi_return = uniffi_buf.with_cursor(|uniffi_reader| {
    {{ return_type.read_fn_rs() }}(uniffi_reader)
})?;
{%- when ReturnStrategy::Primitive(type_node, _) %}
let uniffi_return = {{ type_node.lift_fn_rs() }}(uniffi_env, uniffi_return)?;
{%- when ReturnStrategy::Void %}
let uniffi_return = ();
{%- endmatch %}

{%- if throws_type.is_some() %}
return ::std::result::Result::Ok(::std::result::Result::Ok(uniffi_return));
{%- else %}
return ::std::result::Result::Ok(uniffi_return);
{%- endif %}

