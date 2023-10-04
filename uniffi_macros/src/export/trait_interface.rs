/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};

use crate::{
    export::{attributes::ExportAttributeArguments, gen_method_scaffolding, item::ImplItem},
    fnsig::{FnKind, FnSignature, ReceiverArg},
    object::interface_meta_static_var,
    util::{ident_to_string, tagged_impl_header},
};

pub(super) fn gen_trait_scaffolding(
    mod_path: &str,
    args: ExportAttributeArguments,
    self_ident: Ident,
    items: Vec<ImplItem>,
    udl_mode: bool,
) -> syn::Result<TokenStream> {
    if let Some(rt) = args.async_runtime {
        return Err(syn::Error::new_spanned(rt, "not supported for traits"));
    }

    let name = ident_to_string(&self_ident);
    let foreign_impl_struct = format_ident!("UniFfiTraitForeignImpl{name}");
    let free_fn_ident = Ident::new(
        &uniffi_meta::free_fn_symbol_name(mod_path, &name),
        Span::call_site(),
    );

    let free_tokens = quote! {
        #[doc(hidden)]
        #[no_mangle]
        pub extern "C" fn #free_fn_ident(
            ptr: *const ::std::ffi::c_void,
            call_status: &mut ::uniffi::RustCallStatus
        ) {
            uniffi::rust_call(call_status, || {
                assert!(!ptr.is_null());
                drop(unsafe { ::std::boxed::Box::from_raw(ptr as *mut std::sync::Arc<dyn #self_ident>) });
                Ok(())
            });
        }
    };

    let foreign_impl = foreign_impl(mod_path, &self_ident, &name, &foreign_impl_struct, &items)?;
    let impl_tokens: TokenStream = items
        .into_iter()
        .map(|item| match item {
            ImplItem::Method(sig) => {
                if sig.is_async {
                    return Err(syn::Error::new(
                        sig.span,
                        "async trait methods are not supported",
                    ));
                }
                gen_method_scaffolding(sig, &args, udl_mode)
            }
            _ => unreachable!("traits have no constructors"),
        })
        .collect::<syn::Result<_>>()?;

    let meta_static_var = (!udl_mode).then(|| {
        interface_meta_static_var(&self_ident, true, mod_path)
            .unwrap_or_else(syn::Error::into_compile_error)
    });
    let ffi_converter_tokens = ffi_converter(mod_path, &self_ident, &foreign_impl_struct, false);

    Ok(quote_spanned! { self_ident.span() =>
        #meta_static_var
        #free_tokens
        #foreign_impl
        #ffi_converter_tokens
        #impl_tokens
    })
}

/// Generate a Foreign implementation for the trait
///
/// This generates:
///    * A `repr(C)` VTable struct where each field is the FFI function for the trait method.
///    * A FFI function for foreign code to set their VTable for the interface
///    * An implementation of the trait using that VTable
pub(super) fn foreign_impl(
    mod_path: &str,
    trait_ident: &Ident,
    name: &str,
    foreign_impl_struct: &Ident,
    items: &[ImplItem],
) -> syn::Result<TokenStream> {
    let vtable_type = format_ident!("UniFfiTraitVtable{name}");
    let vtable_cell = format_ident!("UNIFFI_TRAIT_CELL_{}", name.to_uppercase());
    let init_callback = Ident::new(
        &uniffi_meta::init_trait_callback_fn_symbol_name(mod_path, name),
        Span::call_site(),
    );

    let methods = items
        .into_iter()
        .map(|item| match item {
            ImplItem::Constructor(sig) => Err(syn::Error::new(
                sig.span,
                "Constructors not allowed in trait interfaces",
            )),
            ImplItem::Method(sig) => Ok(sig),
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let vtable_fields = methods.iter()
        .map(|sig| {
            let ident = &sig.ident;
            let params = sig.scaffolding_params();
            let lower_return = sig.lower_return_impl();
            quote! {
                #ident: extern "C" fn(handle: *const ::std::os::raw::c_void, #(#params,)* &mut #lower_return::ReturnType, &mut ::uniffi::RustCallStatus),
            }
        });

    let trait_impl_methods = methods
        .iter()
        .map(|sig| gen_method_impl(sig, &vtable_cell))
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        struct #vtable_type {
            #(#vtable_fields)*
            uniffi_free: extern "C" fn(handle: *const ::std::os::raw::c_void),
        }

        static #vtable_cell: ::uniffi::UniffiForeignPointerCell::<#vtable_type> = ::uniffi::UniffiForeignPointerCell::<#vtable_type>::new();

        #[no_mangle]
        extern "C" fn #init_callback(vtable: ::std::ptr::NonNull<#vtable_type>) {
            #vtable_cell.set(vtable);
        }

        #[derive(Debug)]
        struct #foreign_impl_struct {
            handle: *const ::std::os::raw::c_void,
        }

        // Implement Send + Sync to declare that we handle the pointer correctly and you can safely
        // share this type between threads.
        unsafe impl Send for #foreign_impl_struct { }
        unsafe impl Sync for #foreign_impl_struct { }

        impl #trait_ident for #foreign_impl_struct {
            #(#trait_impl_methods)*
        }

        impl ::std::ops::Drop for #foreign_impl_struct {
            fn drop(&mut self) {
                let vtable = #vtable_cell.get();
                (vtable.uniffi_free)(self.handle);
            }
        }
    })
}

