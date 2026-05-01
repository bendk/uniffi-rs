const UNIFFI_RUST_FUTURE_POLL_AGAIN: i32 = 0;
const UNIFFI_RUST_FUTURE_CANCELLED: i32 = 1;
const UNIFFI_RUST_FUTURE_COMPLETE: i32 = 2;
const UNIFFI_RUST_FUTURE_FAILED: i32 = 3;

/// Stores a future and scheduler for a Kotlin -> Rust call
///
/// The future should either write to the FFI buffer it inputted and return
/// `UNIFFI_RUST_FUTURE_COMPLETE` or return `UNIFFI_RUST_FUTURE_FAILED`
struct UniffiRustFuture<T> {
    scheduler: ::std::sync::Mutex<uniffi::Scheduler<UniffiRustFutureContinutation>>,
    future: ::std::sync::Mutex<::std::pin::Pin<::std::boxed::Box<dyn std::future::Future<Output = T> + ::std::marker::Send>>>,
}

impl<T> UniffiRustFuture<T> {
    fn new(future: impl ::std::future::Future<Output = T> + std::marker::Send + 'static) -> ::std::sync::Arc<Self> {
        ::std::sync::Arc::new(Self {
            scheduler: ::std::sync::Mutex::new(uniffi::Scheduler::new()),
            future: ::std::sync::Mutex::new(::std::boxed::Box::pin(future)),
        })
    }

    fn into_handle(self: ::std::sync::Arc<Self>) -> i64 {
        ::std::sync::Arc::into_raw(self).expose_provenance() as i64
    }
}

impl<T> ::std::task::Wake for UniffiRustFuture<T> {
    fn wake(self: ::std::sync::Arc<Self>) {
        self.scheduler.lock().unwrap().wake();
    }

    fn wake_by_ref(self: &::std::sync::Arc<Self>) {
        self.scheduler.lock().unwrap().wake();
    }
}

struct UniffiRustFutureContinutation {
    continuation: uniffi_jni::jobject,
}

// Safety:
// It's safe to pass `jobject` pointers to another thread
unsafe impl ::std::marker::Send for UniffiRustFutureContinutation {}

impl uniffi::RustFutureCallback for UniffiRustFutureContinutation {
    fn invoke(self, _poll: uniffi::RustFuturePoll) {
        // Note: we ignore the `poll` value.  These bindings don't use it, instead they check the
        // return value of the poll function to know when the future is ready.
        static UNIFFI_CONTINUATION_RESUME: uniffi_jni::CachedStaticMethod = uniffi_jni::CachedStaticMethod::new(
            c"uniffi/UniffiKt",
            c"uniffiContinuationResume",
            c"(Lkotlin/coroutines/Continuation;)V",
        );
        // Safety:
        //
        // * uniffi_get_global_jvm() returns a valid JavaVM pointer
        // * The args match the function signature
        unsafe {
            uniffi_jni::attach_current_thread(uniffi_get_global_jvm(), |env| {
                UNIFFI_CONTINUATION_RESUME.call_void(env, [
                    uniffi_jni::jvalue {
                        l: self.continuation,
                    },
                ]).warn_on_exception(env, "uniffiContinuationResume");
                ((**env).v1_2.DeleteGlobalRef)(env, self.continuation);
            });
        }
    }
}

