{%- for type_node in root.rust_throws_types() %}
/// Throw a `{{ type_node.type_kt }}` instance
///
/// # Safety
/// `ffi_buffer` must point to a valid FFI buffer
unsafe fn {{ type_node.throw_error_fn_rs() }}(
    env: *mut uniffi_jni::JNIEnv,
    ffi_buffer: *mut u8,
) -> uniffi::Result<()> {
    static METHOD: uniffi_jni::CachedStaticMethod = uniffi_jni::CachedStaticMethod::new(
        c"uniffi/UniffiKt",
        c"{{ type_node.construct_fn_kt() }}",
        c"(J){{ type_node.jni_signature() }}",
    );
    // Safety:
    // We're using the JNI API correctly.
    unsafe {
        // Exceptions are expected, they'll be thrown when the native method returns.
        let throwable = METHOD.call_object(env, [
            uniffi_jni::jvalue {
                j: ffi_buffer.expose_provenance() as i64,
            }
        ]).to_anyhow_result(env, "{{ type_node.construct_fn_kt() }}")?;
        ((**env).v1_2.Throw)(env, throwable);
    }
    Ok(())
}

{%- endfor %}
