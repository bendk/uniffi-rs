{%- let type_name = rec.self_type.type_rs %}

/// Read a {{ type_name }} from a `FfiBufferCursor`
pub fn {{ rec.self_type.read_fn_rs() }}(
    cursor: &mut uniffi::FfiBufferCursor,
) -> uniffi::Result<{{ type_name }}> {
    Ok({{ type_name }} {
        {%- for field in rec.fields %}
        {{ field.name_rs() }}: {{ field.ty.read_fn_rs() }}(cursor)?,
        {%- endfor %}
    })
}

/// Write a {{ type_name }} to a `FfiBufferCursor`
pub fn {{ rec.self_type.write_fn_rs() }}(
    cursor: &mut uniffi::FfiBufferCursor,
    value: {{ type_name }},
) -> uniffi::Result<()> {
    {%- for field in rec.fields %}
    {{ field.ty.write_fn_rs() }}(cursor, value.{{ field.name_rs() }})?;
    {%- endfor %}
    Ok(())
}

{%- if let Some(deconstructable) = rec.deconstructable %}

pub fn {{ rec.self_type.lower_fn_rs() }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    rec: {{ type_name }},
) -> uniffi::Result<({% for ffi_type in deconstructable.ffi_types() %}{{ ffi_type.type_rs() }}, {% endfor %})> {
    // Prepare by deconstructing all recursive fields

    {%- for source_field in deconstructable.source_fields %}
    // Safety:
    // * uniffi_env points to a valid JNIEnv
    let uniffi_field_lowered_{{ source_field.index }} = unsafe {
        {{ source_field.ty.lower_fn_rs() }}(uniffi_env, rec.{{ source_field.name_rs() }})?
    };
    {%- endfor %}

    Ok((
        {%- for source_field in deconstructable.source_fields %}
        {%- match source_field.kind %}
        {%- when DeconstructableFieldKind::Primitive(ffi_field) %}
        uniffi_field_lowered_{{ source_field.index }},
        {%- when DeconstructableFieldKind::Recursive(ffi_fields) %}
        {%- for ffi_field in ffi_fields %}
        uniffi_field_lowered_{{ source_field.index }}.{{ loop.index0 }},
        {%- endfor %}
        {%- endmatch %}
        {%- endfor %}
    ))
}

pub fn {{ rec.self_type.lift_fn_rs() }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    {%- for ffi_type in deconstructable.ffi_types() %}
    v{{ loop.index0 }}: {{ ffi_type.type_rs() }},
    {%- endfor %}
) -> uniffi::Result<{{ type_name }}> {
    // Safety:
    // * uniffi_env points to a valid JNIEnv
    unsafe {
        Ok({{ type_name }} {
            {%- for source_field in deconstructable.source_fields %}
            {%- match source_field.kind %}
            {%- when DeconstructableFieldKind::Primitive(ffi_field) %}
            {{ source_field.name_rs() }}: {{ source_field.ty.lift_fn_rs() }}(uniffi_env, v{{ ffi_field.index }})?,
            {%- when DeconstructableFieldKind::Recursive(ffi_fields) %}
            {{ source_field.name_rs() }}: {{ source_field.ty.lift_fn_rs() }}(
                uniffi_env,
                {%- for ffi_field in ffi_fields %}
                v{{ ffi_field.index }}),
                {%- endfor %}
            )?,
            {%- endmatch %}
            {%- endfor %}
        })
    }
}
{%- endif %}
