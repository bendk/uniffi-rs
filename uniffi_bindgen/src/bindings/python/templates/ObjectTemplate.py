{%- let obj = ci|get_object_definition(name) %}

class {{ type_name }}:
    _uniffi_handle: ctypes.c_uint32

{%- match obj.primary_constructor() %}
{%-     when Some with (cons) %}
    def __init__(self, {% call py::arg_list_decl(cons) -%}):
        {%- call py::setup_args_extra_indent(cons) %}
        self._uniffi_handle = {% call py::to_ffi_call(cons) %}
{%-     when None %}
{%- endmatch %}

    def __del__(self):
        # In case of partial initialization of instances.
        handle = getattr(self, "_uniffi_handle", None)
        if handle is not None:
            _rust_call(_UniffiLib.{{ obj.ffi_object_free().name() }}, handle)

    # Used by alternative constructors or any methods which return this type.
    @classmethod
    def _make_instance_(cls, handle):
        # Lightly yucky way to bypass the usual __init__ logic
        # and just create a new instance with the required handle.
        inst = cls.__new__(cls)
        inst._uniffi_handle = handle
        return inst

{%- for cons in obj.alternate_constructors() %}

    @classmethod
    def {{ cons.name()|fn_name }}(cls, {% call py::arg_list_decl(cons) %}):
        {%- call py::setup_args_extra_indent(cons) %}
        # Call the (fallible) function before creating any half-baked object instances.
        handle = {% call py::to_ffi_call(cons) %}
        return cls._make_instance_(handle)
{% endfor %}

{%- for meth in obj.methods() -%}
    {%- call py::method_decl(meth.name()|fn_name, meth) %}
{% endfor %}

{%- for tm in obj.uniffi_traits() -%}
{%-     match tm %}
{%-         when UniffiTrait::Debug { fmt } %}
            {%- call py::method_decl("__repr__", fmt) %}
{%-         when UniffiTrait::Display { fmt } %}
            {%- call py::method_decl("__str__", fmt) %}
{%-         when UniffiTrait::Eq { eq, ne } %}
    def __eq__(self, other: object) -> {{ eq.return_type().unwrap()|type_name }}:
        if not isinstance(other, {{ type_name }}):
            return NotImplemented

        return {{ eq.return_type().unwrap()|lift_fn }}({% call py::to_ffi_call_with_prefix("self._uniffi_handle", eq) %})

    def __ne__(self, other: object) -> {{ ne.return_type().unwrap()|type_name }}:
        if not isinstance(other, {{ type_name }}):
            return NotImplemented

        return {{ ne.return_type().unwrap()|lift_fn }}({% call py::to_ffi_call_with_prefix("self._uniffi_handle", ne) %})
{%-         when UniffiTrait::Hash { hash } %}
            {%- call py::method_decl("__hash__", hash) %}
{%      endmatch %}
{% endfor %}

class {{ ffi_converter_name }}:
    @staticmethod
    def lift(handle):
        return {{ type_name }}._make_instance_(handle)

    @staticmethod
    def lower(value):
        if not isinstance(value, {{ type_name }}):
            raise TypeError("Expected {{ type_name }} instance, {} found".format(type(value).__name__))
        return value._uniffi_handle

    @classmethod
    def read(cls, buf):
        ptr = buf.read_u32()
        return cls.lift(ptr)

    @classmethod
    def write(cls, value, buf):
        buf.write_u32(cls.lower(value))
