/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! [`RustFuture`] represents a [`Future`] that can be sent to the foreign code over FFI.
//!
//! This type is not instantiated directly, but via the procedural macros, such as `#[uniffi::export]`.
//!
//! # The big picture
//!
//! What happens when you call an async function exported from the Rust API?
//!
//! 1. You make a call to a generated async function in the foreign bindings.
//! 2. That function makes a scaffolding call to get a [RustFutureHandle]
//! 3. That function suspends itself, then starts up the future, passing a [ForeignExecutor] and a
//!    callback
//! 4. Rust uses the [ForeignExecutor] to schedules polls of the Future until it's ready.  Then
//!    invokes the callback.
//! 5. The suspended async function resumes when either the callback is called or the foreign
//!    language cancels the future.
//! 6. The generated function drops the future
//!
//! # Anatomy of an async call
//!
//! Let's consider the following Rust function:
//!
//! ```rust,ignore
//! #[uniffi::export]
//! async fn hello() -> bool {
//!     true
//! }
//! ```
//!
//! In Rust, this `async fn` syntax is strictly equivalent to a normal function that returns a
//! `Future`:
//!
//! ```rust,ignore
//! #[uniffi::export]
//! fn hello() -> impl Future<Output = bool> { /* … */ }
//! ```
//!
//! `uniffi-bindgen` will generate several scaffolding functions for each exported async function:
//!
//! ```rust,ignore
//! // The scaffolding function, which returns a RustFutureHandle
//! #[no_mangle]
//! pub extern "C" fn uniffi_fn_hello(
//!     // ...If the function inputted arguments, the lowered versions would go here
//! ) -> RustFutureHandle {
//!     ...
//! }
//!
//! // The startup function, which inputs:
//! //  - handle: RustFutureHandle
//! //  - executor: used to schedule polls of the future
//! //  - callback: invoked when the future is ready
//! //  - callback_data: opaque pointer that's passed to the callback.  It points to any state needed to
//! //    resume the async function.
//! //
//! // Rust will continue to poll the future until it's ready, after that.  The callback will
//! // eventually be invoked with these arguments:
//! //   - callback_data
//! //   - FfiConverter::ReturnType (the type that would be returned by a sync function)
//! //   - RustCallStatus (used to signal errors/panics when executing the future)
//! //
//! // Once the callback is called, Rust will stop polling the future.
//! #[no_mangle]
//! pub extern "C" fn uniffi_future_startup_hello(
//!     handle: RustFutureHandle,
//!     uniffi_executor: ForeignExecutor,
//!     uniffi_callback: <bool as FfiConverter<crate::UniFFITag>>::FutureCallback,
//!     uniffi_callback_data: *const (),
//!     uniffi_call_status: &mut ::uniffi::RustCallStatus
//! ) {
//!    ...
//! }
//!
//! // The cancel function, which inputs:
//! //
//! // This causes the future to immediately stop and invoke the callback with the status code
//! // RustCallStatusCode::Cancelled.
//! //
//! // Once the callback is called, Rust will stop polling the future.
//! #[no_mangle]
//! pub extern "C" fn uniffi_future_startup_cancel(handle: RustFutureHandle)
//! ) {
//!     ...
//! }
//!
//! // Drop the future
//! //
//! // If this is called when the future is still Pending, then it will be dropped, releasing any
//! // held resources and Rust will stop polling it.
//! #[no_mangle]
//! pub extern "C" fn uniffi_future_drop_hello(handle: RustFutureHandle) {
//!   ...
//! }
//! ```
//!
//! ## How does `Future` work exactly?
//!
//! A [`Future`] in Rust does nothing. When calling an async function, it just
//! returns a `Future` but nothing has happened yet. To start the computation,
//! the future must be polled. It returns [`Poll::Ready(r)`][`Poll::Ready`] if
//! the result is ready, [`Poll::Pending`] otherwise. `Poll::Pending` basically
//! means:
//!
//! > Please, try to poll me later, maybe the result will be ready!
//!
//! This model is very different than what other languages do, but it can actually
//! be translated quite easily, fortunately for us!
//!
//! But… wait a minute… who is responsible to poll the `Future` if a `Future` does
//! nothing? Well, it's _the executor_. The executor is responsible _to drive_ the
//! `Future`: that's where they are polled.
//!
//! But… wait another minute… how does the executor know when to poll a [`Future`]?
//! Does it poll them randomly in an endless loop? Well, no, actually it depends
//! on the executor! A well-designed `Future` and executor work as follows.
//! Normally, when [`Future::poll`] is called, a [`Context`] argument is
//! passed to it. It contains a [`Waker`]. The [`Waker`] is built on top of a
//! [`RawWaker`] which implements whatever is necessary. Usually, a waker will
//! signal the executor to poll a particular `Future`. A `Future` will clone
//! or pass-by-ref the waker to somewhere, as a callback, a completion, a
//! function, or anything, to the system that is responsible to notify when a
//! task is completed. So, to recap, the waker is _not_ responsible for waking the
//! `Future`, it _is_ responsible for _signaling_ the executor that a particular
//! `Future` should be polled again. That's why the documentation of
//! [`Poll::Pending`] specifies:
//!
//! > When a function returns `Pending`, the function must also ensure that the
//! > current task is scheduled to be awoken when progress can be made.
//!
//! “awakening” is done by using the `Waker`.
//!
//! [`Future`]: https://doc.rust-lang.org/std/future/trait.Future.html
//! [`Future::poll`]: https://doc.rust-lang.org/std/future/trait.Future.html#tymethod.poll
//! [`Pol::Ready`]: https://doc.rust-lang.org/std/task/enum.Poll.html#variant.Ready
//! [`Poll::Pending`]: https://doc.rust-lang.org/std/task/enum.Poll.html#variant.Pending
//! [`Context`]: https://doc.rust-lang.org/std/task/struct.Context.html
//! [`Waker`]: https://doc.rust-lang.org/std/task/struct.Waker.html
//! [`RawWaker`]: https://doc.rust-lang.org/std/task/struct.RawWaker.html

