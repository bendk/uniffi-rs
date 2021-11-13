{% import "macros.kt" as kt %}
{%- let obj = self.inner() %}
class {{ obj.name()|class_name_kt }}(
    pointer: Pointer
    {%- match inner.decorator_name() %}
    {%- when Some with (name) %},
    internal val {{ name|var_name_kt }}: {{ name|class_name_kt }}
    {%- else %}
    {%- endmatch %}
) : FFIObject(pointer) {
        {%- match obj.primary_constructor() %}
        {%- when Some with (cons) %}
    constructor({% call constructor_args_decl(cons) -%}) :
        this({% call super_constructor_args(cons) %})
        {%- else %}
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
            _UniFFILib.INSTANCE.{{ obj.ffi_object_free().name() }}(this.pointer, status)
        }
    }

    internal fun lower(): Pointer = callWithPointer { it }

    internal fun write(buf: RustBufferBuilder) {
        // The Rust code always expects pointers written as 8 bytes,
        // and will fail to compile if they don't fit.
        buf.putLong(Pointer.nativeValue(this.lower()))
    }

    {% for meth in obj.methods() -%}
    fun {{ meth.name()|fn_name_kt }}({% call kt::arg_list_decl(meth) %}) =
        {%- match meth.decorator_method_name() -%}
        {%- when Some with (decorator_method) %}
            {%- match obj.decorator_name() %}{%- when Some with (decorator_name) %}{{ decorator_name|var_name_kt }}{% else %}{% endmatch -%}
                .{{ decorator_method|fn_name_kt }} {
            {% call method_body(meth) %}
        }
        {% else %}
        {% call method_body(meth) %}
        {% endmatch %}
    {% endfor %}

    companion object {
        {%- if obj.decorator_name().is_none() %}
        internal fun lift(ptr: Pointer): {{ obj.name()|class_name_kt }} {
            return {{ obj.name()|class_name_kt }}(ptr)
        }

        internal fun read(buf: ByteBuffer): {{ obj.name()|class_name_kt }} {
            // The Rust code always writes pointers as 8 bytes, and will
            // fail to compile if they don't fit.
            return {{ obj.name()|class_name_kt }}.lift(Pointer(buf.getLong()))
        }
        {% endif %}

        {%- for cons in obj.alternate_constructors() -%}
        fun {{ cons.name()|fn_name_kt }}({% call constructor_args_decl(cons) %}): {{ obj.name()|class_name_kt }} =
            {{ obj.name()|class_name_kt }}({% call super_constructor_args(cons) %})
        {% endfor -%}
    }
}

{# Macros only used in objects -#}
{% macro method_body(meth) -%}
callWithPointer {
    {%- call kt::to_ffi_call_with_prefix("it", meth) %}
}
{%- match meth.return_type() -%}
{%- when Some with (return_type) -%}.let {
    {{ "it"|lift_kt(return_type) }}
}
{%- when None -%}
{%- endmatch %}
{% endmacro -%}

{% macro constructor_args_decl(cons) %}
{% match obj.decorator_name() %}
    {%- when Some with (decorator_name) %}
        {{- decorator_name|var_name_kt }}: {{ decorator_name|class_name_kt -}}
        {%- if cons.arguments().len() != 0 %}, {% endif %}
        {%- call kt::arg_list_decl(cons) -%}
    {%- else %}
        {% call kt::arg_list_decl(cons) -%}
    {%- endmatch %}
{% endmacro %}

{% macro super_constructor_args(cons) %}
{% match obj.decorator_name() %}
    {%- when Some with (decorator_name) %}
        {%- call kt::to_ffi_call(cons) %}, {{ decorator_name|var_name_kt }}
    {%- else %}
        {%- call kt::to_ffi_call(cons) %}
    {%- endmatch %}
{% endmacro %}
