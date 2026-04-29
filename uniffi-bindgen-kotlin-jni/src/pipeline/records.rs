/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use super::*;

pub fn map_record(input: general::Record, context: &Context) -> Result<Record> {
    let fields = map_fields(input.fields, context)?;
    let self_type = input.self_type.map_node(context)?;
    let deconstructable = self_type
        .lowerable
        .is_some()
        .then(|| deconstructable(&input.name, &fields))
        .transpose()?;

    Ok(Record {
        fields_kind: input.fields_kind,
        self_type,
        immutable: context.config()?.record_is_immutable(&input.name),
        name: input.name,
        orig_name: input.orig_name,
        uniffi_trait_methods: input.uniffi_trait_methods.map_node(context)?,
        fields,
        deconstructable,
        docstring: input.docstring,
        recursive: input.recursive,
    })
}

pub fn map_fields(fields: Vec<general::Field>, context: &Context) -> Result<Vec<Field>> {
    fields
        .into_iter()
        .enumerate()
        .map(|(i, f)| {
            Ok(Field {
                name: f.name,
                orig_name: f.orig_name,
                index: i,
                ty: f.ty.map_node(context)?,
                default: f.default.map_node(context)?,
                docstring: f.docstring,
            })
        })
        .collect()
}

fn deconstructable(name: &str, fields: &[Field]) -> Result<DeconstructableRecord> {
    let mut source_fields = vec![];
    let mut field_counter = 0..;
    for f in fields.iter() {
        let Some(deconstructable) = &f.ty.lowerable else {
            bail!(
                "Can't generate DeconstructableRecord for {name} ({} is not deconstructable)",
                f.name
            );
        };
        source_fields.push(DeconstructableField {
            name: f.name.clone(),
            orig_name: f.orig_name.clone(),
            index: f.index,
            ty: f.ty.clone(),
            kind: match deconstructable {
                LowerableType::Primitive(ffi_type) => {
                    DeconstructableFieldKind::Primitive(FfiField {
                        index: field_counter.next().unwrap(),
                        ty: *ffi_type,
                    })
                }
                LowerableType::Deconstructable(ffi_types) => DeconstructableFieldKind::Recursive(
                    ffi_types
                        .iter()
                        .map(|ffi_type| FfiField {
                            index: field_counter.next().unwrap(),
                            ty: *ffi_type,
                        })
                        .collect(),
                ),
            },
        })
    }

    Ok(DeconstructableRecord { source_fields })
}
