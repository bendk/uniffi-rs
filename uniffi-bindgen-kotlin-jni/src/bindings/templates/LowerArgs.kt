{%- if callable.uses_buffer() %}
val uniffiWriter = uniffi.FfiBufferCursor(uniffiBuffer)
{%- endif %}

{%- if let Some(receiver) = callable.receiver %}
{%- match receiver.strategy %}
{%- when ReceiverStrategy::InterfaceRef(_, ffi_arg) %}
val {{ ffi_arg.name_kt() }} = this.uniffiHandle
{%- when ReceiverStrategy::TraitInterfaceRef(_, ffi_arg, ffi_arg2) %}
val {{ ffi_arg.name_kt() }} = this.uniffiHandle
val {{ ffi_arg2.name_kt() }} = this.uniffiHandle2
{%- when ReceiverStrategy::Arg(arg_strategy) %}
{%- match arg_strategy %}
{%- when ArgStrategy::Primitive(ffi_arg) %}
val {{ ffi_arg.name_kt() }} = uniffi.{{ receiver.ty.lower_fn_kt() }}(this)
{%- when ArgStrategy::Deconstruct(ffi_args) %}
{%- let deconstructed_arg_name = "uniffiDeconstructed{}"|format(loop.index0) %}
val {{ deconstructed_arg_name }} = uniffi.{{ receiver.ty.lower_fn_kt() }}(this)
{%- for ffi_arg in ffi_args %}
val {{ ffi_arg.name_kt() }} = {{ deconstructed_arg_name }}.v{{ loop.index0 }}
{%- endfor %}
{%- when ArgStrategy::FfiBuffer %}
uniffi.{{ receiver.ty.write_fn_kt() }}(uniffiWriter, this)
{%- endmatch %}
{%- endmatch %}
{%- endif %}

{%- for arg in callable.arguments %}
{%- match arg.strategy %}
{%- when ArgStrategy::Primitive(ffi_arg) %}
val {{ ffi_arg.name_kt() }} = uniffi.{{ arg.ty.lower_fn_kt() }}({{ arg.name_kt() }})
{%- when ArgStrategy::Deconstruct(ffi_args) %}
{%- let deconstructed_arg_name = "uniffiDeconstructed{}"|format(loop.index0) %}
val {{ deconstructed_arg_name }} = uniffi.{{ arg.ty.lower_fn_kt() }}({{ arg.name_kt() }})
{%- for ffi_arg in ffi_args %}
val {{ ffi_arg.name_kt() }} = {{ deconstructed_arg_name }}.v{{ loop.index0 }}
{%- endfor %}
{%- when ArgStrategy::FfiBuffer %}
uniffi.{{ arg.ty.write_fn_kt() }}(uniffiWriter, {{ arg.name_kt() }})
{%- endmatch %}
{%- endfor %}
