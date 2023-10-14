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

use std::{
    cell::UnsafeCell,
    fmt,
    sync::atomic::{AtomicU64, Ordering},
};

use append_only_vec::AppendOnlyVec;
use const_random::const_random;

// This code assumes that usize is 32 or 64 bits
static_assertions::const_assert!(std::mem::size_of::<usize>() >= std::mem::size_of::<u32>());
static_assertions::const_assert!(std::mem::size_of::<usize>() <= std::mem::size_of::<u64>());

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
    Vacant,
    Occupied,
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UseAfterFree(msg) => write!(f, "Use-after-free detected ({msg})"),
            Self::SlabIdMismatch => write!(f, "Slab id mismatch"),
            Self::ForeignHandle => write!(f, "Handle belongs to a foreign slab"),
            Self::OverCapacity => write!(f, "Slab capacity exceeded"),
            Self::Vacant => write!(f, "Entry unexpectedly vacant"),
            Self::Occupied => write!(f, "Entry unexpectedly occupied"),
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Handle(u64);

impl Handle {
    // Create a new handle
    //
    // `metadata` contains bits for the slab id and generation counter.  Bits 0..32 and 48..64 should
    // always be unset.
    // `index` contains the index of the entry, which must fit in 32 bits.
    const fn new(metadata: u64, index: usize) -> Self {
        Self(metadata | index as u64)
    }

    pub const fn from_raw(val: u64) -> Self {
        Self(val)
    }

    pub const fn as_raw(&self) -> u64 {
        self.0
    }

    fn index(&self) -> usize {
        (self.0 & INDEX_MASK) as usize
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
const END_OF_LIST: u64 = u32::MAX as u64;
// Bit masks to isolate one part of the handle.
// Unit values are used to increment one segment of the handle.
// In general, we avoid bit shifts by updating the u64 value directly.
const INDEX_MASK: u64 = 0x0000_FFFF_FFFF;
const HANDLE_MASK: u64 = 0xFFFF_FFFF_FFFF;
const HANDLE_METADATA_MASK: u64 = 0xFFFF_0000_0000;
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
    Vacant { next: u64 },
    Occupied { value: T },
}

/// Slab entry protected with some safeguards.
///
/// Each entry is protected by a simple RwLock that should never be contended.  If one thread is
/// trying to read the entry and another is trying to write it, this indicates some sort of
/// use-after-free bug (for example calling `get()` and `remove()` at the same time)
///
/// `LockedEntry` also stores the generation counter and the slab id for occupied entries and checks
/// those while it's checking the lock.
struct LockedEntry<T: Clone> {
    // The upper 32 bits matches the upper 32 bits of the handle for this entry (the generation
    // counter and slab id).
    //
    // The lower 31 bits are used for the read counter.
    // The 32nd bit is used for the write lock.
    //
    // Note: This is before `state` in the hope that this will fall in the highest
    // 32 bits of an aligned 64-bit value and Rust/LLVM can optimize some things based on that.
    state: AtomicU64,
    entry: UnsafeCell<Entry<T>>,
}

impl<T: Clone> LockedEntry<T> {
    const WRITE_LOCK_BIT: u64 = 0x8000_0000;

    /// Make a new, occupied entry
    fn new(slab_id: u64, value: T) -> Self {
        Self {
            state: AtomicU64::new(slab_id),
            entry: UnsafeCell::new(Entry::Occupied { value }),
        }
    }

    // Call a function that reads from the current entry with the read lock
    fn read<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(u64, &Entry<T>) -> Result<R>,
    {
        // Increment the read count and check that there are no active writers
        let old_state = self.state.fetch_add(1, Ordering::Acquire);
        if (old_state & Self::WRITE_LOCK_BIT) != 0 {
            self.state.fetch_sub(1, Ordering::Relaxed);
            return Err(Error::UseAfterFree(
                "entry write locked while trying to read",
            ));
        }

        // Safety:
        //
        // We have incremented the read counter while the write lock bit was unset so we know there
        // are no current writers. If write() is called before we're done, it will see the reader
        // count as nonzero and return an error rather than allowing the update to continue.
        let entry = unsafe { &*self.entry.get() };
        let result = f(old_state, entry);
        self.state.fetch_sub(1, Ordering::Release);
        result
    }

