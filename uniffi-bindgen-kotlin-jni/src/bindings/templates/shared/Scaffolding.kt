object Scaffolding {
    @JvmStatic external fun ffiBufferNew(): Long
    @JvmStatic external fun miniBufferNext(endPtr: Long, size: Long): Long
    @JvmStatic external fun ffiBufferFree(ptr: Long)
    @JvmStatic external fun readByte(ptr: Long): Byte
    @JvmStatic external fun readShort(ptr: Long): Short
    @JvmStatic external fun readInt(ptr: Long): Int
    @JvmStatic external fun readLong(ptr: Long): Long
    @JvmStatic external fun readFloat(ptr: Long): Float
    @JvmStatic external fun readDouble(ptr: Long): Double
    @JvmStatic external fun readString(ptr: Long): String
    @JvmStatic external fun writeByte(ptr: Long, value: Byte)
    @JvmStatic external fun writeShort(ptr: Long, value: Short)
    @JvmStatic external fun writeInt(ptr: Long, value: Int)
    @JvmStatic external fun writeLong(ptr: Long, value: Long)
    @JvmStatic external fun writeFloat(ptr: Long, value: Float)
    @JvmStatic external fun writeDouble(ptr: Long, value: Double)
    @JvmStatic external fun writeString(ptr: Long, value: String)

    {%- for package in root.packages %}
    {%- for scaffolding_function in package.scaffolding_functions %}
    @JvmStatic external fun {{ scaffolding_function.jni_method_name }}(
        {%- if scaffolding_function.callable.uses_buffer() %}
        uniffiBuffer: Long,
        {%- endif %}
        {%- for ffi_arg in scaffolding_function.callable.ffi_arguments() %}
        {{ ffi_arg.name_kt() }}: {{ ffi_arg.ty.type_kt() }},
        {%- endfor %}
    )
        {%- if scaffolding_function.callable.is_async %}: Long
        {%- elif let ReturnStrategy::Primitive(_, ffi_type) = scaffolding_function.callable.return_strategy() %}: {{ ffi_type.type_kt() }}
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
        uniffiBuffer: Long,
        {%- when ReturnStrategy::Primitive(_, _) %}
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
            buffer: Long,
            {%- when ReturnStrategy::Primitive(_, ffi_type) %}
            uniffiReturn: {{ ffi_type.type_kt() }},
            {%- when ReturnStrategy::Void %}
            {%- endmatch %}
    )
    {%- if let Some(throws_type) = callback_result.throws_type %}
    @JvmStatic external fun {{ callback_result.async_complete_error_fn() }}(
        kotlinFuture: Long,
        {%- match throws_type.ffi_type %}
        {%- when None %}
        buffer: Long,
        {%- when Some(return_ty_ffi_type) %}
        error: {{ return_ty_ffi_type.type_kt() }},
        {%- endmatch %}
    )
    {%- endif %}
    @JvmStatic external fun {{ callback_result.async_complete_unexpected_error_fn() }}(kotlinFuture: Long)

    {%- endfor %}

    // access `uniffiLibrary` to make sure the cdylib is loaded
    init {
        System.loadLibrary("{{ cdylib }}")
    }
}
