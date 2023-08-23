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
//! `uniffi-bindgen` will generate a scaffolding function for each exported async function that
//! outputs a RustFutureHandle and several global scaffolding functions that input RustFutureHandle
//! values.
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
//! pub extern "C" fn uniffi_future_startup(
//!     handle: RustFutureHandle,
//!     executor: ForeignExecutor,
//!     callback: FutureCallback<<bool as FfiConverter<crate::UniFFITag>>::FutureCallbackT>,
//!     callback_data: *const (),
//! ) {
//!    ...
//! }
//!
//! // Cancel the future, causing it to immediately stop and invoke the callback with the
//! // RustCallStatusCode::Cancelled.  After this, Rust will stop polling the future.
//! #[no_mangle]
//! pub extern "C" fn uniffi_future_cancel(handle: RustFutureHandle)
//! ) {
//!     ...
//! }
//!
//! // Drop the future, releasing any held resources and causing Rust to stop polling it.
//! #[no_mangle]
//! pub extern "C" fn uniffi_future_drop(handle: RustFutureHandle) {
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

/// Opaque handle for a FutureCallback that's stored by the foreign language code
#[repr(transparent)]
pub struct FutureCallbackHandle(*const ());

/// Opaque handle for a Rust future that's stored by the foreign language code
#[repr(transparent)]
pub struct RustFutureHandle(*const ());

/// Create a new [RustFutureHandle]
///
/// For each exported async function, UniFFI will create a scaffolding function that uses this to
/// create the [RustFutureHandle] to pass to the foreign code.
pub fn rust_future_new<F, T, UT>(future: F) -> RustFutureHandle
where
    // The future needs to be `Send`, since it will move to whatever thread the foreign executor
    // chooses.  However, it doesn't need to be `Sync', since we don't share references between
    // threads (see poll_future()).
    F: Future<Output = T> + Send,
    T: FfiConverter<UT>,
{
    // Create a RustFuture and coerce to `Arc<dyn RustFutureFFI>`, which is what we use to
    // implement the FFI
    let future_ffi = RustFuture::new(future) as Arc<dyn RustFutureFFI>;
    // Box the Arc, to convert the wide pointer into a normal sized pointer so that we can pass it
    // to the foreign code.
    let boxed_ffi = Box::new(future_ffi);
    // We can now create a RustFutureHandle
    RustFutureHandle(Box::into_raw(boxed_ffi) as *mut ())
}

/// Startup a Rust future
///
/// This is called when a Rust future is awaited by foreign code.  Each [RustFutureHandle] must
/// only be passed to [rust_future_startup] once.
pub unsafe fn rust_future_startup(
    handle: RustFutureHandle,
    executor_handle: ForeignExecutorHandle,
    callback: FutureCallbackHandle,
    callback_data: *const (),
) {
    let future = &*(handle.0 as *mut Arc<dyn RustFutureFFI>);
    future
        .clone()
        .startup(executor_handle, callback, callback_data)
}

/// Cancel a Rust future
pub unsafe fn rust_future_cancel(handle: RustFutureHandle) {
    let future = &*(handle.0 as *mut Arc<dyn RustFutureFFI>);
    future.clone().cancel()
}

/// Drop a Rust future
///
/// This frees all resources held.   Once a [RustFutureHandle] is passed to [rust_future_free] it
/// must not be used again.
pub unsafe fn rust_future_free(handle: RustFutureHandle) {
    // Reconstruct the box and drop it
    drop(Box::<Arc<dyn RustFutureFFI>>::from_raw(
        handle.0 as *mut Arc<dyn RustFutureFFI>,
    ));
}

/// Future that the foreign code is awaiting
struct RustFuture<F, T, UT>
where
    F: Future<Output = T> + Send,
    T: FfiConverter<UT>,
{
    // Bitfield used for synchronization.  See [LockedRustFuture] for how this is used.
    state: AtomicU8,
    future: UnsafeCell<F>,
    context: UnsafeCell<Option<RustFutureContext<T, UT>>>,
}

/// Context for a Rust future.
///
/// This is the [ForeignExecutor] used to drive the future to completion alongside the callback to
/// call when it's done.
struct RustFutureContext<T: FfiConverter<UT>, UT> {
    executor: ForeignExecutor,
    callback: FutureCallback<T::FutureCallbackT>,
    callback_data: *const (),
}

