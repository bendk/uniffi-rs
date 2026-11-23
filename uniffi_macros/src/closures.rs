/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    PathArguments, TraitBound,
};

pub struct Closure {
    argument_types: Vec<syn::Type>,
    return_type: syn::ReturnType,
}

impl Parse for Closure {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let tb: TraitBound = input.parse()?;
        let span = tb.span();
        let expected_msg = "Expected Fn([arg_types]) -> [return_type]";
        let Some(seg) = tb.path.segments.into_iter().next_back() else {
            return Err(syn::Error::new(span, expected_msg));
        };
        if seg.ident != "Fn" {
            return Err(syn::Error::new(span, expected_msg));
        }
        let PathArguments::Parenthesized(args) = seg.arguments else {
            return Err(syn::Error::new(
                span,
                "Expected Fn([arg_types]) -> [return_type]",
            ));
        };
        Ok(Self {
            argument_types: args.inputs.into_iter().collect(),
            return_type: args.output,
        })
    }
}

pub fn expand_closure(_f: Closure) -> syn::Result<TokenStream> {
    // TODO: implement scaffolding function
    Ok(quote! {})
}
