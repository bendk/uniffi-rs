/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use syn::{spanned::Spanned, PathArguments, TypeParamBound, TypeTraitObject};

use crate::{
    paths::LookupCache, BuiltinItem, Error, Ir, Item, Namespace, RPath, Result, Type, TypeClosure,
};

pub fn maybe_resolve_closure<'ir>(
    item_path: &RPath<'ir>,
    ir: &'ir Ir,
    cache: &mut LookupCache<'ir>,
    tr: &TypeTraitObject,
) -> Result<Option<TypeClosure>> {
    if !tr.bounds.iter().any(|b| match b {
        TypeParamBound::Trait(tb) => {
            item_path.is_builtin(ir, cache, &tb.path, Namespace::Type, BuiltinItem::Fn)
        }
        _ => false,
    }) {
        return Ok(None);
    }

    let mut saw_send = false;
    let mut saw_sync = false;
    let mut closure = TypeClosure {
        argument_types: vec![],
        return_type: Box::new(Type::Unit),
    };

    for bound in tr.bounds.iter() {
        match bound {
            TypeParamBound::Trait(tb) => {
                if item_path.is_builtin(ir, cache, &tb.path, Namespace::Type, BuiltinItem::Fn) {
                    let last = tb.path.segments.last()
                        .ok_or_else(|| Error::internal("maybe_resolve_closure: tb.path.segments empty after `is_builtin()` matched"))?;
                    let PathArguments::Parenthesized(args) = &last.arguments else {
                        return Err(Error::new_syn(
                            item_path.file_id(),
                            syn::Error::new(tb.span(), "Expected parentheses"),
                        ));
                    };
                    for ty in args.inputs.iter() {
                        closure
                            .argument_types
                            .push(item_path.resolve_type(ir, cache, ty)?);
                    }
                    if let syn::ReturnType::Type(_, ty) = &args.output {
                        closure.return_type = Box::new(item_path.resolve_type(ir, cache, ty)?);
                    }
                    continue;
                }
                match item_path
                    .resolve(ir, cache, &tb.path, Namespace::Type)?
                    .item()?
                {
                    Item::Builtin(BuiltinItem::Send) => {
                        saw_send = true;
                    }
                    Item::Builtin(BuiltinItem::Sync) => {
                        saw_sync = true;
                    }
                    _ => {
                        return Err(Error::new_syn(
                            item_path.file_id(),
                            syn::Error::new(tb.span(), "Invalid closure trait bound"),
                        ));
                    }
                }
            }
            TypeParamBound::Lifetime(i) => {
                if i.ident != "static" {
                    return Err(Error::new_syn(
                        item_path.file_id(),
                        syn::Error::new(i.span(), "Closures must be 'static"),
                    ));
                }
            }
            b => {
                return Err(Error::new_syn(
                    item_path.file_id(),
                    syn::Error::new(b.span(), "Invalid closure trait bound"),
                ))
            }
        }
    }
    if !(saw_send && saw_sync) {
        return Err(Error::new_syn(
            item_path.file_id(),
            syn::Error::new(tr.bounds.span(), "Closures must be Send + Sync"),
        ));
    }

    Ok(Some(closure))
}
