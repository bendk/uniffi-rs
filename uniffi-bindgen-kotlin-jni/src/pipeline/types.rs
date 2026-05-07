/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use super::*;

pub fn map_type_node(type_node: general::TypeNode, context: &Context) -> Result<TypeNode> {
    let lowerable = if let Some(primitive) = FfiType::for_primitive(&type_node.ty) {
        Some(LowerableType::Primitive(primitive))
    } else {
        context
            .deconstructable_type_map
            .get(&type_node.ty)
            .map(|primitives| LowerableType::Deconstructable(primitives.clone()))
    };

    Ok(TypeNode {
        is_used_as_error: type_node.is_used_as_error,
        type_kt: type_kt(&type_node.ty, context)?,
        type_rs: type_rs(&type_node.ty, context)?,
        has_from_unexpected_callback_error_impl: type_node.has_from_unexpected_callback_error_impl,
        id: context.get_type_id(&type_node.ty)?,
        ty: type_node.ty.map_node(context)?,
        lowerable,
    })
}

fn type_rs(ty: &Type, context: &Context) -> Result<String> {
    Ok(match ty {
        Type::UInt8 => "u8".into(),
        Type::Int8 => "i8".into(),
        Type::UInt16 => "u16".into(),
        Type::Int16 => "i16".into(),
        Type::UInt32 => "u32".into(),
        Type::Int32 => "i32".into(),
        Type::UInt64 => "u64".into(),
        Type::Int64 => "i64".into(),
        Type::Float32 => "f32".into(),
        Type::Float64 => "f64".into(),
        Type::Boolean => "bool".into(),
        Type::String => "::std::string::String".into(),
        Type::Optional { inner_type } => {
            format!("::std::option::Option<{}>", type_rs(inner_type, context)?)
        }
        Type::Sequence { inner_type } => {
            format!("::std::vec::Vec<{}>", type_rs(inner_type, context)?)
        }
        Type::Map {
            key_type,
            value_type,
        } => {
            format!(
                "::std::collections::HashMap<{}, {}>",
                type_rs(key_type, context)?,
                type_rs(value_type, context)?,
            )
        }
        Type::Record {
            namespace,
            orig_name,
            ..
        }
        | Type::Enum {
            namespace,
            orig_name,
            ..
        }
        | Type::Custom {
            namespace,
            orig_name,
            ..
        } => {
            format!(
                "{}::{orig_name}",
                context.rust_module_path_for_type(namespace, orig_name)?
            )
        }
        Type::Interface {
            namespace,
            orig_name,
            imp,
            ..
        } => {
            if imp.is_trait_interface() {
                format!(
                    "::std::sync::Arc<dyn {}::{orig_name}>",
                    context.rust_module_path_for_type(namespace, orig_name)?
                )
            } else {
                format!(
                    "::std::sync::Arc<{}::{orig_name}>",
                    context.rust_module_path_for_type(namespace, orig_name)?
                )
            }
        }
        Type::CallbackInterface {
            namespace,
            orig_name,
            ..
        } => {
            format!(
                "::std::boxed::Box<dyn {}::{orig_name}>",
                context.rust_module_path_for_type(namespace, orig_name)?
            )
        }
        Type::Box { inner_type } => {
            format!("::std::boxed::Box<{}>", type_rs(inner_type, context)?,)
        }
        Type::Bytes => "::std::vec::Vec<u8>".into(),
        Type::Duration => "::std::time::Duration".into(),
        Type::Timestamp => "::std::time::Instant".into(),
    })
}

