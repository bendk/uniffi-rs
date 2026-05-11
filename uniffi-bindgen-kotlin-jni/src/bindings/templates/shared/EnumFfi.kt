{%- let type_name = en.self_type.type_kt %}

{%- match en.kotlin_kind %}
{%- when KotlinEnumKind::EnumClass { .. } %}

fun {{ en.self_type.read_fn_kt() }}(cursor: uniffi.FfiBufferCursor): {{ type_name }} {
    val uniffiDiscriminent = uniffi.readUInt(cursor).toInt()
    return try {
        {%- if en.use_entries %}
        {{ type_name }}.entries[uniffiDiscriminent]
        {%- else %}
        {{ type_name }}.values()[uniffiDiscriminent]
        {%- endif %}
    } catch (e: IndexOutOfBoundsException) {
        throw uniffi.InternalException("{{ en.self_type.read_fn_kt() }}: Invalid enum value: ${uniffiDiscriminent}")
    }
}

fun {{ en.self_type.write_fn_kt() }}(cursor: uniffi.FfiBufferCursor, value: {{ type_name }}) {
    uniffi.writeUInt(cursor, value.ordinal.toUInt())
}

{%- when KotlinEnumKind::SealedClass %}
fun {{ en.self_type.read_fn_kt() }}(cursor: uniffi.FfiBufferCursor): {{ type_name }} {
    val uniffiDiscriminent = uniffi.readUInt(cursor)
    return when(uniffiDiscriminent) {
        {%- for v in en.variants %}
        {%- if v.fields.is_empty() && !en.self_type.is_used_as_error %}
        {{ loop.index0 }}u -> {{ type_name }}.{{ v.name_kt }}
        {%- else %}
        {{ loop.index0 }}u -> {{ type_name }}.{{ v.name_kt }}(
            {%- for f in v.fields %}
            {{ f.ty.read_fn_kt() }}(cursor),
            {%- endfor %}
        )
        {%- endif %}
        {%- endfor %}
        else -> throw uniffi.InternalException("{{ en.self_type.read_fn_kt() }}: Invalid enum value: ${uniffiDiscriminent}")
    }
}

fun {{ en.self_type.write_fn_kt() }}(cursor: uniffi.FfiBufferCursor, value: {{ type_name }}) {
    when(value) {
        {%- for v in en.variants %}
        is {{ type_name }}.{{ v.name_kt }} -> {
            uniffi.writeUInt(cursor, {{ loop.index0 }}u)
            {%- for f in v.fields %}
            {{ f.ty.write_fn_kt() }}(cursor, value.{{ f.name_kt() }})
            {%- endfor %}
            Unit
        }
        {%- endfor %}
    }
}

{%- when KotlinEnumKind::FlatError %}

fun {{ en.self_type.read_fn_kt() }}(cursor: uniffi.FfiBufferCursor): {{ type_name }} {
    val uniffiDiscriminent = uniffi.readUInt(cursor)
    return when(uniffiDiscriminent) {
        {%- for v in en.variants %}
        {{ loop.index0 }}u -> {{ type_name }}.{{ v.name_kt }}(uniffi.readString(cursor))
        {%- endfor %}
        else -> throw uniffi.InternalException("{{ en.self_type.read_fn_kt() }}: Invalid enum value: ${uniffiDiscriminent}")
    }
}

fun {{ en.self_type.write_fn_kt() }}(cursor: uniffi.FfiBufferCursor, value: {{ type_name }}) {
    throw uniffi.InternalException("{{ en.self_type.write_fn_kt() }}: writing flat errors is not supported")
}

{% endmatch %}

{%- match en.lowerable %}
{%- when Some(LowerableEnum::Primitive) %}

{%- if let KotlinEnumKind::EnumClass { .. } = en.kotlin_kind %}
fun {{ en.self_type.lower_fn_kt() }}(value: {{ type_name }}): Int {
    return value.ordinal
}

fun {{ en.self_type.lift_fn_kt() }}(value: Int): {{ type_name }} {
    {%- if en.use_entries %}
    return {{ type_name }}.entries[value]
    {%- else %}
    return {{ type_name }}.values()[value]
    {%- endif %}
}
{%- else %}
fun {{ en.self_type.lower_fn_kt() }}(value: {{ type_name }}): Int {
    return when (value) {
        {%- for v in en.variants %}
        is {{ type_name }}.{{ v.name_kt }} -> {{ loop.index0 }}
        {%- endfor %}
    }
}