{%- for rust_result in root.rust_async_callable_results() %}
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_uniffi_Scaffolding_{{ rust_result.async_poll_fn() }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    uniffi_future_handle: i64,
    continuation: uniffi_jni::jobject,
    {%- if rust_result.return_strategy().is_lowerable() %}
    completion: uniffi_jni::jobject
    {%- endif %}
) -> i32 {
    uniffi::trace!("RustFuture::poll: {uniffi_future_handle:x}");
    {%- match rust_result.return_strategy() %}
    {%- when ReturnStrategy::Primitive(_, ffi_type) %}
    static UNIFFI_COMPLETE_METHOD: uniffi_jni::CachedMethod = uniffi_jni::CachedMethod::new(
        c"uniffi/{{ rust_result.async_complete_class() }}",
        c"complete",
        c"({{ ffi_type.jni_signature() }})V",
    );
    {%- when ReturnStrategy::Reconstruct(_, ffi_types) %}
    static UNIFFI_COMPLETE_METHOD: uniffi_jni::CachedMethod = uniffi_jni::CachedMethod::new(
        c"uniffi/{{ rust_result.async_complete_class() }}",
        c"complete",
        c"({% for ffi_type in ffi_types %}{{ ffi_type.jni_signature() }}{% endfor %})V",
    );
    {%- else %}
    {%- endmatch %}
    unsafe {
        uniffi_jni::rust_call(uniffi_env, |uniffi_env| {
            // Safety:
            // We assume the Kotlin side of the FFI sent us a future handle
            let uniffi_future: ::std::sync::Arc::<UniffiRustFuture<{{ rust_result.async_rust_future_output() }}>> = unsafe {
                // Increment the strong count since we're creating a new `Arc`.
                let ptr = ::std::ptr::with_exposed_provenance::<UniffiRustFuture<{{ rust_result.async_rust_future_output() }}>>(uniffi_future_handle as usize);
                ::std::sync::Arc::increment_strong_count(ptr);
                ::std::sync::Arc::from_raw(ptr)
            };

            if uniffi_future.scheduler.lock().unwrap().is_cancelled() {
                uniffi::trace!("RustFuture::poll: cancelled");
                return Ok(UNIFFI_RUST_FUTURE_CANCELLED);
            }

            let mut locked = uniffi_future.future.lock().unwrap();
            let waker = ::std::task::Waker::from(::std::sync::Arc::clone(&uniffi_future));
            let pinned: std::pin::Pin<&mut dyn ::std::future::Future<Output = {{ rust_result.async_rust_future_output() }}>> = locked.as_mut();
            match pinned.poll(&mut ::std::task::Context::from_waker(&waker)) {
                {%- if rust_result.throws_type.is_none() %}
                ::std::task::Poll::Ready(uniffi::Result::Ok(uniffi_return)) => {
                {%- else %}
                ::std::task::Poll::Ready(uniffi::Result::Ok(uniffi::Result::Ok(uniffi_return))) => {
                {%- endif %}
                    uniffi::trace!("RustFuture::poll: ready");

                    let uniffi_return_value = || {
                        {%- match rust_result.return_strategy() %}
                        {%- when ReturnStrategy::FfiBuffer(return_type) %}
                        let (uniffi_return, mut uniffi_buf) = uniffi_return;
                        uniffi_buf.with_cursor(|uniffi_writer| {
                            {{ return_type.write_fn_rs() }}(uniffi_writer, uniffi_return)
                        })
                        {%- when ReturnStrategy::Primitive(type_node, ffi_type) %}
                        let uniffi_return_lower = {{ type_node.lower_fn_rs() }}(uniffi_env, uniffi_return)?;
                        UNIFFI_COMPLETE_METHOD.call_void(
                            uniffi_env, 
                            completion,
                            [
                                uniffi_jni::jvalue {
                                    {{ ffi_type.jvalue_field() }}: uniffi_return_lower,
                                },
                            ],
                        ).to_anyhow_result(uniffi_env, "{{ rust_result.async_complete_class() }}.complete")
                        {%- when ReturnStrategy::Reconstruct(type_node, ffi_types) %}
                        let uniffi_return_deconstructed = {{ type_node.lower_fn_rs() }}(uniffi_env, uniffi_return)?;
                        UNIFFI_COMPLETE_METHOD.call_void(
                            uniffi_env, 
                            completion,
                            [
                                {%- for ffi_type in ffi_types %}
                                uniffi_jni::jvalue {
                                    {{ ffi_type.jvalue_field() }}: uniffi_return_deconstructed.{{ loop.index0 }},
                                },
                                {%- endfor %}
                            ],
                        ).to_anyhow_result(uniffi_env, "{{ rust_result.async_complete_class() }}.complete")
                        {%- when ReturnStrategy::Void %}
                        UniffiAnyhowResult::Ok(())
                        {%- endmatch %}
                    };
                    match uniffi_return_value() {
                        Ok(v) => Ok(UNIFFI_RUST_FUTURE_COMPLETE),
                        Err(e) => {
                            eprintln!("{e}");
                            return Ok(UNIFFI_RUST_FUTURE_FAILED)
                        }
                    }
                }
                {%- if let Some(throws_type) = rust_result.throws_type %}
                ::std::task::Poll::Ready(uniffi::Result::Ok(uniffi::Result::Err(error_data))) => {
                    uniffi::trace!("RustFuture::poll: ready (error)");
                    {%- if !throws_type.uses_buffer() %}
                    // Safety:
                    // * `uniffi_env` points to a valid JNIEnv
                    unsafe {
                        {{ throws_type.throw_error_fn_rs() }}(uniffi_env, error_data);
                    };
                    {%- else %}
                    let (uniffi_err, uniffi_buf_from_caller) = error_data;
                    let (mut uniffi_buf, need_to_free_buffer) = match uniffi_buf_from_caller {
                        Some(buf) => (buf, false),
                        None => (uniffi::FfiBuffer::new(), true),
                    };
                    // Safety:
                    // * `uniffi_env` points to a valid JNIEnv
                    // * `uniffi_buf` points to a valid FFI buffer
                    unsafe { {{ throws_type.throw_error_fn_rs() }}(
                        uniffi_env,
                        {%- if throws_type.uses_buffer() %}
                        &mut uniffi_buf,
                        {%- endif %}
                        uniffi_err,
                    ); };
                    if need_to_free_buffer {
                        uniffi_buf.free()
                    }
                    {%- endif %}
                    // The return value doesn't matter, since the Kotlin code will throw once it's
                    // resumes.  Let's use UNIFFI_RUST_FUTURE_FAILED so that if that fails somehow
                    // we the async function will still fail.
                    Ok(UNIFFI_RUST_FUTURE_FAILED)
                }
                {%- endif %}
                ::std::task::Poll::Ready(uniffi::Result::Err(e)) => {
                    eprintln!("UniFFI: unexpected error in Rust future: {e}");
                    Ok(UNIFFI_RUST_FUTURE_FAILED)
                }
                ::std::task::Poll::Pending => {
                    let continuation = UniffiRustFutureContinutation {
                        continuation: ((**uniffi_env).v1_2.NewGlobalRef)(uniffi_env, continuation),
                    };
                    uniffi_future.scheduler.lock().unwrap().store(continuation);
                    Ok(UNIFFI_RUST_FUTURE_POLL_AGAIN)
                }
            }
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_uniffi_Scaffolding_{{ rust_result.async_cancel_fn() }}(
    _: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    uniffi_future_handle: i64,
) {
    uniffi::trace!("RustFuture::cancel: {uniffi_future_handle:x}");
    // Safety:
    // We assume the Kotlin side of the FFI sent us a future handle
    unsafe {
        let ptr = ::std::ptr::with_exposed_provenance::<UniffiRustFuture<{{ rust_result.async_rust_future_output() }}>>(uniffi_future_handle as usize);
        (*ptr).scheduler.lock().unwrap().cancel();
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_uniffi_Scaffolding_{{ rust_result.async_free_fn() }}(
    _: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    uniffi_future_handle: i64,
) {
    uniffi::trace!("RustFuture::free: {uniffi_future_handle:x}");
    // Safety:
    // We assume the Kotlin side of the FFI sent us a future handle
    unsafe {
        let ptr = ::std::ptr::with_exposed_provenance::<UniffiRustFuture<{{ rust_result.async_rust_future_output() }}>>(uniffi_future_handle as usize);
        ::std::sync::Arc::decrement_strong_count(ptr);
    };
}
{%- endfor %}


{%- for callback_result in root.kotlin_async_callable_results() %}
{%- let return_strategy = callback_result.return_strategy() %}
{%- let is_async = true %}
{%- let return_type = callback_result.return_type %}
{%- let throws_type = callback_result.throws_type %}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_uniffi_Scaffolding_{{ callback_result.async_complete_success_fn() }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    future_handle: i64,
    {%- match callback_result.return_strategy() %}
    {%- when ReturnStrategy::FfiBuffer(_) %}
    uniffi_buf_handle: i64,
    {%- when ReturnStrategy::Primitive(_, ffi_type) %}
    uniffi_return: {{ ffi_type.type_rs() }},
    {%- when ReturnStrategy::Reconstruct(_, ffi_types) %}
    {%- for ffi_type in ffi_types %}
    uniffi_return{{ loop.index0 }}: {{ ffi_type.type_rs() }},
    {%- endfor %}
    {%- when ReturnStrategy::Void %}
    {%- endmatch %}
) {
    uniffi::trace!("{{ callback_result.async_complete_success_fn() }}: {future_handle:x}");
    {%- if callback_result.return_strategy().is_ffi_buffer() %}
    let mut uniffi_buf = uniffi::FfiBuffer::from_ptr(
        ::std::ptr::with_exposed_provenance_mut(uniffi_buf_handle as usize)
    );
    {%- endif %}
    // Safety:
    // * uniffi_env points to a valid JniEnv
    // * We assume the Kotlin side sent us valid future/buffer handles
    unsafe {
        let sender = uniffi::oneshot::Sender::<{{ callback_result.async_oneshot_type() }}>::from_raw(
            ::std::ptr::with_exposed_provenance::<_>(future_handle as usize)
        );
        let mut return_result = || {
            {%- filter indent(12) %}{% include "lift_return.rs" %}{% endfilter %}
            {%- if throws_type.is_some() %}
            return ::std::result::Result::Ok(::std::result::Result::Ok(uniffi_return));
            {%- else %}
            return ::std::result::Result::Ok(uniffi_return);
            {%- endif %}
        };
        sender.send(return_result());
    }
}

{%- if let Some(throws_type) = throws_type %}
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_uniffi_Scaffolding_{{ callback_result.async_complete_error_fn() }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    future_handle: i64,
    {%- match throws_type.lowerable %}
    {%- when Some(LowerableType::Primitive(ffi_type)) %}
    error: {{ ffi_type.type_rs() }}
    {%- when Some(LowerableType::Deconstructable(ffi_types)) %}
    {%- for ffi_type in ffi_types %}
    error_v{{ loop.index0 }}: {{ ffi_type.type_rs() }},
    {%- endfor %}
    {%- when None %}
    uniffi_buf_handle: i64,
    {%- endmatch %}
) {
    uniffi::trace!("{{ callback_result.async_complete_error_fn() }}: {future_handle:x}");
    {%- if throws_type.uses_buffer() %}
    let mut uniffi_buf = uniffi::FfiBuffer::from_ptr(
        ::std::ptr::with_exposed_provenance_mut(uniffi_buf_handle as usize)
    );
    {%- endif %}
    // Safety:
    // * uniffi_env points to a valid JniEnv
    // * We assume the Kotlin side sent us valid future/buffer handles
    unsafe {
        let sender = uniffi::oneshot::Sender::<{{ callback_result.async_oneshot_type() }}>::from_raw(
            ::std::ptr::with_exposed_provenance::<_>(future_handle as usize)
        );
        let mut return_err = || {
            {%- match throws_type.lowerable %}
            {%- when Some(LowerableType::Primitive(_)) %}
            return ::std::result::Result::Ok(::std::result::Result::Err({{ throws_type.lift_fn_rs() }}(uniffi_env, error)?));
            {%- when Some(LowerableType::Deconstructable(ffi_types)) %}
            return  ::std::result::Result::Ok(::std::result::Result::Err(
                {{ throws_type.lift_fn_rs() }}(
                    uniffi_env,
                    {%- for _ in ffi_types %}
                    error_v{{ loop.index0 }},
                    {%- endfor %}
                )?
            ));
            {%- when None %}
            return ::std::result::Result::Ok(::std::result::Result::Err(uniffi_buf.with_cursor(|uniffi_reader| {
                {{ throws_type.read_fn_rs() }}(uniffi_reader)
            })?));
            {%- endmatch %}
        };
        sender.send(return_err());
    }
}
{%- endif %}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_uniffi_Scaffolding_{{ callback_result.async_complete_unexpected_error_fn() }}(
    _: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    future_handle: i64,
) {
    uniffi::trace!("{{ callback_result.async_complete_unexpected_error_fn() }}: {future_handle:x}");
    // Safety:
    // * We assume the Kotlin side sent us valid future handles
    let sender = unsafe {
        uniffi::oneshot::Sender::<{{ callback_result.async_oneshot_type() }}>::from_raw(
            ::std::ptr::with_exposed_provenance::<_>(future_handle as usize)
        )
    };
    sender.send(::std::result::Result::Err(uniffi::deps::anyhow::anyhow!("Unexpected callback error")));
}

{%- endfor %}