// Mark [RustFuture] as [Send] + [Sync], since we will be sharing it between threads.  See
// [LockedRustFuture] for how we manage access to the [UnsafeCell] fields.

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

/// Locked RustFuture
///
/// All access to [UnsafeCell] fields and other synchronized operations go through this class.
struct LockedRustFuture<F, T, UT>
where
    F: Future<Output = T> + Send,
    T: FfiConverter<UT>,
{
    rust_future: Arc<RustFuture<F, T, UT>>,
    // Did we observe that the [RustFuture] was cancelled when we locked it?
    // Threads should check when it makes sense, but it's not required since we check the
    // [Self::STATE_CANCELLED] bit in [Self::release].
    cancelled: bool,
}

impl<F, T, UT> LockedRustFuture<F, T, UT>
where
    F: Future<Output = T> + Send,
    T: FfiConverter<UT>,
{
    // ==== Bits on the [RustFuture::state] field ====

    /// Lock bit that's set by the first call to [RustFuture::startup] and never unset.  This
    /// ensures that [RustFuture::startup] is the first method to get the lock and that it can
    /// never be blocked by other methods.
    const STATE_STARTUP_LOCK: u8 = 1 << 0;
    /// General lock bit.
    const STATE_LOCK: u8 = 1 << 1;
    /// Bit indicating that the future needs polling.
    ///
    /// Threads can set this bit while trying to acquire the lock and it's checked in
    /// [Self::release] when unsetting the lock.  This ensures that if the original thread doesn't
    /// get the lock, the thread that holds the lock will poll the future.
    const STATE_NEEDS_POLL: u8 = 1 << 2;
    /// Bit indicating that the future is cancelled.
    ///
    /// Threads can set this bit while trying to acquire the lock and it's checked in
    /// [Self::release] when unsetting the lock.  This ensures that if the original thread doesn't
    /// get the lock, the thread that holds the lock will perform the cancellation.
    const STATE_CANCELLED: u8 = 1 << 3;

    /// Attempt to get a lock for [RustFuture::startup]
    fn try_lock(rust_future: Arc<RustFuture<F, T, UT>>, extra_bits: u8) -> Option<Self> {
        let prev_state = rust_future
            .state
            .fetch_or(Self::STATE_LOCK | extra_bits, Ordering::Acquire);
        let acquired_lock = if extra_bits & Self::STATE_STARTUP_LOCK != 0 {
            // Acquire the startup lock succeeds if it's the first time the STATE_STARTUP_LOCK was
            // set.
            prev_state & Self::STATE_STARTUP_LOCK == 0
        } else {
            // Acquiring the regular lock succeeds if we set STATE_LOCK and STATE_STARTUP_LOCK was
            // already set
            prev_state & (Self::STATE_STARTUP_LOCK | Self::STATE_LOCK) == Self::STATE_STARTUP_LOCK
        };

        if acquired_lock {
            Some(Self {
                rust_future,
                cancelled: (prev_state | extra_bits) & Self::STATE_CANCELLED != 0,
            })
        } else {
            None
        }
    }

    /// Attempt to get a lock for [RustFuture::startup]
    fn try_lock_for_startup(rust_future: Arc<RustFuture<F, T, UT>>) -> Option<Self> {
        Self::try_lock(
            rust_future,
            Self::STATE_STARTUP_LOCK | Self::STATE_NEEDS_POLL,
        )
    }

    /// Attempt to get a lock for the wake method
    fn try_lock_for_wake(rust_future: Arc<RustFuture<F, T, UT>>) -> Option<Self> {
        Self::try_lock(rust_future, Self::STATE_NEEDS_POLL)
    }

    /// Attempt to get a lock for the cancel method.
    fn try_lock_for_cancel(rust_future: Arc<RustFuture<F, T, UT>>) -> Option<Self> {
        Self::try_lock(rust_future, Self::STATE_CANCELLED)
    }

    /// Get LockedRustFuture for the [RustFuture::poll_future] method.
    fn try_lock_for_poll_future(rust_future: Arc<RustFuture<F, T, UT>>) -> Option<Self> {
        // Unset the STATE_NEEDS_POLL bit since we're about to poll the future.
        let prev_state = rust_future
            .state
            .fetch_and(!Self::STATE_NEEDS_POLL, Ordering::Acquire);

        // Self::STATE_LOCKED should still be set by the thread that called
        // [Self::schedule_poll_future]
        if prev_state & (Self::STATE_STARTUP_LOCK | Self::STATE_LOCK)
            == (Self::STATE_STARTUP_LOCK | Self::STATE_LOCK)
        {
            Some(Self {
                rust_future,
                cancelled: (prev_state & Self::STATE_CANCELLED) != 0,
            })
        } else {
            log::error!("invalid state in try_lock_for_poll_future(): {prev_state:b}");
            None
        }
    }

    /// Set the [RustFuture.context] field
    fn set_context(
        &mut self,
        executor_handle: ForeignExecutorHandle,
        callback: FutureCallback<T::FutureCallbackT>,
        callback_data: *const (),
    ) -> bool {
        let context = unsafe { &mut *self.rust_future.context.get() };
        match context {
            None => {
                *context = Some(RustFutureContext {
                    executor: ForeignExecutor::new(executor_handle),
                    callback,
                    callback_data,
                });
                true
            }
            Some(_) => false,
        }
    }

    /// Schedule a [RustFuture::poll_future] call and give the lock to that call.
    fn schedule_poll_future(self) {
        // Don't release the lock, but do perform an Ordering::Release operation so that other
        // threads see all previous operations.
        self.rust_future.state.fetch_or(0, Ordering::Release);
        unsafe {
            let executor_handle = match &*self.rust_future.context.get() {
                Some(context) => context.executor.handle,
                None => {
                    // Something went very wrong for this to happen.  Log an error and give up.
                    log::error!("context not set in schedule_poll_future()");
                    return;
                }
            };

            self.rust_future.schedule_poll_future(executor_handle)
        }
    }

    fn poll_future(&mut self, context: &mut Context<'_>) -> Poll<T> {
        // SAFETY: [RustFuture.future] is pinned because:
        //    - This is the only time we get a &mut to [RustFuture::future]
        //    - We never get a &mut to RustFuture itself.
        //    - RustFuture is private to this module so no other code can get a &mut.
        unsafe { Pin::new_unchecked(&mut *self.rust_future.future.get()).poll(context) }
    }

    /// Release the lock, checking the [Self::STATE_CANCELLED] and [Self::STATE_NEEDS_POLL] bits
    fn release(self) {
        match self.rust_future.state.compare_exchange(
            // Only release the lock if the cancelled and needs poll bits are unset
            Self::STATE_STARTUP_LOCK | Self::STATE_LOCK,
            Self::STATE_STARTUP_LOCK,
            // [Ordering::Release] on success so other threads see all previous operations
            Ordering::Release,
            // [Ordering::Relaxed] on failure since we will still hold the lock
            Ordering::Relaxed,
        ) {
            Ok(_) => (),
            Err(state) if (state & Self::STATE_CANCELLED) != 0 => self.complete_for_cancel(),
            Err(state) if (state & Self::STATE_NEEDS_POLL) != 0 => self.schedule_poll_future(),
            Err(state) => log::error!("Unexpected state in LockedRustFuture::release(): {state:b}"),
        }
    }

    /// Complete the future
    ///
    /// This invokes the foreign callback and keeps the state locked so that no future progress
    /// will be attempted
    fn complete(self, result: T::ReturnType, out_status: RustCallStatus) {
        match unsafe { &*self.rust_future.context.get() } {
            Some(context) => {
                T::invoke_future_callback(
                    context.callback,
                    context.callback_data,
                    result,
                    out_status,
                );
            }
            None => log::error!("context not set in complete()"),
        }
    }

    /// Complete the future after cancellation.
    fn complete_for_cancel(self) {
        self.complete(T::ReturnType::ffi_default(), RustCallStatus::cancelled());
    }
}

