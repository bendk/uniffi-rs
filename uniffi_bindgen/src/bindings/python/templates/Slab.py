class UniffiSlab:
    MAX_ENTRIES = 4_000_000_000
    END_OF_LIST = sys.maxsize
    INDEX_MASK = 0x0000_FFFF_FFFF
    SLAB_ID_MASK = 0x00FF_0000_0000
    SLAB_ID_UNIT = 0x0002_0000_0000
    FOREIGN_BIT = 0x0001_0000_0000
    GENERATION_MASK = 0xFF00_0000_0000
    GENERATION_UNIT = 0x0100_0000_0000

    class UseAfterFree(InternalError):
        pass

    @dataclass
    class Vacant:
        next: int

    @dataclass
    class Occupied:
        generation: int
        value: object

    _slab_id_counter = 0

    def __init__(self, slab_id=None):
        if slab_id is None:
            self._slab_id_counter = (self._slab_id_counter + self.SLAB_ID_UNIT) & self.SLAB_ID_MASK
            self._slab_id = self._slab_id_counter | self.FOREIGN_BIT
        else:
            self._slab_id = (slab_id * self.SLAB_ID_UNIT) & self.SLAB_ID_MASK | self.FOREIGN_BIT
        self._next = self.END_OF_LIST
        self._generation = 0
        self._entries = []
        # Python doesn't support a Rwlock, so we just use a regular lock instead.
        self._lock = threading.Lock()

    def _lookup(self, handle: int) -> int:
        index = handle & self.INDEX_MASK
        handle_slab_id = handle & self.SLAB_ID_MASK
        handle_generation = handle & self.GENERATION_MASK

        if handle_slab_id != self._slab_id:
            if (handle_slab_id & self.FOREIGN_BIT) == 0:
                raise InternalError("Handle belongs to a Rust Slab")
            else:
                raise InternalError("Invalid slab id")

        entry = self._entries[index]
        if not isinstance(entry, self.Occupied):
            raise self.UseAfterFree("entry vacant")
        if entry.generation != handle_generation:
            raise self.UseAfterFree("generation mismatch")
        return index

    def _make_handle(self, generation: int, index: int) -> int:
        return index | generation | self._slab_id

    def _get_vacant_to_use(self) -> int:
        if self._next == self.END_OF_LIST:
            self._entries.append(self.Vacant(self.END_OF_LIST))
            return len(self._entries) - 1
        else:
            old_next = self._next
            next_entry = self._entries[old_next]
            if not isinstance(next_entry, self.Vacant):
                raise InternalError("self._next is not Vacant")
            self._next = next_entry.next
            return old_next

    def insert(self, value: object) -> int:
        with self._lock:
            idx = self._get_vacant_to_use()
            self._generation = (self._generation + self.GENERATION_UNIT) & self.GENERATION_MASK
            self._entries[idx] = self.Occupied(self._generation, value)
            return self._make_handle(self._generation, idx)

    def remove(self, handle: int) -> object:
        with self._lock:
            idx = self._lookup(handle)
            entry = self._entries[idx]
            self._entries[idx] = self.Vacant(self._next)
            self._next = idx
            return entry.value

    def get(self, handle: int) -> object:
        with self._lock:
            idx = self._lookup(handle)
            return self._entries[idx].value

    def special_value(self, val: int) -> int:
        return self.MAX_ENTRIES + val
