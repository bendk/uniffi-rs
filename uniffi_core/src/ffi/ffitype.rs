/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! FfiType trait
//!
//! This is implemented for all FfiType
//! When we make a FFI call into Rust we always need to return a value, even if that value will be
//! ignored because we're flagging an exception.  This trait defines what that value is for our
//! supported FFI types.

use std::sync::Arc;

use paste::paste;

use crate::{RustFutureFfi, Slab};

pub trait FfiType: 'static {
    /// Default/placeholder value
    ///
    /// The main use for this is when this type is passed as an out pointer.
    fn ffi_default() -> Self;

    /// Slab that stores RustFuture handles
    fn future_handle_slab() -> &'static Slab<Arc<dyn RustFutureFfi<Self>>>;
}

// Most types can be handled by delegating to Default
macro_rules! ffi_type {
    ($( ($($type:tt)*) => ($name:ident, $default:expr),)*) => {
        $(
            paste! {
                // Put the impl in its own module to avoid any namespacing issues.
                mod [< ffi_type_impl_ $name >] {
                    use super::*;
                    type Type = $($type)*;

                    static SLAB: Slab<Arc<dyn RustFutureFfi<Type>>> = Slab::<Arc<dyn RustFutureFfi<Type>>>::new();

                    impl FfiType for Type {
                        fn ffi_default() -> Self {
                            $default
                        }

                        fn future_handle_slab() -> &'static Slab<Arc<dyn RustFutureFfi<Type>>> {
                            &SLAB
                        }
                    }
                }
            }
        )*
    };
}

ffi_type! {
    (bool) => (bool, false),
    (i8) => (i8, 0),
    (u8) => (u8, 0),
    (i16) => (i16, 0),
    (u16) => (u16, 0),
    (i32) => (i32, 0),
    (u32) => (u32, 0),
    (i64) => (i64, 0),
    (u64) => (u64, 0),
    (f32) => (f32, 0.0),
    (f64) => (f64, 0.0),
    (()) => (unit, ()),
    (crate::Handle) => (handle, crate::Handle::default()),
    (*const std::ffi::c_void) => (c_void, std::ptr::null()),
    (crate::RustBuffer) => (rustbuffer, crate::RustBuffer::default()),
    (crate::ForeignExecutorHandle) => (foreign_executor, crate::ForeignExecutorHandle(std::ptr::null())),
}
