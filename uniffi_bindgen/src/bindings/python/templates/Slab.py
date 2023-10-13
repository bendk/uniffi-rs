class UniffiSlab:
    MAX_ENTRIES = 1_000_000_000;
    MAX_SPECIAL_VALUES = 2**30 - MAX_ENTRIES;
    END_OF_LIST = 2**32 - 1
    TAG_MASK = 0xC0000000
    INDEX_MASK = 0x3FFFFFFF

    @dataclass
    class Vacant:
        next: int

    @dataclass
    class Occupied:
        value: object

    def __init__(self, tag=0):
        if tag >= 4:
            raise InternalError("Tag does not fit in 2 bits")
        self._tag = tag << 30
        self._next = 0
        self._entries = []
        self._lock = threading.Lock()

    def _get_index(self, handle: int) -> int:
        if handle & self.TAG_MASK == self._tag:
            return handle & self.INDEX_MASK
        else:
            raise InternalError("Invalid tag")

    def _get_handle(self, index: int) -> int:
        return index | self._tag

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
            self._entries[idx] = self.Occupied(value)
            return self._get_handle(idx)

    def remove(self, handle: int) -> object:
        idx = self._get_index(handle)
        with self._lock:
            entry = self._entries[idx]
            if not isinstance(entry, self.Occupied):
                raise InternalError("entry is Occupied, was the handle already removed?")
            self._entries[idx] = self.Vacant(self.next)
            self.next = idx
            return entry.value

    def get(self, handle: int) -> object:
        idx = self._get_index(handle)
        with self._lock:
            entry = self._entries[idx]
            if not isinstance(entry, self.Occupied):
                raise InternalError("entry is Occupied, was the handle already removed?")
            else:
                return entry.value

    def special_value(self, val: int) -> int:
        return self.MAX_ENTRIES + val
