{%- if callable.uses_buffer() %}
val uniffiBuffer = uniffi.Scaffolding.ffiBufferNew()
try {
{%- endif %}
{% filter indent(4) %}{% include "LowerArgs.kt" %}{% endfilter %}
{%- if !callable.is_async %}
uniffi.Scaffolding.{{ jni_method_name }}(
    {%- if callable.uses_buffer() %}uniffiBuffer,{% endif %}
)
{% filter indent(4) %}{% include "LiftReturn.kt" %}{% endfilter %}
{%- else %}
val uniffiFuture = uniffi.Scaffolding.{{ jni_method_name }}(
    {%- if callable.uses_buffer() %}uniffiBuffer,{% endif %}
)
val uniffiCode = uniffi.awaitFuture(uniffiFuture);
when(uniffiCode) {
    uniffi.UNIFFI_RUST_FUTURE_CANCELLED -> throw kotlin.coroutines.cancellation.CancellationException()
    uniffi.UNIFFI_RUST_FUTURE_COMPLETE -> {
        {% filter indent(12) %}{% include "LiftReturn.kt" %}{% endfilter %}
    }
    else -> throw uniffi.InternalException("Error polling Rust future (code: $uniffiCode)")
}
{%- endif %}
{%- if callable.uses_buffer() %}
} finally {
    uniffi.Scaffolding.ffiBufferFree(uniffiBuffer)
}
{%- endif %}
