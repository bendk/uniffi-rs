/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use super::*;

pub fn map_type_node(type_node: general::TypeNode, context: &Context) -> Result<TypeNode> {
    Ok(TypeNode {
        is_used_as_error: type_node.is_used_as_error,
        type_kt: type_kt(&type_node.ty, context)?,
        type_rs: type_rs(&type_node.ty, context)?,
        id: *context
            .type_id_map
            .get(&type_node.ty)
            .ok_or_else(|| anyhow!("Type missing from Context.type_id_map: {:?}", type_node.ty))?,
        ty: type_node.ty.map_node(context)?,
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
        Type::Record {
            namespace,
            orig_name,
            ..
        }
        | Type::Enum {
            namespace,
            orig_name,
            ..
        } => {
            format!(
                "::{}::{orig_name}",
                context.module_path_for_type(namespace, orig_name)?
            )
        }
        _ => todo!(),
    })
}

pub fn type_kt(ty: &Type, context: &Context) -> Result<String> {
    Ok(match ty {
        Type::UInt8 => "UByte".into(),
        Type::Int8 => "Byte".into(),
        Type::UInt16 => "UShort".into(),
        Type::Int16 => "Short".into(),
        Type::UInt32 => "UInt".into(),
        Type::Int32 => "Int".into(),
        Type::UInt64 => "ULong".into(),
        Type::Int64 => "Long".into(),
        Type::Float32 => "Float".into(),
        Type::Float64 => "Double".into(),
        Type::Boolean => "Boolean".into(),
        Type::String => "String".into(),
        Type::Record {
            namespace, name, ..
        }
        | Type::Enum {
            namespace, name, ..
        } => {
            format!("{}.{name}", context.package_name(namespace)?)
        }
        _ => todo!(),
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
}
