{%- if callable.is_primary_constructor() %}
val uniffiReader = uniffi.FfiBufferCursor(uniffiBuffer)
this.uniffiHandle = uniffi.readLong(uniffiReader)
{%- elif let Some(return_ty) = callable.return_type() %}
val uniffiReader = uniffi.FfiBufferCursor(uniffiBuffer)
return uniffi.{{ return_ty.read_fn_kt() }}(uniffiReader)
{%- endif %}
