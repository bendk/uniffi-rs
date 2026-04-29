{%- let callable = scaffolding_function.callable %}
{%- let return_type = callable.return_type() %}
{%- let throws_type = callable.throws_type() %}
{%- if !callable.is_async %}
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_uniffi_Scaffolding_{{ scaffolding_function.jni_method_name }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    {%- if callable.uses_buffer() %}uniffi_buf_handle: i64,{% endif %}
    {%- for ffi_arg in callable.ffi_arguments() %}
    {{ ffi_arg.name_rs() }}: {{ ffi_arg.ty.type_rs() }},
    {%- endfor %}
)
{%- if let ReturnStrategy::Primitive(_, ffi_type) = callable.return_strategy() %} -> {{ ffi_type.type_rs() }}
{%- endif %}
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
                    {%- if !callable.uses_buffer() %}
                    // Need to allocate a new buffer for the exception since we didn't input one
                    let mut uniffi_buf = uniffi::FfiBuffer::new();
                    {% endif %}
                    // Safety:
                    // `uniffi_buf` points to a valid FFI buffer
                    unsafe { {{ throws_ty.throw_error_fn_rs() }}(uniffi_env, &mut uniffi_buf, uniffi_err)?; };
                    {%- if !callable.uses_buffer() %}
                    uniffi_buf.free();
                    {% endif %}
                    return Ok(::std::default::Default::default());
                }
            };
            {%- endif %}

            {%- match callable.return_strategy() %}
            {%- when ReturnStrategy::FfiBuffer(return_type) %}
            uniffi_buf.with_cursor(|uniffi_writer| {
                {{ return_type.write_fn_rs() }}(uniffi_writer, uniffi_return_value)
            })?;
            {%- when ReturnStrategy::Primitive(type_node, _) %}
            let uniffi_return_value = {{ type_node.lower_fn_rs() }}(uniffi_env, uniffi_return_value)?;
            {%- when ReturnStrategy::Void %}
            {%- endmatch %}
            {%- if callable.return_strategy().is_primitive() %}
            Ok(uniffi_return_value)
            {%- else %}
            Ok(())
            {%- endif %}
        })
    }
}
{% else %}
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_uniffi_Scaffolding_{{ scaffolding_function.jni_method_name }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    {%- if callable.uses_buffer() %}uniffi_buf_handle: i64,{% endif %}
    {%- for ffi_arg in callable.ffi_arguments() %}
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
                            {%- if callable.uses_buffer() %}
                            ::std::option::Option::Some(uniffi_buf)
                            {%- else %}
                            ::std::option::Option::<uniffi::FfiBuffer>::None,
                            {%- endif %}
                        )));
                    }
                };
                {%- endif %}

                {%- match callable.return_strategy() %}
                {%- when ReturnStrategy::FfiBuffer(return_type) %}
                uniffi_buf.with_cursor(|uniffi_writer| {
                    {{ return_type.write_fn_rs() }}(uniffi_writer, uniffi_return_value)
                })?;
                let uniffi_return_value = ();
                {%- else %}
                {%- endmatch %}
                {%- if callable.throws_type().is_none() %}
                UniffiAnyhowResult::Ok(uniffi_return_value)
                {%- else %}
                UniffiAnyhowResult::Ok(::std::result::Result::Ok(uniffi_return_value))
                {%- endif %}
            };
            Ok(UniffiRustFuture::new(uniffi_future).into_handle())
        })
    }
}
{% endif %}