use crate::{
    ffi::foreignexecutor::RustTaskCallbackCode, rust_call_with_out_status, schedule_raw,
    FfiConverter, FfiDefault, ForeignExecutor, ForeignExecutorHandle, RustCallStatus,
    RustCallStatusCode,
};
use std::{
    cell::UnsafeCell,
    future::Future,
    panic,
    pin::Pin,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc, Weak,
    },
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

/// Callback that we invoke when a `RustFuture` is ready.
///
/// The foreign code passes a pointer to one of these callbacks along with an opaque data pointer.
/// When the future is ready, we invoke the callback.
pub type FutureCallback<T> =
    extern "C" fn(callback_data: *const (), result: T, status: RustCallStatus);

/// Opaque handle for a Rust future that's stored by the foreign language code
#[repr(transparent)]
pub struct RustFutureHandle(*const ());

impl RustFutureHandle {
    pub fn null() -> Self {
        Self(std::ptr::null())
    }
}

/// Future that the foreign code is awaiting
struct RustFuture<F, T, UT>
where
    F: Future<Output = T> + Send,
    T: FfiConverter<UT>,
{
    // Bitfield used for synchronization.  See the `STATE_*` constants for details on how each bit
    // it used.
    state: AtomicU8,
    future: UnsafeCell<F>,
    context: UnsafeCell<Option<RustFutureContext<T, UT>>>,
}

/// Context for a Rust future.
///
/// These is the ForeignExecutor used to drive the future to completion alongside the callback to
/// call when it's done.
struct RustFutureContext<T: FfiConverter<UT>, UT> {
    executor: ForeignExecutor,
    callback: T::FutureCallback,
    callback_data: *const (),
}

// Mark [RustFuture] as [Send] + [Sync], since we will be sharing it between threads.  See the code
// at the top of the impl block for now we manage access to the [UnsafeCell] fields.

