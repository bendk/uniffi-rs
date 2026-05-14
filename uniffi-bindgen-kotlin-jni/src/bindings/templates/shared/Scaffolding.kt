object Scaffolding {
    @JvmStatic external fun ffiBufferNew(): java.nio.ByteBuffer
    @JvmStatic external fun miniBufferNext(ptr: java.nio.ByteBuffer, size: Long): java.nio.ByteBuffer
    @JvmStatic external fun ffiBufferFree(ptr: java.nio.ByteBuffer)
    //@JvmStatic external fun ffiBufferByteBuffer(ptr: Long, size: Long): java.nio.ByteBuffer
    // @JvmStatic external fun readByte(ptr: Long): Byte
    // @JvmStatic external fun readShort(ptr: Long): Short
    // @JvmStatic external fun readInt(ptr: Long): Int
    // @JvmStatic external fun readLong(ptr: Long): Long
    // @JvmStatic external fun readFloat(ptr: Long): Float
    // @JvmStatic external fun readDouble(ptr: Long): Double
    @JvmStatic external fun readString(data: Long, length: Long, capacity: Long): String
    // @JvmStatic external fun writeByte(ptr: Long, value: Byte)
    // @JvmStatic external fun writeShort(ptr: Long, value: Short)
    // @JvmStatic external fun writeInt(ptr: Long, value: Int)
    // @JvmStatic external fun writeLong(ptr: Long, value: Long)
    // @JvmStatic external fun writeFloat(ptr: Long, value: Float)
    // @JvmStatic external fun writeDouble(ptr: Long, value: Double)
    @JvmStatic external fun writeString(ptr: java.nio.ByteBuffer, index: Int, value: String)

    {%- for package in root.packages %}
    {%- for scaffolding_function in package.scaffolding_functions %}
    @JvmStatic external fun {{ scaffolding_function.jni_method_name }}(
        {%- if scaffolding_function.callable.uses_buffer() %}
        uniffiBuffer: java.nio.ByteBuffer,
        {%- endif %}
        {%- for ffi_arg in scaffolding_function.callable.ffi_arguments_including_receiver() %}
        {{ ffi_arg.name_kt() }}: {{ ffi_arg.ty.type_kt() }},
        {%- endfor %}
    )
        {%- if scaffolding_function.callable.is_async %}: Long
        {%- else %}
        {%- match scaffolding_function.callable.return_strategy() %}
        {%- when ReturnStrategy::Primitive(_, ffi_type) %}: {{ ffi_type.type_kt() }}
        {%- when ReturnStrategy::Reconstruct(type_node, _) %}: {{ type_node.type_kt }}
        {%- else %}
        {%- endmatch %}
        {%- endif %}
    {%- endfor %}

    {%- for cls in package.classes() %}
    {%- if !cls.imp.is_trait_interface() %}
    @JvmStatic external fun {{ cls.jni_free_name() }}(handle: Long)
    @JvmStatic external fun {{ cls.jni_addref_name() }}(handle: Long)
    {%- else %}
    @JvmStatic external fun {{ cls.jni_free_name() }}(handle: Long, handle2: Long)
    @JvmStatic external fun {{ cls.jni_addref_name() }}(handle: Long, handle2: Long)
    {%- endif %}
    {%- endfor  %}
    {%- endfor  %}

    {%- for rust_result in root.rust_async_callable_results() %}
    @JvmStatic external fun {{ rust_result.async_poll_fn() }}(
        rustFuture: Long,
        continuation: kotlin.coroutines.Continuation<Int>,
        {%- match rust_result.return_strategy() %}
        {%- when ReturnStrategy::FfiBuffer(_) %}
        uniffiBuffer: java.nio.ByteBuffer,
        {%- when ReturnStrategy::Primitive(_, _) | ReturnStrategy::Reconstruct(_, _) %}
        completion: {{ rust_result.async_complete_class() }},
        {%- when ReturnStrategy::Void %}
        {%- endmatch %}
    ): Int
    @JvmStatic external fun {{ rust_result.async_cancel_fn() }}(rustFuture: Long)
    @JvmStatic external fun {{ rust_result.async_free_fn() }}(rustFuture: Long)
    {%- endfor %}

    {%- for callback_result in root.kotlin_async_callable_results() %}
    @JvmStatic external fun {{ callback_result.async_complete_success_fn() }}(
            kotlinFuture: Long,
            {%- match callback_result.return_strategy() %}
            {%- when ReturnStrategy::FfiBuffer(_) %}
            buffer: java.nio.ByteBuffer,
            {%- when ReturnStrategy::Primitive(_, ffi_type) %}
            uniffiReturn: {{ ffi_type.type_kt() }},
            {%- when ReturnStrategy::Reconstruct(_, ffi_types) %}
            {%- for ffi_type in ffi_types %}
            uniffiReturnV{{ loop.index0 }}: {{ ffi_type.type_kt() }},
            {%- endfor %}
            {%- when ReturnStrategy::Void %}
            {%- endmatch %}
    )
    {%- if let Some(throws_type) = callback_result.throws_type %}
    @JvmStatic external fun {{ callback_result.async_complete_error_fn() }}(
        kotlinFuture: Long,
        {%- match throws_type.lowerable %}
        {%- when Some(LowerableType::Primitive(ffi_type)) %}
        error: {{ ffi_type.type_kt() }},
        {%- when Some(LowerableType::Deconstructable(ffi_types)) %}
        {%- for ffi_type in ffi_types %}
        errorV{{ loop.index0 }}: {{ ffi_type.type_kt() }},
        {%- endfor %}
        {%- when None %}
        buffer: java.nio.ByteBuffer,
        {%- endmatch %}
    )
    {%- endif %}
    @JvmStatic external fun {{ callback_result.async_complete_unexpected_error_fn() }}(kotlinFuture: Long)
    {%- endfor %}

    {%- for callback_result in root.kotlin_sync_callable_results() %}

    {%- if let Some(return_type) = callback_result.return_type %}
    {%- if let Some(LowerableType::Deconstructable(ffi_types)) = return_type.lowerable %}
    @JvmStatic external fun {{ callback_result.set_callback_return_fn_kt() }}(
        resultPointer: Long,
        {%- for ffi_type in ffi_types %}
        v{{ loop.index0 }}: {{ ffi_type.type_kt() }},
        {%- endfor %}
    )
    {%- endif %}
    {%- endif %}

    {%- if let Some(throws_type) = callback_result.throws_type %}
    {%- match throws_type.lowerable %}
    {%- when Some(LowerableType::Primitive(ffi_type)) %}
    @JvmStatic external fun {{ callback_result.set_callback_err_fn_kt() }}(
        resultPointer: Long,
        errorValue: {{ ffi_type.type_kt() }},
    )
    {%- when Some(LowerableType::Deconstructable(ffi_types)) %}
    @JvmStatic external fun {{ callback_result.set_callback_err_fn_kt() }}(
        resultPointer: Long,
        {%- for ffi_type in ffi_types %}
        v{{ loop.index0 }}: {{ ffi_type.type_kt() }},
        {%- endfor %}
    )
    {%- when None %}
    @JvmStatic external fun {{ callback_result.set_callback_err_fn_kt() }}(
        resultPointer: Long,
        uniffiBuffer: java.nio.ByteBuffer,
    )
    {%- endmatch %}
    {%- endif %}

    {%- endfor %}

    // access `uniffiLibrary` to make sure the cdylib is loaded
    init {
        System.loadLibrary("{{ cdylib }}")
    }
}
