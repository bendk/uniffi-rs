const val UNIFFI_RUST_FUTURE_POLL_AGAIN = 0
const val UNIFFI_RUST_FUTURE_CANCELLED = 1
const val UNIFFI_RUST_FUTURE_COMPLETE = 2
const val UNIFFI_RUST_FUTURE_FAILED = 3

fun uniffiContinuationResume(continuation: kotlin.coroutines.Continuation<Int>) {
    continuation.resumeWith(Result.success(UNIFFI_RUST_FUTURE_POLL_AGAIN))
}

{%- for rust_result in root.rust_async_callable_results() %}
suspend fun {{ rust_result.async_await_future_fn() }}(
    rustFuture: Long,
    {%- if rust_result.return_strategy().is_ffi_buffer() %}
    uniffiBuffer: Long,
    {%- endif %}
){%- if let Some(return_type) = rust_result.return_type %} : {{ return_type.type_kt }}{% endif %}
{
    try {
        {%- match rust_result.return_strategy() %}
        {%- when ReturnStrategy::Primitive(_, _) | ReturnStrategy::Reconstruct(_, _) %}
        val completion = {{ rust_result.async_complete_class() }}();
        {%- else %}
        {%- endmatch %}
        while(true) {
            val continuationResult = kotlin.coroutines.suspendCoroutine<Int> { continuation ->
                val pollResult = Scaffolding.{{ rust_result.async_poll_fn() }}(
                    rustFuture,
                    continuation,
                    {%- match rust_result.return_strategy() %}
                    {%- when ReturnStrategy::FfiBuffer(_) %}
                    uniffiBuffer,
                    {%- when ReturnStrategy::Primitive(_, _) | ReturnStrategy::Reconstruct(_, _) %}
                    completion,
                    {%- when ReturnStrategy::Void %}
                    {%- endmatch %}
            )
                if (pollResult != UNIFFI_RUST_FUTURE_POLL_AGAIN) {
                    continuation.resumeWith(Result.success(pollResult));
                }
            }
            when (continuationResult) {
                UNIFFI_RUST_FUTURE_POLL_AGAIN -> continue
                UNIFFI_RUST_FUTURE_COMPLETE -> {
                    {%- match rust_result.return_strategy() %}
                    {%- when ReturnStrategy::FfiBuffer(type_node) %}
                    val uniffiReader = uniffi.FfiBufferCursor(uniffiBuffer)
                    return uniffi.{{ type_node.read_fn_kt() }}(uniffiReader)
                    {%- when ReturnStrategy::Primitive(type_node, ffi_type) %}
                    return {{ type_node.lift_fn_kt() }}(completion.value)
                    {%- when ReturnStrategy::Reconstruct(_, _) %}
                    return completion.value!!
                    {%- when ReturnStrategy::Void %}
                    return
                    {%- endmatch %}
                }
                UNIFFI_RUST_FUTURE_CANCELLED -> throw kotlin.coroutines.cancellation.CancellationException()
                else -> throw uniffi.InternalException("Error polling Rust future (code: $continuationResult)")
            }
        }
    } finally {
        Scaffolding.{{ rust_result.async_free_fn() }}(rustFuture)
    }
}


{%- match rust_result.return_strategy() %}
{%- when ReturnStrategy::Primitive(_, ffi_type) %}
class {{ rust_result.async_complete_class() }} {
    var value: {{ ffi_type.type_kt() }} = {{ ffi_type.default_kt() }}

    fun complete(value: {{ ffi_type.type_kt() }}) {
        this.value = value
    }
}
{%- when ReturnStrategy::Reconstruct(type_node, ffi_types) %}
class {{ rust_result.async_complete_class() }} {
    var value: {{ type_node.type_kt }}? = null;

    fun complete(
        {%- for ffi_type in ffi_types %}
        v{{loop.index0 }}: {{ ffi_type.type_kt() }},
        {%- endfor %}
    ) {
        this.value = {{ type_node.lift_fn_kt() }}(
            {%- for _ in ffi_types %}
            v{{loop.index0 }},
            {%- endfor %}
        )
    }
}
{%- else %}
{%- endmatch %}
{%- endfor %}
