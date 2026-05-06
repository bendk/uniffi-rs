/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use super::*;

// Maximum number of arguments to use to pass primitive/deconstructable types
const MAX_PRIMITIVE_ARGS: usize = 32;

pub fn map_callable(input: general::Callable, context: &Context) -> Result<Callable> {
    let fully_qualified_name_rs = fully_qualified_name_rs(&input, context)?;
    let result_id = context.get_callback_result_id(&input)?;
    let arguments = map_arguments(input.arguments, context)?;
    let kind = input.kind.map_node(context)?;
    let result = CallableResult {
        for_callback: kind.is_callback_method(),
        return_type: input.return_type.ty.map_node(context)?,
        throws_type: input.throws_type.ty.map_node(context)?,
        id: result_id,
    };
    Ok(Callable {
        kind,
        name: input.name,
        orig_name: input.orig_name,
        is_async: input.async_data.is_some(),
        arguments,
        result,
        fully_qualified_name_rs,
    })
}

fn fully_qualified_name_rs(callable: &general::Callable, context: &Context) -> Result<String> {
    match &callable.kind {
        general::CallableKind::Function => {
            let module_path = context
                .rust_module_path_for_func(context.namespace_name()?, &callable.orig_name)?;
            Ok(format!(
                "{module_path}::{}",
                names::escape_rust(&callable.orig_name)
            ))
        }
        general::CallableKind::Method { self_type, .. }
        | general::CallableKind::Constructor { self_type, .. }
        | general::CallableKind::VTableMethod { self_type, .. } => {
            fully_qualified_method_name_rs(&self_type.ty, callable, context)
        }
    }
}

fn fully_qualified_method_name_rs(
    self_ty: &Type,
    callable: &general::Callable,
    context: &Context,
) -> Result<String> {
    let Some(namespace) = self_ty.namespace() else {
        bail!("Invalid callable self type: {self_ty:?}");
    };
    let Some(name) = self_ty.orig_name() else {
        bail!("Invalid callable self type: {self_ty:?}");
    };
    let module_path = context.rust_module_path_for_type(namespace, name)?;
    let Some(self_name) = self_ty.orig_name() else {
        bail!("Invalid Callable self type: {:?}", callable.kind);
    };
    Ok(format!(
        "{module_path}::{}::{}",
        names::escape_rust(self_name),
        names::escape_rust(&callable.orig_name)
    ))
}

pub fn function_jni_method_name(func: &general::Function, context: &Context) -> Result<String> {
    Ok(format!(
        "function{}{}",
        context.current_crate_name()?.to_upper_camel_case(),
        func.callable.name.to_upper_camel_case()
    ))
}

pub fn constructor_jni_method_name(
    cons: &general::Constructor,
    context: &Context,
) -> Result<String> {
    let self_ty = match &cons.callable.kind {
        general::CallableKind::Constructor { self_type, .. } => self_type,
        _ => bail!("Invalid method callable kind: {:?}", cons.callable.kind),
    };
    let Some(self_name) = self_ty.ty.name() else {
        bail!("Invalid method callable kind: {:?}", cons.callable.kind);
    };
    Ok(format!(
        "constructor{}{}{}",
        context.current_crate_name()?.to_upper_camel_case(),
        self_name.to_upper_camel_case(),
        cons.callable.name.to_upper_camel_case()
    ))
}

pub fn method_jni_method_name(meth: &general::Method, context: &Context) -> Result<String> {
    let self_ty = match &meth.callable.kind {
        general::CallableKind::Method { self_type }
        | general::CallableKind::VTableMethod { self_type, .. } => self_type,
        _ => bail!("Invalid method callable kind: {:?}", meth.callable.kind),
    };
    let Some(self_name) = self_ty.ty.name() else {
        bail!("Invalid method callable kind: {:?}", meth.callable.kind);
    };
    Ok(format!(
        "method{}{}{}",
        context.current_crate_name()?.to_upper_camel_case(),
        self_name.to_upper_camel_case(),
        meth.callable.name.to_upper_camel_case()
    ))
}

fn map_arguments(inputs: Vec<general::Argument>, context: &Context) -> Result<Vec<Argument>> {
    let mut mapped = vec![];
    let mut allocator = FfiArgAllocator::default();
    for input in inputs {
        let ty = input.ty.map_node(context)?;
        let strategy = match &ty.lowerable {
            Some(lowerable) => match (allocator.can_lower_args(lowerable), lowerable) {
                (false, _) => ArgStrategy::FfiBuffer,
                (true, LowerableType::Primitive(ffi_type)) => ArgStrategy::Primitive(FfiArgument {
                    name: allocator.next(),
                    ty: ffi_type.clone(),
                }),
                (true, LowerableType::Deconstructable(primitive_types)) => {
                    ArgStrategy::Deconstruct(
                        primitive_types
                            .iter()
                            .map(|ffi_type| FfiArgument {
                                name: allocator.next(),
                                ty: ffi_type.clone(),
                            })
                            .collect(),
                    )
                }
            },
            None => ArgStrategy::FfiBuffer,
        };
        mapped.push(Argument {
            name: input.name,
            orig_name: input.orig_name,
            ty,
            by_ref: input.by_ref,
            optional: input.optional,
            default: input.default.map_node(context)?,
            strategy,
        });
    }
    Ok(mapped)
}

/// Generates argument names for FFI arguments that we're passing
#[derive(Default)]
pub struct FfiArgAllocator(usize);

impl FfiArgAllocator {
    pub fn next(&mut self) -> String {
        let i = self.0;
        self.0 += 1;
        format!("uniffi_arg_{i}")
    }

    pub fn can_lower_args(&self, lowerable: &LowerableType) -> bool {
        match lowerable {
            LowerableType::Primitive(_) => self.0 < MAX_PRIMITIVE_ARGS,
            LowerableType::Deconstructable(ffi_types) => {
                (self.0 + ffi_types.len()) <= MAX_PRIMITIVE_ARGS
            }
        }
    }
}
