{%- if let CallableKind::Constructor { self_type, primary: true } = callable.kind %}
this.uniffiHandle = uniffiReturn
{%- else %}
{%- match callable.return_strategy() %}
{%- when ReturnStrategy::FfiBuffer(return_type) %}
val uniffiReader = uniffi.FfiBufferCursor(uniffiBuffer)
return uniffi.{{ return_type.read_fn_kt() }}(uniffiReader)
{%- when ReturnStrategy::Primitive(type_node, _) %}
return uniffi.{{ type_node.lift_fn_kt() }}(uniffiReturn)
{%- when ReturnStrategy::Reconstruct(_, _) %}
{# Rust calls the reconstruct function via JNI, so we can just return the value directly #}
return uniffiReturn
{%- when ReturnStrategy::Void %}
{%- endmatch %}
{%- endif %}
