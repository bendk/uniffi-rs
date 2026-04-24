{%- if callable.is_primary_constructor() %}
val uniffiReader = uniffi.FfiBufferCursor(uniffiBuffer)
this.uniffiHandle = uniffi.readLong(uniffiReader)
{%- else %}
{%- match callable.return_strategy() %}
{%- when ReturnStrategy::FfiBuffer(return_type) %}
val uniffiReader = uniffi.FfiBufferCursor(uniffiBuffer)
return uniffi.{{ return_type.read_fn_kt() }}(uniffiReader)
{%- when ReturnStrategy::Primitive(type_node, _) %}
return uniffi.{{ type_node.lift_fn_kt() }}(uniffiReturn)
{%- when ReturnStrategy::Void %}
{%- endmatch %}
{%- endif %}
