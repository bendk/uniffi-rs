{%- let name = type_|type_name %}
{%- let module = ci.namespace_for_type(type_) %}
{%- let ffi_converter_name = type_|ffi_converter_name %}

# External type `{{module}}.{{name}}`
{%- let ffi_converter_name = "_UniffiConverterType{}"|format(name) %}
{{ self.add_import_of(module, ffi_converter_name) }}
{{ self.add_import_of(module, name) }} {#- import the type alias itself -#}

{%- let rustbuffer_local_name = "_UniffiRustBuffer{}"|format(name) %}
{{ self.add_import_of_as(module, "_UniffiRustBuffer", rustbuffer_local_name) }}
