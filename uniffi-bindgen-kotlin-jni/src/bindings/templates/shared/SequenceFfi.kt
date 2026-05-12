{%- let type_name = seq.self_type.type_kt %}
{%- match seq.inner.ty %}
{%- when Type::UInt32 %}

fun {{ seq.self_type.read_fn_kt() }}(cursor: uniffi.FfiBufferCursor): {{ type_name }} {
    val len = readULong(cursor).toInt()
    return List(len) {
        {{ seq.inner.read_fn_kt() }}(cursor)
    }
}

fun {{ seq.self_type.write_fn_kt() }}(cursor: uniffi.FfiBufferCursor, value: {{ type_name }}) {
    writeULong(cursor, value.size.toULong())
    var remaining = value.size.toLong()

    var pos = 0;
    while (true) {
        val remainingSpace = cursor.minibufRemaining() / 4
        if (remaining <= remainingSpace) {
            val indexEnd = cursor.index + (remaining.toInt() * 4)
            for(i in cursor.index..<indexEnd step 4) {
                cursor.byteBuf.putInt(i, value[pos].toInt())
                pos += 1
            }
            cursor.index = indexEnd
            break
        }
        for(i in cursor.index..<(cursor.end-cursor.ptr) step 4) {
            cursor.byteBuf.putInt(i.toInt(), value[pos].toInt())
            pos += 1
        }
        cursor.advanceToNextMinibuf()
        remaining -= remainingSpace
    }

    // value.iterator().forEach {
    //     cursor.prepare(4, 4)
    //     //cursor.byteBuf.putInt(cursor.index, value)
    //     cursor.index += 4
    // }
}

{%- when Type::Int32 %}
fun {{ seq.self_type.read_fn_kt() }}(cursor: uniffi.FfiBufferCursor): {{ type_name }} {
    val len = readULong(cursor).toInt()
    return List(len) {
        {{ seq.inner.read_fn_kt() }}(cursor)
    }
}

fun {{ seq.self_type.write_fn_kt() }}(cursor: uniffi.FfiBufferCursor, value: {{ type_name }}) {
    writeULong(cursor, value.size.toULong())
    value.iterator().forEach {
        {{ seq.inner.write_fn_kt() }}(cursor, it)
    }
}

{%- else %}
fun {{ seq.self_type.read_fn_kt() }}(cursor: uniffi.FfiBufferCursor): {{ type_name }} {
    val len = readULong(cursor).toInt()
    return List<{{ seq.inner.type_kt }}>(len) {
        {{ seq.inner.read_fn_kt() }}(cursor)
    }
}

fun {{ seq.self_type.write_fn_kt() }}(cursor: uniffi.FfiBufferCursor, value: {{ type_name }}) {
    writeULong(cursor, value.size.toULong())
    value.iterator().forEach {
        {{ seq.inner.write_fn_kt() }}(cursor, it)
    }
}
{%- endmatch %}