    // Call a function that updates the current entry with the write lock
    fn write<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(u64, &mut Entry<T>) -> Result<R>,
    {
        // Set the write lock bit and check that there are no active readers or writers
        let old_state = self.state.fetch_or(Self::WRITE_LOCK_BIT, Ordering::Acquire);
        if (old_state & INDEX_MASK) != 0 {
            if (old_state & Self::WRITE_LOCK_BIT) != 0 {
                return Err(Error::UseAfterFree(
                    "entry write locked while trying to write",
                ));
            } else {
                // We were the thread that set the write lock bit, so we need to unset it
                self.state
                    .fetch_and(!Self::WRITE_LOCK_BIT, Ordering::Relaxed);
                return Err(Error::UseAfterFree(
                    "entry read locked while trying to write",
                ));
            }
        }

        // Safety: We have set the write lock bit while all other bits were unset so we know there
        // are no current readers or writers. If read() or write() is called before we're done, it
        // will see the writer bit set and return an error rather than allowing the operation to
        // continue.
        let entry = unsafe { &mut *self.entry.get() };
        let result = f(old_state, entry);
        // Unset the write lock bit
        self.state
            .fetch_and(!Self::WRITE_LOCK_BIT, Ordering::Release);
        result
    }

    fn check_handle_against_state(&self, handle: Handle, state: u64) -> Result<()> {
        let raw_handle = handle.as_raw();
        if raw_handle & HANDLE_METADATA_MASK != state & HANDLE_METADATA_MASK {
            if raw_handle & FOREIGN_BIT != 0 {
                Err(Error::ForeignHandle)
            } else if raw_handle & SLAB_ID_MASK != state & SLAB_ID_MASK {
                Err(Error::SlabIdMismatch)
            } else if raw_handle & GENERATION_MASK != state & GENERATION_MASK {
                Err(Error::UseAfterFree("generation mismatch"))
            } else {
                Err(Error::Other(format!(
                    "handle/state mismatch, but we failed to detect it: {state}, {raw_handle}"
                )))
            }
        } else {
            Ok(())
        }
    }

    /// Get the next value from a vacant entry
    fn get_next(&self) -> Result<u64> {
        self.read(|_, entry| match entry {
            Entry::Vacant { next } => Ok(*next),
            Entry::Occupied { .. } => Err(Error::Vacant),
        })
    }

    /// Get the next value from a vacant entry
    fn set_next(&self, new_next: u64) -> Result<()> {
        self.write(|_, entry| match entry {
            Entry::Vacant { next } => {
                *next = new_next;
                Ok(())
            }
            Entry::Occupied { .. } => Err(Error::Vacant),
        })
    }

    /// Get a cloned value from an occupied entry.
    fn get_clone(&self, handle: Handle) -> Result<T> {
        self.read(|state, entry| {
            self.check_handle_against_state(handle, state)?;
            match entry {
                Entry::Vacant { .. } => Err(Error::UseAfterFree("vacant entry")),
                Entry::Occupied { value } => Ok(value.clone()),
            }
        })
    }

    /// Insert a value into a vacant entry, transitioning it to occupied.
    ///
    /// Returns the handle metadata bits (the slab ID, and generation counter)
    fn insert(&self, value: T) -> Result<u64> {
        self.write(|_, entry| {
            if let Entry::Occupied { .. } = entry {
                return Err(Error::Occupied);
            }
            *entry = Entry::Occupied { value };
            // increment the generation counter portion of `self.state`
            self.state.fetch_add(GENERATION_UNIT, Ordering::Relaxed);
            self.state.fetch_and(HANDLE_MASK, Ordering::Relaxed);
            Ok(self.state.load(Ordering::Relaxed) & HANDLE_METADATA_MASK)
        })
    }

    /// Take the value of an entry, transitioning it from occupied to vacant
    ///
    /// Sets the `next` value to `END_OF_LIST`.  Use `set_next` to update this.
    fn take(&self, handle: Handle) -> Result<T> {
        self.write(|state, entry| {
            self.check_handle_against_state(handle, state)?;
            if let Entry::Vacant { .. } = entry {
                return Err(Error::UseAfterFree("vacant entry"));
            }
            let value = match std::mem::replace(entry, Entry::Vacant { next: END_OF_LIST }) {
                Entry::Occupied { value } => value,
                Entry::Vacant { .. } => unreachable!(),
            };
            Ok(value)
        })
    }
}

// Index of the next vacant entry in the free list for a Slab.
//
// The bottom 32 bits are the index of the entry.
// The top 32 bits are a generation counter that we use when atomically updating this to avoid
// the ABA problem.
pub struct Next(AtomicU64);

impl Next {
    const INDEX_MASK: u64 = 0x0000_FFFF_FFFF;
    const GENERATION_MASK: u64 = 0xFFFF_FFFF_0000_0000;
    const GENERATION_UNIT: u64 = 0x0000_0001_0000_0000;

