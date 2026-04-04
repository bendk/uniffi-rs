{# We currently only need to construct error types #}

{%- for type_node in root.rust_throws_types() %}
/// Construct a new `{{ type_node.type_kt }}` instance
fun {{ type_node.construct_fn_kt() }}(buffer: Long) : {{ type_node.type_kt }} {
    return {{ type_node.read_fn_kt() }}(uniffi.FfiBufferCursor(buffer))
}

{%- endfor %}
