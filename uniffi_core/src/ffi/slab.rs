/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Store Arc references owned by the foreign side and use handles to manage them
//!
//! This module defines the [Slab] class allows us to insert `Arc<>` values and use [Handle] values to manage the allocation.
//! It's named "Slab" because it's designed like a slab-allocator, for example the `tokio` `slab` crate (https://github.com/tokio-rs/slab).
//!
//! Usage:
//! * Create a `Slab` that will store Arc<T> values.
//! * Call `insert()` to store a value and allocated a handle that represents a single strong ref.
//! * Pass the handle across the FFI to the foreign side.
//! * When the foreign side wants to use that value, it passes back the handle.
//!   Use `get()` to get a reference to the stored value
//! * When the foreign side is finished with the value, it passes the handle to a free scaffolding function.
//!   That function calls `remove` to free the allocation.
//!
//! Using handles to manage arc references provides several benefits:
//! * Handles are simple `u64` values, which are simpler to work with than pointers.
//! * The implementation is 100% safe code.
//! * Handles store a generation counter, which can usually detect use-after-free bugs.
//! * Handles store an slab id, which can usually detect using handles with the wrong Slab.
//! * Handles only use 48 bits, which makes them easier to work with on languages like JS that don't support full 64-bit integers.
//!   Also languages like Kotlin, which prefer signed values, can treat them as signed without issues.
//! * Handles have a bit to differentiate between foreign-allocated handles and rust-allocated ones.
//!   The trait interface code uses this to differentiate between Rust-implemented and foreign-implemented traits.

use std::{fmt, sync::RwLock};

use const_random::const_random;

// This code assumes that usize is at least as wide as u32
static_assertions::const_assert!(std::mem::size_of::<usize>() >= std::mem::size_of::<u32>());

