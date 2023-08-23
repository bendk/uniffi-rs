/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use super::attributes::{AsyncRuntime, ExportAttributeArguments};
use crate::fnsig::{FnKind, FnSignature, NamedArg};

pub(super) fn gen_fn_scaffolding(
    sig: FnSignature,
    arguments: &ExportAttributeArguments,
) -> syn::Result<TokenStream> {
    if sig.receiver.is_some() {
        return Err(syn::Error::new(
            sig.span,
            "Unexpected self param (Note: uniffi::export must be used on the impl block, not its containing fn's)"
        ));
    }
    if !sig.is_async {
        if let Some(async_runtime) = &arguments.async_runtime {
            return Err(syn::Error::new_spanned(
                async_runtime,
                "this attribute is only allowed on async functions",
            ));
        }
    }
    let metadata_items = sig.metadata_items()?;
    let scaffolding_func = gen_ffi_function(&sig, arguments)?;
    Ok(quote! {
        #scaffolding_func
        #metadata_items
    })
}

pub(super) fn gen_constructor_scaffolding(
    sig: FnSignature,
    arguments: &ExportAttributeArguments,
) -> syn::Result<TokenStream> {
    if sig.receiver.is_some() {
        return Err(syn::Error::new(
            sig.span,
            "constructors must not have a self parameter",
        ));
    }
    if sig.is_async {
        return Err(syn::Error::new(sig.span, "constructors can't be async"));
    }
    let metadata_items = sig.metadata_items()?;
    let scaffolding_func = gen_ffi_function(&sig, arguments)?;
    Ok(quote! {
        #scaffolding_func
        #metadata_items
    })
}

pub(super) fn gen_method_scaffolding(
    sig: FnSignature,
    arguments: &ExportAttributeArguments,
) -> syn::Result<TokenStream> {
    let scaffolding_func = if sig.receiver.is_none() {
        return Err(syn::Error::new(
            sig.span,
            "associated functions are not currently supported",
        ));
    } else {
        gen_ffi_function(&sig, arguments)?
    };

    let metadata_items = sig.metadata_items()?;
    Ok(quote! {
        #scaffolding_func
        #metadata_items
    })
}

// Pieces of code for the scaffolding function
struct ScaffoldingBits {
    /// Self argument of the scaffolding function (which is not tracked by FnSignature)
    self_arg: Option<NamedArg>,
    /// Statements to execute before `rust_fn_call`
    pre_fn_call: TokenStream,
    /// Tokenstream for the call to the actual Rust function
    rust_fn_call: TokenStream,
}