unsafe impl<F, T, UT> Send for RustFuture<F, T, UT>
where
    F: Future<Output = T> + Send,
    T: FfiConverter<UT>,
{
}

unsafe impl<F, T, UT> Sync for RustFuture<F, T, UT>
where
    F: Future<Output = T> + Send,
    T: FfiConverter<UT>,
{
}

/// Create a [RustFuture] and return a handle to it
///
/// The returned `RustFutureHandle` represents the one permanent strong reference to the future.
/// When the corresponding foreign async function completes or is cancelled, the returned
/// [RustFutureHandle] must be passed to rust_future_free().
///
/// This is needed to handle cancellation correctly.  When the async function is cancelled, the foreign language
/// will call `rust_future_free()` through a scaffolding function.  When that happens, we want to
/// immediately drop the future and release all resources -- even when the waker is stored by a
/// Rust executor.  See #1669 for an example of this.
///
/// For each async function, UniFFI generates a scaffolding function that forward calls to
/// [rust_future_startup] and scaffolding functions to forward calls to each of the
/// [RustFutureVTable] functions.
pub fn rust_future_new<F, T, UT>(future: F) -> RustFutureHandle
where
    // The future needs to be `Send`, since it will move to whatever thread the foreign executor
    // chooses.  However, it doesn't need to be `Sync', since we don't share references between
    // threads (see do_wake()).
    F: Future<Output = T> + Send,
    T: FfiConverter<UT>,
{
    let future = RustFuture::new(future);
    RustFutureHandle(Arc::into_raw(future) as *const ())
}