    const fn new_eol() -> Self {
        Self(AtomicU64::new(END_OF_LIST as u64))
    }

    // Get the current index and a way to modify it
    //
    // Returns the current index and a token to pass to `try_update()` to update the index
    fn get(&self) -> (usize, u64) {
        let value = self.0.load(Ordering::Relaxed);
        ((value & Self::INDEX_MASK) as usize, value)
    }

    // Try to update the index
    //
    // This will only work if the value has not changed since the call to `get()`
    // Returns true if the update was successful
    fn try_update(&self, index: usize, token_from_get: u64) -> bool {
        #[cfg(test)]
        cas_loop_testing::wait();

        let new_value =
            ((token_from_get & Self::GENERATION_MASK) + Self::GENERATION_UNIT) | index as u64;
        self.0
            .compare_exchange_weak(
                token_from_get,
                new_value,
                Ordering::Relaxed,
                Ordering::Relaxed,
            )
            .is_ok()
    }
}

/// Allocates handles that represent stored values and can be shared by the foreign code
pub struct Slab<T: Clone> {
    slab_id: u64,
    // Use an append-only vec, which has the nice property that we can push to it with a shared
    // reference
    entries: AppendOnlyVec<LockedEntry<T>>,
    // Next entry in the free list.
    //
    // The bottom 32 bits are the index of the entry.
    // The top 32 bits are a generation counter that we use when atomically updating this to avoid
    // the ABA problem.
    next: Next,
}

impl<T: Clone> Slab<T> {
    pub const fn new() -> Self {
        Self::new_with_id(const_random!(u8))
    }

    pub const fn new_with_id(slab_id: u8) -> Self {
        Self {
            slab_id: (slab_id as u64 * SLAB_ID_UNIT) & SLAB_ID_MASK,
            entries: AppendOnlyVec::new(),
            next: Next::new_eol(),
        }
    }

    fn get_entry(&self, handle: Handle) -> Result<&LockedEntry<T>> {
        let index = handle.index();
        if index < self.entries.len() {
            Ok(&self.entries[index])
        } else if handle.as_raw() & SLAB_ID_MASK != self.slab_id {
            Err(Error::SlabIdMismatch)
        } else {
            Err(Error::Other("Index out of bounds".to_owned()))
        }
    }

    /// Insert a new item into the Slab
    pub fn insert(&self, value: T) -> Result<Handle> {
        // CAS loop to ensure the operation is atomic
        loop {
            let (index, token) = self.next.get();
            if index == END_OF_LIST as usize {
                // Next is pointing at END_OF_LIST, push a new entry
                let index = self.entries.push(LockedEntry::new(self.slab_id, value));
                // Note we can use `slab_id` as the handle metadata since we know that the
                // generation counter is 0.
                return Ok(Handle::new(self.slab_id, index));
            } else {
                // Next is pointing at some index, try to atomically update `next` to point to
                // the second item in the free list.  If that fails, then continue with the CAS
                // loop.
                let next2 = self.entries[index].get_next()?;
                if self.next.try_update(next2 as usize, token) {
                    let metadata = self.entries[index].insert(value)?;
                    return Ok(Handle::new(metadata, index));
                }
            }
        }
    }

    /// Get a cloned value from a handle
    pub fn get_clone(&self, handle: Handle) -> Result<T> {
        self.get_entry(handle)?.get_clone(handle)
    }

