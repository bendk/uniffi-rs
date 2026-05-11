{%- let type_name = en.self_type.type_rs %}

{%- if en.is_flat_error() %}
// Note: no read function, since passing flat errors from Kotlin to Rust is not allowed.

/// Write a {{ type_name }} to a `FfiBufferCursor`
pub fn {{ en.self_type.write_fn_rs() }}(
    cursor: &mut uniffi::FfiBufferCursor,
    value: {{ type_name }},
) -> uniffi::Result<()> {
    match value {
        {%- for v in en.variants %}
        {{ type_name }}::{{ v.name_rs() }} { .. } => {
            uniffi::FfiBufferCursor::write_u32(cursor, {{ loop.index0 }})?;
            uniffi::FfiBufferCursor::write_string(cursor, value.to_string())?;
        }
        {%- endfor %}
    }
    Ok(())
}
{%- else %}
/// Read a {{ type_name }} from a `FfiBufferCursor`
pub fn {{ en.self_type.read_fn_rs() }}(
    cursor: &mut uniffi::FfiBufferCursor,
) -> uniffi::Result<{{ type_name }}> {
    Ok(match uniffi::FfiBufferCursor::read_u32(cursor)? {
        {%- for v in en.variants %}
        {{ loop.index0 }} => {
        {%- match v.fields_kind %}
            {%- when FieldsKind::Unit %}
            {{ type_name }}::{{ v.name_rs() }}
            {%- when FieldsKind::Named %}
            {{ type_name }}::{{ v.name_rs() }} {
                {%- for f in v.fields %}
                {{ f.name_rs() }}: {{ f.ty.read_fn_rs() }}(cursor)?,
                {%- endfor %}
            }
            {%- when FieldsKind::Unnamed %}
            {{ type_name }}::{{ v.name_rs() }} (
                {%- for f in v.fields %}
                {{ f.ty.read_fn_rs() }}(cursor)?,
                {%- endfor %}
            )
            {%- endmatch %}
        }
        {%- endfor %}
        d => uniffi::deps::anyhow::bail!("Invalid {{ type_name }} discriminent: {d}"),
    })
}

/// Write a {{ type_name }} to a `FfiBufferCursor`
pub fn {{ en.self_type.write_fn_rs() }}(
    cursor: &mut uniffi::FfiBufferCursor,
    value: {{ type_name }},
) -> uniffi::Result<()> {
    match value {
    {%- for v in en.variants %}
        {%- match v.fields_kind %}
        {%- when FieldsKind::Unit %}
        {{ type_name }}::{{ v.name_rs() }} => {
            uniffi::FfiBufferCursor::write_u32(cursor, {{ loop.index0 }})?;
        }
        {%- when FieldsKind::Named %}
        {{ type_name }}::{{ v.name_rs() }} {
            {%- for f in v.fields %}
            {{ f.name_rs() }},
            {%- endfor %}
        } => {
            uniffi::FfiBufferCursor::write_u32(cursor, {{ loop.index0 }})?;
            {%- for f in v.fields %}
            {{ f.ty.write_fn_rs() }}(cursor, {{ f.name_rs() }})?;
            {%- endfor %}
        }
        {%- when FieldsKind::Unnamed %}
        {{ type_name }}::{{ v.name_rs() }} (
            {%- for f in v.fields %}
            v{{ loop.index }},
            {%- endfor %}
        ) => {
            uniffi::FfiBufferCursor::write_u32(cursor, {{ loop.index0 }})?;
            {%- for f in v.fields %}
            {{ f.ty.write_fn_rs() }}(cursor, v{{ loop.index }})?;
            {%- endfor %}
        }
        {%- endmatch %}
        {%- endfor %}
    }
    Ok(())
}
{%- endif %}

{%- match en.lowerable %}
{%- when Some(LowerableEnum::Primitive) %}

pub fn {{ en.self_type.lower_fn_rs() }}(
    _uniffi_env: *mut uniffi_jni::JNIEnv,
    uniffi_value: {{ type_name }},
) -> uniffi::Result<i32> {
    match uniffi_value {
        {%- for v in en.variants %}
        {{ type_name }}::{{ v.name_rs() }} { .. } => Ok({{ loop.index0 }}_u32 as i32),
        {%- endfor %}
    }
}

{%- if !en.is_flat_error() %}
pub fn {{ en.self_type.lift_fn_rs() }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    v0: i32,
) -> uniffi::Result<{{ type_name }}> {
    match v0 as u32 {
        {%- for v in en.variants %}
        {{ loop.index0 }} => Ok({{ type_name }}::{{ v.name_rs() }}),
        {%- endfor %}
        d => uniffi::deps::anyhow::bail!("{{ en.self_type.lift_fn_rs() }}: invalid discriminent: {d}"),
    }
}
{%- endif %}

