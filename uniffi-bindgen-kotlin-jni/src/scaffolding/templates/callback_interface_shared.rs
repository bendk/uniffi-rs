{%- for callback_result in root.kotlin_sync_callable_results() %}
{%- if let Some(return_type) = callback_result.return_type %}
{%- if let Some(LowerableType::Deconstructable(ffi_types)) = return_type.lowerable %}
// Return FN for {{ callback_result.id }}
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_uniffi_Scaffolding_{{ callback_result.set_callback_return_fn_kt() }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    result_handle: i64,
    {%- for ffi_type in ffi_types %}
    v{{ loop.index0 }}: {{ ffi_type.type_rs() }},
    {%- endfor %}
) {
    let result_pointer = ::std::ptr::with_exposed_provenance_mut::<::std::option::Option<{{ callback_result.return_type_rs() }}>>(result_handle as usize);
    match {{ return_type.lift_fn_rs() }}(
        uniffi_env,
        {%- for _ in ffi_types %}
        v{{ loop.index0 }},
        {%- endfor %}
    ) {
        Ok(return_value) => {
            // Safety:
            // We assume the Kotlin side of the FFI passed us a valid pointer
            unsafe {
                {%- if callback_result.throws_type.is_none() %}
                result_pointer.write(::std::option::Option::Some(return_value));
                {%- else %}
                result_pointer.write(::std::option::Option::Some(::std::result::Result::Ok(return_value)));
                {%- endif %}
            }
        }
        Err(e) => {
            // Safety:
            // uniffi_env points to a valid JNIEnv
            unsafe {
                uniffi_jni::throw_internal_exception(uniffi_env, format!("{{ callback_result.set_callback_return_fn_kt() }} failed: {e}").into());
            }
        }
    }
}

{%- endif %}

{%- if let Some(throws_type) = callback_result.throws_type %}
{%- match throws_type.lowerable %}
{%- when Some(LowerableType::Primitive(ffi_type)) %}
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_uniffi_Scaffolding_{{ callback_result.set_callback_err_fn_kt() }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    result_handle: i64,
    error_value: {{ ffi_type.type_rs() }},
) {
    let result_pointer = ::std::ptr::with_exposed_provenance_mut::<::std::option::Option<{{ callback_result.return_type_rs() }}>>(result_handle as usize);
    match {{ throws_type.lift_fn_rs() }}(uniffi_env, error_value) {
        Ok(return_value) => {
            result_pointer.write(::std::option::Option::Some(::std::result::Result::Err(return_value)));
        }
        Err(e) => {
            // Safety:
            // uniffi_env points to a valid JNIEnv
            unsafe {
                uniffi_jni::throw_internal_exception(uniffi_env, format!("{{ callback_result.set_callback_err_fn_kt() }} failed: {e}").into());
            }
        }
    }
}

{%- when Some(LowerableType::Deconstructable(ffi_types)) %}
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_uniffi_Scaffolding_{{ callback_result.set_callback_err_fn_kt() }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    result_handle: i64,
    {%- for ffi_type in ffi_types %}
    v{{ loop.index0 }}: {{ ffi_type.type_rs() }},
    {%- endfor %}
) {
    let result_pointer = ::std::ptr::with_exposed_provenance_mut::<::std::option::Option<{{ callback_result.return_type_rs() }}>>(result_handle as usize);
    match {{ throws_type.lift_fn_rs() }}(
        uniffi_env,
        {%- for ffi_type in ffi_types %}
        v{{ loop.index0 }},
        {%- endfor %}
    ) {
        Ok(return_value) => {
            result_pointer.write(::std::option::Option::Some(::std::result::Result::Err(return_value)));
        }
        Err(e) => {
            // Safety:
            // uniffi_env points to a valid JNIEnv
            unsafe {
                uniffi_jni::throw_internal_exception(uniffi_env, format!("{{ callback_result.set_callback_err_fn_kt() }} failed: {e}").into());
            }
        }
    }
}

{%- when None %}
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_uniffi_Scaffolding_{{ callback_result.set_callback_err_fn_kt() }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    result_handle: i64,
    buf_handle: i64,
) {
    let result_pointer = ::std::ptr::with_exposed_provenance_mut::<::std::option::Option<{{ callback_result.return_type_rs() }}>>(result_handle as usize);
    let mut uniffi_buf = uniffi::FfiBuffer::from_ptr(
        ::std::ptr::with_exposed_provenance_mut(buf_handle as usize)
    );
    match uniffi_buf.with_cursor({{ throws_type.read_fn_rs() }}) {
        Ok(return_value) => {
            result_pointer.write(::std::option::Option::Some(::std::result::Result::Err(return_value)));
        }
        Err(e) => {
            // Safety:
            // uniffi_env points to a valid JNIEnv
            unsafe {
                uniffi_jni::throw_internal_exception(uniffi_env, format!("{{ callback_result.set_callback_err_fn_kt() }} failed: {e}").into());
            }
        }
    }
}
{%- endmatch %}
{%- endif %}

{%- endif %}
{%- endfor %}