impl ScaffoldingBits {
    fn new_for_function(sig: &FnSignature) -> Self {
        let ident = &sig.ident;
        let param_lifts = sig.lift_exprs();

        Self {
            self_arg: None,
            pre_fn_call: quote! {},
            rust_fn_call: quote! { #ident(#(#param_lifts,)*) },
        }
    }

    fn new_for_method(sig: &FnSignature, self_ident: &Ident, is_trait: bool) -> Self {
        let ident = &sig.ident;
        let self_type = if is_trait {
            quote! { ::std::sync::Arc<dyn #self_ident> }
        } else {
            quote! { ::std::sync::Arc<#self_ident> }
        };
        let ffi_converter = quote! { <#self_type as ::uniffi::FfiConverter<crate::UniFfiTag>> };
        let param_lifts = sig.lift_exprs();

        Self {
            self_arg: Some(NamedArg::new(
                format_ident!("uniffi_self_lowered"),
                self_type,
            )),
            pre_fn_call: quote! {
                let uniffi_self = #ffi_converter::try_lift(uniffi_self_lowered).unwrap_or_else(|err| {
                    ::std::panic!("Failed to convert arg 'self': {}", err)
                });
            },
            rust_fn_call: quote! { uniffi_self.#ident(#(#param_lifts,)*) },
        }
    }

    fn new_for_constructor(sig: &FnSignature, self_ident: &Ident) -> Self {
        let ident = &sig.ident;
        let param_lifts = sig.lift_exprs();

        Self {
            self_arg: None,
            pre_fn_call: quote! {},
            rust_fn_call: quote! { #self_ident::#ident(#(#param_lifts,)*) },
        }
    }
}

/// Generate a scaffolding function
///
/// `pre_fn_call` is the statements that we should execute before the rust call
/// `rust_fn` is the Rust function to call.
fn gen_ffi_function(
    sig: &FnSignature,
    arguments: &ExportAttributeArguments,
) -> syn::Result<TokenStream> {
    let ScaffoldingBits {
        self_arg,
        pre_fn_call,
        rust_fn_call,
    } = match &sig.kind {
        FnKind::Function => ScaffoldingBits::new_for_function(sig),
        FnKind::Method { self_ident } => ScaffoldingBits::new_for_method(sig, self_ident, false),
        FnKind::TraitMethod { self_ident, .. } => {
            ScaffoldingBits::new_for_method(sig, self_ident, true)
        }
        FnKind::Constructor { self_ident } => ScaffoldingBits::new_for_constructor(sig, self_ident),
    };

    let ffi_ident = sig.scaffolding_fn_ident()?;
    let name = &sig.name;
    let return_ty = &sig.return_ty;
    let param_names: Vec<_> = self_arg.iter().chain(&sig.args).map(|a| &a.ident).collect();
    let param_types: Vec<_> = self_arg
        .iter()
        .chain(&sig.args)
        .map(NamedArg::ffi_type)
        .collect();

    Ok(if !sig.is_async {
        quote! {
            #[doc(hidden)]
            #[no_mangle]
            pub extern "C" fn #ffi_ident(
                #(#param_names: #param_types,)*
                call_status: &mut ::uniffi::RustCallStatus,
            ) -> <#return_ty as ::uniffi::FfiConverter<crate::UniFfiTag>>::ReturnType {
                ::uniffi::deps::log::debug!(#name);
                ::uniffi::rust_call(call_status, || {
                    #pre_fn_call
                    <#return_ty as ::uniffi::FfiConverter<crate::UniFfiTag>>::lower_return(#rust_fn_call)
                })
            }
        }
    } else {
        let future_startup_ident = sig.scaffolding_future_method_ident("startup")?;
        let future_free_ident = sig.scaffolding_future_method_ident("free")?;
        let vtable_ident = format_ident!(
            "UNIFFI_FUTURE_VTABLE_{}",
            ffi_ident.to_string().to_ascii_uppercase()
        );

        // Function that maps the output of the async function to the future that we will actually
        // use.  This is needed to handle the `async_runtime` argument.
        let future_mapper = match &arguments.async_runtime {
            None => quote! { ::std::convert::identity },
            Some(AsyncRuntime::Tokio(_)) => quote! { ::uniffi::deps::async_compat::Compat::new },
        };

        // Closure a that wraps the async function to:
        //   - input a single tuple of scaffolding args
        //   - send the result through `future_mapper`
        //
        //  This is used to construct the RustFutureVTable
        let wrapped_rust_fn = quote! {
            |args: (#(#param_types,)*)| {
                let (#(#param_names,)*) = args;
                #pre_fn_call
                #future_mapper(#rust_fn_call)
            }
        };

        quote! {
            const #vtable_ident: ::uniffi::RustFutureVTable<#return_ty, crate::UniFfiTag> = ::uniffi::RustFutureVTable::new(&#wrapped_rust_fn);

            #[doc(hidden)]
            #[no_mangle]
            pub extern "C" fn #ffi_ident(#(#param_names: #param_types,)*) -> ::uniffi::RustFutureHandle {
                ::uniffi::deps::log::debug!(#name);
                // Use the same closure as the RustFutureVTable to make sure everything lines up
                ::uniffi::rust_future_new::<_, _, crate::UniFfiTag>((#wrapped_rust_fn)((#(#param_names,)*)))
            }

            #[doc(hidden)]
            #[no_mangle]
            pub extern "C" fn #future_startup_ident(handle: ::uniffi::RustFutureHandle,
                executor_handle: ::uniffi::ForeignExecutorHandle,
                uniffi_callback: ::uniffi::FutureCallback<<#return_ty as ::uniffi::FfiConverter<crate::UniFfiTag>>::FutureCallbackT>,
                callback_data: *const (),
            )
            {
                unsafe { (#vtable_ident.startup)(handle, executor_handle, callback, callback_data) };
            }

            #[doc(hidden)]
            #[no_mangle]
            pub extern "C" fn #future_free_ident(handle: ::uniffi::RustFutureHandle) {
                unsafe { (#vtable_ident.free)(handle) };
            }
        }
    })
}