pub fn type_kt(ty: &Type, context: &Context) -> Result<String> {
    Ok(match ty {
        Type::UInt8 => "kotlin.UByte".into(),
        Type::Int8 => "kotlin.Byte".into(),
        Type::UInt16 => "kotlin.UShort".into(),
        Type::Int16 => "kotlin.Short".into(),
        Type::UInt32 => "kotlin.UInt".into(),
        Type::Int32 => "kotlin.Int".into(),
        Type::UInt64 => "kotlin.ULong".into(),
        Type::Int64 => "kotlin.Long".into(),
        Type::Float32 => "kotlin.Float".into(),
        Type::Float64 => "kotlin.Double".into(),
        Type::Boolean => "kotlin.Boolean".into(),
        Type::String => "kotlin.String".into(),
        Type::Optional { inner_type } => {
            format!("{}?", type_kt(inner_type, context)?)
        }
        Type::Sequence { inner_type } => {
            format!("kotlin.collections.List<{}>", type_kt(inner_type, context)?)
        }
        Type::Map {
            key_type,
            value_type,
        } => {
            format!(
                "kotlin.collections.Map<{}, {}>",
                type_kt(key_type, context)?,
                type_kt(value_type, context)?,
            )
        }
        Type::Record {
            namespace, name, ..
        }
        | Type::Enum {
            namespace, name, ..
        }
        | Type::Interface {
            namespace, name, ..
        }
        | Type::CallbackInterface {
            namespace, name, ..
        }
        | Type::Custom {
            namespace, name, ..
        } => {
            format!(
                "{}.{}",
                context.package_name(namespace)?,
                names::class_name_kt(name, context.types_used_as_error.contains(ty)),
            )
        }
        Type::Box { inner_type } => type_kt(inner_type, context)?,
        Type::Bytes => "kotlin.ByteArray".into(),
        Type::Duration => "java.time.Duration".into(),
        Type::Timestamp => "java.time.Instant".into(),
    })
}

impl TypeNode {
    pub fn read_fn_rs(&self) -> String {
        match &self.ty {
            Type::UInt8 => "uniffi::FfiBufferCursor::read_u8".into(),
            Type::Int8 => "uniffi::FfiBufferCursor::read_i8".into(),
            Type::UInt16 => "uniffi::FfiBufferCursor::read_u16".into(),
            Type::Int16 => "uniffi::FfiBufferCursor::read_i16".into(),
            Type::UInt32 => "uniffi::FfiBufferCursor::read_u32".into(),
            Type::Int32 => "uniffi::FfiBufferCursor::read_i32".into(),
            Type::UInt64 => "uniffi::FfiBufferCursor::read_u64".into(),
            Type::Int64 => "uniffi::FfiBufferCursor::read_i64".into(),
            Type::Float32 => "uniffi::FfiBufferCursor::read_f32".into(),
            Type::Float64 => "uniffi::FfiBufferCursor::read_f64".into(),
            Type::Boolean => "uniffi::FfiBufferCursor::read_bool".into(),
            Type::String => "uniffi::FfiBufferCursor::read_string".into(),
            _ => self.fn_name_rs("read"),
        }
    }

    pub fn write_fn_rs(&self) -> String {
        match &self.ty {
            Type::UInt8 => "uniffi::FfiBufferCursor::write_u8".into(),
            Type::Int8 => "uniffi::FfiBufferCursor::write_i8".into(),
            Type::UInt16 => "uniffi::FfiBufferCursor::write_u16".into(),
            Type::Int16 => "uniffi::FfiBufferCursor::write_i16".into(),
            Type::UInt32 => "uniffi::FfiBufferCursor::write_u32".into(),
            Type::Int32 => "uniffi::FfiBufferCursor::write_i32".into(),
            Type::UInt64 => "uniffi::FfiBufferCursor::write_u64".into(),
            Type::Int64 => "uniffi::FfiBufferCursor::write_i64".into(),
            Type::Float32 => "uniffi::FfiBufferCursor::write_f32".into(),
            Type::Float64 => "uniffi::FfiBufferCursor::write_f64".into(),
            Type::Boolean => "uniffi::FfiBufferCursor::write_bool".into(),
            Type::String => "uniffi::FfiBufferCursor::write_string".into(),
            _ => self.fn_name_rs("write"),
        }
    }

