{%- let type_name = opt.self_type.type_kt %}

fun {{ opt.self_type.read_fn_kt() }}(cursor: uniffi.FfiBufferCursor): {{ type_name }} {
    if (readUByte(cursor) == 0.toUByte()) {
        return null
    }
    return {{ opt.inner.read_fn_kt() }}(cursor)
}

fun {{ opt.self_type.write_fn_kt() }}(cursor: uniffi.FfiBufferCursor, value: {{ type_name }}) {
    if (value == null) {
        writeUByte(cursor, 0.toUByte())
    } else {
        writeUByte(cursor, 1.toUByte())
        {{ opt.inner.write_fn_kt() }}(cursor, value)
    }
}

{%- if let Some(LowerableType::Deconstructable(ffi_types)) = opt.self_type.lowerable %}
{%- let deconstructed_type = opt.self_type.deconstructed_type_kt() %}

// Deconstructed version of {{ type_name }}
class {{ deconstructed_type }}(
    {%- for ffi_type in ffi_types %}
    val v{{ loop.index0 }}: {{ ffi_type.type_kt() }},
    {%- endfor %}
)

fun {{ opt.self_type.lower_fn_kt() }}(value: {{ type_name }}): {{ deconstructed_type }} {
    if (value == null) {
        return {{ deconstructed_type }}(
            false,
            {%- for ffi_type in ffi_types.iter().skip(1) %}
            {{ ffi_type.default_kt() }},
            {%- endfor %}
        )
    } else {
        val uniffiFieldLowered = {{ opt.inner.lower_fn_kt() }}(value)
        return {{ deconstructed_type }}(
            true,
            {%- if opt.inner.is_primitive() %}
            uniffiFieldLowered,
            {%- else %}
            {%- for _ in ffi_types.iter().skip(1) %}
            uniffiFieldLowered.v{{ loop.index0 }},
            {%- endfor %}
            {%- endif %}
        )
    }
}

fun {{ opt.self_type.lift_fn_kt() }}(
    {%- for ffi_type in ffi_types %}
    v{{ loop.index0 }}: {{ ffi_type.type_kt() }},
    {%- endfor %}
): {{ type_name }} {
    if (v0) {
        return {{ opt.inner.lift_fn_kt() }}(
            {%- for _ in ffi_types.iter().skip(1) %}
            v{{ loop.index0 + 1 }},
            {%- endfor %}
        )
    } else {
        return null
    }
}
{%- endif %}
