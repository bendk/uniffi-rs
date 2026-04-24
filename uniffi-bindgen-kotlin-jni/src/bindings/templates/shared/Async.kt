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
)
{%- if let ReturnStrategy::Primitive(_, ffi_type) = rust_result.return_strategy() %}: {{ ffi_type.type_kt() }}
{%- endif %}
{
    try {
        {%- if rust_result.return_strategy().is_primitive() %}
        val completion = {{ rust_result.async_complete_class() }}();
        {%- endif %}
        while(true) {
            val continuationResult = kotlin.coroutines.suspendCoroutine<Int> { continuation ->
                val pollResult = Scaffolding.{{ rust_result.async_poll_fn() }}(
                    rustFuture,
                    continuation,
                    {%- match rust_result.return_strategy() %}
                    {%- when ReturnStrategy::FfiBuffer(_) %}
                    uniffiBuffer,
                    {%- when ReturnStrategy::Primitive(_, _) %}
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
                    {%- if rust_result.return_strategy().is_primitive() %}
                    return completion.value
                    {%- else %}
                    return
                    {%- endif %}
                }
                UNIFFI_RUST_FUTURE_CANCELLED -> throw kotlin.coroutines.cancellation.CancellationException()
                else -> throw uniffi.InternalException("Error polling Rust future (code: $continuationResult)")
            }
        }
    } finally {
        Scaffolding.{{ rust_result.async_free_fn() }}(rustFuture)
    }
}


{%- if let ReturnStrategy::Primitive(_, ffi_type) = rust_result.return_strategy() %}
class {{ rust_result.async_complete_class() }} {
    var value: {{ ffi_type.type_kt() }} = {{ ffi_type.default_kt() }}

    fun complete(value: {{ ffi_type.type_kt() }}) {
        this.value = value
    }
}
{%- endif %}
{%- endfor %}