impl<F, T, UT> RustFuture<F, T, UT>
where
    F: Future<Output = T> + Send,
    T: FfiConverter<UT>,
{
    fn new(future: F) -> Arc<Self> {
        Arc::new(Self {
            state: AtomicU8::new(0),
            // Rules for handling the [UnsafeCell] fields:
            //   - Only access them if you possess the lock (see [Self::STATE_LOCKED]).
            //   - Only read from them after performing a [Ordering::Acquire] operation on
            //     `self.state`
            //   - If you write to them, perform a [Ordering::Release] operation on `self.state`
            future: UnsafeCell::new(future),
            context: UnsafeCell::new(None),
        })
    }

    /// Startup a new future
    ///
    /// This starts the process to drive the future to completion, then call the foreign callback.
    /// `startup()` can only be called once.
    fn startup(
        self: Arc<Self>,
        executor_handle: ForeignExecutorHandle,
        callback: FutureCallback<T::FutureCallbackT>,
        callback_data: *const (),
    ) {
        if let Some(mut lock) = LockedRustFuture::try_lock_for_startup(self) {
            if !lock.set_context(executor_handle, callback, callback_data) {
                log::error!("context already set in startup()");
                return;
            }
            if lock.cancelled {
                lock.complete_for_cancel();
            } else {
                lock.schedule_poll_future();
            }
        }
    }

    /// Wake up and poll our future.
    fn wake(self: Arc<Self>) {
        if let Some(lock) = LockedRustFuture::try_lock_for_wake(self) {
            // Skip checking cancelled here.  It's unlikely to be cancelled yet and poll_future
            // will also check.
            lock.schedule_poll_future();
        }
    }

    fn wake_from_weak(weak: &Weak<Self>) {
        if let Some(rust_future) = weak.upgrade() {
            rust_future.wake();
        }
    }

    /// Cancel a future
    ///
    /// This causes the future to immediately invoke the callback with a
    /// [RustCallStatusCode::Cancelled] error code
    fn cancel(self: Arc<Self>) {
        if let Some(lock) = LockedRustFuture::try_lock_for_cancel(self) {
            lock.complete_for_cancel()
        }
    }

    /// Schedule `poll_future`.
    ///
    /// This is unsafe since only one [Self::poll_future] call can be scheduled at once.  All calls
    /// should go through [LockedRustFuture].
    unsafe fn schedule_poll_future(self: Arc<Self>, executor_handle: ForeignExecutorHandle) {
        let raw_ptr = Arc::into_raw(self);
        // SAFETY: The `into_raw()` / `from_raw()` contract guarantees that our executor cannot
        // be dropped before we call `from_raw()` on the raw pointer. This means we can safely
        // use its handle to schedule a callback.
        if !schedule_raw(
            executor_handle,
            0,
            RustFuture::<F, T, UT>::poll_future_callback,
            raw_ptr as *const (),
        ) {
            // There was an error scheduling the callback, drop the arc reference since
            // `wake_callback()` will never be called
            //
            // Note: specifying the `<Self>` generic is a good safety measure.  Things would go
            // very bad if Rust inferred the wrong type.
            //
            // However, the `Pin<>` part doesn't matter since its `repr(transparent)`.
            unsafe { Arc::<Self>::decrement_strong_count(raw_ptr) }
        }
    }

    extern "C" fn poll_future_callback(self_ptr: *const (), status_code: RustTaskCallbackCode) {
        // No matter what, call `Arc::from_raw()` to balance the `Arc::into_raw()` call in
        // `schedule_poll_future()`.
        let task = unsafe { Arc::from_raw(self_ptr as *const Self) };
        if status_code == RustTaskCallbackCode::Success {
            // Only drive the future forward on `RustTaskCallbackCode::Success`.
            // `RUST_TASK_CALLBACK_CANCELED` indicates the foreign executor has been cancelled /
            // shutdown and we should not continue.
            task.poll_future();
        }
    }

    // Does the work for wake, we take care to ensure this always runs in a serialized fashion.
    fn poll_future(self: Arc<Self>) {
        let waker = self.make_waker();
        if let Some(mut lock) = LockedRustFuture::try_lock_for_poll_future(self) {
            if lock.cancelled {
                lock.complete_for_cancel();
                return;
            }
            // Run the poll and lift the result if it's ready
            let mut out_status = RustCallStatus::default();
            let result: Option<Poll<T::ReturnType>> = rust_call_with_out_status(
                &mut out_status,
                // This closure uses a `&mut F` value, which means it's not UnwindSafe by default.  If
                // the closure panics, the future may be in an invalid state.
                //
                // However, we can safely use `AssertUnwindSafe` since a panic will lead the `Err()`
                // case below.  In that case, we will never run `poll_future()` again and will no longer
                // access the future.
                panic::AssertUnwindSafe(|| {
                    match lock.poll_future(&mut Context::from_waker(&waker)) {
                        Poll::Pending => Ok(Poll::Pending),
                        Poll::Ready(v) => T::lower_return(v).map(Poll::Ready),
                    }
                }),
            );

            // All the main work is done, time to finish up
            match result {
                Some(Poll::Pending) => lock.release(),
                Some(Poll::Ready(v)) => lock.complete(v, out_status),
                // Error/panic polling the future.  Call the callback with a default value.
                // `out_status` contains the error code and serialized error.
                None => lock.complete(T::ReturnType::ffi_default(), out_status),
            };
        }
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

/// RustFuture FFI trait.  This allows `Arc<RustFuture<_, _, _>>` to be cast to
/// `Arc<dyn RustFutureFFI>`, which is needed to implement the public FFI API.  In particular, this
/// allows you to use RustFuture functionality without knowing the concrete Future type, which is
/// generally un-namable.
#[doc(hidden)]
trait RustFutureFFI {
    /// Startup a [RustFuture].  `callback` must be a FfiConverter::FutureCallback pointer for the
    /// return type of the async function.
    unsafe fn startup(
        self: Arc<Self>,
        executor_handle: ForeignExecutorHandle,
        callback: FutureCallbackHandle,
        callback_data: *const (),
    );
    unsafe fn cancel(self: Arc<Self>);
}

impl<F, T, UT> RustFutureFFI for RustFuture<F, T, UT>
where
    F: Future<Output = T> + Send,
    T: FfiConverter<UT>,
{
    unsafe fn startup(
        self: Arc<Self>,
        executor_handle: ForeignExecutorHandle,
        callback: FutureCallbackHandle,
        callback_data: *const (),
    ) {
        self.startup(
            executor_handle,
            std::mem::transmute::<FutureCallbackHandle, FutureCallback<T::FutureCallbackT>>(
                callback,
            ),
            callback_data,
        );
    }

    unsafe fn cancel(self: Arc<Self>) {
        self.cancel()
    }

    // unsafe fn startup(handle: RustFutureHandle, executor_handle: ForeignExecutorHandle, callback: FutureCallbackHandle, callback_data: *const ()){
    //     let arc = Arc::from_raw(handle.0 as *const RustFuture<F, T, UT>);
    //     // Create a new Arc to run startup
    //     Arc::clone(&arc).startup(executor_handle, std::mem::transmute::<FutureCallbackHandle, FutureCallback<T::FutureCallbackT>>(callback), callback_data);
    //     // Forget the original Arc, since it represented a reference
    //     std::mem::forget(arc);
    // }
    //
    // unsafe fn cancel(handle: RustFutureHandle) {
    //     let arc = Arc::from_raw(handle.0 as *const RustFuture<F, T, UT>);
    //     // Create a new Arc to run startup
    //     Arc::clone(&arc).cancel();
    //     // Forget the original Arc, since it represented a reference
    //     std::mem::forget(arc);
    // }
    //
    // unsafe fn drop(handle: RustFutureHandle) {
    //     drop(Arc::from_raw(handle.0 as *const RustFuture<F, T, UT>))
    // }
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

        fn cancel(&self) {
            self.rust_future.clone().cancel()
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
    fn test_cancel() {
        let eventloop = MockEventLoop::new();
        let mut test_env = TestFutureEnvironment::new(&eventloop);
        test_env.startup();
        eventloop.run_all_calls();
        test_env.cancel();
        eventloop.run_all_calls();
        let result = test_env
            .foreign_result
            .take()
            .expect("Expected result to be set");
        assert_eq!(result.status.code, RustCallStatusCode::Cancelled);
        assert_eq!(eventloop.call_count(), 0);
    }

    #[test]
    fn test_cancel_before_startup() {
        let eventloop = MockEventLoop::new();
        let mut test_env = TestFutureEnvironment::new(&eventloop);
        test_env.cancel();
        eventloop.run_all_calls();
        test_env.startup();
        let result = test_env
            .foreign_result
            .take()
            .expect("Expected result to be set");
        assert_eq!(result.status.code, RustCallStatusCode::Cancelled);
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
