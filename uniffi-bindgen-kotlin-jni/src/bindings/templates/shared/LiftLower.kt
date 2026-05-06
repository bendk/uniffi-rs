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
fun liftOptionUByte(v: Long): UByte? = if (v == Long.MAX_VALUE) { null } else { v.toUByte() }
fun liftOptionByte(v: Long): Byte? = if (v == Long.MAX_VALUE) { null } else { v.toByte() }
fun liftOptionUShort(v: Long): UShort? = if (v == Long.MAX_VALUE) { null } else { v.toUShort() }
fun liftOptionShort(v: Long): Short? = if (v == Long.MAX_VALUE) { null } else { v.toShort() }
fun liftOptionUInt(v: Long): UInt? = if (v == Long.MAX_VALUE) { null } else { v.toUInt() }
fun liftOptionInt(v: Long): Int? = if (v == Long.MAX_VALUE) { null } else { v.toInt() }
fun liftOptionBoolean(v: Long): Boolean? = if (v == Long.MAX_VALUE) { null } else { v == 1L }
fun liftOptionString(v: String?): String? = v

fun liftOptionFloat(v: Int): Float? {
    return if (v == 0xFFFF_FFFF.toInt()) {
        null
    } else {
        Float.fromBits(v)
    }
}

fun liftOptionDouble(v: Long): Double? {
    return if (v.toULong() == 0xFFFF_FFFF_FFFF_FFFFuL) {
        null
    } else {
        Double.fromBits(v)
    }
}

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
fun lowerOptionUByte(v: UByte?): Long = if (v == null) { Long.MAX_VALUE } else { v.toLong() }
fun lowerOptionByte(v: Byte?): Long = if (v == null) { Long.MAX_VALUE } else { v.toLong() }
fun lowerOptionUShort(v: UShort?): Long = if (v == null) { Long.MAX_VALUE } else { v.toLong() }
fun lowerOptionShort(v: Short?): Long = if (v == null) { Long.MAX_VALUE } else { v.toLong() }
fun lowerOptionUInt(v: UInt?): Long = if (v == null) { Long.MAX_VALUE } else { v.toLong() }
fun lowerOptionInt(v: Int?): Long = if (v == null) { Long.MAX_VALUE } else { v.toLong() }
fun lowerOptionBoolean(v: Boolean?): Long = if (v == null) { Long.MAX_VALUE } else { if (v) { 1 } else { 0 } }
fun lowerOptionString(v: String?): String? = v

fun lowerOptionFloat(v: Float?): Int {
    return if (v == null) {
        0xFFFF_FFFF.toInt()
    } else {
        val bits = v.toRawBits()
        if (bits == 0xFFFF_FFFF.toInt()) {
            // The float was encoded using our special-cased NaN value.
            // Convert it to the "preferred" NaN value
            0xFFC0_0000.toInt()
        } else {
            bits
        }
    }
}

fun lowerOptionDouble(v: Double?): Long {
    return if (v == null) {
        0xFFFF_FFFF_FFFF_FFFFuL.toLong()
    } else {
        val bits = v.toRawBits()
        if (bits.toULong() == 0xFFFF_FFFF_FFFF_FFFFuL) {
            // The float was encoded using our special-cased NaN value.
            // Convert it to the "preferred" NaN value
            0xFFF8_0000
        } else {
            bits
        }
    }
}

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