fun {{ en.self_type.lift_fn_kt() }}(value: Int): {{ type_name }} {
    return when (value) {
        {%- for v in en.variants %}
        {{ loop.index0 }} -> {{ type_name }}.{{ v.name_kt }}
        {%- endfor %}
        else -> {
            throw uniffi.InternalException("{{ en.self_type.lift_fn_kt() }}: Invalid enum value: ${v0.toUInt()}")
        }
    }
}
{%- endif %}

{%- when Some(LowerableEnum::Deconstructable(deconstructable)) %}
{%- let deconstructed_type = en.self_type.deconstructed_type_kt() %}

// Deconstructed version of {{ en.name }}
class {{ deconstructed_type }}(
    {%- for ffi_field in deconstructable.ffi_fields %}
    val v{{ loop.index0 }}: {{ ffi_field.ty.type_kt() }},
    {%- endfor %}
)

fun {{ en.self_type.lower_fn_kt() }}(value: {{ type_name }}): {{ deconstructed_type }} {
    when (value) {
        {%- for v in deconstructable.variants %}
        is {{ type_name }}.{{ v.name_kt }} -> {
            // Prepare by lowering/deconstructing all fields
            {%- for source_field in v.source_fields %}
            val uniffiFieldLowered{{ source_field.index }} = {{ source_field.ty.lower_fn_kt() }}(value. {{ source_field.name_kt() }})
            {%- endfor %}

            return {{ deconstructed_type }}(
                {{ loop.index0 }}u.toInt(),
                {%- for ffi_field_source in v.enum_ffi_field_sources %}
                {%- match ffi_field_source %}
                {%- when EnumFfiFieldSource::Default { ffi_type } %}
                {{ ffi_type.default_kt() }},
                {%- when EnumFfiFieldSource::Primitive { source_field } %}
                uniffiFieldLowered{{ source_field }},
                {%- when EnumFfiFieldSource::Recursive { source_field, index } %}
                uniffiFieldLowered{{ source_field }}.v{{ index }},
                {%- endmatch %}
                {%- endfor %}
            )
        }
        {%- endfor %}
    }
}

fun {{ en.self_type.lift_fn_kt() }}(
    {%- for ffi_field in deconstructable.ffi_fields %}
    v{{ ffi_field.index }}: {{ ffi_field.ty.type_kt() }},
    {%- endfor %}
): {{ type_name }} {
    when (v0.toUInt()) {
        {%- for v in deconstructable.variants %}
        {{ loop.index0 }}u -> {
            {%- if v.source_fields.is_empty() && !en.self_type.is_used_as_error %}
            return {{ type_name }}.{{ v.name_kt }}
            {%- else %}
            {%- for source_field in v.source_fields %}
            {%- match source_field.kind %}
            {%- when DeconstructableFieldKind::Primitive(ffi_field) %}
            val uniffiField{{ source_field.index }} = {{ source_field.ty.lift_fn_kt() }}(
                {%- if source_field.ty.is_string() %}
                {# Strings are always nullible in the function signtature, so that variants that don't contain them can be passed as `null`.
                 # However, the other side of the FFI never sends `null` for variants that do contain the string.
                 #}
                v{{ ffi_field.index }}!!
                {%- else %}
                v{{ ffi_field.index }}
                {%- endif %}
            )
            {%- when DeconstructableFieldKind::Recursive(ffi_fields) %}
            val uniffiField{{ source_field.index }} = {{ source_field.ty.lift_fn_kt() }}(
                uniffi_env,
                {%- for ffi_field in ffi_fields %}
                {%- if source_field.ty.is_string() %}
                {# Strings are always nullible in the function signtature, so that variants that don't contain them can be passed as `null`.
                 # However, the other side of the FFI never sends `null` for variants that do contain the string.
                 #}
                v{{ ffi_field.index }}!!,
                {%- else %}
                v{{ ffi_field.index }},
                {%- endif %}
                {%- endfor %}
            )
            {%- endmatch %}
            {%- endfor %}

            return {{ type_name }}.{{ v.name_kt }}(
                {%- for source_field in v.source_fields %}
                uniffiField{{ source_field.index }},
                {%- endfor %}
            )
            {%- endif %}
        }
        {%- endfor %}
        else -> {
            throw uniffi.InternalException("{{ en.self_type.lift_fn_kt() }}: Invalid enum value: ${v0.toUInt()}")
        }
    }
}
{%- when None %}
{%- endmatch %}
