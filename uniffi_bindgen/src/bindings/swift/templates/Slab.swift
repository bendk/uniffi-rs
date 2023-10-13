let UNIFFI_SLAB_TAG_MASK: UInt32 = 0xC0000000
let UNIFFI_SLAB_INDEX_MASK: UInt32 = 0x3FFFFFFF
let UNIFFI_SLAB_MAX_ENTRIES: Int = 1_000_000_000
let UNIFFI_SLAB_END_OF_LIST: Int = Int.max

/// Store values and generate handles that can be used by the foreign code
struct UniffiSlab<T> {
    enum Entry {
        case vacant(next: Int)
        case occupied(value: T)
    }

    private let tag: UInt32
    private let lock = NSLock()
    private var next: Int = 0
    private var entries: [Entry] = []

    init() {
        tag = 0;
    }

    init(tag: UInt32) {
        if (tag >= 4) {
            fatalError("Tag can't fit in 2 bits")
        }
        self.tag = tag << 30;
    }

    private func getIndex(handle: UInt32) -> Int {
        if (handle & UNIFFI_SLAB_TAG_MASK == tag) {
            return Int(handle & UNIFFI_SLAB_INDEX_MASK)
        } else {
            fatalError("Invalid tag")
        }
    }

    private func getHandle(index: Int) -> UInt32 {
        return UInt32(index) | tag
    }

    private mutating func getVacantToUse() -> Int {
        if (next == UNIFFI_SLAB_END_OF_LIST) {
            entries.append(Entry.vacant(next: UNIFFI_SLAB_END_OF_LIST))
            return entries.count - 1
        } else {
            let oldNext = next
            switch entries[next] {
                case Entry.occupied: fatalError("next is occupied")
                case Entry.vacant(let next):
                    self.next = next
                    return oldNext
            }
        }
    }

    public mutating func insert(value: T) -> UInt32 {
        return lock.withLock {
            let idx = getVacantToUse()
            entries[idx] = Entry.occupied(value: value)
            return getHandle(index: idx)
        }
    }

    public mutating func remove(handle: UInt32) -> T {
        let idx = getIndex(handle: handle)
        return lock.withLock {
            switch entries[idx] {
                case Entry.vacant:
                    fatalError("Entry vacant (was the handle already removed?)")

                case Entry.occupied(let value):
                    entries[idx] = Entry.vacant(next: self.next)
                    self.next = idx
                    return value
            }
        }
    }

    public func get(handle: UInt32) -> T {
        let idx = getIndex(handle: handle)
        return lock.withLock {
            switch entries[idx] {
                case Entry.vacant:
                    fatalError("Entry vacant (was the handle already removed?)")

                case Entry.occupied(let value):
                    return value
            }
        }
    }

    public func specialValue(value: Int) -> UInt32 {
        return UInt32(UNIFFI_SLAB_MAX_ENTRIES + value)
    }
}
