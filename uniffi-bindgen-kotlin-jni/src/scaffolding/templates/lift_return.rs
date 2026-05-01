{%- match return_strategy %}
{%- when ReturnStrategy::FfiBuffer(return_type) %}
let uniffi_return = uniffi_buf.with_cursor(|uniffi_reader| {
    {{ return_type.read_fn_rs() }}(uniffi_reader)
})?;
{%- when ReturnStrategy::Primitive(type_node, _) %}
let uniffi_return = {{ type_node.lift_fn_rs() }}(uniffi_env, uniffi_return)?;
{%- when ReturnStrategy::Reconstruct(type_node, ffi_types) %}
{%- if is_async %}
{# For async functions, the Kotlin code passes the primitive values to the completion functions #}
let uniffi_return = {{ type_node.lift_fn_rs() }}(
    uniffi_env,
    {%- for _ in ffi_types %}
    uniffi_return{{ loop.index0 }},
    {%- endfor %}
)?;
{% else %}
{# Sync functions, use the `set_callback_return_fn_kt` function to handle the return #}
{% endif %}
{%- when ReturnStrategy::Void %}
let uniffi_return = ();
{%- endmatch %}
