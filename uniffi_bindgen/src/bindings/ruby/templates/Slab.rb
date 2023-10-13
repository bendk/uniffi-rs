class UniffiSlab
  MAX_ENTRIES = 4_000_000_000
  # Ruby doesn't supply something like Int.MAX, but this is larger than any possible index
  END_OF_LIST = 2 ** 32
  INDEX_MASK = 0x0000_FFFF_FFFF
  SLAB_ID_MASK = 0x00FF_0000_0000
  SLAB_ID_UNIT = 0x0002_0000_0000
  FOREIGN_BIT = 0x0001_0000_0000
  GENERATION_MASK = 0xFF00_0000_0000
  GENERATION_UNIT = 0x0100_0000_0000

  Vacant = Data.define(:next)
  Occupied = Data.define(:generation, :value)

  @@slab_id_counter = 0

  def initialize(slab_id=nil)
    if slab_id.nil?
      @@slab_id_counter = (@@slab_id_counter + SLAB_ID_UNIT) & SLAB_ID_MASK
      @slab_id = @@slab_id_counter | FOREIGN_BIT
    else
      @slab_id = ((slab_id * SLAB_ID_UNIT) & SLAB_ID_MASK) | FOREIGN_BIT
    end
    @next = END_OF_LIST
    @generation = 0
    @entries = []
    # Ruby doesn't provide a standard RwLock, so we use a Mutex instead.
    @lock = Mutex.new
  end

  private def lookup(handle)
    index = handle & INDEX_MASK
    handle_slab_id = handle & SLAB_ID_MASK
    handle_generation = handle & GENERATION_MASK

    if handle_slab_id != @slab_id
      if handle_slab_id & FOREIGN_BIT == 0
        raise InternalError, "Handle belongs to Rust slab"
      else
        raise InternalError, "Invalid slab id"
      end
    end

    entry = @entries[index]
    if not entry.is_a? Occupied
        raise InternalError, "use-after-free (entry vacant)"
    end
    if entry.generation != handle_generation
        raise InternalError, "use-after-free (generation mismatch)"
    end
    return index
  end

  private def make_handle(generation, index)
    return index | generation | @slab_id
  end

  private def get_vacant_to_use()
    if @next == END_OF_LIST
      @entries.append(Vacant.new(END_OF_LIST))
      return @entries.length - 1
    else
      old_next = @next
      next_entry = @entries[old_next]
      if not isinstance(next_entry, Vacant)
        raise InternalError("next is not Vacant")
      end
      @next = next_entry.next
      return old_next
    end
  end

    def insert(value)
      @lock.synchronize do
        idx = get_vacant_to_use()
        @generation = (@generation + GENERATION_UNIT) & GENERATION_MASK
        @entries[idx] = Occupied.new(@generation, value)
        return make_handle(@generation, idx)
      end
    end

    def remove(handle)
      @lock.synchronize do
        idx = lookup(handle)
        entry = @entries[idx]
        @entries[idx] = Vacant.new(@next)
        @next = idx
        return entry.value
      end
    end

    def get(handle)
      @lock.synchronize do
        idx = lookup(handle)
        return @entries[idx].value
      end
    end

    def special_value(val)
      return MAX_ENTRIES + val
    end
end
