/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Slab allocator-style storage for types that are shared across the FFI.
//!
//! The [Slab] type stores Rust values and generates handles that can be passed across the FFI.
//! Usage:
//! * Create a `Slab` that will store values.
//! * Call `insert()` to store a value and get a handle that can be passed to the foreign side.
//! * When the foreign side wants to use that value, it passes back the handle.
//!   Use `get()` to get a reference to the stored value
//! * When the foreign side is finished with the value, it passes the handle to a free scaffolding function.
//!   That function calls `remove` to free the allocation.
//!
//! This system provides several benefits:
//! * Handles are simple integer values, which are simpler to work with than pointers.
//! * The implementation is 100% safe code.
//! * Handles are 32 bits, which makes them easier to work with on languages like JS that don't support full 64-bit integers.
//! * Handles support a tag value, which can be used to differentiate different types of handles.
//!   The trait interface code uses this to differentiate between Rust-implemented and foreign-implemented traits.

use std::{fmt, sync::Mutex};

use crate::FfiDefault;

/// Handle for a value stored in the slab
///
/// * The first 30 bits are used for indexes into the `entries` table and special values
///   - The first 1,000,000,000 values are used for indexes in the `entries` table.
///   - The next values are reserved for special cases (see `rust_future.rs` for an example).
/// * The last 2 bits are used for the tag
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Handle(u32);

