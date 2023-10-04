{%- let handle_map = "uniffiTraitHandleMap{name}"|format %}
internal val {{ handle_map }} = UniFfiHandleMap<{{ interface_name }}>()

@Structure.FieldOrder({% for meth in obj.methods() %}"{{ meth.name() }}", {% endfor %}"uniffi_free")
class {{ name|trait_vtable_class }} : Structure() {
    // For each method, define an interface for that method and a nested class that implements it.
    // It would be nice to use a Kotlin `object`, but that doesn't work with JNA.
    {%- for meth in obj.methods() %}
    internal interface {{ meth.name()|class_name }}Interface : com.sun.jna.Callback {
        fun invoke({{ meth|trait_vtable_method_args }})
    }

    internal class {{ meth.name()|class_name }}Impl : {{ meth.name()|class_name }}Interface {
        override fun invoke({{ meth|trait_vtable_method_args }}) {
            val uniffiObj = {{ handle_map }}.get(handle) ?: throw NullPointerException("Invalid handle passed to {{ name }}.{{ meth.name() }}")
            val makeCall = { ->
                uniffiObj.{{ meth.name()|fn_name() }}(
                    {%- for arg in meth.arguments() %}
                    {{ arg|lift_fn }}({{ arg.name() }}),
                    {%- endfor %}
                )
            }

            {%- match meth.return_type() %}
            {%- when Some(return_type) %}
            val writeReturn = { value: {{ return_type|type_name }} -> uniffiOutReturn.setValue({{ return_type|lower_fn }}(value)) }
            {%- when None %}
            val writeReturn = { _: Unit -> Unit }
            {%- endmatch %}

            {%- match meth.throws_type() %}
            {%- when None %}
            uniffiTraitInterfaceCall(uniffiOutCallStatus, makeCall, writeReturn)
            {%- when Some(error_type) %}
            uniffiTraitInterfaceCallWithError(
                uniffiOutCallStatus,
                makeCall,
                writeReturn,
                { e: {{error_type|error_type_name }} -> {{ error_type|lower_fn }}(e) }
            )
            {%- endmatch %}
        }
    }

    {%- endfor %}

    internal interface UniffiFreeInterface : com.sun.jna.Callback {
        fun invoke(handle: USize);
    }

    internal class UniffiFreeImpl : UniffiFreeInterface {
        override fun invoke(handle: USize) {
            {{ handle_map }}.remove(handle)
        }
    }

    {%- for meth in obj.methods() %}
    @JvmField internal var {{ meth.name() }} : {{ meth.name()|class_name }}Interface = {{ meth.name()|class_name }}Impl()
    {%- endfor %}
    @JvmField internal var uniffi_free : UniffiFreeInterface = UniffiFreeImpl()

    internal fun initialize(lib: _UniFFILib) {
        lib.{{ obj.ffi_init_trait_callback().name() }}(this)
    }
}

val {{ name|trait_vtable_obj }} : {{ name|trait_vtable_class }} = {{ name|trait_vtable_class }}()
