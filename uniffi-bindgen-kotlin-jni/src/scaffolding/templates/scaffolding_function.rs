{%- let callable = scaffolding_function.callable %}
{%- if !callable.is_async %}
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_uniffi_Scaffolding_{{ scaffolding_function.jni_method_name }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    {%- if callable.uses_buffer() %}uniffi_buf_handle: i64,{% endif %}
) {
    uniffi::trace!("Calling {{ callable.name }}");
    // Safety:
    // * uniffi_env points to a valid JNIEnv
    // * We assume the Kotlin side of the FFI sent us a valid buffer handle with arguments
    //   correctly serialized.
    unsafe {
        uniffi_jni::rust_call(uniffi_env, |uniffi_env| {
            {%- if callable.uses_buffer() %}
            let mut uniffi_buf = uniffi::FfiBuffer::from_ptr(
                ::std::ptr::with_exposed_provenance_mut(uniffi_buf_handle as usize)
            );
            {% endif %}
            {% filter indent(12) %}{% include "lift_args.rs" %}{% endfilter %}
 
            {%- match scaffolding_function.kind %}
            {%- when ScaffoldingFunctionKind::Method %}
            let uniffi_return_value = uniffi_self.{{ callable.name_rs() }}(
                {%- for arg in callable.arguments %}
                {{ arg.pass_to_rust_fn() }},
                {%- endfor %}
            );
            {%- when ScaffoldingFunctionKind::Function %}
            let uniffi_return_value = {{ callable.fully_qualified_name_rs }}(
                {%- for arg in callable.arguments %}
                {{ arg.pass_to_rust_fn() }},
                {%- endfor %}
            );
            {%- when ScaffoldingFunctionKind::TraitMethodDisplayFmt %}
            {%- let self_type = callable.receiver_type().unwrap().type_rs %}
            uniffi::deps::static_assertions::assert_impl_all!({{ self_type }}: ::std::fmt::Display);
            let uniffi_return_value = format!("{uniffi_self}");
            {%- when ScaffoldingFunctionKind::TraitMethodDebugFmt %}
            {%- let self_type = callable.receiver_type().unwrap().type_rs %}
            uniffi::deps::static_assertions::assert_impl_all!({{ self_type }}: ::std::fmt::Debug);
            let uniffi_return_value = format!("{uniffi_self:?}");
            {%- when ScaffoldingFunctionKind::TraitMethodEqEq %}
            {%- let self_type = callable.receiver_type().unwrap().type_rs %}
            uniffi::deps::static_assertions::assert_impl_all!({{ self_type }}: ::std::cmp::PartialEq);
            let uniffi_return_value = ::std::cmp::PartialEq::eq(&uniffi_self, &other);
            {%- when ScaffoldingFunctionKind::TraitMethodEqNe %}
            {%- let self_type = callable.receiver_type().unwrap().type_rs %}
            uniffi::deps::static_assertions::assert_impl_all!({{ self_type }}: ::std::cmp::PartialEq);
            let uniffi_return_value = ::std::cmp::PartialEq::ne(&uniffi_self, &other);
            {%- when ScaffoldingFunctionKind::TraitMethodHashHash %}
            {%- let self_type = callable.receiver_type().unwrap().type_rs %}
            uniffi::deps::static_assertions::assert_impl_all!({{ self_type }}: ::std::hash::Hash);
            let mut uniffi_hasher = ::std::collections::hash_map::DefaultHasher::new();
            ::std::hash::Hash::hash(&uniffi_self, &mut uniffi_hasher);
            let uniffi_return_value =  ::std::hash::Hasher::finish(&uniffi_hasher);
            {%- when ScaffoldingFunctionKind::TraitMethodOrdCmp %}
            {%- let self_type = callable.receiver_type().unwrap().type_rs %}
            uniffi::deps::static_assertions::assert_impl_all!({{ self_type }}: ::std::cmp::Ord);
            let uniffi_return_value = ::std::cmp::Ord::cmp(&uniffi_self, &other) as i8;
            {%- endmatch %}

            {%- if let Some(throws_ty) = callable.throws_type() %}
            let uniffi_return_value = match uniffi_return_value {
                Ok(v) => v,
                Err(uniffi_err) => {
                    {%- if !callable.uses_buffer() %}
                    // Need to allocate a new buffer for the exception since we didn't input one
                    let mut uniffi_buf = uniffi::FfiBuffer::new();
                    {% endif %}
                    uniffi_buf.with_cursor(|uniffi_writer| {
                        {{ throws_ty.write_fn_rs() }}(uniffi_writer, uniffi_err)
                    })?;
                    // Safety:
                    // `uniffi_buf` points to a valid FFI buffer
                    unsafe { {{ throws_ty.throw_error_fn_rs() }}(uniffi_env, uniffi_buf.into_ptr())?; };
                    {%- if !callable.uses_buffer() %}
                    uniffi_buf.free();
                    {% endif %}
                    return Ok(::std::default::Default::default());
                }
            };
            {%- endif %}

            {%- if let Some(return_ty) = callable.return_type() %}
            uniffi_buf.with_cursor(|uniffi_writer| {
                {{ return_ty.write_fn_rs() }}(uniffi_writer, uniffi_return_value)
            })?;
            {%- endif %}
            return Ok(());
        })
    }
}
{% else %}
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_uniffi_Scaffolding_{{ scaffolding_function.jni_method_name }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    {%- if callable.uses_buffer() %}uniffi_buf_handle: i64,{% endif %}
) -> i64 {
    uniffi::trace!("Calling {{ callable.name }}");
    // Safety:
    // * uniffi_env points to a valid JNIEnv
    // * We assume the Kotlin side of the FFI sent us a valid buffer handle with arguments
    //   correctly serialized.
    unsafe {
        uniffi_jni::rust_call(uniffi_env, |uniffi_env| {
            {%- if callable.uses_buffer() %}
            let mut uniffi_buf = uniffi::FfiBuffer::from_ptr(
                ::std::ptr::with_exposed_provenance_mut(uniffi_buf_handle as usize)
            );
            {% endif %}
            {% filter indent(16) %}{% include "lift_args.rs" %}{% endfilter %}
            let uniffi_future = async move {
                {%- if callable.has_receiver() %}
                let uniffi_return_value = uniffi_self.{{ callable.name_rs() }}(
                    {%- for arg in callable.arguments %}
                    {{ arg.name_rs() }},
                    {%- endfor %}
                ).await;
                {%- else %}
                let uniffi_return_value = {{ callable.fully_qualified_name_rs }}(
                    {%- for arg in callable.arguments %}
                    {{ arg.name_rs() }},
                    {%- endfor %}
                ).await;
                {%- endif %}

                {%- if let Some(throws_ty) = callable.throws_type() %}
                let uniffi_return_value = match uniffi_return_value {
                    Ok(v) => v,
                    Err(uniffi_error) => {
                        {%- if !callable.uses_buffer() %}
                        // Need to allocate a new buffer for the exception since we didn't input one
                        let mut uniffi_buf = uniffi::FfiBuffer::new();
                        {%- endif %}
                        uniffi_buf.with_cursor(|uniffi_writer| {
                            {{ throws_ty.write_fn_rs() }}(uniffi_writer, uniffi_error)
                        })?;
                        return UniffiAnyhowResult::Ok(uniffi_jni::RustFutureResult::Err {
                            throw_fn: {{ throws_ty.throw_error_fn_rs() }},
                            buf: uniffi_buf,
                            rust_frees_buf: {{ !callable.uses_buffer() }},
                        })
                    }
                };
                {%- endif %}

                {%- if let Some(return_ty) = callable.return_type() %}
                uniffi_buf.with_cursor(|uniffi_writer| {
                    {{ return_ty.write_fn_rs() }}(uniffi_writer, uniffi_return_value)
                })?;
                {%- endif %}
                UniffiAnyhowResult::Ok(uniffi_jni::RustFutureResult::Ok)
            };
            Ok(UniffiRustFuture::new(async move {
                match uniffi_future.await {
                    Ok(result) => result,
                    Err(e) => {
                        eprintln!("Error in Rust future: {e}");
                        uniffi_jni::RustFutureResult::UnexpectedError
                    }
                }
            }).into_handle())
        })
    }
}
{% endif %}
