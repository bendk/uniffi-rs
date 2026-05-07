/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use super::*;

// Maximum number of arguments to use to pass primitive/deconstructable types
const MAX_PRIMITIVE_ARGS: usize = 32;

pub fn map_callable(input: general::Callable, context: &Context) -> Result<Callable> {
    let fully_qualified_name_rs = fully_qualified_name_rs(&input, context)?;
    let result_id = context.get_callback_result_id(&input)?;
    let kind = input.kind.map_node(context)?;
    let mut allocator = FfiArgAllocator::default();
    let receiver = match &kind {
        CallableKind::Method {
            self_type,
            takes_self_by_arc,
            ..
        } => Some(Receiver {
            ty: self_type.clone(),
            strategy: allocator.receiver_strategy_for_type(
                self_type,
                *takes_self_by_arc,
                context,
            )?,
        }),
        CallableKind::VTableMethod { self_type, .. } => Some(Receiver {
            ty: self_type.clone(),
            strategy: allocator.receiver_strategy_for_type(self_type, false, context)?,
        }),
        _ => None,
    };

    let arguments = map_arguments(&mut allocator, input.arguments, context)?;
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
        receiver,
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
        general::CallableKind::Method { self_type, .. }
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

fn map_arguments(
    allocator: &mut FfiArgAllocator,
    inputs: Vec<general::Argument>,
    context: &Context,
) -> Result<Vec<Argument>> {
    let mut mapped = vec![];
    for input in inputs {
        let ty = input.ty.map_node(context)?;
        let strategy = allocator.strategy_for_type(&ty);
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

    pub fn strategy_for_type(&mut self, ty: &TypeNode) -> ArgStrategy {
        match &ty.lowerable {
            Some(lowerable) => match (self.can_lower_args(lowerable), lowerable) {
                (false, _) => ArgStrategy::FfiBuffer,
                (true, LowerableType::Primitive(ffi_type)) => ArgStrategy::Primitive(FfiArgument {
                    name: self.next(),
                    ty: *ffi_type,
                }),
                (true, LowerableType::Deconstructable(primitive_types)) => {
                    ArgStrategy::Deconstruct(
                        primitive_types
                            .iter()
                            .map(|ffi_type| FfiArgument {
                                name: self.next(),
                                ty: *ffi_type,
                            })
                            .collect(),
                    )
                }
            },
            None => ArgStrategy::FfiBuffer,
        }
    }

    pub fn receiver_strategy_for_type(
        &mut self,
        ty: &TypeNode,
        takes_self_by_arc: bool,
        context: &Context,
    ) -> Result<ReceiverStrategy> {
        Ok(match &ty.ty {
            Type::Interface {
                imp,
                orig_name,
                namespace,
                ..
            } if !takes_self_by_arc => {
                let inner_type_name = format!(
                    "{}::{}",
                    context.rust_module_path_for_type(namespace, orig_name)?,
                    names::escape_rust(orig_name),
                );

                if !imp.is_trait_interface() {
                    ReceiverStrategy::InterfaceRef(
                        inner_type_name,
                        FfiArgument {
                            name: self.next(),
                            ty: FfiType::Int64,
                        },
                    )
                } else {
                    ReceiverStrategy::TraitInterfaceRef(
                        inner_type_name,
                        FfiArgument {
                            name: self.next(),
                            ty: FfiType::Int64,
                        },
                        FfiArgument {
                            name: self.next(),
                            ty: FfiType::Int64,
                        },
                    )
                }
            }
            _ => ReceiverStrategy::Arg(self.strategy_for_type(ty)),
        })
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
