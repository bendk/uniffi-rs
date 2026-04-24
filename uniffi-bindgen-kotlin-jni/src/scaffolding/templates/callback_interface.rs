{%- let type_name = cbi.self_type.type_rs %}
{%- let trait_name = "{}::{}"|format(cbi.module_path, cbi.name_rs()) %}

struct {{ cbi.impl_struct_rs() }} {
    handle: i64,
}

{% if cbi.has_async_method() %}#[uniffi::deps::async_trait::async_trait]{% endif %}
impl {{ trait_name }} for {{ cbi.impl_struct_rs() }} {
    {%- for meth in cbi.methods %}
    {%- let callable = meth.callable %}
    {%- let return_type = callable.return_type() %}
    {%- let throws_type = callable.throws_type() %}
    {% if callable.is_async %}async {% endif %}fn {{ callable.name_rs() }}(
        &self,
        {%- for a in callable.arguments %}
        {{ a.name_rs() }}: {{ a.ty.type_rs }},
        {%- endfor %}
    ) -> {{ callable.result.return_type_rs() }} {
        uniffi::trace!("Callback call: {{ callable.name }}");
        {%- if callable.uses_buffer() %}
        let mut uniffi_buf = uniffi::FfiBuffer::new();
        uniffi::trace!("{{ callable.name }}: new buffer {uniffi_buf:?}");
        {%- endif %}
        let uniffi_result = {{ meth.dispatch_fn_rs }}(
            self.handle,
            {%- if callable.uses_buffer() %}
            &mut uniffi_buf,
            {%- endif %}
            {%- for a in callable.arguments %}
            {{ a.name_rs() }},
            {%- endfor %}
        ){% if callable.is_async %}.await{% endif %};
        {%- if callable.uses_buffer() %}
        uniffi::trace!("{{ callable.name }}: free buffer {uniffi_buf:?}");
        uniffi_buf.free();
        {%- endif %}
        match uniffi_result {
            Ok(v) => v,
            Err(e) => {
                {%- if let Some(throws_type) = callable.throws_type() %}
                {%- if throws_type.has_from_unexpected_callback_error_impl %}
                Err(<{{ throws_type.type_rs }} as ::std::convert::From<uniffi::UnexpectedUniFFICallbackError>>::from(
                    uniffi::UnexpectedUniFFICallbackError {
                        reason: e.to_string(),
                    }
                ))
                {%- else %}
                panic!("Error calling UniFFI callback method: {e}")
                {%- endif %}
                {%- else %}
                panic!("Error calling UniFFI callback method: {e}")
                {%- endif %}
            }
        }
    }
    {%- endfor %}
}

