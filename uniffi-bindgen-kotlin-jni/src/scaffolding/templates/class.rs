{%- let type_name = cls.self_type.type_rs %}
{%- let inner_type_name = "{}::{}"|format(cls.module_path, cls.name_rs()) %}
{#
 # lift/lower functions for objects.
 # These work like the FFI lift/lower except they never fail and don't need to input a `JNIEnv`.
 #}

{%- let lift_fn = "_uniffi_lift_object_{}"|format(cls.self_type.id) %}
{%- let lower_fn = "_uniffi_lower_object_{}"|format(cls.self_type.id) %}

{%- match cls.imp %}
{%- when ObjectImpl::Struct %}
/// Read a {{ type_name }} from a `FfiBufferCursor`
pub fn {{ cls.self_type.read_fn_rs() }}(
    cursor: &mut uniffi::FfiBufferCursor,
) -> uniffi::Result<{{ type_name }}> {
    let handle = cursor.read_i64()?;
    uniffi::trace!("{{ cls.name }} read: {handle:x}");
    Ok({{ lift_fn }}(handle))
}

/// Write a {{ type_name }} to a `FfiBufferCursor`
///
/// Inputs ArcOrOwned<T> so that it's compatible with both `T` and `Arc<T>`.
pub fn {{ cls.self_type.write_fn_rs() }}(
    cursor: &mut uniffi::FfiBufferCursor,
    value: impl uniffi::ArcOrOwned<{{ inner_type_name }}>,
) -> uniffi::Result<()> {
    let handle = {{ lower_fn }}(value);
    uniffi::trace!("{{ cls.name }} write: {handle:x}");
    cursor.write_i64(handle)?;
    Ok(())
}

fn {{ lift_fn }}(handle: i64) -> {{ type_name }} {
    unsafe {
        ::std::sync::Arc::from_raw(::std::ptr::with_exposed_provenance(handle as usize))
    }
}

fn {{ lower_fn }}(value: impl uniffi::ArcOrOwned<{{ inner_type_name }}>) -> i64 {
    let raw_ptr = ::std::sync::Arc::into_raw(value.into_arc());
    raw_ptr.expose_provenance() as i64
}

{%- when ObjectImpl::Trait %}
/// Read a {{ type_name }} from a `FfiBufferCursor`
pub fn {{ cls.self_type.read_fn_rs() }}(
    cursor: &mut uniffi::FfiBufferCursor,
) -> uniffi::Result<::std::sync::Arc<dyn {{ inner_type_name }}>> {
    let handle1 = cursor.read_i64()?;
    let handle2 = cursor.read_i64()?;
    uniffi::trace!("{{ cls.name }} read: {handle1:x} {handle2:x}");
    Ok({{ lift_fn }}(handle1, handle2))
}

/// Write a {{ type_name }} to a `FfiBufferCursor`
pub fn {{ cls.self_type.write_fn_rs() }}(
    cursor: &mut uniffi::FfiBufferCursor,
    value: ::std::sync::Arc<dyn {{ inner_type_name }}>,
) -> uniffi::Result<()> {
    let (handle1, handle2) = {{ lower_fn }}(value);

    uniffi::trace!("{{ cls.name }} write: {handle1:x} {handle2:x}");
    cursor.write_i64(handle1)?;
    cursor.write_i64(handle2)?;
    Ok(())
}

fn {{ lift_fn }}(handle1: i64, handle2: i64) -> {{ type_name }} {
    // Safety:
    // This is reversing a transmute/into_raw by {{ lower_fn }}
    unsafe {
        let raw_ptr1 = ::std::ptr::with_exposed_provenance_mut::<()>(handle1 as usize);
        let raw_ptr2 = ::std::ptr::with_exposed_provenance_mut::<()>(handle2 as usize);
        let raw_ptr: *const dyn {{ inner_type_name }} = ::std::mem::transmute([raw_ptr1, raw_ptr2]);
        ::std::sync::Arc::from_raw(raw_ptr)
    }
}

fn {{ lower_fn }}(value: ::std::sync::Arc<dyn {{ inner_type_name }}>) -> (i64, i64) {
    let raw_ptr = ::std::sync::Arc::into_raw(value);

    // Safety:
    // A wide pointer has the same layout as 2 normal pointers
    let [raw_ptr1, raw_ptr2]: [*mut (); 2] = unsafe {
        ::std::mem::transmute(raw_ptr)
    };
    (
        raw_ptr1.expose_provenance() as i64,
        raw_ptr2.expose_provenance() as i64,
    )
}

{%- when ObjectImpl::CallbackTrait %}
/// Read a {{ type_name }} from a `FfiBufferCursor`
pub fn {{ cls.self_type.read_fn_rs() }}(
    cursor: &mut uniffi::FfiBufferCursor,
) -> uniffi::Result<::std::sync::Arc<dyn {{ inner_type_name }}>> {
    let handle1 = cursor.read_i64()?;
    let handle2 = cursor.read_i64()?;
    uniffi::trace!("{{ cls.name }} read: {handle1:x} {handle2:x}");
    Ok({{ lift_fn }}(handle1, handle2))
}

/// Write a {{ type_name }} to a `FfiBufferCursor`
pub fn {{ cls.self_type.write_fn_rs() }}(
    cursor: &mut uniffi::FfiBufferCursor,
    value: ::std::sync::Arc<dyn {{ inner_type_name }}>,
) -> uniffi::Result<()> {
    let (handle1, handle2) = {{ lower_fn }}(value);
    uniffi::trace!("{{ cls.name }} write: {handle1:x} {handle2:x}");
    cursor.write_i64(handle1)?;
    cursor.write_i64(handle2)?;
    Ok(())
}

fn {{ lift_fn }}(handle1: i64, handle2: i64) -> {{ type_name }} {
    if handle1 == 0 {
        // Callback interface from Kotlin
        ::std::sync::Arc::new({{ cls.impl_struct_rs() }} {
            handle: handle2
        })
    } else {
        // Arc<dyn Trait> impl from Rust
        // Safety:
        // This is reversing a transmute/into_raw by {{ cls.self_type.write_fn_rs() }}
        unsafe {
            let raw_ptr1 = ::std::ptr::with_exposed_provenance::<()>(handle1 as usize);
            let raw_ptr2 = ::std::ptr::with_exposed_provenance::<()>(handle2 as usize);
            let raw_ptr: *const dyn {{ inner_type_name }} = ::std::mem::transmute([raw_ptr1, raw_ptr2]);
            ::std::sync::Arc::from_raw(raw_ptr)
        }
    }
}

fn {{ lower_fn }}(value: ::std::sync::Arc<dyn {{ inner_type_name }}>) -> (i64, i64) {
    match value.uniffi_foreign_handle() {
        Some(handle) => {
            // Callback interface from Kotlin
            (0, handle.as_raw() as i64)
        }
        None => {
            // Arc<dyn Trait> impl from Rust
            let raw_ptr = ::std::sync::Arc::into_raw(value);
            // Safety:
            // A wide pointer has the same layout as 2 normal pointers
            let [raw_ptr1, raw_ptr2]: [*mut (); 2] = unsafe {
                std::mem::transmute(raw_ptr)
            };
            (
                raw_ptr1.expose_provenance() as i64,
                raw_ptr2.expose_provenance() as i64,
            )
        }
    }
}
{%- endmatch %}

{%- if !cls.imp.is_trait_interface() %}
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_uniffi_Scaffolding_{{ cls.jni_free_name() }}(
    _: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    handle: i64,
) {
    let raw_ptr = ::std::ptr::with_exposed_provenance_mut::<{{ inner_type_name }}>(handle as usize);
    uniffi::trace!("{{ cls.name }} free: {raw_ptr:?}");
    // Safety:
    // raw_ptr came from an `into_raw()` call
    unsafe {
        drop(::std::sync::Arc::from_raw(raw_ptr))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_uniffi_Scaffolding_{{ cls.jni_addref_name() }}(
    _: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    handle: i64,
) {
    let raw_ptr = ::std::ptr::with_exposed_provenance_mut::<{{ inner_type_name }}>(handle as usize);
    uniffi::trace!("{{ cls.name }} addref: {raw_ptr:?}");
    // Safety:
    // raw_ptr came from an `into_raw()` call
    unsafe {
        ::std::sync::Arc::increment_strong_count(raw_ptr);
    }
}
{%- else %}
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_uniffi_Scaffolding_{{ cls.jni_free_name() }}(
    _: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    handle1: i64,
    handle2: i64,
) {
    let raw_ptr1 = ::std::ptr::with_exposed_provenance::<()>(handle1 as usize);
    let raw_ptr2 = ::std::ptr::with_exposed_provenance::<()>(handle2 as usize);
    uniffi::trace!("{{ cls.name }} free: {raw_ptr1:?} {raw_ptr2:?}");
    // Safety:
    // This is reversing a transmute/into_raw by {{ cls.self_type.write_fn_rs() }}
    unsafe {
        let raw_ptr: *const dyn {{ inner_type_name }} = ::std::mem::transmute([raw_ptr1, raw_ptr2]);
        drop(::std::sync::Arc::from_raw(raw_ptr))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_uniffi_Scaffolding_{{ cls.jni_addref_name() }}(
    _: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    handle1: i64,
    handle2: i64,
) {
    let raw_ptr1 = ::std::ptr::with_exposed_provenance::<()>(handle1 as usize);
    let raw_ptr2 = ::std::ptr::with_exposed_provenance::<()>(handle2 as usize);
    uniffi::trace!("{{ cls.name }} addref: {raw_ptr1:?} {raw_ptr2:?}");
    // Safety:
    // This is reversing a transmute/into_raw by {{ cls.self_type.write_fn_rs() }}
    unsafe {
        let raw_ptr: *const dyn {{ inner_type_name }} = ::std::mem::transmute([raw_ptr1, raw_ptr2]);
        ::std::sync::Arc::increment_strong_count(raw_ptr);
    }
}
{%- endif %}

{%- match cls.self_type.lowerable %}
{%- when Some(LowerableType::Primitive(ffi_type)) %}

pub fn {{ cls.self_type.lower_fn_rs() }}(
    _uniffi_env: *mut uniffi_jni::JNIEnv,
    value: impl uniffi::ArcOrOwned<{{ inner_type_name }}>,
) -> uniffi::Result<i64> {
    Ok({{ lower_fn }}(value))
}

pub fn {{ cls.self_type.lift_fn_rs() }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    handle: i64,
) -> uniffi::Result<{{ type_name }}> {
    Ok({{ lift_fn }}(handle))
}

{%- when Some(LowerableType::Deconstructable(ffi_types)) %}

pub fn {{ cls.self_type.lower_fn_rs() }}(
    _uniffi_env: *mut uniffi_jni::JNIEnv,
    value: ::std::sync::Arc<dyn {{ inner_type_name }}>,
) -> uniffi::Result<(i64, i64)> {
    Ok({{ lower_fn }}(value))
}

pub fn {{ cls.self_type.lift_fn_rs() }}(
    uniffi_env: *mut uniffi_jni::JNIEnv,
    handle1: i64,
    handle2: i64,
) -> uniffi::Result<{{ type_name }}> {
    Ok({{ lift_fn }}(handle1, handle2))
}
{%- else %}
{%- endmatch %}