    /// Function to lower this type
    ///
    /// For primitive types, this converts this type into a single FfiType.
    /// For deconstructable types, this converts this type into a multiple FfiTypes.
    /// Otherwise, this function is not defined
    pub fn lower_fn_rs(&self) -> String {
        match &self.ty {
            Type::Int8 => "uniffi_jni::lower_i8".into(),
            Type::Int16 => "uniffi_jni::lower_i16".into(),
            Type::Int32 => "uniffi_jni::lower_i32".into(),
            Type::Int64 => "uniffi_jni::lower_i64".into(),
            Type::UInt8 => "uniffi_jni::lower_u8".into(),
            Type::UInt16 => "uniffi_jni::lower_u16".into(),
            Type::UInt32 => "uniffi_jni::lower_u32".into(),
            Type::UInt64 => "uniffi_jni::lower_u64".into(),
            Type::Float32 => "uniffi_jni::lower_f32".into(),
            Type::Float64 => "uniffi_jni::lower_f64".into(),
            Type::Boolean => "uniffi_jni::lower_bool".into(),
            Type::String => "uniffi_jni::lower_string".into(),
            Type::Optional { inner_type } => match &**inner_type {
                Type::Boolean => "uniffi_jni::lower_option_bool".into(),
                Type::Int8 => "uniffi_jni::lower_option_i8".into(),
                Type::UInt8 => "uniffi_jni::lower_option_u8".into(),
                Type::Int16 => "uniffi_jni::lower_option_i16".into(),
                Type::UInt16 => "uniffi_jni::lower_option_u16".into(),
                Type::Int32 => "uniffi_jni::lower_option_i32".into(),
                Type::UInt32 => "uniffi_jni::lower_option_u32".into(),
                Type::Float32 => "uniffi_jni::lower_option_f32".into(),
                Type::Float64 => "uniffi_jni::lower_option_f64".into(),
                Type::String => "uniffi_jni::lower_option_string".into(),
                _ => self.fn_name_rs("lower"),
            },
            _ => self.fn_name_rs("lower"),
        }
    }

    /// Function to lift this type
    ///
    /// For primitive types, this converts a single FfiType into this type.
    /// For deconstructable types, this converts multiple FfiTypes into this type.
    /// Otherwise, this function is not defined
    pub fn lift_fn_rs(&self) -> String {
        match &self.ty {
            Type::Int8 => "uniffi_jni::lift_i8".into(),
            Type::Int16 => "uniffi_jni::lift_i16".into(),
            Type::Int32 => "uniffi_jni::lift_i32".into(),
            Type::Int64 => "uniffi_jni::lift_i64".into(),
            Type::UInt8 => "uniffi_jni::lift_u8".into(),
            Type::UInt16 => "uniffi_jni::lift_u16".into(),
            Type::UInt32 => "uniffi_jni::lift_u32".into(),
            Type::UInt64 => "uniffi_jni::lift_u64".into(),
            Type::Float32 => "uniffi_jni::lift_f32".into(),
            Type::Float64 => "uniffi_jni::lift_f64".into(),
            Type::Boolean => "uniffi_jni::lift_bool".into(),
            Type::String => "uniffi_jni::lift_string".into(),
            Type::Optional { inner_type } => match &**inner_type {
                Type::Boolean => "uniffi_jni::lift_option_bool".into(),
                Type::Int8 => "uniffi_jni::lift_option_i8".into(),
                Type::UInt8 => "uniffi_jni::lift_option_u8".into(),
                Type::Int16 => "uniffi_jni::lift_option_i16".into(),
                Type::UInt16 => "uniffi_jni::lift_option_u16".into(),
                Type::Int32 => "uniffi_jni::lift_option_i32".into(),
                Type::UInt32 => "uniffi_jni::lift_option_u32".into(),
                Type::Float32 => "uniffi_jni::lift_option_f32".into(),
                Type::Float64 => "uniffi_jni::lift_option_f64".into(),
                Type::String => "uniffi_jni::lift_option_string".into(),
                _ => self.fn_name_rs("lift"),
            },
            _ => self.fn_name_rs("lift"),
        }
    }

    pub fn read_fn_kt(&self) -> String {
        match &self.ty {
            Type::UInt8 => "readUByte".into(),
            Type::Int8 => "readByte".into(),
            Type::UInt16 => "readUShort".into(),
            Type::Int16 => "readShort".into(),
            Type::UInt32 => "readUInt".into(),
            Type::Int32 => "readInt".into(),
            Type::UInt64 => "readULong".into(),
            Type::Int64 => "readLong".into(),
            Type::Float32 => "readFloat".into(),
            Type::Float64 => "readDouble".into(),
            Type::Boolean => "readBool".into(),
            Type::String => "readString".into(),
            _ => self.fn_name_kt("read"),
        }
    }

