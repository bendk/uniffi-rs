{%- for type_node in root.rust_throws_types() %}
/// Throw a `{{ type_node.type_kt }}` instance
///
/// # Safety
/// `ffi_buffer` must point to a valid FFI buffer
unsafe fn {{ type_node.throw_error_fn_rs() }}(
    env: *mut uniffi_jni::JNIEnv,
    uniffi_buf: &mut uniffi::FfiBuffer,
    uniffi_err: {{ type_node.type_rs }},
) -> uniffi::Result<()> {
    static METHOD: uniffi_jni::CachedStaticMethod = uniffi_jni::CachedStaticMethod::new(
        c"uniffi/UniffiKt",
        c"{{ type_node.construct_fn_kt() }}",
        c"(J){{ type_node.jni_signature() }}",
    );
    uniffi_buf.with_cursor(|uniffi_writer| {
        {{ type_node.write_fn_rs() }}(uniffi_writer, uniffi_err)
    })?;
    // Safety:
    // We're using the JNI API correctly.
    unsafe {
        // Exceptions are expected, they'll be thrown when the native method returns.
        let throwable = METHOD.call_object(env, [
            uniffi_jni::jvalue {
                j: uniffi_buf.as_ptr().expose_provenance() as i64,
            }
        ]).to_anyhow_result(env, "{{ type_node.construct_fn_kt() }}")?;
        ((**env).v1_2.Throw)(env, throwable);
    }
    Ok(())
}

{%- endfor %}