/// Slab error type
///
/// This is extremely simple, since we almost always use the `*_or_panic` methods in scaffolding
/// code.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    SlabIdMismatch,
    ForeignHandle,
    UseAfterFree(&'static str),
    OverCapacity,
    Other(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UseAfterFree(msg) => write!(f, "Use-after-free detected ({msg})"),
            Self::SlabIdMismatch => write!(f, "Slab id mismatch"),
            Self::ForeignHandle => write!(f, "Handle belongs to a foreign slab"),
            Self::OverCapacity => write!(f, "Slab capacity exceeded"),
            Self::Other(msg) => write!(f, "Slab internal error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// Handle for a value stored in the slab
///
/// * The first 32 bits identify the value.
///   - The first 4,000,000,000 values are used for indexes in the `entries` table.
///   - The next values are reserved for special cases (see `rust_future.rs` for an example).
/// * The next 8 bits are for an slab id:
///   - The first bit is always 0, which indicates the handle came from Rust.
///   - The next 7 bits are initialized to a random value.
///   - this means that using a handle with the wrong Slab will be detected > 99% of the time.
/// * The next 8 bits are a generation counter value, this means that use-after-free bugs will be
///   detected until at least 256 inserts are performed after the free.
/// * The last 16 bits are intentionally unset, so that these can be easily used on languages like
///   JS that don't support full 64-bit integers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Handle(u64);

impl Handle {
    pub const fn from_raw(val: u64) -> Self {
        Self(val)
    }

    pub const fn as_raw(&self) -> u64 {
        self.0
    }

    pub fn is_from_rust(&self) -> bool {
        (self.0 & FOREIGN_BIT) == 0
    }

    pub fn is_foreign(&self) -> bool {
        (self.0 & FOREIGN_BIT) != 0
    }
}

// 4 billion entries seems like plenty.  Starting special values at 4,000,000,000, makes them
// easier to recognized when printed out.
const MAX_ENTRIES: u64 = 4_000_000_000;
const END_OF_LIST: usize = usize::MAX;
// Bit masks to isolate one part of the handle.
// Unit values are used to increment one segment of the handle.
// In general, we avoid bit shifts by updating the u64 value directly.
const INDEX_MASK: u64 = 0x0000_FFFF_FFFF;
const SLAB_ID_MASK: u64 = 0x00FF_0000_0000;
const SLAB_ID_UNIT: u64 = 0x0002_0000_0000;
const FOREIGN_BIT: u64 = 0x0001_0000_0000;
const GENERATION_MASK: u64 = 0xFF00_0000_0000;
const GENERATION_UNIT: u64 = 0x0100_0000_0000;

/// Entry in the slab table.
///
/// Note: We only use the `T: Clone` trait bound, but currently this is always used with Arc<T>.
#[derive(Debug)]
enum Entry<T: Clone> {
    // Vacant entry. These form a kind of linked-list in the EntryList.  Each inner value is the
    // index of the next vacant entry.
    Vacant { next: usize },
    Occupied { generation: u64, value: T },
}

/// Allocates handles that represent stored values and can be shared by the foreign code
pub struct Slab<T: Clone> {
    slab_id: u64,
    inner: RwLock<SlabInner<T>>,
}

#[derive(Debug)]
struct SlabInner<T: Clone> {
    generation: u64,
    next: usize,
    entries: Vec<Entry<T>>,
}

impl<T: Clone> Slab<T> {
    pub const fn new() -> Self {
        Self::new_with_id(const_random!(u8))
    }

    pub const fn new_with_id(slab_id: u8) -> Self {
        Self {
            slab_id: (slab_id as u64 * SLAB_ID_UNIT) & SLAB_ID_MASK,
            inner: RwLock::new(SlabInner {
                generation: 0,
                next: END_OF_LIST,
                entries: vec![],
            }),
        }
    }

    fn with_read_lock<F, R>(&self, operation: F) -> R
    where
        F: FnOnce(&SlabInner<T>) -> R,
    {
        let guard = self.inner.read().unwrap();
        operation(&*guard)
    }

    fn with_write_lock<F, R>(&self, operation: F) -> R
    where
        F: FnOnce(&mut SlabInner<T>) -> R,
    {
        let mut guard = self.inner.write().unwrap();
        operation(&mut *guard)
    }

    // Lookup a handle
    //
    // This inputs a handle, validates it's tag, generation counter, etc.
    // Returns an index that points to an allocated entry.
    fn lookup(&self, inner: &SlabInner<T>, handle: Handle) -> Result<usize> {
        if handle.is_foreign() {
            return Err(Error::ForeignHandle);
        }
        let raw = handle.as_raw();
        let index = (raw & INDEX_MASK) as usize;
        let handle_slab_id = raw & SLAB_ID_MASK;
        let handle_generation = raw & GENERATION_MASK;

        if handle_slab_id != self.slab_id {
            return Err(Error::SlabIdMismatch);
        }
        let entry = &inner.entries[index];
        match entry {
            Entry::Vacant { .. } => Err(Error::UseAfterFree("vacant")),
            Entry::Occupied { generation, .. } => {
                if *generation != handle_generation {
                    Err(Error::UseAfterFree("generation mismatch"))
                } else {
                    Ok(index)
                }
            }
        }
    }

    // Create a handle from an index
    fn make_handle(&self, generation: u64, index: usize) -> Handle {
        Handle(index as u64 | generation | self.slab_id)
    }

    /// Find a vacant entry to insert a new item in
    ///
    /// This removes it from the vacancy list and returns the index
    fn get_vacant_to_use<'a>(&self, inner: &'a mut SlabInner<T>) -> Result<usize> {
        match inner.next {
            END_OF_LIST => {
                // No vacant entries, push a new entry and use that
                inner.entries.push(Entry::Vacant { next: END_OF_LIST });
                Ok(inner.entries.len() - 1)
            }
            old_next => {
                // There's at least one vacant entry, pop it off the list
                match inner.entries[old_next as usize] {
                    Entry::Occupied { .. } => return Err(Error::Other("self.next is occupied")),
                    Entry::Vacant { next } => {
                        inner.next = next;
                    }
                };
                Ok(old_next as usize)
            }
        }
    }

    /// Insert a new item into the Slab and get a Handle to access it with
    pub fn insert(&self, value: T) -> Result<Handle> {
        self.with_write_lock(move |inner| {
            let idx = self.get_vacant_to_use(inner)?;
            let generation = inner.generation;
            inner.generation = (inner.generation + GENERATION_UNIT) & GENERATION_MASK;
            inner.entries[idx] = Entry::Occupied { value, generation };
            Ok(self.make_handle(generation, idx))
        })
    }

    /// Get a cloned value from a handle
    pub fn get_clone(&self, handle: Handle) -> Result<T> {
        self.with_read_lock(|inner| {
            let idx = self.lookup(inner, handle)?;
            match &inner.entries[idx] {
                Entry::Occupied { value, .. } => Ok(value.clone()),
                // lookup ensures the entry is occupied
                Entry::Vacant { .. } => unreachable!(),
            }
        })
    }

    // Remove an item from the slab, returning the original value
    pub fn remove(&self, handle: Handle) -> Result<T> {
        self.with_write_lock(move |inner| {
            let idx = self.lookup(inner, handle)?;
            // Push the entry to the head of the vacant list
            let old_value =
                std::mem::replace(&mut inner.entries[idx], Entry::Vacant { next: inner.next });
            inner.next = idx;
            match old_value {
                Entry::Occupied { value, .. } => Ok(value),
                // lookup ensures the entry is occupied
                Entry::Vacant { .. } => unreachable!(),
            }
        })
    }

    pub fn insert_or_panic(&self, value: T) -> Handle {
        self.insert(value).unwrap_or_else(|e| panic!("{e}"))
    }

    pub fn get_clone_or_panic(&self, handle: Handle) -> T {
        self.get_clone(handle).unwrap_or_else(|e| panic!("{e}"))
    }

    pub fn remove_or_panic(&self, handle: Handle) -> T {
        self.remove(handle).unwrap_or_else(|e| panic!("{e}"))
    }

    // Create a special-cased handle
    //
    // This is guaranteed to not equal any real handles allocated by the slab.
    pub const fn special_value(&self, val: u32) -> Handle {
        let raw = MAX_ENTRIES + val as u64;
        if (raw & !INDEX_MASK) != 0 {
            panic!("special value too large");
        }
        Handle::from_raw(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn check_slab_size<T: Clone>(slab: &Slab<T>, occupied_count: usize, vec_size: usize) {
        let inner = slab.inner.read().unwrap();
        assert_eq!(
            inner
                .entries
                .iter()
                .filter(|e| matches!(e, Entry::Occupied { .. }))
                .count(),
            occupied_count
        );
        assert_eq!(inner.entries.len(), vec_size);
    }

    #[test]
    fn test_simple_usage() {
        let slab = Slab::new();
        check_slab_size(&slab, 0, 0);
        let handle1 = slab.insert(Arc::new("Hello")).unwrap();
        check_slab_size(&slab, 1, 1);
        let handle2 = slab.insert(Arc::new("Goodbye")).unwrap();
        check_slab_size(&slab, 2, 2);
        assert_eq!(*slab.get_clone(handle1).unwrap(), "Hello");
        slab.remove(handle1).unwrap();
        check_slab_size(&slab, 1, 2);
        assert_eq!(*slab.get_clone(handle2).unwrap(), "Goodbye");
        slab.remove(handle2).unwrap();
        check_slab_size(&slab, 0, 2);
    }

    #[test]
    fn test_special_values() {
        let slab = Slab::<String>::new();
        assert_eq!(slab.special_value(0), Handle(4_000_000_000));
        assert_eq!(slab.special_value(1), Handle(4_000_000_001));
    }

    #[test]
    fn test_slab_id_check() {
        let slab = Slab::<Arc<&str>>::new_with_id(1);
        let slab2 = Slab::<Arc<&str>>::new_with_id(2);
        let handle = slab.insert(Arc::new("Hello")).unwrap();
        assert_eq!(Err(Error::SlabIdMismatch), slab2.get_clone(handle));
        assert_eq!(Err(Error::SlabIdMismatch), slab2.remove(handle));
    }

    #[test]
    fn test_foreign_handle() {
        let slab = Slab::<Arc<&str>>::new_with_id(1);
        let handle = slab.insert(Arc::new("Hello")).unwrap();
        let foreign_handle = Handle::from_raw(handle.as_raw() | FOREIGN_BIT);
        assert_eq!(Err(Error::ForeignHandle), slab.get_clone(foreign_handle));
    }
}

#[cfg(test)]
mod stress_tests {
    use super::*;
    use anyhow::{bail, Result};
    use rand::{rngs::StdRng, RngCore, SeedableRng};
    use std::sync::Arc;

    // Single operation performed by the stress test.  This exists so we can log operations and
    // print them out when the tests fail.
    #[derive(Debug)]
    enum Operation {
        Insert { value: usize },
        Remove { index: usize },
    }

    struct SlabStressTester {
        rng: StdRng,
        slab: Slab<Arc<usize>>,
        next_value: usize,
        allocated: Vec<(Handle, usize)>,
        freed: Vec<Handle>,
        operations: Vec<Operation>,
    }

    impl SlabStressTester {
        fn new(seed: u64) -> Self {
            Self {
                rng: StdRng::seed_from_u64(seed),
                slab: Slab::new(),
                next_value: 0,
                allocated: vec![],
                freed: vec![],
                operations: vec![],
            }
        }

        fn perform_operation(&mut self) -> Result<()> {
            if self.allocated.is_empty() || self.rng.next_u32() % 2 == 0 {
                self.perform_insert()
            } else {
                self.perform_remove()
            }
        }

        fn perform_insert(&mut self) -> Result<()> {
            self.operations.push(Operation::Insert {
                value: self.next_value,
            });
            let handle = self.slab.insert(Arc::new(self.next_value))?;
            self.allocated.push((handle, self.next_value));
            self.next_value += 1;
            Ok(())
        }

        fn perform_remove(&mut self) -> Result<()> {
            let index = (self.rng.next_u32() as usize) % self.allocated.len();
            self.operations.push(Operation::Remove { index });

            let (handle, expected) = self.allocated.remove(index);
            let removed_value = self.slab.remove(handle)?;
            if *removed_value != expected {
                bail!("Removed value doesn't match: {removed_value} (expected: {expected})")
            }
            self.freed.push(handle);
            Ok(())
        }

        fn check(&self) -> Result<()> {
            // Test getting all handles, allocated or freed
            for (handle, expected) in self.allocated.iter() {
                let value = self.slab.get_clone(*handle)?;
                if *value != *expected {
                    bail!("Value from get_clone doesn't match: {value} (expected: {expected})")
                }
            }
            for handle in self.freed.iter() {
                let result = self.slab.get_clone(*handle);
                if !matches!(result, Err(Error::UseAfterFree(_))) {
                    bail!(
                        "Get clone on freed handle didn't return Error::UseAfterFree ({result:?})"
                    )
                }
            }
            Ok(())
        }
    }

    #[test]
    fn stress_test() {
        for _ in 0..100 {
            let mut tester = SlabStressTester::new(0);
            let mut one_loop = || {
                // Note; the inner loop is 255 elements, because that's the limit of insertions before
                // our use-after-free detection can fail.
                for _ in 0..255 {
                    tester.perform_operation()?;
                    tester.check()?;
                }
                Ok(())
            };
            one_loop().unwrap_or_else(|e: anyhow::Error| {
                panic!(
                    "{e}\nOperations:\n{}",
                    tester
                        .operations
                        .into_iter()
                        .map(|operation| format!("{operation:?}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            });
        }
    }
}