    pub fn write_fn_kt(&self) -> String {
        match &self.ty {
            Type::UInt8 => "writeUByte".into(),
            Type::Int8 => "writeByte".into(),
            Type::UInt16 => "writeUShort".into(),
            Type::Int16 => "writeShort".into(),
            Type::UInt32 => "writeUInt".into(),
            Type::Int32 => "writeInt".into(),
            Type::UInt64 => "writeULong".into(),
            Type::Int64 => "writeLong".into(),
            Type::Float32 => "writeFloat".into(),
            Type::Float64 => "writeDouble".into(),
            Type::Boolean => "writeBool".into(),
            Type::String => "writeString".into(),
            _ => self.fn_name_kt("write"),
        }
    }

    /// Function to lower this type
    ///
    /// For primitive types, this converts this type into a single FfiType.
    /// For deconstructable types, this converts this type into a multiple FfiTypes.
    /// Otherwise, this function is not defined
    pub fn lower_fn_kt(&self) -> String {
        match &self.ty {
            Type::Int8 => "lowerByte".into(),
            Type::Int16 => "lowerShort".into(),
            Type::Int32 => "lowerInt".into(),
            Type::Int64 => "lowerLong".into(),
            Type::UInt8 => "lowerUByte".into(),
            Type::UInt16 => "lowerUShort".into(),
            Type::UInt32 => "lowerUInt".into(),
            Type::UInt64 => "lowerULong".into(),
            Type::Float32 => "lowerFloat".into(),
            Type::Float64 => "lowerDouble".into(),
            Type::Boolean => "lowerBoolean".into(),
            Type::String => "lowerString".into(),
            Type::Optional { inner_type } => match &**inner_type {
                Type::Boolean => "lowerOptionBoolean".into(),
                Type::Int8 => "lowerOptionByte".into(),
                Type::UInt8 => "lowerOptionUByte".into(),
                Type::Int16 => "lowerOptionShort".into(),
                Type::UInt16 => "lowerOptionUShort".into(),
                Type::Int32 => "lowerOptionInt".into(),
                Type::UInt32 => "lowerOptionUInt".into(),
                Type::Float32 => "lowerOptionFloat".into(),
                Type::Float64 => "lowerOptionDouble".into(),
                Type::String => "lowerOptionString".into(),
                _ => self.fn_name_kt("lower"),
            },
            _ => self.fn_name_kt("lower"),
        }
    }

    /// Function to lift this type
    ///
    /// For primitive types, this converts a single FfiType into this type.
    /// For deconstructable types, this converts multiple FfiTypes into this type.
    /// Otherwise, this function is not defined
    pub fn lift_fn_kt(&self) -> String {
        match &self.ty {
            Type::Int8 => "liftByte".into(),
            Type::Int16 => "liftShort".into(),
            Type::Int32 => "liftInt".into(),
            Type::Int64 => "liftLong".into(),
            Type::UInt8 => "liftUByte".into(),
            Type::UInt16 => "liftUShort".into(),
            Type::UInt32 => "liftUInt".into(),
            Type::UInt64 => "liftULong".into(),
            Type::Float32 => "liftFloat".into(),
            Type::Float64 => "liftDouble".into(),
            Type::Boolean => "liftBoolean".into(),
            Type::String => "liftString".into(),
            Type::Optional { inner_type } => match &**inner_type {
                Type::Boolean => "liftOptionBoolean".into(),
                Type::Int8 => "liftOptionByte".into(),
                Type::UInt8 => "liftOptionUByte".into(),
                Type::Int16 => "liftOptionShort".into(),
                Type::UInt16 => "liftOptionUShort".into(),
                Type::Int32 => "liftOptionInt".into(),
                Type::UInt32 => "liftOptionUInt".into(),
                Type::Float32 => "liftOptionFloat".into(),
                Type::Float64 => "liftOptionDouble".into(),
                Type::String => "liftOptionString".into(),
                _ => self.fn_name_kt("lift"),
            },
            _ => self.fn_name_kt("lift"),
        }
    }

    /// Kotlin type that the deconstruct function returns in Kotlin
    ///
    /// This is a tuple-like class, where all fields are named `v{index}`
    pub fn deconstructed_type_kt(&self) -> String {
        format!("DeconstructedType{}", self.id)
    }

