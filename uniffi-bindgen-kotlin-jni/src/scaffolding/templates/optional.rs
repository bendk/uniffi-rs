{%- let type_name = opt.self_type.type_rs %}

/// Read a {{ type_name }} from a `FfiBufferCursor`
pub fn {{ opt.self_type.read_fn_rs() }}(
    cursor: &mut uniffi::FfiBufferCursor,
) -> uniffi::Result<{{ type_name }}> {
    Ok(match cursor.read_u8()? {
        0 => ::std::option::Option::None,
        1 => ::std::option::Option::Some({{ opt.inner.read_fn_rs() }}(cursor)?),
        n => uniffi::deps::anyhow::bail!("{{ opt.self_type.read_fn_rs() }}: invalid discriminent: {n}"),
    })
}

/// Write a {{ type_name }} to a `FfiBufferCursor`
pub fn {{ opt.self_type.write_fn_rs() }}(
    cursor: &mut uniffi::FfiBufferCursor,
    value: {{ type_name }},
) -> uniffi::Result<()> {
    match value {
        ::std::option::Option::None => cursor.write_u8(0)?,
        ::std::option::Option::Some(inner_value) => {
            cursor.write_u8(1)?;
            {{ opt.inner.write_fn_rs() }}(cursor, inner_value)?
        }
    }
    Ok(())
}

{%- if let Some(LowerableType::Deconstructable(ffi_types)) = opt.self_type.lowerable %}

pub fn {{ opt.self_type.lower_fn_rs() }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    value: {{ type_name }},
) -> uniffi::Result<({%- for ffi_type in ffi_types %}{{ ffi_type.type_rs() }}, {%- endfor %})> {
    Ok(match value {
        ::std::option::Option::Some(v) => {
            let inner_lowered = {{ opt.inner.lower_fn_rs() }}(uniffi_env, v)?;
            (
                true,
                {%- if opt.inner.is_primitive() %}
                inner_lowered,
                {%- else %}
                {%- for _ in ffi_types.iter().skip(1) %}
                inner_lowered.{{ loop.index0 }},
                {%- endfor %}
                {%- endif %}
            )
        }
        ::std::option::Option::None => {
            (
                false,
                {%- for ffi_type in ffi_types.iter().skip(1) %}
                <{{ ffi_type.type_rs() }} as ::std::default::Default>::default(),
                {%- endfor %}
            )
        }
    })
}

pub fn {{ opt.self_type.lift_fn_rs() }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    {%- for ffi_type in ffi_types %}
    v{{ loop.index0 }}: {{ ffi_type.type_rs() }},
    {%- endfor %}
) -> uniffi::Result<{{ type_name }}> {
    Ok(if v0 {
        ::std::option::Option::Some({{ opt.inner.lift_fn_rs() }}(
            uniffi_env,
            {%- for _ in ffi_types.iter().skip(1) %}
            v{{ loop.index0 + 1 }},
            {%- endfor %}
        )?)
    } else {
        ::std::option::Option::None
    })
}
{%- endif %}
