{%- let mod_name = crate_name|fn_name %}

{%- let ffi_converter_name = "FfiConverterType{}"|format(name) %}
{{ self.add_import_of(mod_name, ffi_converter_name) }}

{%- let rustbuffer_local_name = "RustBuffer{}"|format(name) %}
{{ self.add_import_of_as(mod_name, "RustBuffer", rustbuffer_local_name) }}

{%- match kind %}
{%- when ExternalKind::CallbackInterface %}
# Register the callback function with our _UniFFILib.
# Each Python module has its own copy of the dynamic library, so we need to do
# this both in the crate where the callback interface is defined and any crates
# where it's used.
{{ ffi_converter_name }}.register(_UniFFILib)
{%- else %}
{%- endmatch %}
