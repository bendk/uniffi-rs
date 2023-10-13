sealed class UniffiSlabEntry<T> {
    data class Vacant<T>(val next: Int) : UniffiSlabEntry<T>()
    data class Occupied<T>(val value: T) : UniffiSlabEntry<T>()
}

private const val UNIFFI_SLAB_MAX_ENTRIES: Int = 1_000_000_000
private const val UNIFFI_SLAB_END_OF_LIST: Int = Int.MAX_VALUE
private const val UNIFFI_SLAB_TAG_MASK: UInt = 0xC0000000u
private const val UNIFFI_SLAB_INDEX_MASK: UInt = 0x3FFFFFFFu

class UniffiSlab<T>(tag: Int = 0) {
    init {
        if (tag >= 4) {
            throw InternalException("Tag does not fit in 2 bits")
        }
    }

    private val tag = tag.toUInt() shl 30
    private var next = 0
    private val lock = ReentrantLock()
    private val entries = ArrayList<UniffiSlabEntry<T>>()
    private var capacity = 0

    private fun getIndex(handle: UInt): Int {
        if (handle and UNIFFI_SLAB_TAG_MASK == tag) {
            return (handle and UNIFFI_SLAB_INDEX_MASK).toInt()
        } else {
            throw InternalException("Invalid tag")
        }
    }

    private fun getHandle(index: Int): UInt {
        return index.toUInt() or tag
    }

    private fun getVacantToUse(): Int {
        if (next == UNIFFI_SLAB_END_OF_LIST) {
            entries.add(UniffiSlabEntry.Vacant(UNIFFI_SLAB_END_OF_LIST))
            return entries.size - 1
        } else {
            val oldNext = next
            val entry = entries[next]
            return when (entry) {
                is UniffiSlabEntry.Occupied -> throw InternalException("next is occupied")
                is UniffiSlabEntry.Vacant -> {
                    next = entry.next
                    oldNext
                }
            }
        }
    }

    public fun insert(value: T): UInt {
        return lock.withLock {
            val idx = getVacantToUse()
            entries[idx] = UniffiSlabEntry.Occupied(value)
            getHandle(idx)
        }
    }

    public fun remove(handle: UInt): T {
        val idx = getIndex(handle)
        return lock.withLock {
            val entry = entries[idx]
            when (entry) {
                is UniffiSlabEntry.Vacant -> {
                    throw InternalException("Entry vacant (was the handle already removed?)")
                }
                is UniffiSlabEntry.Occupied -> {
                    entries[idx] = UniffiSlabEntry.Vacant(next)
                    next = idx
                    entry.value
                }
            }
        }
    }

    public fun get(handle: UInt): T {
        val idx = getIndex(handle)
        return lock.withLock {
            val entry = entries[idx]
            when (entry) {
                is UniffiSlabEntry.Vacant -> {
                    throw InternalException("Entry vacant (was the handle already removed?)")
                }
                is UniffiSlabEntry.Occupied -> entry.value
            }
        }
    }

    public fun specialValue(value: Int): UInt {
        return UNIFFI_SLAB_MAX_ENTRIES.toUInt() + value.toUInt()
    }
}
