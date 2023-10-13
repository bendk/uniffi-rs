{%- let obj = ci|get_object_definition(name) %}
{%- if self.include_once_check("ObjectRuntime.kt") %}{% include "ObjectRuntime.kt" %}{% endif %}

public interface {{ type_name }}Interface {
    {% for meth in obj.methods() -%}
    {%- match meth.throws_type() -%}
    {%- when Some with (throwable) -%}
    @Throws({{ throwable|error_type_name }}::class)
    {%- when None -%}
    {%- endmatch %}
    {% if meth.is_async() -%}
    suspend fun {{ meth.name()|fn_name }}({% call kt::arg_list_decl(meth) %})
    {%- else -%}
    fun {{ meth.name()|fn_name }}({% call kt::arg_list_decl(meth) %})
    {%- endif %}
    {%- match meth.return_type() -%}
    {%- when Some with (return_type) %}: {{ return_type|type_name -}}
    {%- when None -%}
    {%- endmatch -%}

    {% endfor %}
    companion object
}

class {{ type_name }}(
    handle: Int
) : FFIObject(handle), {{ type_name }}Interface {

    {%- match obj.primary_constructor() %}
    {%- when Some with (cons) %}
    constructor({% call kt::arg_list_decl(cons) -%}) :
        this({% call kt::to_ffi_call(cons) %})
    {%- when None %}
    {%- endmatch %}

    /**
     * Disconnect the object from the underlying Rust object.
     *
     * It can be called more than once, but once called, interacting with the object
     * causes an `IllegalStateException`.
     *
     * Clients **must** call this method once done with the object, or cause a memory leak.
     */
    override protected fun freeRustArcPtr() {
        rustCall() { status ->
            _UniFFILib.INSTANCE.{{ obj.ffi_object_free().name() }}(this.uniffiHandle, status)
        }
    }

    {% for meth in obj.methods() -%}
    {%- match meth.throws_type() -%}
    {%- when Some with (throwable) %}
    @Throws({{ throwable|error_type_name }}::class)
    {%- else -%}
    {%- endmatch -%}
    {%- if meth.is_async() %}
    @Suppress("ASSIGNED_BUT_NEVER_ACCESSED_VARIABLE")
    override suspend fun {{ meth.name()|fn_name }}({%- call kt::arg_list_decl(meth) -%}){% match meth.return_type() %}{% when Some with (return_type) %} : {{ return_type|type_name }}{% when None %}{%- endmatch %} {
        return uniffiRustCallAsync(
            callWithHandle { handle ->
                _UniFFILib.INSTANCE.{{ meth.ffi_func().name() }}(
                    handle,
                    {% call kt::arg_list_lowered(meth) %}
                )
            },
            { future, continuation -> _UniFFILib.INSTANCE.{{ meth.ffi_rust_future_poll(ci) }}(future, continuation) },
            { future, status -> _UniFFILib.INSTANCE.{{ meth.ffi_rust_future_complete(ci) }}(future, status) },
            { future -> _UniFFILib.INSTANCE.{{ meth.ffi_rust_future_free(ci) }}(future) },
            // lift function
            {%- match meth.return_type() %}
            {%- when Some(return_type) %}
            { {{ return_type|lift_fn }}(it) },
            {%- when None %}
            { Unit },
            {% endmatch %}
            // Error FFI converter
            {%- match meth.throws_type() %}
            {%- when Some(e) %}
            {{ e|error_type_name }}.ErrorHandler,
            {%- when None %}
            NullCallStatusErrorHandler,
            {%- endmatch %}
        )
    }
    {%- else -%}
    {%- match meth.return_type() -%}
    {%- when Some with (return_type) -%}
    override fun {{ meth.name()|fn_name }}({% call kt::arg_list_protocol(meth) %}): {{ return_type|type_name }} =
        callWithHandle {
            {%- call kt::to_ffi_call_with_prefix("it", meth) %}
        }.let {
            {{ return_type|lift_fn }}(it)
        }

    {%- when None -%}
    override fun {{ meth.name()|fn_name }}({% call kt::arg_list_protocol(meth) %}) =
        callWithHandle {
            {%- call kt::to_ffi_call_with_prefix("it", meth) %}
        }
    {% endmatch %}
    {% endif %}
    {% endfor %}

    {% if !obj.alternate_constructors().is_empty() -%}
    companion object {
        {% for cons in obj.alternate_constructors() -%}
        fun {{ cons.name()|fn_name }}({% call kt::arg_list_decl(cons) %}): {{ type_name }} =
            {{ type_name }}({% call kt::to_ffi_call(cons) %})
        {% endfor %}
    }
    {% else %}
    companion object
    {% endif %}
}

public object {{ obj|ffi_converter_name }}: FfiConverter<{{ type_name }}, Int> {
    override fun lower(value: {{ type_name }}): Int = value.uniffiHandle

    override fun lift(value: Int): {{ type_name }} {
        return {{ type_name }}(value)
    }

    override fun read(buf: ByteBuffer): {{ type_name }} {
        return lift(buf.getInt())
    }

    override fun allocationSize(value: {{ type_name }}) = 4

    override fun write(value: {{ type_name }}, buf: ByteBuffer) {
        buf.putInt(lower(value))
    }
}