impl Handle {
    pub fn from_u32(val: u32) -> Self {
        Self(val)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

impl FfiDefault for Handle {
    fn ffi_default() -> Self {
        Self(0)
    }
}

const MAX_ENTRIES: u32 = 1_000_000_000;
const END_OF_LIST: u32 = u32::MAX;
// Mask to get the tag bits of the handle
const TAG_MASK: u32 = 0xC0000000;
// Mask to get the inedx bits of the handle
const INDEX_MASK: u32 = 0x3FFFFFFF;

/// Entry in the slab table
#[derive(Debug)]
enum Entry<T> {
    // Vacant entry. These form a kind of linked-list in the EntryList.  Each inner value is the
    // index of the next vacant entry.
    Vacant { next: u32 },
    Occupied { value: T },
}

/// Slab error type
///
/// This is intentially simple since we always unwrap it in practice
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Vacant,
    InvalidTag,
    OverCapacity,
    NextOccuied,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vacant => write!(
                f,
                "Handle points to a vacant entry (likely a use-after-free bug)"
            ),
            Self::InvalidTag => write!(f, "Invalid tag"),
            Self::OverCapacity => write!(f, "Slab capacity exceeded"),
            Self::NextOccuied => write!(f, "Slab internal error: inner.next is occupied"),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Store values and generate handles that can be used by the foreign code
pub struct Slab<T> {
    tag: u32,
    inner: Mutex<SlabInner<T>>,
}

#[derive(Debug)]
struct SlabInner<T> {
    next: u32,
    entries: Vec<Entry<T>>,
}

impl<T: Clone> Slab<T> {
    pub const fn new() -> Self {
        Self::new_with_tag(0)
    }

    pub const fn new_with_tag(tag: u32) -> Self {
        if tag >= 4 {
            panic!("Tag does not fit in 2 bits")
        }
        Self {
            tag: tag << 30, // pre-shift the tag to simplify future operations
            inner: Mutex::new(SlabInner {
                next: END_OF_LIST,
                entries: vec![],
            }),
        }
    }

    // Get an index from a handle
    fn get_index(&self, handle: Handle) -> Result<usize> {
        if handle.0 & TAG_MASK == self.tag {
            Ok((handle.0 & INDEX_MASK) as usize)
        } else {
            Err(Error::InvalidTag)
        }
    }

    // Get a handle from an index
    fn get_handle(&self, index: usize) -> Handle {
        Handle(index as u32 | self.tag)
    }

    fn with_lock<F, R>(&self, operation: F) -> R
    where
        F: FnOnce(&mut SlabInner<T>) -> R,
    {
        let mut guard = self.inner.lock().unwrap();
        operation(&mut *guard)
    }

    /// Find a vacant index to insert a new item in and remove it from the vacant list
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
                    Entry::Occupied { .. } => return Err(Error::NextOccuied),
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
        self.with_lock(move |inner| {
            let idx = self.get_vacant_to_use(inner)?;
            inner.entries[idx] = Entry::Occupied { value };
            Ok(self.get_handle(idx))
        })
    }

    /// Get an cloned entry using a handle
    pub fn get_clone(&self, handle: Handle) -> Result<T> {
        let idx = self.get_index(handle)?;
        self.with_lock(|inner| match &inner.entries[idx] {
            Entry::Vacant { .. } => Err(Error::Vacant),
            Entry::Occupied { value } => Ok(value.clone()),
        })
    }

    // Remove an item from the slab, returning the original value
    pub fn remove(&self, handle: Handle) -> Result<T> {
        let idx = self.get_index(handle)?;
        self.with_lock(move |inner| {
            if let Entry::Vacant { .. } = inner.entries[idx] {
                return Err(Error::Vacant);
            }
            // Push the entry to the head of the vacant list
            let old_value =
                std::mem::replace(&mut inner.entries[idx], Entry::Vacant { next: inner.next });
            inner.next = idx as u32;
            match old_value {
                Entry::Occupied { value } => Ok(value),
                Entry::Vacant { .. } => unreachable!(),
            }
        })
    }

    pub fn insert_unchecked(&self, value: T) -> Handle {
        self.insert(value).unwrap_or_else(|e| panic!("{e}"))
    }

    /// Get an cloned entry using a handle
    pub fn get_clone_unchecked(&self, handle: Handle) -> T {
        self.get_clone(handle).unwrap_or_else(|e| panic!("{e}"))
    }

    // Remove an item from the slab, returning the original value
    pub fn remove_unchecked(&self, handle: Handle) -> T {
        self.remove(handle).unwrap_or_else(|e| panic!("{e}"))
    }

    // Create a special-cased handle
    //
    // This is guaranteed to not equal any real handles allocated by the slab.
    pub const fn special_value(&self, val: u32) -> Handle {
        Handle(MAX_ENTRIES as u32 + val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_slab_size<T>(slab: &Slab<T>, occupied_count: usize, vec_size: usize) {
        let inner = slab.inner.lock().unwrap();
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
        let handle1 = slab.insert("Hello").unwrap();
        check_slab_size(&slab, 1, 1);
        let handle2 = slab.insert("Goodbye").unwrap();
        check_slab_size(&slab, 2, 2);
        assert_eq!(slab.get_clone(handle1).unwrap(), "Hello");
        slab.remove(handle1).unwrap();
        check_slab_size(&slab, 1, 2);
        assert_eq!(slab.get_clone(handle2).unwrap(), "Goodbye");
        slab.remove(handle2).unwrap();
        check_slab_size(&slab, 0, 2);
    }

    #[test]
    fn test_special_values() {
        let slab = Slab::<String>::new();
        assert_eq!(slab.special_value(0), Handle(1_000_000_000));
        assert_eq!(slab.special_value(1), Handle(1_000_000_001));
    }

    #[test]
    fn test_tag_check() {
        let slab = Slab::<&str>::new_with_tag(0);
        let slab2 = Slab::<&str>::new_with_tag(1);
        let handle = slab.insert("Hello").unwrap();
        assert_eq!(Err(Error::InvalidTag), slab2.get_clone(handle));
        assert_eq!(Err(Error::InvalidTag), slab2.remove(handle));
    }

    #[test]
    fn test_use_after_free() {
        let slab = Slab::<&str>::new_with_tag(0);
        let handle = slab.insert("Hello").unwrap();
        slab.remove(handle).unwrap();
        assert_eq!(Err(Error::Vacant), slab.get_clone(handle));
        assert_eq!(Err(Error::Vacant), slab.remove(handle));
    }
}

#[cfg(test)]
mod property_test {
    use super::*;
    use proptest::collection::vec;
    use proptest::prelude::*;
    use std::collections::HashSet;

    // We will test lots of combinations of these operations
    #[derive(Debug, Clone)]
    enum Operation {
        // Insert a new value
        Insert,
        // Remove a value, using the inner value modulo the current size of the slab
        Remove(u32),
    }

    fn operation() -> impl Strategy<Value = Operation> {
        (0..2).prop_perturb(|value, mut rng| match value {
            0 => Operation::Insert,
            1 => Operation::Remove(rng.next_u32()),
            _ => unreachable!(),
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Run various combinations of Insert/Remove then test that the slab has the correct
        // behavior for both allocated and freed handles
        #[test]
        fn test_slab_operations(operations in vec(operation(), 0..100)) {
            let mut next_value = 0;
            let mut allocated = vec![];
            let mut freed = HashSet::new();

            let slab = Slab::new();
            for operation in operations {
                // Perform some operation
                match operation {
                    Operation::Insert => {
                        let handle = slab.insert(next_value).unwrap();
                        allocated.push((handle, next_value));
                        freed.remove(&handle);
                        next_value += 1;
                    }
                    Operation::Remove(value) => {
                        if allocated.is_empty() {
                            continue
                        }
                        let (handle, value) = allocated.remove(value as usize % allocated.len());
                        assert_eq!(slab.remove(handle).unwrap(), value);
                        freed.insert(handle);
                    }
                }
                // Test getting all handles, allocated or freed
                for (handle, value) in allocated.iter() {
                    assert_eq!(slab.get_clone(handle.clone()).unwrap(), *value)
                }
                for handle in freed.iter() {
                    assert_eq!(slab.get_clone(handle.clone()), Err(Error::Vacant));
                }
            }
        }
    }
}
