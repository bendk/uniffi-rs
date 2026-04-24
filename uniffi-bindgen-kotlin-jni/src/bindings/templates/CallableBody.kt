{% let return_type = callable.return_type() %}
{% let throws_type = callable.throws_type() %}
{%- if callable.uses_buffer() %}
val uniffiBuffer = uniffi.Scaffolding.ffiBufferNew()
try {
{%- endif %}
{% filter indent(4) %}{% include "LowerArgs.kt" %}{% endfilter %}
{%- if !callable.is_async %}
val uniffiReturn = uniffi.Scaffolding.{{ jni_method_name }}(
    {%- if callable.uses_buffer() %}uniffiBuffer,{% endif %}
    {%- for arg in callable.ffi_arguments() %}
    {{ arg.name_kt() }},
    {%- endfor %}
)
{% filter indent(4) %}{% include "LiftReturn.kt" %}{% endfilter %}
{%- else %}
val uniffiFuture = uniffi.Scaffolding.{{ jni_method_name }}(
    {%- if callable.uses_buffer() %}uniffiBuffer,{% endif %}
    {%- for arg in callable.ffi_arguments() %}
    {{ arg.name_kt() }},
    {%- endfor %}
)
val uniffiReturn = uniffi.{{ callable.result.async_await_future_fn() }}(
    uniffiFuture,
    {%- if callable.return_strategy().is_ffi_buffer() %}
    uniffiBuffer,
    {%- endif %}
);
{% include "LiftReturn.kt" %}
{%- endif %}
{%- if callable.uses_buffer() %}
} finally {
    uniffi.Scaffolding.ffiBufferFree(uniffiBuffer)
}
{%- endif %}
