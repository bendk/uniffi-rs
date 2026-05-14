
// Note: Most of this code was copied from `uniffi_core/src/ffi/ffibuffer.rs` and adapted to Kotlin
const val BASE_MINI_BUFFER_SIZE: Long = 256L

/**
 * Used to read/write from a FFIbuffer
 *
 * This tracks the current position in the buffer.
 */
class FfiBufferCursor(byteBuffer: java.nio.ByteBuffer) {
    internal var byteBuf = byteBuffer.order(java.nio.ByteOrder.nativeOrder())
    internal var miniBufSize = BASE_MINI_BUFFER_SIZE
    internal var index = 0
    internal var end = BASE_MINI_BUFFER_SIZE - 8

    fun minibufRemaining(): Long {
        return end - index
    }

    fun advanceToNextMinibuf() {
        byteBuf = uniffi.Scaffolding.miniBufferNext(byteBuf, miniBufSize).order(java.nio.ByteOrder.nativeOrder())
        miniBufSize *= 2L
        end = miniBufSize - 8L
        index = 0
    }

    // Prepare for a read or write
    //
    // This ensures `this.ptr` is properly aligned and there's enough space left in the current mini
    // buffer.
    fun prepare(align: Int, size: Int) {
        // Offset needed to properly align the pointer
        val alignOffset = (-index).mod(align)
        if (index + size > end) {
            advanceToNextMinibuf()
        } else {
            index += alignOffset
        }
    }
}

fun readByte(cursor: FfiBufferCursor): Byte {
    cursor.prepare(1, 1)
    val value = cursor.byteBuf.get(cursor.index)
    cursor.index += 1
    return value
}

fun readUByte(cursor: FfiBufferCursor): UByte {
    return readByte(cursor).toUByte()
}

fun readShort(cursor: FfiBufferCursor): Short {
    cursor.prepare(2, 2)
    val value = cursor.byteBuf.getShort(cursor.index)
    cursor.index += 2
    return value
}

fun readUShort(cursor: FfiBufferCursor): UShort {
    return readShort(cursor).toUShort()
}

fun readInt(cursor: FfiBufferCursor): Int {
    cursor.prepare(4, 4)
    val value = cursor.byteBuf.getInt(cursor.index)
    cursor.index += 4
    return value
}

fun readUInt(cursor: FfiBufferCursor): UInt {
    return readInt(cursor).toUInt()
}

fun readLong(cursor: FfiBufferCursor): Long {
    cursor.prepare(8, 8)
    val value = cursor.byteBuf.getLong(cursor.index)
    cursor.index += 8
    return value
}

fun readULong(cursor: FfiBufferCursor): ULong {
    return readLong(cursor).toULong()
}

fun readFloat(cursor: FfiBufferCursor): Float {
    cursor.prepare(4, 4)
    val value = cursor.byteBuf.getFloat(cursor.index)
    cursor.index += 4
    return value
}

fun readDouble(cursor: FfiBufferCursor): Double {
    cursor.prepare(8, 8)
    val value = cursor.byteBuf.getDouble(cursor.index)
    cursor.index += 8
    return value
}

fun readBool(cursor: FfiBufferCursor): Boolean {
    return readByte(cursor) == 1.toByte()
}

fun readString(cursor: FfiBufferCursor): String {
    // Strings are stored as 3 64-bit values
    cursor.prepare(8, 24)
    val data = cursor.byteBuf.getLong(cursor.index)
    val length = cursor.byteBuf.getLong(cursor.index + 8)
    val capacity = cursor.byteBuf.getLong(cursor.index + 16)
    val value = Scaffolding.readString(data, length, capacity)
    cursor.index += 24
    return value
}

fun writeByte(cursor: FfiBufferCursor, value: Byte) {
    cursor.prepare(1, 1)
    cursor.byteBuf.put(cursor.index, value)
    cursor.index += 1
}

fun writeUByte(cursor: FfiBufferCursor, value: UByte) {
    writeByte(cursor, value.toByte())
}

fun writeShort(cursor: FfiBufferCursor, value: Short) {
    cursor.prepare(2, 2)
    cursor.byteBuf.putShort(cursor.index, value)
    cursor.index += 2
}

fun writeUShort(cursor: FfiBufferCursor, value: UShort) {
    writeShort(cursor, value.toShort())
}

fun writeInt(cursor: FfiBufferCursor, value: Int) {
    cursor.prepare(4, 4)
    cursor.byteBuf.putInt(cursor.index, value)
    cursor.index += 4
}

fun writeUInt(cursor: FfiBufferCursor, value: UInt) {
    writeInt(cursor, value.toInt())
}

fun writeLong(cursor: FfiBufferCursor, value: Long) {
    cursor.prepare(8, 8)
    cursor.byteBuf.putLong(cursor.index, value)
    cursor.index += 8
}

fun writeULong(cursor: FfiBufferCursor, value: ULong) {
    writeLong(cursor, value.toLong())
}

fun writeFloat(cursor: FfiBufferCursor, value: Float) {
    cursor.prepare(4, 4)
    cursor.byteBuf.putFloat(cursor.index, value)
    cursor.index += 4
}

fun writeDouble(cursor: FfiBufferCursor, value: Double) {
    cursor.prepare(8, 8)
    cursor.byteBuf.putDouble(cursor.index, value)
    cursor.index += 8
}

fun writeBool(cursor: FfiBufferCursor, value: Boolean) {
    writeByte(cursor, if (value) { 1.toByte() } else { 0.toByte() })
}

fun writeString(cursor: FfiBufferCursor, value: String) {
    // Strings are stored as 3 64-bit values
    cursor.prepare(8, 24)
    Scaffolding.writeString(cursor.byteBuf, cursor.index, value)
    cursor.index += 24
}
