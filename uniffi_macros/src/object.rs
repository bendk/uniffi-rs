use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::DeriveInput;
use uniffi_meta::free_fn_symbol_name;

use crate::util::{
    create_metadata_items, derive_ffi_traits, ident_to_string, mod_path, tagged_impl_header,
};

pub fn expand_object(input: DeriveInput, udl_mode: bool) -> syn::Result<TokenStream> {
    let module_path = mod_path()?;
    let ident = &input.ident;
    let name = ident_to_string(ident);
    let free_fn_ident = Ident::new(&free_fn_symbol_name(&module_path, &name), Span::call_site());
    let meta_static_var = (!udl_mode).then(|| {
        interface_meta_static_var(ident, false, &module_path)
            .unwrap_or_else(syn::Error::into_compile_error)
    });
    let interface_impl = interface_impl(ident, udl_mode);

    Ok(quote! {
        #[doc(hidden)]
        #[no_mangle]
        pub extern "C" fn #free_fn_ident(
            handle: ::uniffi::Handle,
            call_status: &mut ::uniffi::RustCallStatus
        ) {
            uniffi::rust_call(call_status, || {
                <#ident as ::uniffi::SlabAlloc<crate::UniFfiTag>>::remove(handle);
                Ok(())
            });
        }

        #interface_impl
        #meta_static_var
    })
}

pub(crate) fn interface_impl(ident: &Ident, udl_mode: bool) -> TokenStream {
    let name = ident_to_string(ident);
    let impl_spec = tagged_impl_header("FfiConverterArc", ident, udl_mode);
    let lift_ref_impl_spec = tagged_impl_header("LiftRef", ident, udl_mode);
    let derive_ffi_traits = derive_ffi_traits(ident, udl_mode, &["SlabAlloc"]);
    let mod_path = match mod_path() {
        Ok(p) => p,
        Err(e) => return e.into_compile_error(),
    };

    quote! {
        // All Object structs must be `Sync + Send`. The generated scaffolding will fail to compile
        // if they are not, but unfortunately it fails with an unactionably obscure error message.
        // By asserting the requirement explicitly, we help Rust produce a more scrutable error message
        // and thus help the user debug why the requirement isn't being met.
        uniffi::deps::static_assertions::assert_impl_all!(#ident: Sync, Send);

        #derive_ffi_traits

        #[doc(hidden)]
        #[automatically_derived]
        /// Support for passing reference-counted shared objects via the FFI.
        ///
        /// To avoid dealing with complex lifetime semantics over the FFI, any data passed
        /// by reference must be encapsulated in an `Arc`, and must be safe to share
        /// across threads.
        unsafe #impl_spec {
            type FfiType = ::uniffi::Handle;

            /// When lowering, we have an owned `Arc` and we transfer that ownership
            /// to the foreign-language code, "leaking" it out of Rust's ownership system
            /// as a raw pointer. This works safely because we have unique ownership of `self`.
            /// The foreign-language code is responsible for freeing this by calling the
            /// `ffi_object_free` FFI function provided by the corresponding UniFFI type.
            ///
            /// Safety: when freeing the resulting pointer, the foreign-language code must
            /// call the destructor function specific to the type `T`. Calling the destructor
            /// function for other types may lead to undefined behaviour.
            fn lower(obj: ::std::sync::Arc<Self>) -> Self::FfiType {
                <#ident as ::uniffi::SlabAlloc<crate::UniFfiTag>>::insert(obj)
            }

            /// When lifting, we receive a "borrow" of the `Arc` that is owned by
            /// the foreign-language code, and make a clone of it for our own use.
            ///
            /// Safety: the provided value must be a pointer previously obtained by calling
            /// the `lower()` or `write()` method of this impl.
            fn try_lift(v: Self::FfiType) -> ::uniffi::Result<::std::sync::Arc<Self>> {
                Ok(<#ident as ::uniffi::SlabAlloc<crate::UniFfiTag>>::get_clone(v))
            }

            /// When writing as a field of a complex structure, make a clone and transfer ownership
            /// of it to the foreign-language code by writing its pointer into the buffer.
            /// The foreign-language code is responsible for freeing this by calling the
            /// `ffi_object_free` FFI function provided by the corresponding UniFFI type.
            ///
            /// Safety: when freeing the resulting pointer, the foreign-language code must
            /// call the destructor function specific to the type `T`. Calling the destructor
            /// function for other types may lead to undefined behaviour.
            fn write(obj: ::std::sync::Arc<Self>, buf: &mut Vec<u8>) {
                ::uniffi::deps::bytes::BufMut::put_i64(buf, <Self as ::uniffi::FfiConverterArc<crate::UniFfiTag>>::lower(obj).as_raw())
            }

            /// When reading as a field of a complex structure, we receive a "borrow" of the `Arc`
            /// that is owned by the foreign-language code, and make a clone for our own use.
            ///
            /// Safety: the buffer must contain a pointer previously obtained by calling
            /// the `lower()` or `write()` method of this impl.
            fn try_read(buf: &mut &[u8]) -> ::uniffi::Result<::std::sync::Arc<Self>> {
                ::uniffi::check_remaining(buf, 8)?;
                <Self as ::uniffi::FfiConverterArc<crate::UniFfiTag>>::try_lift(
                    ::uniffi::Handle::from_raw(::uniffi::deps::bytes::Buf::get_i64(buf))
                )
            }

            const TYPE_ID_META: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(::uniffi::metadata::codes::TYPE_INTERFACE)
                .concat_str(#mod_path)
                .concat_str(#name)
                .concat_bool(false);
        }

        unsafe #lift_ref_impl_spec {
            type LiftType = ::std::sync::Arc<Self>;
        }
    }
}

pub(crate) fn interface_meta_static_var(
    ident: &Ident,
    is_trait: bool,
    module_path: &str,
) -> syn::Result<TokenStream> {
    let name = ident_to_string(ident);
    Ok(create_metadata_items(
        "interface",
        &name,
        quote! {
                ::uniffi::MetadataBuffer::from_code(::uniffi::metadata::codes::INTERFACE)
                    .concat_str(#module_path)
                    .concat_str(#name)
                    .concat_bool(#is_trait)
        },
        None,
    ))
}