{%- for meth in cbi.methods %}
{%- let callable = meth.callable %}
{%- let return_strategy = callable.return_strategy() %}
{%- let return_type = callable.return_type() %}
{%- let throws_type = callable.throws_type() %}
// Dispatch function for the {{ cbi.name }}::{{ callable.name }}
//
// Ok returns represent a regular call
// Err returns represent an unexpected error, for example failure to lift arguments
{% if callable.is_async %}async {% endif %}fn {{ meth.dispatch_fn_rs }}(
    uniffi_callback_handle: i64,
    {%- if callable.uses_buffer() %}
    uniffi_buf: &mut uniffi::FfiBuffer,
    {%- endif %}
    {%- for a in callable.arguments %}
    {{ a.name_rs() }}: {{ a.ty.type_rs }},
    {%- endfor %}
) -> uniffi::Result<{{ callable.result.return_type_rs() }}> {
    static METHOD: uniffi_jni::CachedStaticMethod = uniffi_jni::CachedStaticMethod::new(
        c"uniffi/UniffiKt",
        c"{{ meth.dispatch_fn_kt }}",
        c"{{ meth.jni_signature }}",
    );
    {%- if !callable.is_async %}
    // Safety:
    //
    // * uniffi_get_global_jvm() returns a valid JavaVM pointer
    // * We use the JNI API correctly
    unsafe {
        uniffi_jni::attach_current_thread(uniffi_get_global_jvm(), |uniffi_env| {
            {% filter indent(12) %}{% include "lower_args.rs" %}{% endfilter %}
            let uniffi_result = METHOD.{{ meth.jni_method_call_name }}(uniffi_env, [
                uniffi_jni::jvalue {
                    j: uniffi_callback_handle,
                },
                {%- if callable.uses_buffer() %}
                uniffi_jni::jvalue {
                    j: uniffi_buf.as_ptr().expose_provenance() as i64,
                },
                {%- endif %}
                {%- for ffi_arg in callable.ffi_arguments() %}
                uniffi_jni::jvalue {
                    {{ ffi_arg.ty.jvalue_field() }}: {{ ffi_arg.name_rs() }},
                },
                {%- endfor %}
            ]);

            match uniffi_result {
                Ok(uniffi_return) => {
                    // Callback returned normally, read the return value
                    {% filter indent(20) %}{% include "lift_return.rs" %}{% endfilter %}
                }
                Err(uniffi_exc) => {
                    ((**uniffi_env).v1_2.ExceptionClear)(uniffi_env);
                    {%- if let Some(throws_type) = callable.throws_type() %}
                    if uniffi_jni::is_callback_exception(uniffi_env, uniffi_exc) {
                        // Handle the case the callback throwing a `uniffi.CallbackException` error
                        //
                        // In this case we need to read the E side of the Result from the FFI buffer
                        let uniffi_err = uniffi_buf.with_cursor(|uniffi_reader| {
                            {{ throws_type.read_fn_rs() }}(uniffi_reader)
                        })?;
                        return Ok(Err(uniffi_err));
                    }
                    {%- endif %}
                    Err(uniffi::deps::anyhow::anyhow!("{}", uniffi_jni::throwable_get_message(uniffi_env, uniffi_exc)))
                }
            }
        })
    }

    {%- else %}
    let (uniffi_sender, uniffi_receiver) = uniffi::oneshot::channel::<{{ callable.result.async_oneshot_type() }}>();
    // Safety:
    // * uniffi_get_global_jvm() returns a valid JavaVM pointer
    // * We use the JNI API correctly
    // * Closure panics won't cause `uniffi_buf` to be invalid
    // * We don't use the buffer while the Kotlin side has it
     unsafe {
        {%- if callable.uses_buffer() %}
        let mut uniffi_buf = ::std::panic::AssertUnwindSafe(uniffi_buf);
        {%- endif %}
        uniffi_jni::attach_current_thread(uniffi_get_global_jvm(), move |uniffi_env| {
            {% filter indent(12) %}{% include "lower_args.rs" %}{% endfilter %}
            METHOD.call_void(uniffi_env, [
                uniffi_jni::jvalue {
                    j: uniffi_callback_handle,
                },
                uniffi_jni::jvalue {
                    j: uniffi_sender.into_raw().expose_provenance() as i64,
                },
                {%- if callable.uses_buffer() %}
                uniffi_jni::jvalue {
                    j: uniffi_buf.as_ptr().expose_provenance() as i64
                },
                {%- endif %}
                {%- for ffi_arg in callable.ffi_arguments() %}
                uniffi_jni::jvalue {
                    {{ ffi_arg.ty.jvalue_field() }}: {{ ffi_arg.name_rs() }},
                },
                {%- endfor %}
            ]).map_err(|_| {
                ((**uniffi_env).v1_2.ExceptionClear)(uniffi_env);
                uniffi::deps::anyhow::anyhow!("Exception calling {{ meth.dispatch_fn_kt }}")
            }).map(|_| {
                // Return `uniffi_buf` back so that we can continue to use it in the code below.
                // This allows us to continue to use the `&mut` after "moving" it into AssertUnwindSafe
                uniffi_buf
            })
        })?
    };
    uniffi_receiver.await
    {%- endif %}
}
{%- endfor %}

impl Drop for {{ cbi.impl_struct_rs() }} {
    fn drop(&mut self) {
        static METHOD: uniffi_jni::CachedStaticMethod = uniffi_jni::CachedStaticMethod::new(
            c"uniffi/UniffiKt",
            c"{{ cbi.free_fn_kt() }}",
            c"(J)V",
        );

        // Safety:
        //
        // * uniffi_get_global_jvm() returns a valid JavaVM pointer
        // * The arguments match the method signature
        unsafe {
            uniffi_jni::attach_current_thread(uniffi_get_global_jvm(), |env| {
                if METHOD.call_void(env, [
                    uniffi_jni::jvalue {
                        j: self.handle,
                    }
                ]).is_err() {
                    ((**env).v1_2.ExceptionClear)(env);
                    eprintln!("Exception calling {{ cbi.free_fn_kt() }}");
                }
            });
        }
    }
}

{% if !cbi.for_trait_interface %}
/// Read a {{ type_name }} from a `FfiBufferCursor`
pub fn {{ cbi.self_type.read_fn_rs() }}(
    cursor: &mut uniffi::FfiBufferCursor,
) -> uniffi::Result<{{ type_name }}> {
    let handle = cursor.read_i64()?;
    Ok(::std::boxed::Box::new({{ cbi.impl_struct_rs() }} {
        handle
    }))
}

// Note: no write function, since passing callback interfaces from Rust to Kotlin is not allowed

{%- endif %}
