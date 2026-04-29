{%- let type_name = rec.self_type.type_kt %}

fun {{ rec.self_type.read_fn_kt() }}(cursor: uniffi.FfiBufferCursor): {{ type_name }} {
    return {{ type_name }}(
        {%- for field in rec.fields %}
        {{ field.ty.read_fn_kt() }}(cursor),
        {%- endfor %}
    )
}

fun {{ rec.self_type.write_fn_kt() }}(cursor: uniffi.FfiBufferCursor, value: {{ type_name }}) {
    {%- for field in rec.fields %}
    {{ field.ty.write_fn_kt() }}(cursor, value.{{ field.name_kt() }})
    {%- endfor %}
}

{%- if let Some(deconstructable) = rec.deconstructable %}
{%- let deconstructed_type = rec.self_type.deconstructed_type_kt() %}

// Deconstructed version of {{ rec.name }}
class {{ deconstructed_type }}(
    {%- for ffi_type in deconstructable.ffi_types() %}
    val v{{ loop.index0 }}: {{ ffi_type.type_kt() }},
    {%- endfor %}
)

fun {{ rec.self_type.lower_fn_kt() }}(rec: {{ type_name }}): {{ deconstructed_type }} {
    // Prepare by deconstructing all recursive types
    {%- for source_field in deconstructable.source_fields %}
    {%- if source_field.is_recursive() %}
    val uniffiFieldDeconstructed{{ source_field.index }} = {{ source_field.ty.lower_fn_kt() }}(rec.{{ source_field.name_kt() }})
    {%- endif %}
    {%- endfor %}

    return {{ deconstructed_type }}(
        {%- for source_field in deconstructable.source_fields %}
        {%- match source_field.kind %}
        {%- when DeconstructableFieldKind::Primitive(ffi_field) %}
        {{ source_field.ty.lower_fn_kt() }}(rec.{{ source_field.name_kt() }}),
        {%- when DeconstructableFieldKind::Recursive(ffi_fields) %}
        {%- for ffi_field in ffi_fields %}
        {{ source_field.ty.lower_fn_kt() }}(uniffiFieldDeconstructed{{ source_field.index }}.v{{ loop.index0 }}),
        {%- endfor %}
        {%- endmatch %}
        {%- endfor %}
    )
}

fun {{ rec.self_type.lift_fn_kt() }}(
    {%- for ffi_type in deconstructable.ffi_types() %}
    v{{ loop.index0 }}: {{ ffi_type.type_kt() }},
    {%- endfor %}
): {{ type_name }} {
    return {{ type_name }}(
        {%- for source_field in deconstructable.source_fields %}
        {%- match source_field.kind %}
        {%- when DeconstructableFieldKind::Primitive(ffi_field) %}
        {{ source_field.ty.lift_fn_kt() }}(v{{ ffi_field.index }}),
        {%- when DeconstructableFieldKind::Recursive(ffi_fields) %}
        {{ source_field.ty.lift_fn_kt() }}(
            {%- for ffi_field in ffi_fields %}
            v{{ ffi_field.index }},
            {%- endfor %}
        ),
        {%- endmatch %}
        {%- endfor %}
    )
}
{%- endif %}