    /// Generate a standard Rust function name
    fn fn_name_rs(&self, prefix: &str) -> String {
        let prefix = prefix.to_snake_case();
        match &self.ty {
            Type::UInt8 => format!("uniffi_{prefix}_u8"),
            Type::Int8 => format!("uniffi_{prefix}_i8"),
            Type::UInt16 => format!("uniffi_{prefix}_u16"),
            Type::Int16 => format!("uniffi_{prefix}_i16"),
            Type::UInt32 => format!("uniffi_{prefix}_u32"),
            Type::Int32 => format!("uniffi_{prefix}_i32"),
            Type::UInt64 => format!("uniffi_{prefix}_u64"),
            Type::Int64 => format!("uniffi_{prefix}_i64"),
            Type::Float32 => format!("uniffi_{prefix}_f32"),
            Type::Float64 => format!("uniffi_{prefix}_f64"),
            Type::Boolean => format!("uniffi_{prefix}_bool"),
            Type::String => format!("uniffi_{prefix}_string"),
            Type::Bytes => format!("uniffi_{prefix}_bytes"),
            Type::Timestamp => format!("uniffi_{prefix}_timestamp"),
            Type::Duration => format!("uniffi_{prefix}_duration"),
            Type::Record { .. }
            | Type::Enum { .. }
            | Type::Interface { .. }
            | Type::CallbackInterface { .. }
            | Type::Custom { .. }
            | Type::Optional { .. }
            | Type::Sequence { .. }
            | Type::Map { .. }
            | Type::Box { .. } => format!("uniffi_{prefix}_type_{}", self.id),
        }
    }

    /// Generate a standard Kotlin function name
    fn fn_name_kt(&self, prefix: &str) -> String {
        let prefix = prefix.to_lower_camel_case();
        match &self.ty {
            Type::UInt8 => format!("{prefix}UByte"),
            Type::Int8 => format!("{prefix}Byte"),
            Type::UInt16 => format!("{prefix}UShort"),
            Type::Int16 => format!("{prefix}Short"),
            Type::UInt32 => format!("{prefix}UInt"),
            Type::Int32 => format!("{prefix}Int"),
            Type::UInt64 => format!("{prefix}ULong"),
            Type::Int64 => format!("{prefix}Long"),
            Type::Float32 => format!("{prefix}Float"),
            Type::Float64 => format!("{prefix}Double"),
            Type::Boolean => format!("{prefix}Boolean"),
            Type::String => format!("{prefix}String"),
            Type::Bytes => format!("{prefix}Bytes"),
            Type::Timestamp => format!("{prefix}Timestamp"),
            Type::Duration => format!("{prefix}Duration"),
            Type::Record { .. }
            | Type::Enum { .. }
            | Type::Interface { .. }
            | Type::CallbackInterface { .. }
            | Type::Custom { .. }
            | Type::Optional { .. }
            | Type::Sequence { .. }
            | Type::Map { .. }
            | Type::Box { .. } => format!("{prefix}Type{}", self.id),
        }
    }

    pub fn throw_error_fn_rs(&self) -> String {
        format!("uniffi_throw_error_{}", self.id)
    }

    pub fn construct_fn_kt(&self) -> String {
        format!("construct{}", self.id)
    }

    pub fn lift_fn_jni_signature(&self) -> String {
        let args = match &self.lowerable {
            None => "J".to_string(),
            Some(LowerableType::Deconstructable(ffi_types)) => ffi_types
                .iter()
                .map(|ffi_type| ffi_type.jni_signature())
                .collect(),
            Some(LowerableType::Primitive(ffi_type)) => ffi_type.jni_signature().to_string(),
        };
        let ret = format!("L{};", self.type_kt.replace(".", "/").replace("`", ""));
        format!("({args}){ret}")
    }

    pub fn is_primitive(&self) -> bool {
        matches!(self.lowerable, Some(LowerableType::Primitive(_)))
    }

    pub fn uses_buffer(&self) -> bool {
        self.lowerable.is_none()
    }

    /// Static Rust variable to call the Kotlin lift function from Rust
    pub fn lift_kt_from_rust_var(&self) -> String {
        format!("UNIFFI_CACHED_LIFT_KT_{}", self.id)
    }
}