/// Generate a single method for for [foreign_impl].  This implements a trait method by invoking a
/// foreign-supplied callback.
fn gen_method_impl(sig: &FnSignature, vtable_cell: &Ident) -> syn::Result<TokenStream> {
    let FnSignature {
        ident,
        return_ty,
        kind,
        receiver,
        name,
        span,
        ..
    } = sig;

    if !matches!(kind, FnKind::TraitMethod { .. }) {
        return Err(syn::Error::new(
            *span,
            format!(
                "Internal UniFFI error: Unexpected function kind for callback interface {name}: {kind:?}",
            ),
        ));
    }

    let self_param = match receiver {
        Some(ReceiverArg::Ref) => quote! { &self },
        Some(ReceiverArg::Arc) => quote! { self: Arc<Self> },
        None => {
            return Err(syn::Error::new(
                *span,
                "callback interface methods must take &self as their first argument",
            ));
        }
    };

    let params = sig.params();
    let lower_exprs = sig.args.iter().map(|a| {
        let lower_impl = a.lower_impl();
        let ident = &a.ident;
        quote! { #lower_impl::lower(#ident) }
    });

    let lift_return = sig.lift_return_impl();

    Ok(quote! {
        fn #ident(#self_param, #(#params),*) -> #return_ty {
            let vtable = #vtable_cell.get();
            let mut uniffi_call_status = ::uniffi::RustCallStatus::new();
            let mut return_value: #lift_return::ReturnType = ::uniffi::FfiDefault::ffi_default();
            (vtable.#ident)(self.handle, #(#lower_exprs,)* &mut return_value, &mut uniffi_call_status);
            #lift_return::lift_foreign_return(return_value, uniffi_call_status)
        }
    })
}

pub(crate) fn ffi_converter(
    mod_path: &str,
    trait_ident: &Ident,
    foreign_impl_struct: &Ident,
    udl_mode: bool,
) -> TokenStream {
    let impl_spec = tagged_impl_header("FfiConverterArc", &quote! { dyn #trait_ident }, udl_mode);
    let lift_ref_impl_spec = tagged_impl_header("LiftRef", &quote! { dyn #trait_ident }, udl_mode);
    let name = ident_to_string(trait_ident);

    quote! {
        // All traits must be `Sync + Send`. The generated scaffolding will fail to compile
        // if they are not, but unfortunately it fails with an unactionably obscure error message.
        // By asserting the requirement explicitly, we help Rust produce a more scrutable error message
        // and thus help the user debug why the requirement isn't being met.
        uniffi::deps::static_assertions::assert_impl_all!(dyn #trait_ident: Sync, Send);

        unsafe #impl_spec {
            type FfiType = *const ::std::os::raw::c_void;

            fn lower(obj: ::std::sync::Arc<Self>) -> Self::FfiType {
                ::std::boxed::Box::into_raw(::std::boxed::Box::new(obj)) as *const ::std::os::raw::c_void
            }

            fn write(obj: ::std::sync::Arc<Self>, buf: &mut Vec<u8>) {
                ::uniffi::deps::static_assertions::const_assert!(::std::mem::size_of::<*const ::std::ffi::c_void>() <= 8);
                ::uniffi::deps::bytes::BufMut::put_u64(
                    buf,
                    <Self as ::uniffi::FfiConverterArc<crate::UniFfiTag>>::lower(obj) as u64,
                );
            }

            fn try_lift(v: Self::FfiType) -> ::uniffi::Result<::std::sync::Arc<Self>> {
                Ok(Arc::new(#foreign_impl_struct { handle: v }))
            }

            fn try_read(buf: &mut &[u8]) -> ::uniffi::Result<::std::sync::Arc<Self>> {
                ::uniffi::deps::static_assertions::const_assert!(::std::mem::size_of::<*const ::std::ffi::c_void>() <= 8);
                ::uniffi::check_remaining(buf, 8)?;
                <Self as ::uniffi::FfiConverterArc<crate::UniFfiTag>>::try_lift(
                    ::uniffi::deps::bytes::Buf::get_u64(buf) as Self::FfiType)
            }

            const TYPE_ID_META: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(::uniffi::metadata::codes::TYPE_INTERFACE)
                .concat_str(#mod_path)
                .concat_str(#name)
                .concat_bool(true);
        }

        unsafe #lift_ref_impl_spec {
            type LiftType = ::std::sync::Arc<dyn #trait_ident>;
        }
    }
}
