/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use super::*;

impl FfiType {
    /// Get the FFIType for a type -- if it's a primitive
    pub fn for_primitive(ty: &Type) -> Option<Self> {
        match ty {
            Type::Int8 => Some(Self::Int8),
            Type::Int16 => Some(Self::Int16),
            Type::Int32 => Some(Self::Int32),
            Type::Int64 => Some(Self::Int64),
            Type::UInt8 => Some(Self::UInt8),
            Type::UInt16 => Some(Self::UInt16),
            Type::UInt32 => Some(Self::UInt32),
            Type::UInt64 => Some(Self::UInt64),
            Type::Float32 => Some(Self::Float32),
            Type::Float64 => Some(Self::Float64),
            Type::Boolean => Some(Self::Boolean),
            Type::String => Some(Self::String),
            Type::Optional { inner_type } => match &**inner_type {
                Type::UInt8 => Some(Self::Int64),
                Type::Int8 => Some(Self::Int64),
                Type::UInt16 => Some(Self::Int64),
                Type::Int16 => Some(Self::Int64),
                Type::UInt32 => Some(Self::Int64),
                Type::Int32 => Some(Self::Int64),
                Type::Boolean => Some(Self::Int64),
                Type::Float32 => Some(Self::Int32),
                Type::Float64 => Some(Self::Int64),
                Type::String => Some(Self::NullableString),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn type_kt(&self) -> String {
        match self {
            Self::Int8 => "Byte".into(),
            Self::Int16 => "Short".into(),
            Self::Int32 => "Int".into(),
            Self::Int64 => "Long".into(),
            // Unsigned types are always passed as their signed versions in Kotlin to since that's
            // what JNI expects
            Self::UInt8 => "Byte".into(),
            Self::UInt16 => "Short".into(),
            Self::UInt32 => "Int".into(),
            Self::UInt64 => "Long".into(),
            Self::Float32 => "Float".into(),
            Self::Float64 => "Double".into(),
            Self::Boolean => "Boolean".into(),
            Self::String => "String".into(),
            Self::NullableString => "String?".into(),
        }
    }

    pub fn type_rs(&self) -> String {
        match self {
            Self::Int8 => "i8".into(),
            Self::Int16 => "i16".into(),
            Self::Int32 => "i32".into(),
            Self::Int64 => "i64".into(),
            // JNI only works with signed types
            Self::UInt8 => "i8".into(),
            Self::UInt16 => "i16".into(),
            Self::UInt32 => "i32".into(),
            Self::UInt64 => "i64".into(),
            Self::Float32 => "f32".into(),
            Self::Float64 => "f64".into(),
            Self::Boolean => "bool".into(),
            // JNI uses the `jstring` type, we convert to `String` in the lift/lower functions.
            Self::String | Self::NullableString => "uniffi_jni::jstring".into(),
        }
    }

    pub fn default_kt(&self) -> String {
        match self {
            Self::Int8 | Self::UInt8 => "0.toByte()".into(),
            Self::Int16 | Self::UInt16 => "0.toShort()".into(),
            Self::Int32 | Self::UInt32 => "0".into(),
            Self::Int64 | Self::UInt64 => "0L".into(),
            Self::Float32 => "0.0f".into(),
            Self::Float64 => "0.0".into(),
            Self::Boolean => "false".into(),
            Self::String => "\"\"".into(),
            Self::NullableString => "null".into(),
        }
    }

    pub fn jni_signature(&self) -> &'static str {
        match self {
            Self::UInt8 | Self::Int8 => "B",
            Self::UInt16 | Self::Int16 => "S",
            Self::UInt32 | Self::Int32 => "I",
            Self::UInt64 | Self::Int64 => "J",
            Self::Float32 => "F",
            Self::Float64 => "D",
            Self::Boolean => "Z",
            Self::String | Self::NullableString => "Ljava/lang/String;",
        }
    }

    pub fn jvalue_field(&self) -> &'static str {
        match self {
            Self::UInt8 | Self::Int8 => "b",
            Self::UInt16 | Self::Int16 => "s",
            Self::UInt32 | Self::Int32 => "i",
            Self::UInt64 | Self::Int64 => "j",
            Self::Float32 => "f",
            Self::Float64 => "d",
            Self::Boolean => "z",
            Self::String | Self::NullableString => "l",
        }
    }
}

pub fn create_deconstructable_map(root: &general::Root) -> Result<HashMap<Type, Vec<FfiType>>> {
    let records: HashMap<&Type, &general::Record> = root
        .namespaces
        .values()
        .flat_map(|namespace| {
            namespace
                .type_definitions
                .iter()
                .filter_map(|type_def| match type_def {
                    general::TypeDefinition::Record(rec) => Some((&rec.self_type.ty, rec)),
                    _ => None,
                })
        })
        .collect();

    let mut context = CreateDeconstructableTypeContext {
        deconstructable_types: HashMap::new(),
        visited: HashSet::new(),
        records,
    };

    root.try_visit(|ty: &Type| {
        create_deconstructable_types_recurse(ty, &mut context)?;
        Ok(())
    })?;
    Ok(context.deconstructable_types)
}

/// Context for `create_deconstructable_map_recurse`
struct CreateDeconstructableTypeContext<'a> {
    deconstructable_types: HashMap<Type, Vec<FfiType>>,
    visited: HashSet<&'a Type>,
    records: HashMap<&'a Type, &'a general::Record>,
}

fn create_deconstructable_types_recurse<'a>(
    ty: &'a Type,
    context: &mut CreateDeconstructableTypeContext<'a>,
) -> Result<Option<Vec<FfiType>>> {
    if let Some(deconstructable_type) = context.deconstructable_types.get(ty) {
        return Ok(Some(deconstructable_type.clone()));
    }
    if !context.visited.insert(ty) {
        // We've already visited this record and didn't insert it into the map.
        // This means that either we've already determined there's no strategy
        // or we've just detected a cycle in the dependency graph
        // which means the type is not deconstructable.
        return Ok(None);
    }

    let ffi_types = match ty {
        Type::Record { .. } => {
            let mut field_ffi_types = vec![];
            let rec = context.records.get(ty).ok_or_else(|| {
                anyhow!("create_deconstructable_types_recurse: missing record {ty:?}")
            })?;
            for f in rec.fields.iter() {
                if let Some(ffi_type) = FfiType::for_primitive(&f.ty.ty) {
                    // Primitive field
                    field_ffi_types.push(ffi_type);
                } else if let Some(child_primitives) =
                    create_deconstructable_types_recurse(ty, context)?
                {
                    field_ffi_types.extend(child_primitives.iter());
                } else {
                    // Field can't be deconstructed, give up on this record
                    return Ok(None);
                }
            }
            field_ffi_types
        }
        Type::Optional { inner_type } => {
            match create_deconstructable_types_recurse(inner_type, context)? {
                None => return Ok(None),
                Some(mut ffi_types) => {
                    ffi_types.insert(0, FfiType::Boolean);
                    ffi_types
                }
            }
        }
        // TODO handle more types
        _ => return Ok(None),
    };
    context
        .deconstructable_types
        .insert(ty.clone(), ffi_types.clone());
    Ok(Some(ffi_types))
}
