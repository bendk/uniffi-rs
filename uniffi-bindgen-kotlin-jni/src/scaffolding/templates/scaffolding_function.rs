{%- let callable = scaffolding_function.callable %}
{%- let return_type = callable.return_type() %}
{%- let throws_type = callable.throws_type() %}
{%- if !callable.is_async %}
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_uniffi_Scaffolding_{{ scaffolding_function.jni_method_name }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    {%- if callable.uses_buffer() %}uniffi_buf_handle: i64,{% endif %}
    {%- for ffi_arg in callable.ffi_arguments_including_receiver() %}
    {{ ffi_arg.name_rs() }}: {{ ffi_arg.ty.type_rs() }},
    {%- endfor %}
)
{%- match callable.return_strategy() %}
{%- when ReturnStrategy::Primitive(_, ffi_type) %} -> {{ ffi_type.type_rs() }}
{%- when ReturnStrategy::Reconstruct(_, _) %} -> uniffi_jni::jobject
{%- else %}
{%- endmatch %}
{
    uniffi::trace!("Calling {{ callable.name }}");
    // Safety:
    // * uniffi_env points to a valid JniEnv
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
                    {%- if throws_ty.uses_buffer() && callable.uses_buffer() %}
                    // Safety:
                    // * uniffi_env is a valid JNIEnv
                    // * `uniffi_buf` points to a valid FFI buffer
                    unsafe { {{ throws_ty.throw_error_fn_rs() }}(uniffi_env, &mut uniffi_buf, uniffi_err)?; };
                    {%- elif throws_ty.uses_buffer() && !callable.uses_buffer() %}
                    // Need to allocate a new buffer for the exception since we didn't input one
                    let mut uniffi_buf = uniffi::FfiBuffer::new();
                    // Safety:
                    // * uniffi_env is a valid JNIEnv
                    // * `uniffi_buf` points to a valid FFI buffer
                    unsafe { {{ throws_ty.throw_error_fn_rs() }}(uniffi_env, &mut uniffi_buf, uniffi_err)?; };
                    uniffi_buf.free();
                    {%- else %}
                    // Safety:
                    // * uniffi_env is a valid JNIEnv
                    unsafe { {{ throws_ty.throw_error_fn_rs() }}(uniffi_env, uniffi_err)?; };
                    {%- endif %}

                    return Ok(::std::default::Default::default());
                }
            };
            {%- endif %}

            {%- match callable.return_strategy() %}
            {%- when ReturnStrategy::FfiBuffer(return_type) %}
            uniffi_buf.with_cursor(|uniffi_writer| {
                {{ return_type.write_fn_rs() }}(uniffi_writer, uniffi_return_value)
            })?;
            Ok(())
            {%- when ReturnStrategy::Primitive(type_node, _) %}
            {{ type_node.lower_fn_rs() }}(uniffi_env, uniffi_return_value)
            {%- when ReturnStrategy::Reconstruct(type_node, ffi_types) %}
            let uniffi_return_deconstructed = {{ type_node.lower_fn_rs() }}(uniffi_env, uniffi_return_value)?;
            {{ type_node.lift_kt_from_rust_var() }}.call_object(
                uniffi_env,
                [
                    {%- for ffi_type in ffi_types %}
                    uniffi_jni::jvalue {
                        {{ ffi_type.jvalue_field() }}: uniffi_return_deconstructed.{{ loop.index0 }},
                    },
                    {%- endfor %}
                ]
            ).to_anyhow_result(uniffi_env, "{{ type_node.lift_fn_kt() }}")
            {%- when ReturnStrategy::Void %}
            Ok(())
            {%- endmatch %}
        })
    }
}
{% else %}
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_uniffi_Scaffolding_{{ scaffolding_function.jni_method_name }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    {%- if callable.uses_buffer() %}uniffi_buf_handle: i64,{% endif %}
    {%- for ffi_arg in callable.ffi_arguments_including_receiver() %}
    {{ ffi_arg.name_rs() }}: {{ ffi_arg.ty.type_rs() }},
    {%- endfor %}
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
                    Err(uniffi_err) => {
                        return UniffiAnyhowResult::Ok(::std::result::Result::Err((
                            uniffi_err, 
                            {%- if throws_ty.uses_buffer() && callable.uses_buffer() %}
                            ::std::option::Option::Some(uniffi_buf)
                            {%- elif throws_ty.uses_buffer() && !callable.uses_buffer() %}
                            ::std::option::Option::<uniffi::FfiBuffer>::None,
                            {%- endif %}
                        )));
                    }
                };
                {%- endif %}

                {%- if callable.return_strategy().is_ffi_buffer() %}
                let uniffi_return_value = (uniffi_return_value, uniffi_buf);
                {%- endif %}
                {%- if callable.throws_type().is_none() %}
                UniffiAnyhowResult::Ok(uniffi_return_value)
                {%- else %}
                UniffiAnyhowResult::Ok(::std::result::Result::Ok(uniffi_return_value))
                {%- endif %}
            };
            Ok(UniffiRustFuture::<{{ callable.result.async_rust_future_output() }}>::new(uniffi_future).into_handle())
        })
    }
}
{% endif %}
