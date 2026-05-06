/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use super::*;

pub fn map_callback_interface(
    input: general::CallbackInterface,
    context: &Context,
) -> Result<CallbackInterface> {
    Ok(CallbackInterface {
        self_type: input.self_type.map_node(context)?,
        methods: map_methods(
            &input.name,
            input.vtable.methods.into_iter().map(|m| m.callable),
            context,
        )?,
        name: input.name,
        orig_name: input.orig_name,
        module_path: context.normalize_rust_module_path(&input.module_path)?,
        docstring: input.docstring,
        crate_name: context.current_crate_name()?.to_string(),
        for_trait_interface: false,
    })
}

pub fn map_trait_interface(
    input: general::Interface,
    context: &Context,
) -> Result<CallbackInterface> {
    Ok(CallbackInterface {
        self_type: input.self_type.map_node(context)?,
        methods: map_methods(
            &input.name,
            input.methods.into_iter().map(|m| m.callable),
            context,
        )?,
        name: input.name,
        orig_name: input.orig_name,
        module_path: context.normalize_rust_module_path(&input.module_path)?,
        docstring: input.docstring,
        crate_name: context.current_crate_name()?.to_string(),
        for_trait_interface: true,
    })
}

pub fn interface_for_callback_interface(
    cbi: &general::CallbackInterface,
    context: &Context,
) -> Result<Interface> {
    Ok(Interface {
        name: cbi.name.to_upper_camel_case(),
        methods: cbi.methods.clone().map_node(context)?,
        docstring: cbi.docstring.clone(),
    })
}

fn map_methods(
    interface_name: &str,
    methods: impl Iterator<Item = general::Callable>,
    context: &Context,
) -> Result<Vec<CallbackMethod>> {
    methods
        .map(|callable| {
            let mut callable = callable.map_node(context)?;
            callable.kind = match callable.kind {
                CallableKind::VTableMethod { self_type, .. }
                | CallableKind::Method { self_type, .. } => CallableKind::VTableMethod {
                    self_type,
                    for_callback_interface: true,
                },
                kind => bail!("callbacks::map_methods: invalid CallableKind: {kind:?}"),
            };

            Ok(CallbackMethod {
                dispatch_fn_rs: format!(
                    "uniffi_callback_dispatch_{}_{}_{}",
                    context.namespace_name()?.to_snake_case(),
                    interface_name.to_snake_case(),
                    callable.name.to_snake_case(),
                ),
                dispatch_fn_kt: format!(
                    "callbackInterface{}{}{}",
                    context.namespace_name()?.to_upper_camel_case(),
                    interface_name.to_upper_camel_case(),
                    callable.name.to_upper_camel_case(),
                ),
                jni_signature: jni_signature(&callable)?,
                jni_method_call_name: jni_method_call_name(&callable)?,
                callable,
            })
        })
        .collect()
}

fn jni_signature(callable: &Callable) -> Result<String> {
    let mut args = String::from("J");
    if callable.uses_buffer() {
        // Buffer handle
        args.push('J');
    }
    if callable.is_async {
        // Future handle
        args.push('J');
    } else if callable.return_strategy().is_reconstruct() || callable.throws_type().is_some() {
        // Return value pointer
        args.push('J');
    }
    // Arg for each primitive arg
    for a in callable.ffi_arguments() {
        args.push_str(a.ty.jni_signature());
    }

    let ret = match (callable.is_async, callable.return_strategy()) {
        (false, ReturnStrategy::Primitive(_, ffi_type)) => ffi_type.jni_signature(),
        _ => "V",
    };

    Ok(format!("({args}){ret}"))
}

// Method name to make the call, for the `CachedMethod` and `CachedStaticMethod` types.
fn jni_method_call_name(callable: &Callable) -> Result<String> {
    Ok(match callable
        .return_type()
        .and_then(|type_node| type_node.lowerable.as_ref())
    {
        Some(LowerableType::Primitive(ffi_type)) => match ffi_type {
            FfiType::Int8 | FfiType::UInt8 => "call_byte",
            FfiType::Int16 | FfiType::UInt16 => "call_short",
            FfiType::Int32 | FfiType::UInt32 => "call_int",
            FfiType::Int64 | FfiType::UInt64 => "call_long",
            FfiType::Float32 => "call_float",
            FfiType::Float64 => "call_double",
            FfiType::Boolean => "call_boolean",
            FfiType::String | FfiType::NullableString => "call_object",
        },
        Some(LowerableType::Deconstructable(_)) => "call_object",
        _ => "call_void",
    }
    .into())
}
