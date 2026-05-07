{%- if callable.receiver_uses_buffer() || callable.has_ffi_buffer_arg() %}
let (
    {%- if callable.receiver_uses_buffer() %}
    uniffi_self,
    {%- endif %}
    {%- for arg in callable.arguments %}
    {%- if arg.uses_buffer() %}
    {{ arg.name_rs() }},
    {%- endif %}
    {%- endfor %}
) = uniffi_buf.with_cursor(|uniffi_reader| Ok((
    {%- if let Some(receiver_type) = callable.receiver_type() %}
    {%- if callable.receiver_uses_buffer() %}
    {{ receiver_type.read_fn_rs() }}(uniffi_reader)?,
    {%- endif %}
    {%- endif %}
    {%- for arg in callable.arguments %}
    {%- if arg.uses_buffer() %}
    {{ arg.ty.read_fn_rs() }}(uniffi_reader)?,
    {%- endif %}
    {%- endfor %}
)))?;
{%- endif %}

{%- if let Some(receiver) = callable.receiver %}
{%- match receiver.strategy %}
{%- when ReceiverStrategy::InterfaceRef(inner_type_rs, ffi_arg) %}
let uniffi_self_ptr = ::std::ptr::with_exposed_provenance::<{{ inner_type_rs }}>({{ ffi_arg.name_rs() }} as usize);
let uniffi_self = &*uniffi_self_ptr;
{%- when ReceiverStrategy::TraitInterfaceRef(inner_type_rs, ffi_arg, ffi_arg2) %}
let uniffi_raw_ptr1 = ::std::ptr::with_exposed_provenance_mut::<()>({{ ffi_arg.name_rs() }} as usize);
let uniffi_raw_ptr2 = ::std::ptr::with_exposed_provenance_mut::<()>({{ ffi_arg2.name_rs() }} as usize);
let uniffi_self_ptr: *const dyn {{ inner_type_rs }} = ::std::mem::transmute([uniffi_raw_ptr1, uniffi_raw_ptr2]);
let uniffi_self = &*uniffi_self_ptr;
{%- when ReceiverStrategy::Arg(arg_strategy) %}
{%- match arg_strategy %}
{%- when ArgStrategy::Primitive(ffi_arg) %}
let uniffi_self = {{ receiver.ty.lift_fn_rs() }}(uniffi_env, {{ ffi_arg.name_rs() }})?;
{%- when ArgStrategy::Deconstruct(ffi_args) %}
let uniffi_self = {{ receiver.ty.lift_fn_rs() }}(
    uniffi_env,
    {%- for ffi_arg in ffi_args %}
    {{ ffi_arg.name_rs() }},
    {%- endfor %}
)?;
{%- when ArgStrategy::FfiBuffer %}
{%- endmatch %}
{%- endmatch %}
{%- endif %}

{%- for arg in callable.arguments %}
{%- match arg.strategy %}
{%- when ArgStrategy::Primitive(ffi_arg) %}
let {{ arg.name_rs() }} = {{ arg.ty.lift_fn_rs() }}(uniffi_env, {{ ffi_arg.name_rs() }})?;
{%- when ArgStrategy::Deconstruct(ffi_args) %}
let {{ arg.name_rs() }} = {{ arg.ty.lift_fn_rs() }}(
    uniffi_env,
    {%- for ffi_arg in ffi_args %}
    {{ ffi_arg.name_rs() }},
    {%- endfor %}
)?;
{%- when ArgStrategy::FfiBuffer %}
{%- endmatch %}
{%- endfor %}