{%- when Some(LowerableEnum::Deconstructable(deconstructable)) %}

pub fn {{ en.self_type.lower_fn_rs() }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    uniffi_value: {{ type_name }},
) -> uniffi::Result<({% for f in deconstructable.ffi_fields %}{{ f.ty.type_rs() }}, {% endfor %})> {
    match uniffi_value {
        {%- for v in deconstructable.variants %}
        {%- match v.fields_kind %}
        {%- when FieldsKind::Unit %}
        {{ type_name }}::{{ v.name_rs() }} => {
        {%- when FieldsKind::Named %}
        {{ type_name }}::{{ v.name_rs() }} {
            {%- for f in v.source_fields %}
            {{ f.name_rs() }}: uniffi_field{{ loop.index0 }},
            {%- endfor %}
        } => {
        {%- when FieldsKind::Unnamed %}
        {{ type_name }}::{{ v.name_rs() }} (
            {%- for f in v.source_fields %}
            uniffi_field{{ loop.index0 }},
            {%- endfor %}
        ) => {
        {%- endmatch %}
            // Prepare by lowering/deconstructing all fields
            {%- for source_field in v.source_fields %}
            // Safety:
            // * uniffi_env points to a valid JNIEnv
            let uniffi_field_lowered_{{ source_field.index }} = unsafe {
                {{ source_field.ty.lower_fn_rs() }}(uniffi_env, uniffi_field{{ loop.index0 }})?
            };
            {%- endfor %}

            Ok((
                {{ loop.index0 }},
                {%- for ffi_field_source in v.enum_ffi_field_sources %}
                {%- match ffi_field_source %}
                {%- when EnumFfiFieldSource::Default { .. } %}
                ::std::default::Default::default(),
                {%- when EnumFfiFieldSource::Primitive { source_field } %}
                uniffi_field_lowered_{{ source_field }},
                {%- when EnumFfiFieldSource::Recursive { source_field, index } %}
                uniffi_field_lowered_{{ source_field}}.{{ index }},
                {%- endmatch %}
                {%- endfor %}
            ))
        }
        {%- endfor %}
    }
}

pub fn {{ en.self_type.lift_fn_rs() }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    {%- for ffi_field in deconstructable.ffi_fields %}
    v{{ ffi_field.index }}: {{ ffi_field.ty.type_rs() }},
    {%- endfor %}
) -> uniffi::Result<{{ type_name }}> {
    match v0 as u32 {
        {%- for v in deconstructable.variants %}
        {{ loop.index0 }} => {
            {%- for source_field in v.source_fields %}
            {%- match source_field.kind %}
            {%- when DeconstructableFieldKind::Primitive(ffi_field) %}
            // Safety:
            // * uniffi_env points to a valid JNIEnv
            let uniffi_field{{ source_field.index }} = unsafe {
                {{ source_field.ty.lift_fn_rs() }}(uniffi_env, v{{ ffi_field.index }})?
            };
            {%- when DeconstructableFieldKind::Recursive(ffi_fields) %}
            // Safety:
            // * uniffi_env points to a valid JNIEnv
            let uniffi_field{{ source_field.index }} = unsafe {
                {{ source_field.ty.lift_fn_rs() }}(
                    uniffi_env,
                    {%- for ffi_field in ffi_fields %}
                    v{{ ffi_field.index }}),
                    {%- endfor %}
                )?
            };
            {%- endmatch %}
            {%- endfor %}

            {%- match v.fields_kind %}
            {%- when FieldsKind::Unit %}
            Ok({{ type_name }}::{{ v.name_rs() }})
            {%- when FieldsKind::Named %}
            Ok({{ type_name }}::{{ v.name_rs() }} {
                {%- for source_field in v.source_fields %}
                {{ source_field.name_rs() }}: uniffi_field{{ source_field.index }},
                {%- endfor %}
            })
            {%- when FieldsKind::Unnamed %}
            Ok({{ type_name }}::{{ v.name_rs() }}(
                {%- for source_field in v.source_fields %}
                uniffi_field{{ source_field.index }},
                {%- endfor %}
            ))
            {%- endmatch %}
        }
        {%- endfor %}
        d => uniffi::deps::anyhow::bail!("{{ en.self_type.lift_fn_rs() }}: invalid discriminent: {d}"),
    }
}
{%- when None %}
{%- endmatch %}
