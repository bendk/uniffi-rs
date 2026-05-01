{%- let type_name = cbi.self_type.type_kt %}

private val {{ cbi.handle_map_kt() }} = HandleMap<{{ type_name }}>();

{%- for meth in cbi.methods %}
{%- let callable = meth.callable %}
{%- let return_type = callable.return_type() %}
{%- let throws_type = callable.throws_type() %}
{%- if !callable.is_async %}
fun {{ meth.dispatch_fn_kt }}(
    uniffiHandle: Long,
    {%- if callable.uses_buffer() %}
    uniffiBuffer: Long,
    {%- endif %}
    {%- if callable.return_strategy().is_reconstruct() || callable.throws_type().is_some() %}
    uniffiReturnPointer: Long,
    {%- endif %}
    {%- for ffi_arg in callable.ffi_arguments() %}
    {{ ffi_arg.name_kt() }}: {{ ffi_arg.ty.type_kt() }},
    {%- endfor %}
)
{%- if let ReturnStrategy::Primitive(_, ffi_type) = callable.return_strategy() %}: {{ ffi_type.type_kt() }}
{%- endif %}
{
    val uniffiObj = {{ cbi.handle_map_kt() }}.get(uniffiHandle)
    {%- filter indent(4) %}{% include "LiftArgs.kt" %}{% endfilter %}
    {%- match callable.throws_type() %}
    {%- when None %}
    val uniffiReturn = uniffiObj.{{ callable.name_kt() }}(
        {%- for a in callable.arguments %}
        {{ a.name_kt() }},
        {%- endfor %}
    )
    {%- when Some(throws_ty) %}
    val uniffiReturn = try {
        uniffiObj.{{ callable.name_kt() }}(
            {%- for a in callable.arguments %}
            {{ a.name_kt() }},
            {%- endfor %}
        )
    } catch(uniffiErr: {{ throws_ty.type_kt }}) {
        {%- match throws_ty.lowerable %}
        {%- when Some(LowerableType::Deconstructable(ffi_types)) %}
        val uniffiErrDeconstructed = {{ throws_ty.lower_fn_kt() }}(uniffiErr)
        Scaffolding.{{ callable.result.set_callback_err_fn_kt() }}(
            uniffiReturnPointer,
            {%- for _ in ffi_types %}
            uniffiErrDeconstructed.v{{ loop.index0 }},
            {%- endfor %}
        )
        {%- when Some(LowerableType::Primitive(ffi_type)) %}
        Scaffolding.{{ callable.result.set_callback_err_fn_kt() }}(uniffiReturnPointer, {{ throws_ty.lower_fn_kt() }}(uniffiErr))
        {%- when None %}
        {%- if !callable.uses_buffer() %}
        val uniffiBuffer = Scaffolding.ffiBufferNew()
        try {
        {%- endif %}
            val uniffiWriter = FfiBufferCursor(uniffiBuffer)
            {{ throws_ty.write_fn_kt() }}(uniffiWriter, uniffiErr)
            Scaffolding.{{ callable.result.set_callback_err_fn_kt() }}(uniffiReturnPointer, uniffiBuffer)
        {%- if !callable.uses_buffer() %}
        } finally {
            Scaffolding.ffiBufferFree(uniffiBuffer)
        }
        {%- endif %}
        {%- endmatch %}
        {%- if let ReturnStrategy::Primitive(_, ffi_type) = callable.return_strategy() %}
        return {{ ffi_type.default_kt() }}
        {%- else %}
        return
        {%- endif %}
    }
    {%- endmatch %}
    {%- match callable.return_strategy() %}
    {%- when ReturnStrategy::FfiBuffer(return_type) %}
    val uniffiWriter = FfiBufferCursor(uniffiBuffer)
    {{ return_type.write_fn_kt() }}(uniffiWriter, uniffiReturn)
    {%- when ReturnStrategy::Primitive(type_node, _) %}
    return {{ type_node.lower_fn_kt() }}(uniffiReturn)
    {%- when ReturnStrategy::Reconstruct(type_node, ffi_types) %}
    val uniffiErrDeconstructed = {{ type_node.lower_fn_kt() }}(uniffiReturn)
    Scaffolding.{{ callable.result.set_callback_return_fn_kt() }}(
        uniffiReturnPointer,
        {%- for _ in ffi_types %}
        uniffiErrDeconstructed.v{{ loop.index0 }},
        {%- endfor %}
    )
    {%- when ReturnStrategy::Void %}
    {% endmatch %}
}
{%- else %}
fun {{ meth.dispatch_fn_kt }}(
    uniffiHandle: Long,
    uniffiKotlinFutureHandle: Long,
    {%- if callable.uses_buffer() %}
    uniffiBuffer: Long,
    {%- endif %}
    {%- for ffi_arg in callable.ffi_arguments() %}
    {{ ffi_arg.name_kt() }}: {{ ffi_arg.ty.type_kt() }},
    {%- endfor %}
) {
    val uniffiObj = {{ cbi.handle_map_kt() }}.get(uniffiHandle)
    {%- filter indent(4) %}{% include "LiftArgs.kt" %}{% endfilter %}

    // Using `GlobalScope` is labeled as a "delicate API" and generally discouraged in Kotlin programs, since it breaks structured concurrency.
    // However, our parent task is a Rust future, so we're going to need to break structure concurrency in any case.
    //
    // Uniffi does its best to support structured concurrency across the FFI.
    // If the Rust future is dropped, `uniffiForeignFutureDroppedCallbackImpl` is called, which will cancel the Kotlin coroutine if it's still running.
    @OptIn(kotlinx.coroutines.DelicateCoroutinesApi::class)
    val job = kotlinx.coroutines.GlobalScope.launch uniffiCoroutineBlock@ {
        try {
            val uniffiReturn = uniffiObj.{{ callable.name_kt() }}(
                {%- for a in callable.arguments %}
                {{ a.name_kt() }},
                {%- endfor %}
            )
            {%- match callable.return_strategy() %}
            {%- when ReturnStrategy::FfiBuffer(return_type) %}
            val uniffiWriter = FfiBufferCursor(uniffiBuffer)
            {{ return_type.write_fn_kt() }}(uniffiWriter, uniffiReturn)
            Scaffolding.{{ callable.result.async_complete_success_fn() }}(uniffiKotlinFutureHandle, uniffiBuffer)
            {%- when ReturnStrategy::Primitive(type_node, _) %}
            val uniffiReturnLowered = {{ type_node.lower_fn_kt() }}(uniffiReturn)
            Scaffolding.{{ callable.result.async_complete_success_fn() }}(uniffiKotlinFutureHandle, uniffiReturnLowered)
            {%- when ReturnStrategy::Reconstruct(type_node, ffi_types) %}
            val uniffiReturnDeconstructed = {{ type_node.lower_fn_kt() }}(uniffiReturn)
            Scaffolding.{{ callable.result.async_complete_success_fn() }}(
                uniffiKotlinFutureHandle,
                {%- for _ in ffi_types %}
                uniffiReturnDeconstructed.v{{ loop.index0 }},
                {%- endfor %}
            )
            {%- when ReturnStrategy::Void %}
            Scaffolding.{{ callable.result.async_complete_success_fn() }}(uniffiKotlinFutureHandle)
            {% endmatch %}
        } catch(uniffiErr: Throwable) {
            {%- if let Some(throws_type) = callable.throws_type() %}
            try {
                if (uniffiErr is {{ throws_type.type_kt }}) {
                    {%- if throws_type.uses_buffer() && !callable.uses_buffer() %}
                    val uniffiBuffer = uniffi.Scaffolding.ffiBufferNew()
                    try {
                    {%- endif %}
                        {%- match throws_type.lowerable %}
                        {%- when Some(LowerableType::Primitive(_)) %}
                        val uniffiErrLowered = {{ throws_type.lower_fn_kt() }}(uniffiErr)
                        Scaffolding.{{ callable.result.async_complete_error_fn() }}(uniffiKotlinFutureHandle, uniffiErrorLowered)
                        {%- when Some(LowerableType::Deconstructable(ffi_types)) %}
                        val uniffiErrDeconstructed = {{ throws_type.lower_fn_kt() }}(uniffiErr)
                        Scaffolding.{{ callable.result.async_complete_error_fn() }}(
                            uniffiKotlinFutureHandle, 
                            {%- for _ in ffi_types %}
                            uniffiErrDeconstructed.v{{ loop.index0 }},
                            {%- endfor %}
                        )
                        {%- when None %}
                        val uniffiWriter = FfiBufferCursor(uniffiBuffer)
                        {{ throws_type.write_fn_kt() }}(uniffiWriter, uniffiErr)
                        Scaffolding.{{ callable.result.async_complete_error_fn() }}(uniffiKotlinFutureHandle, uniffiBuffer)
                        {%- endmatch %}
                    {%- if throws_type.uses_buffer() && !callable.uses_buffer() %}
                    } finally {
                        uniffi.Scaffolding.ffiBufferFree(uniffiBuffer)
                    }
                    {%- endif %}
                    return@uniffiCoroutineBlock;
                }
            } catch(e: Throwable) {
                // Exception trying to return the regular error value, fall through to the
                // unexpected error handling code
            }
            {%- endif %}
            Scaffolding.{{ callable.result.async_complete_unexpected_error_fn() }}(uniffiKotlinFutureHandle)
        }
    }
}
{%- endif %}
{%- endfor %}

fun {{ cbi.free_fn_kt() }}(handle: Long) {
    {{ cbi.handle_map_kt() }}.remove(handle)
}

{# Class.kt generates a read/write function for trait interfaces #}
{%- if !cbi.for_trait_interface %}
// Note: no read function, since callback interfaces can't be passed back from Rust to Kotlin

fun {{ cbi.self_type.write_fn_kt() }}(cursor: FfiBufferCursor, value: {{ type_name }}) {
    writeLong(cursor, {{ cbi.handle_map_kt() }}.insert(value))
}

{%- endif %}
