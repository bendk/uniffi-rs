# Put all the bits inside a class to keep the top-level namespace clean
{%- let trait_impl="UniffiTraitImpl{name}"|format %}
class {{ trait_impl }}:
    _UniffiPointerManager = _UniffiPointerManager()
    {%- for meth in obj.methods() %}

    @{{ meth.ffi_func()|ctypes_ffi_prototype }}
    def {{ meth.name() }}(
            uniffi_handle,
            {%- for arg in meth.arguments() %}
            {{ arg.name() }},
            {%- endfor %}
            return_ptr,
            uniffi_call_status_ptr,
        ):
        uniffi_obj = {{ trait_impl }}._UniffiPointerManager.lookup(uniffi_handle)
        def make_call():
            args = ({% for arg in meth.arguments() %}{{ arg|lift_fn }}({{ arg.name() }}), {% endfor %})
            method = uniffi_obj.{{ meth.name() }}
            return method(*args)
        {%- match meth.return_type() %}
        {%- when Some(return_type) %}
        def write_return_value(v):
            return_ptr[0] = {{ return_type|lower_fn }}(v)
        {%- when None %}
        write_return_value = lambda v: None
        {%- endmatch %}

        {%- match meth.throws_type() %}
        {%- when None %}
        _uniffi_trait_interface_call(
                uniffi_call_status_ptr.contents,
                make_call,
                write_return_value,
        )
        {%- when Some(error) %}
        _uniffi_trait_interface_call_with_error(
                uniffi_call_status_ptr.contents,
                make_call,
                write_return_value,
                {{ error|type_name }},
                {{ error|lower_fn }},
        )
        {%- endmatch %}
    {%- endfor %}

    @ctypes.CFUNCTYPE(None, ctypes.c_void_p)
    def uniffi_free(uniffi_handle):
        {{ trait_impl }}._UniffiPointerManager.release_pointer(uniffi_handle)

    class UniffiVtable(ctypes.Structure):
        _fields_ = [
            {%- for meth in obj.methods() %}
            ("{{ meth.name() }}", {{ meth.ffi_func()|ctypes_ffi_prototype }}),
            {%- endfor %}
            ("uniffi_free", ctypes.CFUNCTYPE(None, ctypes.c_void_p)),
    ]
    uniffi_vtable = UniffiVtable(
        {%- for meth in obj.methods() %}
        {{ meth.name() }},
        {%- endfor %}
        uniffi_free
    )
    _UniffiLib.{{ obj.ffi_init_trait_callback().name() }}(ctypes.byref(uniffi_vtable))
