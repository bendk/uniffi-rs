#[unsafe(no_mangle)]
pub extern "system" fn Java_uniffi_Scaffolding_ffiBufferNew(
    env: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
) -> uniffi_jni::jobject {
    // SAFETY:
    // env points to a valid JNIEnv
    unsafe {
        ((**env).v1_4.NewDirectByteBuffer)(
            env,
            uniffi::ffi_buffer_alloc().cast(),
            uniffi::BASE_MINI_BUFFER_SIZE as i64,
        )
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_uniffi_Scaffolding_miniBufferNext(
    env: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    buffer: uniffi_jni::jobject,
    size: i64,
) -> uniffi_jni::jobject {
    // Safety:
    // * env points to a valid JNIEnv
    // * We assume the Kotlin side of the FFI sent us a ByteBuffer pointing to a FfiBuffer
    unsafe {
        let ptr = ((**env).v1_4.GetDirectBufferAddress)(env, buffer);
        let size = size as usize;
        let end = ptr.cast::<u8>().wrapping_add(size - 8);
        let next_ptr = unsafe { uniffi::mini_buffer_next(end, size) };
        ((**env).v1_4.NewDirectByteBuffer)(
            env,
            next_ptr.cast(),
            (size * 2) as i64,
        )
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_uniffi_Scaffolding_ffiBufferFree(
    env: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    buffer: uniffi_jni::jobject,
) {
    // Safety:
    // * env points to a valid JNIEnv
    // * We assume the other side of the FFI sent a ByteBuffer pointing to a FfiBuffer
    unsafe {
        let ptr = ((**env).v1_4.GetDirectBufferAddress)(env, buffer);
        uniffi::ffi_buffer_free(ptr.cast());
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_uniffi_Scaffolding_readByte(
    _: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    ptr: i64,
) -> i8 {
    // Safety:
    // We assume the other side of the FFI gave us a valid address
    unsafe {
        ::std::ptr::with_exposed_provenance::<i8>(ptr as usize).read()
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_uniffi_Scaffolding_readShort(
    _: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    ptr: i64,
) -> i16 {
    // Safety:
    // We assume the other side of the FFI gave us a valid address
    unsafe {
        ::std::ptr::with_exposed_provenance::<i16>(ptr as usize).read()
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_uniffi_Scaffolding_readInt(
    _: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    ptr: i64,
) -> i32 {
    // Safety:
    // We assume the other side of the FFI gave us a valid address
    unsafe {
        ::std::ptr::with_exposed_provenance::<i32>(ptr as usize).read()
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_uniffi_Scaffolding_readLong(
    _: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    ptr: i64,
) -> i64 {
    // Safety:
    // We assume the other side of the FFI gave us a valid address
    unsafe {
        ::std::ptr::with_exposed_provenance::<i64>(ptr as usize).read()
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_uniffi_Scaffolding_readFloat(
    _: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    ptr: i64,
) -> f32 {
    // Safety:
    // We assume the other side of the FFI gave us a valid address
    unsafe {
        ::std::ptr::with_exposed_provenance::<f32>(ptr as usize).read()
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_uniffi_Scaffolding_readDouble(
    _: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    ptr: i64,
) -> f64 {
    // Safety:
    // We assume the other side of the FFI gave us a valid address
    unsafe {
        ::std::ptr::with_exposed_provenance::<f64>(ptr as usize).read()
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_uniffi_Scaffolding_readString(
    env: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    data: i64,
    length: i64,
    capacity: i64,
) -> uniffi_jni::jstring {
    // Safety:
    //
    // * env points to a valid JNIEnv
    // * We assume the other side of the FFI passed us valid data
    uniffi::trace!("read_string_from_pointer (0x{data:x}, {length}, {capacity})");
    unsafe {
        let value = String::from_raw_parts(
            ::std::ptr::with_exposed_provenance_mut(data as usize),
            length as usize,
            capacity as usize,
        );
        let value = uniffi_jni::JniString::from(value);
        value.into_jstring(env)
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_uniffi_Scaffolding_writeByte(
    _: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    ptr: i64,
    value: i8,
) {
    // Safety:
    // We assume the other side of the FFI gave us a valid address
    unsafe {
        ::std::ptr::with_exposed_provenance_mut::<i8>(ptr as usize).write(value)
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_uniffi_Scaffolding_writeShort(
    _: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    ptr: i64,
    value: i16,
) {
    // Safety:
    // We assume the other side of the FFI gave us a valid address
    unsafe {
        ::std::ptr::with_exposed_provenance_mut::<i16>(ptr as usize).write(value)
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_uniffi_Scaffolding_writeInt(
    _: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    ptr: i64,
    value: i32,
) {
    // Safety:
    // We assume the other side of the FFI gave us a valid address
    unsafe {
        ::std::ptr::with_exposed_provenance_mut::<i32>(ptr as usize).write(value)
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_uniffi_Scaffolding_writeLong(
    _: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    ptr: i64,
    value: i64,
) {
    // Safety:
    // We assume the other side of the FFI gave us a valid address
    unsafe {
        ::std::ptr::with_exposed_provenance_mut::<i64>(ptr as usize).write(value)
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_uniffi_Scaffolding_writeFloat(
    _: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    ptr: i64,
    value: f32,
) {
    // Safety:
    // We assume the other side of the FFI gave us a valid address
    unsafe {
        ::std::ptr::with_exposed_provenance_mut::<f32>(ptr as usize).write(value)
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_uniffi_Scaffolding_writeDouble(
    _: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    ptr: i64,
    value: f64,
) {
    // Safety:
    // We assume the other side of the FFI gave us a valid address
    unsafe {
        ::std::ptr::with_exposed_provenance_mut::<f64>(ptr as usize).write(value)
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_uniffi_Scaffolding_writeString(
    env: *mut uniffi_jni::JNIEnv,
    _: *mut uniffi_jni::jclass,
    buffer: uniffi_jni::jobject,
    index: i32,
    value: uniffi_jni::jstring,
) {

    // Safety:
    // * env points to a valid JNIEnv
    // * We assume the Kotlin side of the FFI sent us a ByteBuffer pointing to a FfiBuffer
    unsafe {
        uniffi_jni::rust_call(env, |env| {
            let value = uniffi_jni::decode_jni_string(env, value)?;
            let ptr = ((**env).v1_4.GetDirectBufferAddress)(env, buffer);
            let ptr = ptr.cast::<u8>().wrapping_add(index as usize);
            uniffi::write_string_to_pointer(ptr, value);
            Ok(())
        })
    }
}