impl<F, T, UT> RustFuture<F, T, UT>
where
    F: Future<Output = T> + Send,
    T: FfiConverter<UT>,
{
    fn new(future: F) -> Arc<Self> {
        Arc::new(Self {
            state: AtomicU8::new(0),
            future: UnsafeCell::new(future),
            /// The context gets set in startup()
            context: UnsafeCell::new(None),
        })
    }

    /// Lock bit on [Self::state].  Set this in order to access the [UnsafeCell] fields.
    const STATE_LOCKED: u8 = 1 << 0;
    /// Needs wake bit on [Self::state].  If set, then wake was called and so [Self::do_wake] needs
    /// to be executed at some point.
    const STATE_NEEDS_DO_WAKE: u8 = 1 << 1;
    /// Cancelled bit on [Self::state].  If set, then stop driving the future and call the foreign
    /// callback with a [RustCallStatusCode::Cancelled] code.
    const STATE_CANCELLED: u8 = 1 << 2;

    /// Startup a new future
    ///
    /// This starts the process to drive the future to completion, then call the foreign callback.
    /// `startup()` can only be called once.
    fn startup(
        self: Arc<Self>,
        executor_handle: ForeignExecutorHandle,
        callback: T::FutureCallback,
        callback_data: *const (),
    ) {
        match self.state.compare_exchange(
            // The state should previously have no bits set
            0,
            // We want to aquire a lock
            Self::STATE_LOCKED,
            Ordering::Acquire,
            Ordering::Relaxed,
        ) {
            Ok(_) => {
                // Successfully acquired the lock
                let context = unsafe { &mut *self.context.get() };
                if context.is_none() {
                    *context = Some(RustFutureContext {
                        executor: ForeignExecutor::new(executor_handle),
                        callback,
                        callback_data,
                    });
                    self.state.store(
                        Self::STATE_LOCKED | Self::STATE_INITIALIZED | Self::STATE_WAKE_CALLED,
                        Ordering::Release,
                    );
                    self.schedule_do_wake(executor_handle);
                } else {
                    log::error!("context already set in startup()")
                }
            }
            Err(state) if (state | Self::STATE_CANCELLED) != 0 => {
                // task cancelled before it was started
                T::invoke_future_callback(
                    callback,
                    callback_data,
                    T::ReturnType::ffi_default(),
                    RustCallStatus::cancelled(),
                );
            }
            Err(state) => {
                log::error!("Unexpected state in startup(): {state:b}");
            }
        }
    }

    /// Cancel a future
    ///
    /// This causes the future to immediately invoke the callback with a
    /// [RustCallStatusCode::Cancelled] error code
    fn cancel(self: Arc<Self>) {
        // Try to acquire the lock.  Ensure that the STATE_CANCELLED bit is set regardless.
        let prev_state = self.state.fetch_or(
            Self::STATE_LOCKED | Self::STATE_CANCELLED,
            Ordering::Acquire,
        );
        if prev_state & Self::STATE_INITIALIZED == 0 {
            // task hasn't started yet, release the lock and cancel once startup() is called
            self.state.fetch_and(!Self::STATE_LOCKED, Ordering::Relaxed);
        } else if (prev_state & Self::STATE_LOCKED) == 0 {
            self.complete_after_cancel();
        }
    }

    // Complete the future after cancellation.
    //
    // The lock must have been acquired before calling this function
    fn complete_after_cancel(&self) {
        // We can take a reference to context the lock must be acquired before calling the function
        match unsafe { &*self.context.get() } {
            Some(context) => {
                // Call the callback.  Keep STATE_LOCKED set since we don't want to continue
                // polling the future.
                T::invoke_future_callback(
                    context.callback,
                    context.callback_data,
                    T::ReturnType::ffi_default(),
                    RustCallStatus::cancelled(),
                )
            }
            None => log::error!("context not set in complete_after_cancel()"),
        };
    }


    /// Wake up soon and poll our future.
    fn wake(self: Arc<Self>) {
        // Try to acquire the lock.  Ensure that the STATE_WAKE_CALLED bit is set regardless.
        let prev_state = self.state.fetch_or(
            Self::STATE_LOCKED | Self::STATE_WAKE_CALLED,
            Ordering::Acquire,
        );
        if prev_state & Self::STATE_INITIALIZED == 0 {
            log::error!("Uninitialized state in wake(): {prev_state:b}");
        } else if prev_state & (Self::STATE_LOCKED | Self::STATE_CANCELLED) == Self::STATE_CANCELLED {
            // Successfully acquired the lock, but the future was already cancelled
            self.complete_after_cancel();
        } else if (prev_state & Self::STATE_LOCKED) == 0 {
            // Successfully acquired the lock
            let context = unsafe { &*self.context.get() };
            match context {
                Some(context) => self.schedule_do_wake(context.executor.handle),
                None => log::error!("Context not set in wake()"),
            }
        }
    }

    /// Start a `do_wake()` call
    fn start_do_wake<'a>(&'a self) -> Option<Pin<&'a mut F>> {
        // Unset the STATE_WAKE_CALLED bit so that we can test it at the end of start_do_wake.
        let prev_state = self
            .state
            .fetch_and(!Self::STATE_WAKE_CALLED, Ordering::Acquire);

        if prev_state & Self::STATE_INITIALIZED == 0 {
            log::error!("Uninitialized state in start_do_wake(): {prev_state:b}");
        }

        if prev_state & (Self::STATE_INITIALIZED | Self::STATE_LOCKED)
            == Self::STATE_INITIALIZED | Self::STATE_LOCKED
        {
            // SAFETY:
            //  - Getting a &mut to self.future is safe because:
            //    - A previous caller locked the state for us
            //    - We called Ordering::Acquire when unsetting the STATE_WAKE_CALLED bit
            //    - continue_after_do_wake() uses Ordering::Release
            //    - complete_after_do_wake() keeps the state locked, so there will be no more access
            //      to self.future.
            //  - Pin::new_unchecked() is safe because:
            //    - This is the only time we take a &mut to self.future
            //    - We never take a &mut to Self
            //    - RustFuture is private to this module so no other code can take a &mut to it.
            unsafe { Some(Pin::new_unchecked(&mut *self.future.get())) }
        } else {
            log::error!("Unexpected state in start_do_wake(): {prev_state:b}");
            None
        }
    }

    /// Continue after a call to `do_wake()` where the future was still pending
    fn continue_after_do_wake(self: Arc<Self>) {
        // If STATE_WAKE_CALLED is still not set, then unset the STATE_LOCKED bit.
        match self.state.compare_exchange(
            Self::STATE_INITIALIZED | Self::STATE_LOCKED,
            Self::STATE_INITIALIZED,
            Ordering::Release,
            Ordering::Relaxed,
        ) {
            Ok(_) => (),
            Err(Self::STATE_INITIALIZED | Self::STATE_LOCKED | Self::STATE_WAKE_CALLED) => {
                // compare_exchange doesn't do a Release when the comparison fails, so do one now
                self.state.fetch_or(0, Ordering::Release);
                // SAFETY: the state is still locked and we used an Ordering::Acquire operation in
                // start_do_wake().
                let context = unsafe { &*self.context.get() };
                match context {
                    Some(context) => self.schedule_do_wake(context.executor.handle),
                    None => log::error!("Context not set in continue_after_do_wake()"),
                }
            }
            Err(state) => log::error!("Unexpected state in continue_after_do_wake(): {state:b}"),
        }
    }

    /// Complete the future after a call to `do_wake()` where the future was ready
    fn complete_after_do_wake(self: Arc<Self>, result: T::ReturnType, out_status: RustCallStatus) {
        // We can take a reference to context because we never unlocked the state
        match unsafe { &*self.context.get() } {
            Some(context) => {
                // Call the callback.  Keep STATE_LOCKED set since we must not poll future again
                T::invoke_future_callback(
                    context.callback,
                    context.callback_data,
                    result,
                    out_status,
                );
            }
            None => log::error!("context not set in complete_after_do_wake()"),
        }
    }

    fn wake_from_weak(weak: &Weak<Self>) {
        if let Some(rust_future) = weak.upgrade() {
            rust_future.wake();
        }
    }

    /// Schedule `do_wake`.
    ///
    /// `self` is consumed but _NOT_ dropped, it's purposely leaked via `into_raw()`.
    /// `wake_callback()` will call `from_raw()` to reverse the leak.
    fn schedule_do_wake(self: Arc<Self>, executor_handle: ForeignExecutorHandle) {
        unsafe {
            let raw_ptr = Arc::into_raw(self);
            // SAFETY: The `into_raw()` / `from_raw()` contract guarantees that our executor cannot
            // be dropped before we call `from_raw()` on the raw pointer. This means we can safely
            // use its handle to schedule a callback.
            if !schedule_raw(
                executor_handle,
                0,
                Self::wake_callback,
                raw_ptr as *const (),
            ) {
                // There was an error scheduling the callback, drop the arc reference since
                // `wake_callback()` will never be called
                //
                // Note: specifying the `<Self>` generic is a good safety measure.  Things would go
                // very bad if Rust inferred the wrong type.
                //
                // However, the `Pin<>` part doesn't matter since its `repr(transparent)`.
                Arc::<Self>::decrement_strong_count(raw_ptr);
            }
        }
    }

    extern "C" fn wake_callback(self_ptr: *const (), status_code: RustTaskCallbackCode) {
        // No matter what, call `Arc::from_raw()` to balance the `Arc::into_raw()` call in
        // `schedule_do_wake()`.
        let task = unsafe { Arc::from_raw(self_ptr as *const Self) };
        if status_code == RustTaskCallbackCode::Success {
            // Only drive the future forward on `RustTaskCallbackCode::Success`.
            // `RUST_TASK_CALLBACK_CANCELED` indicates the foreign executor has been cancelled /
            // shutdown and we should not continue.
            task.do_wake();
        }
    }

    // Does the work for wake, we take care to ensure this always runs in a serialized fashion.
    fn do_wake(self: Arc<Self>) {
        let future = match self.start_do_wake() {
            Some(future) => future,
            None => return, // Note: start_do_wake() already logged an error for us
        };

        let waker = self.make_waker();

        // Run the poll and lift the result if it's ready
        let mut out_status = RustCallStatus::default();
        let result: Option<Poll<T::ReturnType>> = rust_call_with_out_status(
            &mut out_status,
            // This closure uses a `&mut F` value, which means it's not UnwindSafe by default.  If
            // the closure panics, the future may be in an invalid state.
            //
            // However, we can safely use `AssertUnwindSafe` since a panic will lead the `Err()`
            // case below.  In that case, we will never run `do_wake()` again and will no longer
            // access the future.
            panic::AssertUnwindSafe(|| match future.poll(&mut Context::from_waker(&waker)) {
                Poll::Pending => Ok(Poll::Pending),
                Poll::Ready(v) => T::lower_return(v).map(Poll::Ready),
            }),
        );

        // All the main work is done, time to finish up
        match result {
            Some(Poll::Pending) => self.continue_after_do_wake(),
            Some(Poll::Ready(v)) => self.complete_after_do_wake(v, out_status),
            // Error/panic polling the future.  Call the callback with a default value.
            // `out_status` contains the error code and serialized error.
            None => self.complete_after_do_wake(T::ReturnType::ffi_default(), out_status),
        };
    }

    fn make_waker(self: &Arc<Self>) -> Waker {
        // This is safe as long as we implement the waker interface correctly.
        unsafe {
            Waker::from_raw(RawWaker::new(
                Arc::downgrade(self).into_raw() as *const (),
                &Self::RAW_WAKER_VTABLE,
            ))
        }
    }

    // Implement the waker interface by defining a RawWakerVTable
    //
    // Each function inputs a data pointer that has been created from `Weak::<Self>::into_raw()`
    const RAW_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
        Self::raw_clone,
        Self::raw_wake,
        Self::raw_wake_by_ref,
        Self::raw_drop,
    );

    /// This function will be called when the `RawWaker` gets cloned, e.g. when
    /// the `Waker` in which the `RawWaker` is stored gets cloned.
    unsafe fn raw_clone(self_ptr: *const ()) -> RawWaker {
        let weak = Weak::from_raw(self_ptr as *const Self);
        let raw_waker = RawWaker::new(
            weak.clone().into_raw() as *const (),
            &Self::RAW_WAKER_VTABLE,
        );
        // `self_ptr` represents a reference.  Use forget() to avoid decrementing the weak count.
        std::mem::forget(weak);
        raw_waker
    }

    /// This function will be called when `wake` is called on the `Waker`. It
    /// must wake up the task associated with this `RawWaker`.
    unsafe fn raw_wake(self_ptr: *const ()) {
        Self::wake_from_weak(&Weak::from_raw(self_ptr as *const Self))
    }

    /// This function will be called when `wake_by_ref` is called on the
    /// `Waker`. It must wake up the task associated with this `RawWaker`.
    unsafe fn raw_wake_by_ref(self_ptr: *const ()) {
        let weak = Weak::from_raw(self_ptr as *const Self);
        Self::wake_from_weak(&weak);
        // `self_ptr` represents a reference.  Use forget() to avoid decrementing the weak count.
        std::mem::forget(weak);
    }

    /// This function gets called when a `RawWaker` gets dropped.
    unsafe fn raw_drop(self_ptr: *const ()) {
        drop(Weak::from_raw(self_ptr as *const Self));
    }
}

