let UNIFFI_SLAB_MAX_ENTRIES: UInt64 = 4_000_000_000
let UNIFFI_SLAB_END_OF_LIST: Int = Int.max
let UNIFFI_SLAB_INDEX_MASK: UInt64 = 0x0000_FFFF_FFFF
let UNIFFI_SLAB_SLAB_ID_MASK: UInt64 = 0x00FF_0000_0000
let UNIFFI_SLAB_SLAB_ID_UNIT: UInt64 = 0x0002_0000_0000
let UNIFFI_SLAB_FOREIGN_BIT: UInt64 = 0x0001_0000_0000
let UNIFFI_SLAB_GENERATION_MASK: UInt64 = 0xFF00_0000_0000
let UNIFFI_SLAB_GENERATION_UNIT: UInt64 = 0x0100_0000_0000

private var uniffiSlabIdCounter: UInt64 = 0

{{ internal_component_vis }} struct UniffiSlab<T> {
    enum Entry {
        case vacant(next: Int)
        case occupied(generation: UInt64, value: T)
    }

    private let slabId: UInt64
    // Should we use `pthread_rwlock_t` instead?
    private let lock = NSLock()
    private var next: Int = UNIFFI_SLAB_END_OF_LIST
    private var generation: UInt64 = 0
    private var entries: [Entry] = []

    {{ internal_component_vis }} init() {
        uniffiSlabIdCounter = (uniffiSlabIdCounter + UNIFFI_SLAB_SLAB_ID_UNIT) & UNIFFI_SLAB_SLAB_ID_MASK
        self.slabId = uniffiSlabIdCounter | UNIFFI_SLAB_FOREIGN_BIT
    }

    {{ internal_component_vis }} init(_ slabId: Int) {
        self.slabId = ((UInt64(slabId) * UNIFFI_SLAB_SLAB_ID_UNIT) & UNIFFI_SLAB_SLAB_ID_MASK) | UNIFFI_SLAB_FOREIGN_BIT
    }

    private func lookup(handle: UInt64) throws -> Int {
        let index = Int(handle & UNIFFI_SLAB_INDEX_MASK)
        let handleSlabId = handle & UNIFFI_SLAB_SLAB_ID_MASK
        let handleGeneration = handle & UNIFFI_SLAB_GENERATION_MASK

        if handleSlabId != self.slabId {
            if handleSlabId & UNIFFI_SLAB_FOREIGN_BIT == 0 {
                throw UniffiInternalError.slabError("Handle belongs to a Rust slab")
            } else {
                throw UniffiInternalError.slabError("slab ID mismatch")
            }
        }
        let entry = entries[index]
        switch entry {
        case Entry.vacant:
            throw UniffiInternalError.slabUseAfterFree("entry vacant")

        case Entry.occupied(let generation, _):
            if generation != handleGeneration {
                throw UniffiInternalError.slabUseAfterFree("generation mismatch")
            } else {
                return index
            }
        }
    }

    private func makeHandle(generation: UInt64, index: Int) -> UInt64 {
        return UInt64(index) | generation | self.slabId
    }

    private mutating func getVacantToUse() throws -> Int {
        if (next == UNIFFI_SLAB_END_OF_LIST) {
            entries.append(Entry.vacant(next: UNIFFI_SLAB_END_OF_LIST))
            return entries.count - 1
        } else {
            let oldNext = next
            switch entries[next] {
            case Entry.occupied:
                throw UniffiInternalError.slabError("next is occupied")
            case Entry.vacant(let next):
                self.next = next
                return oldNext
            }
        }
    }

    {{ internal_component_vis }} mutating func insert(_ value: T) throws -> UInt64 {
        return try lock.withLock {
            let idx = try getVacantToUse()
            let generation = self.generation
            self.generation = (self.generation + UNIFFI_SLAB_GENERATION_UNIT) & UNIFFI_SLAB_GENERATION_MASK
            entries[idx] = Entry.occupied(generation: generation, value: value)
            return makeHandle(generation: generation, index: idx)
        }
    }

    {{ internal_component_vis }} mutating func remove(_ handle: UInt64) throws -> T {
        return try lock.withLock {
            let idx = try lookup(handle: handle)
            switch entries[idx] {
            case Entry.vacant:
                throw UniffiInternalError.slabError("Entry vacant (and lookup missed it)")

            case Entry.occupied(_, let value):
                entries[idx] = Entry.vacant(next: self.next)
                self.next = idx
                return value
            }
        }
    }

    {{ internal_component_vis }} func get(_ handle: UInt64) throws -> T {
        return try lock.withLock {
            let idx = try lookup(handle: handle)
            switch entries[idx] {
            case Entry.vacant:
                throw UniffiInternalError.slabError("Entry vacant (and lookup missed it)")

            case Entry.occupied(_, let value):
                return value
            }
        }
    }

    {{ internal_component_vis }} func specialValue(_ value: Int) -> UInt64 {
        return UNIFFI_SLAB_MAX_ENTRIES + UInt64(value)
    }
}
