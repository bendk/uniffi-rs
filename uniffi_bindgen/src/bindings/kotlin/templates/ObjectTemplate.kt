{%- let obj = ci|get_object_definition(name) %}
{%- if self.include_once_check("ObjectRuntime.kt") %}{% include "ObjectRuntime.kt" %}{% endif %}
{%- let (interface_name, impl_class_name) = obj|object_names %}
{%- let methods = obj.methods() %}

{% include "Interface.kt" %}

class {{ impl_class_name }} internal constructor(handleWrapper: UniffiHandleWrapper)
 : FFIObject(), {{ interface_name }} {
    val handle = handleWrapper.handle

    {%- match obj.primary_constructor() %}
    {%- when Some with (cons) %}
    constructor({% call kt::arg_list_decl(cons) -%}) :
        this(UniffiHandleWrapper({% call kt::to_ffi_call(cons) %}))
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
            _UniFFILib.INSTANCE.{{ obj.ffi_object_free().name() }}(this.handle, status)
        }
    }

    internal fun uniffiCloneHandle(): UniffiHandle {
        rustCall() { status ->
            _UniFFILib.INSTANCE.{{ obj.ffi_object_inc_ref().name() }}(handle, status)
        }
        return handle
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
            _UniFFILib.INSTANCE.{{ meth.ffi_func().name() }}(
                uniffiCloneHandle(),
                {% call kt::arg_list_lowered(meth) %}
            ),
            {{ meth|async_poll(ci) }},
            {{ meth|async_complete(ci) }},
            {{ meth|async_free(ci) }},
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
        {%- call kt::to_ffi_call_with_prefix("uniffiCloneHandle()", meth) %}.let {
            {{ return_type|lift_fn }}(it)
        }

    {%- when None -%}
    override fun {{ meth.name()|fn_name }}({% call kt::arg_list_protocol(meth) %}) =
        {%- call kt::to_ffi_call_with_prefix("uniffiCloneHandle()", meth) %}
    {% endmatch %}
    {% endif %}
    {% endfor %}

    {% if !obj.alternate_constructors().is_empty() -%}
    companion object {
        {% for cons in obj.alternate_constructors() -%}
        fun {{ cons.name()|fn_name }}({% call kt::arg_list_decl(cons) %}): {{ impl_class_name }} =
            {{ impl_class_name }}(UniffiHandleWrapper({% call kt::to_ffi_call(cons) %}))
        {% endfor %}
    }
    {% else %}
    companion object
    {% endif %}
}

{%- if obj.is_trait_interface() %}
{%- let callback_handler_class = format!("UniffiCallbackInterface{}", name) %}
{%- let callback_handler_obj = format!("uniffiCallbackInterface{}", name) %}
{%- let ffi_init_callback = obj.ffi_init_callback() %}
{% include "CallbackInterfaceImpl.kt" %}
{%- endif %}

public object {{ obj|ffi_converter_name }}: FfiConverter<{{ type_name }}, UniffiHandle> {
    {%- if obj.is_trait_interface() %}
    internal val slab = UniffiSlab<{{ type_name }}>()
    {%- endif %}

    override fun lower(value: {{ type_name }}): UniffiHandle {
        {%- match obj.imp() %}
        {%- when ObjectImpl::Struct %}
        return value.uniffiCloneHandle()
        {%- when ObjectImpl::Trait %}
        return slab.insert(value)
        {%- endmatch %}
    }

    override fun lift(value: UniffiHandle): {{ type_name }} {
        return {{ impl_class_name }}(UniffiHandleWrapper(value))
    }

    override fun read(buf: ByteBuffer): {{ type_name }} {
        return lift(buf.getLong())
    }

    override fun allocationSize(value: {{ type_name }}) = 8

    override fun write(value: {{ type_name }}, buf: ByteBuffer) {
        buf.putLong(lower(value))
    }
}
