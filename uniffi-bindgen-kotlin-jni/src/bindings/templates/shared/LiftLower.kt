fun liftUByte(v: Byte): UByte = v.toUByte()
fun liftByte(v: Byte): Byte = v
fun liftUShort(v: Short): UShort = v.toUShort()
fun liftShort(v: Short): Short = v
fun liftUInt(v: Int): UInt = v.toUInt()
fun liftInt(v: Int): Int = v
fun liftULong(v: Long): ULong = v.toULong()
fun liftLong(v: Long): Long = v
fun liftFloat(v: Float): Float = v
fun liftDouble(v: Double): Double = v
fun liftBoolean(v: Boolean): Boolean = v
fun liftString(v: String): String = v

fun lowerUByte(v: UByte): Byte = v.toByte()
fun lowerByte(v: Byte): Byte = v
fun lowerUShort(v: UShort): Short = v.toShort()
fun lowerShort(v: Short): Short = v
fun lowerUInt(v: UInt): Int = v.toInt()
fun lowerInt(v: Int): Int = v
fun lowerULong(v: ULong): Long = v.toLong()
fun lowerLong(v: Long): Long = v
fun lowerFloat(v: Float): Float = v
fun lowerDouble(v: Double): Double = v
fun lowerBoolean(v: Boolean): Boolean = v
fun lowerString(v: String): String = v

{#
 # Define lift functions for FfiBuffer-based types.
 # This is essentially the read function, except it inputs a buffer handle instead of a `FfiBufferCursor`
 #}

{%- for type_node in root.rust_return_and_throws_types() %}
{%- if type_node.uses_buffer() %}
/// Construct a new `{{ type_node.type_kt }}` instance
fun {{ type_node.lift_fn_kt() }}(buffer: Long) : {{ type_node.type_kt }} {
    return {{ type_node.read_fn_kt() }}(uniffi.FfiBufferCursor(buffer))
}
{%- endif %}
{%- endfor %}