/// VTable to implement the RustFuture FFI.
///
/// This is an array of function pointers that use [RustFutureHandle] and forward the call to
/// the corresponding `RustFuture` method.
///
/// The main point is to get around the fact that the `F` type in `RustFuture` is an anonymous
/// type, which can't be named.
#[doc(hidden)]
pub struct RustFutureVTable<T, UT>
where
    T: FfiConverter<UT>,
{
    pub startup: unsafe fn(RustFutureHandle, ForeignExecutorHandle, T::FutureCallback, *const ()),
    pub free: unsafe fn(RustFutureHandle),
}

impl<T, UT> RustFutureVTable<T, UT>
where
    T: FfiConverter<UT>,
{
    /// Create a RustFutureVTable
    ///
    /// Inputs an async function that inputs a single tuple argument and outputs the Future that
    /// will be wrapped by [RustFuture].
    pub const fn new<Func, Args, F>(_: &Func) -> Self
    where
        Func: Fn(Args) -> F,
        F: Future<Output = T> + Send,
    {
        unsafe {
            Self {
                startup: |handle, executor_handle, callback, callback_data| {
                    let arc = Arc::from_raw(handle.0 as *const RustFuture<F, T, UT>);
                    // Create a new Arc to run startup
                    Arc::clone(&arc).startup(executor_handle, callback, callback_data);
                    // Forget the original Arc, since it represented a reference
                    std::mem::forget(arc);
                },
                free: |handle: RustFutureHandle| {
                    drop(Arc::from_raw(handle.0 as *const RustFuture<F, T, UT>))
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{try_lift_from_rust_buffer, MockEventLoop, RustCallStatusCode};
    use std::sync::Weak;

    // Mock future that we can manually control using an Option<>
    struct MockFuture(Option<Result<bool, String>>);

    impl Future for MockFuture {
        type Output = Result<bool, String>;

        fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
            match &self.0 {
                Some(v) => Poll::Ready(v.clone()),
                None => Poll::Pending,
            }
        }
    }

    // Type alias for the RustFuture we'll use in our tests
    type TestRustFuture = RustFuture<MockFuture, Result<bool, String>, crate::UniFfiTag>;

    // Stores the result that we send to the foreign code
    #[derive(Default)]
    struct MockForeignResult {
        value: i8,
        status: RustCallStatus,
    }

    extern "C" fn mock_foreign_callback(data_ptr: *const (), value: i8, status: RustCallStatus) {
        let result: &mut Option<MockForeignResult> =
            unsafe { &mut *(data_ptr as *mut Option<MockForeignResult>) };
        *result = Some(MockForeignResult { value, status });
    }

    // Bundles everything together so that we can run some tests
    struct TestFutureEnvironment {
        rust_future: Arc<TestRustFuture>,
        foreign_result: Pin<Box<Option<MockForeignResult>>>,
        eventloop: Arc<MockEventLoop>,
    }

    impl TestFutureEnvironment {
        fn new(eventloop: &Arc<MockEventLoop>) -> Self {
            let foreign_result = Box::pin(None);

            let rust_future = TestRustFuture::new(MockFuture(None));
            Self {
                rust_future,
                foreign_result,
                eventloop: Arc::clone(eventloop),
            }
        }

        fn startup(&self) {
            let foreign_result_ptr = &*self.foreign_result as *const Option<_> as *const ();
            Arc::clone(&self.rust_future).startup(
                self.eventloop.new_handle(),
                mock_foreign_callback,
                foreign_result_ptr,
            );
        }

        fn wake(&self) {
            RustFuture::wake(Arc::clone(&self.rust_future))
        }

        fn rust_future_weak(&self) -> Weak<TestRustFuture> {
            Arc::downgrade(&self.rust_future)
        }

        fn complete_future(&self, value: Result<bool, String>) {
            unsafe {
                (*self.rust_future.future.get()).0 = Some(value);
            }
        }
    }

    #[test]
    fn test_wake() {
        let eventloop = MockEventLoop::new();
        let mut test_env = TestFutureEnvironment::new(&eventloop);
        // Initially, we shouldn't have a result and nothing should be scheduled
        assert!(test_env.foreign_result.is_none());
        assert_eq!(eventloop.call_count(), 0);

        // startup() should schedule a wakeup call
        test_env.startup();
        assert_eq!(eventloop.call_count(), 1);

        // When that call runs, we should still not have a result yet
        eventloop.run_all_calls();
        assert!(test_env.foreign_result.is_none());
        assert_eq!(eventloop.call_count(), 0);

        // Multiple wakes should only result in 1 scheduled call
        test_env.wake();
        test_env.wake();
        assert_eq!(eventloop.call_count(), 1);

        // Make the future ready, which should call mock_foreign_callback and set the result
        test_env.complete_future(Ok(true));
        eventloop.run_all_calls();
        let result = test_env
            .foreign_result
            .take()
            .expect("Expected result to be set");
        assert_eq!(result.value, 1);
        assert_eq!(result.status.code, RustCallStatusCode::Success);
        assert_eq!(eventloop.call_count(), 0);

        // Future wakes shouldn't schedule any calls
        test_env.wake();
        assert_eq!(eventloop.call_count(), 0);
    }

    #[test]
    fn test_error() {
        let eventloop = MockEventLoop::new();
        let mut test_env = TestFutureEnvironment::new(&eventloop);
        test_env.complete_future(Err("Something went wrong".into()));
        test_env.startup();
        eventloop.run_all_calls();
        let result = test_env
            .foreign_result
            .take()
            .expect("Expected result to be set");
        assert_eq!(result.status.code, RustCallStatusCode::Error);
        unsafe {
            assert_eq!(
                try_lift_from_rust_buffer::<String, crate::UniFfiTag>(
                    result.status.error_buf.assume_init()
                )
                .unwrap(),
                String::from("Something went wrong"),
            )
        }
        assert_eq!(eventloop.call_count(), 0);
    }

    #[test]
    fn test_ref_counts() {
        // Create a rust future and 2 wakers.
        //
        // The future itself should hold a strong reference and each waker should hold a weakref
        let test_env = TestFutureEnvironment::new(&MockEventLoop::new());
        let waker = test_env.rust_future.make_waker();
        let waker2 = test_env.rust_future.make_waker();
        // Create one more Weak to handle the reference counting.
        let weak_ref = test_env.rust_future_weak();

        assert_eq!((weak_ref.strong_count(), weak_ref.weak_count()), (1, 3));

        // Dropping each waker should reduce the weak count by one
        drop(waker);
        assert_eq!((weak_ref.strong_count(), weak_ref.weak_count()), (1, 2));
        drop(waker2);
        assert_eq!((weak_ref.strong_count(), weak_ref.weak_count()), (1, 1));

        // Dropping the RustFuture should reduce the strong count to 0 (which also makes the weak
        // count 0, even though we still have a weak ref).
        drop(test_env);
        assert_eq!(weak_ref.strong_count(), 0);
    }

    // Test trying to create a RustFuture before the executor is shutdown.
    //
    // The main thing we're testing is that we correctly drop the Future in this case
    #[test]
    fn test_executor_shutdown() {
        let eventloop = MockEventLoop::new();
        eventloop.shutdown();
        let test_env = TestFutureEnvironment::new(&eventloop);
        let weak_ref = test_env.rust_future_weak();
        // When we wake the future, it should try to schedule a callback and fail.  This should
        // cause the future to be dropped
        test_env.wake();
        drop(test_env);
        assert!(weak_ref.upgrade().is_none());
    }

    // Similar run a similar test to the last, but simulate an executor shutdown after the future was
    // scheduled, but before the callback is called.
    #[test]
    fn test_executor_shutdown_after_schedule() {
        let eventloop = MockEventLoop::new();
        let test_env = TestFutureEnvironment::new(&eventloop);
        let weak_ref = test_env.rust_future_weak();
        test_env.complete_future(Ok(true));
        test_env.startup();
        eventloop.shutdown();
        eventloop.run_all_calls();

        // Test that the foreign async side wasn't completed.  Even though we could have
        // driven the future to completion, we shouldn't have since the executor was shutdown
        assert!(test_env.foreign_result.is_none());
        // Also test that we've dropped all references to the future
        drop(test_env);
        assert!(weak_ref.upgrade().is_none());
    }
}