    // Remove an item from the slab, returning the original value
    pub fn remove(&self, handle: Handle) -> Result<T> {
        // Take the occupied value, leaving a vacant entry that we will fill out.
        //
        // Note: If the `set_next()` call below fails, we'll be left with a vacant entry that's not
        // part of the free list, and therefore will never be re-used.  That's not great, but not
        // disastrous either.
        let index = handle.index();
        let value = self.get_entry(handle)?.take(handle)?;
        // CAS loop to ensure the operation is atomic
        loop {
            let (current_next, token) = self.next.get();
            self.entries[index].set_next(current_next as u64)?;
            if self.next.try_update(index, token) {
                return Ok(value);
            }
        }
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

unsafe impl<T: Clone> Send for Slab<T> { }
unsafe impl<T: Clone> Sync for Slab<T> { }

// Simple mechanism to test the CAS loops.  This allows us to block threads right before the
// `compare_exchange_weak` call, then unblock threads in a controlled manner.
#[cfg(test)]
mod cas_loop_testing {
    use std::{
        collections::HashMap,
        sync::{Condvar, Mutex},
        thread::{self, ThreadId},
    };
    use once_cell::sync::OnceCell;

    enum BlockType {
        Block,
        RunOnceThenBlock,
    }

    static MAP: OnceCell<Mutex<HashMap<ThreadId, BlockType>>> = OnceCell::new();
    static CONDITION: Condvar = Condvar::new();

    fn map() -> &'static Mutex<HashMap<ThreadId, BlockType>> {
        MAP.get_or_init(|| Mutex::new(HashMap::new()))
    }

    pub fn wait() {
        let mut map = map().lock().unwrap();
        loop {
            match map.get_mut(&thread::current().id()) {
                None => break,
                Some(block_type) => match block_type {
                    BlockType::RunOnceThenBlock => {
                        *block_type = BlockType::Block;
                        break;
                    }
                    _ => ()
                }
            }
            map = CONDITION.wait(map).unwrap();
        }
    }

    pub fn block(thread_id: ThreadId) {
        map().lock().unwrap().insert(thread_id, BlockType::Block);
    }

    pub fn unblock_once(thread_id: ThreadId) {
        map().lock().unwrap().insert(thread_id, BlockType::RunOnceThenBlock);
        CONDITION.notify_all()
    }

    pub fn unblock(thread_id: ThreadId) {
        map().lock().unwrap().remove(&thread_id);
        CONDITION.notify_all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_entry_locking() {
        let entry = LockedEntry::<Arc<&str>>::new(0, Arc::new("Hello"));
        entry
            .read(|_, _| {
                // A second reader is allowed
                assert!(entry.read(|_, _| { Ok(()) }).is_ok());
                // But a writer is not
                assert!(entry.write(|_, _| { Ok(()) }).is_err());
                Ok(())
            })
            .unwrap();

        entry
            .write(|_, _| {
                // A reader is not allowed while the entry is being written
                assert!(entry.read(|_, _| { Ok(()) }).is_err());
                // Neither is a writer
                assert!(entry.write(|_, _| { Ok(()) }).is_err());
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn test_simple_usage() {
        let slab = Slab::new();
        let handle1 = slab.insert(Arc::new("Hello")).unwrap();
        let handle2 = slab.insert(Arc::new("Goodbye")).unwrap();
        assert_eq!(slab.entries.len(), 2);
        assert_eq!(*slab.get_clone(handle1).unwrap(), "Hello");
        slab.remove(handle1).unwrap();
        assert_eq!(*slab.get_clone(handle2).unwrap(), "Goodbye");
        slab.remove(handle2).unwrap();
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

        // Test the CAS loops of the `Next` type
        //
        // We create 4 threads that each perform an operation "simultaneously", meaning:
        //   * Each thread runs until after the read, before the `compare_exchange_weak`.
        //   * Unblock first blocked thread, allowing `compare_exchange_weak` to succeed.
        //   * Unblock the remaining threads for one more loop.  These will block again before
        //     `compare_exchange_weak` and we repeat the loop with one less thread.
        #[test]
        fn test_slab_threading(all_operations in vec(vec(operation(), 5), 0..100)) {
            let slab = Slab::new();
            let mut send_errors = vec![];
            thread::scope(|scope| {
                let (senders, handles): (Vec<_>, Vec<_>) = (0..4).into_iter().map(|_| {
                    let slab = &slab;
                    let (sender, receiever) = channel::<Option<Operation>>();
                    let handle = scope.spawn(move || {
                        let mut tester = SlabTester::new(slab);
                        for operation in receiever {
                            match operation {
                                None => return,
                                Some(operation) => tester.run(operation),
                            };
                        }
                        tester.check();
                    });
                    (sender, handle)
                })
                .unzip();

                for operations in all_operations {
                    for thread in handles.iter() {
                        cas_loop_testing::block(thread.thread().id());
                    }
                    for (sender, operation) in zip(senders.iter(), operations) {
                        if let Err(e) = sender.send(Some(operation)) {
                            send_errors.push(e.to_string())
                        }
                    }
                    for i in 0..handles.len() {
                        cas_loop_testing::unblock(handles[i].thread().id());
                        for j in (i + 1)..handles.len() {
                            cas_loop_testing::unblock_once(handles[j].thread().id());
                        }
                    }
                }
                for sender in senders.iter() {
                    if let Err(e) = sender.send(None) {
                        send_errors.push(e.to_string())
                    }
                }
            });
            assert_eq!(send_errors, Vec::<String>::new());
        }
    }
}
