// Slab and handle handling.
//
// This is a copy of `uniffi_core/src/ffi/slab.rs`.  See that module for documentation.

// Note: Handles are u64 values, but the top 16 bits are unset, so we can treat them as signed
// without issue.  The JNA integration is easier if they're signed, so let's go with that.
internal typealias UniffiKotlinHandle = Long
class UniffiSlabUseAfterFreeException(message: String) : InternalException(message) { }

{{ internal_component_vis }} class UniffiSlab<T>(slabId: Long? = null) {
    private val slabId = if (slabId == null) {
        nextSlabId()
    } else {
        ((slabId * SLAB_ID_UNIT) and SLAB_ID_MASK) or FOREIGN_BIT
    }
    private var generation = 0L
    private var next = END_OF_LIST
    private val lock = ReentrantReadWriteLock()
    private val entries = ArrayList<Entry<T>>()

    sealed class Entry<T> {
        data class Vacant<T>(val next: Int) : Entry<T>()
        data class Occupied<T>(val generation: Long, val value: T) : Entry<T>()
    }

    companion object {
        internal var idCounter: Long = 0

        private const val MAX_ENTRIES: Long = 4_000_000_000;
        private const val END_OF_LIST: Int = Int.MAX_VALUE
        private const val INDEX_MASK: Long = 0x0000_FFFF_FFFF;
        private const val SLAB_ID_MASK: Long = 0x00FF_0000_0000;
        private const val SLAB_ID_UNIT: Long = 0x0002_0000_0000;
        private const val FOREIGN_BIT: Long = 0x0001_0000_0000;
        private const val GENERATION_MASK: Long = 0xFF00_0000_0000;
        private const val GENERATION_UNIT: Long = 0x0100_0000_0000;

        fun nextSlabId(): Long {
            idCounter = (idCounter + SLAB_ID_UNIT) and SLAB_ID_MASK
            return idCounter or FOREIGN_BIT
        }
    }

    private fun lookup(handle: UniffiKotlinHandle): Int {
        val index = (handle and INDEX_MASK).toInt()
        val handleSlabId = handle and SLAB_ID_MASK
        val handleGeneration = handle and GENERATION_MASK

        if (handleSlabId != this.slabId) {
            if (handleSlabId and FOREIGN_BIT == 0L) {
                throw InternalException("Handle belongs to a Rust slab")
            } else {
                throw InternalException("Slab id mismatch")
            }
        }
        val entry = entries[index]
        when (entry) {
            is Entry.Vacant -> throw UniffiSlabUseAfterFreeException("entry vacant")
            is Entry.Occupied -> {
                if (entry.generation != handleGeneration) {
                    throw UniffiSlabUseAfterFreeException("generation mismatch")
                }
                return index
            }
        }
    }

    private fun makeHandle(generation: Long, index: Int): UniffiKotlinHandle {
        return index.toLong() or generation or slabId
    }

    private fun getVacantToUse(): Int {
        if (next == END_OF_LIST) {
            entries.add(Entry.Vacant(END_OF_LIST))
            return entries.size - 1
        } else {
            val oldNext = next
            val entry = entries[next]
            return when (entry) {
                is Entry.Occupied -> throw InternalException("next is occupied")
                is Entry.Vacant -> {
                    next = entry.next
                    oldNext
                }
            }
        }
    }

    {{ internal_component_vis }} fun insert(value: T): UniffiKotlinHandle {
        return lock.writeLock().withLock {
            val index = getVacantToUse()
            generation = (generation + GENERATION_UNIT) and GENERATION_MASK;
            entries[index] = Entry.Occupied(generation, value)
            makeHandle(generation, index)
        }
    }

    {{ internal_component_vis }} fun remove(handle: UniffiKotlinHandle): T {
        return lock.writeLock().withLock {
            val index = lookup(handle)
            val entry = entries[index]
            when (entry) {
                is Entry.Vacant -> throw InternalException("Lookup gave us a vacant entry")
                is Entry.Occupied -> {
                    entries[index] = Entry.Vacant(next)
                    next = index
                    entry.value
                }
            }
        }
    }

    {{ internal_component_vis }} fun get(handle: UniffiKotlinHandle): T {
        return lock.readLock().withLock {
            val index = lookup(handle)
            val entry = entries[index]
            when (entry) {
                is Entry.Vacant -> throw InternalException("Lookup gave us a vacant entry")
                is Entry.Occupied -> entry.value
            }
        }
    }

    {{ internal_component_vis }} fun specialValue(value: Int): UniffiKotlinHandle {
        return MAX_ENTRIES.toLong() + value.toLong()
    }
}
