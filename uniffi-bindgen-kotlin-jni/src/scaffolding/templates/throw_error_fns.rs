{%- for type_node in root.rust_throws_types() %}

/// Throw a `{{ type_node.type_kt }}` instance
///
/// # Safety
/// `ffi_buffer` must point to a valid FFI buffer
unsafe fn {{ type_node.throw_error_fn_rs() }}(
    env: *mut uniffi_jni::JNIEnv,
    {%- if type_node.uses_buffer() %}
    uniffi_buf: &mut uniffi::FfiBuffer,
    {%- endif %}
    uniffi_err: {{ type_node.type_rs }},
) -> uniffi::Result<()> {
    static METHOD: uniffi_jni::CachedStaticMethod = uniffi_jni::CachedStaticMethod::new(
        c"uniffi/UniffiKt",
        c"{{ type_node.lift_fn_kt() }}",
        c"{{ type_node.lift_fn_jni_signature() }}",
    );
    // Safety:
    // We're using the JNI API correctly.
    unsafe {
        {%- match type_node.lowerable %}
        {%- when None %}
        uniffi_buf.with_cursor(|uniffi_writer| {
            {{ type_node.write_fn_rs() }}(uniffi_writer, uniffi_err)
        })?;
        {%- when Some(LowerableType::Deconstructable(_)) %}
        let uniffi_lowered = {{ type_node.lower_fn_rs() }}(env, uniffi_err)?;
        {%- when Some(LowerableType::Primitive(_)) %}
        let uniffi_lowered = {{ type_node.lower_fn_rs() }}(env, uniffi_err)?;
        {%- endmatch %}

        // Exceptions are expected, they'll be thrown when the native method returns.
        let throwable = METHOD.call_object(env, [
            {%- match type_node.lowerable %}
            {%- when None %}
            uniffi_jni::jvalue {
                l: ((**env).v1_4.NewDirectByteBuffer)(
                    env,
                    uniffi_buf.as_ptr().cast(),
                    uniffi::BASE_MINI_BUFFER_SIZE as i64,
                ),
            },
            {%- when Some(LowerableType::Deconstructable(ffi_types)) %}
            {%- for ffi_type in ffi_types %}
            uniffi_jni::jvalue {
                {{ ffi_type.jvalue_field() }}: uniffi_lowered.{{ loop.index0 }},
            },
            {%- endfor %}
            {%- when Some(LowerableType::Primitive(ffi_type)) %}
            uniffi_jni::jvalue {
                {{ ffi_type.jvalue_field() }}: uniffi_lowered,
            },
            {%- endmatch %}
        ]).to_anyhow_result(env, "{{ type_node.lift_fn_kt() }}")?;
        ((**env).v1_2.Throw)(env, throwable);
    }
    Ok(())
}

{%- endfor %}
