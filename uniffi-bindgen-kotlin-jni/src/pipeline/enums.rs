/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use super::*;

pub fn map_enum(en: general::Enum, context: &Context) -> Result<Enum> {
    let mut context = context.clone();
    context.update_from_enum(&en);
    let discr_type = en.discr_type.map_node(&context)?;

    let mut base_classes = vec![];
    let self_type = en.self_type.map_node(&context)?;
    let kotlin_kind = if matches!(en.shape, EnumShape::Error { flat: true }) {
        KotlinEnumKind::FlatError
    } else if self_type.is_used_as_error || !en.is_flat {
        if self_type.is_used_as_error {
            base_classes.push("Exception()".to_string());
        }
        if en.uniffi_trait_methods.ord_cmp.is_some() {
            base_classes.push(format!("Comparable<{}>", self_type.type_kt));
        }
        KotlinEnumKind::SealedClass
    } else {
        KotlinEnumKind::EnumClass {
            discr_type: en.discr_specified.then(|| discr_type.type_kt.clone()),
        }
    };

    let variants = en.variants.map_node(&context)?;
    let lowerable = match &self_type.lowerable {
        Some(LowerableType::Primitive(_)) => Some(LowerableEnum::Primitive),
        Some(LowerableType::Deconstructable(ffi_types)) => Some(LowerableEnum::Deconstructable(
            deconstructable(&en.name, &variants, ffi_types)?,
        )),
        None => None,
    };

    Ok(Enum {
        is_flat: en.is_flat,
        use_entries: context.config()?.use_enum_entries(),
        self_type,
        discr_type,
        discr_specified: en.discr_specified,
        variants,
        name: en.name,
        orig_name: en.orig_name,
        base_classes,
        uniffi_trait_methods: en.uniffi_trait_methods.map_node(&context)?,
        shape: en.shape,
        kotlin_kind,
        docstring: en.docstring,
        recursive: en.recursive,
        lowerable,
    })
}

pub fn map_variant(variant: general::Variant, context: &Context) -> Result<Variant> {
    let en = context.current_enum()?;
    let name_kt = if !en.is_flat || matches!(en.shape, EnumShape::Error { flat: true }) {
        names::class_name_kt(&variant.name, en.self_type.is_used_as_error)
    } else {
        format!("`{}`", variant.name.to_shouty_snake_case())
    };
    let discr: LiteralNode = variant.discr.map_node(context)?;

    Ok(Variant {
        name_kt,
        name: variant.name,
        orig_name: variant.orig_name,
        discr,
        fields_kind: variant.fields_kind,
        fields: records::map_fields(variant.fields, context)?,
        docstring: variant.docstring,
    })
}

fn deconstructable(
    name: &str,
    variants: &[Variant],
    all_ffi_types: &[FfiType],
) -> Result<DeconstructableEnum> {
    let all_ffi_fields: Vec<FfiField> = all_ffi_types
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, ty)| FfiField { index, ty })
        .collect();

    let mut deconstructable_variants = vec![];
    for v in variants {
        let mut field_finder = FfiFieldFinder::new(name.to_string(), &all_ffi_fields);
        let mut source_fields = vec![];
        for f in v.fields.iter() {
            let Some(deconstructable) = &f.ty.lowerable else {
                bail!(
                    "Can't generate DeconstructableRecord for {name} ({}::{} is not deconstructable)",
                    v.name, f.name
                );
            };
            source_fields.push(DeconstructableField {
                name: f.name.clone(),
                orig_name: f.orig_name.clone(),
                index: f.index,
                ty: f.ty.clone(),
                kind: match deconstructable {
                    LowerableType::Primitive(ffi_type) => {
                        DeconstructableFieldKind::Primitive(field_finder.find(*ffi_type)?)
                    }
                    LowerableType::Deconstructable(ffi_types) => {
                        DeconstructableFieldKind::Recursive(
                            ffi_types
                                .iter()
                                .map(|ffi_type| field_finder.find(*ffi_type))
                                .collect::<Result<Vec<_>>>()?,
                        )
                    }
                },
            })
        }

        let enum_ffi_field_sources =
            create_field_sources(field_finder.available_fields, &source_fields);

        deconstructable_variants.push(DeconstructableVariant {
            name_kt: v.name_kt.clone(),
            orig_name: v.orig_name.clone(),
            fields_kind: v.fields_kind.clone(),
            source_fields,
            enum_ffi_field_sources,
        })
    }
    Ok(DeconstructableEnum {
        variants: deconstructable_variants,
        ffi_fields: all_ffi_fields,
    })
}

/// Find FFI fields for an Enum variant.
///
/// Finds a FfiField for a variant from the total fields available for the enum.
struct FfiFieldFinder {
    name: String,
    available_fields: Vec<FfiField>,
}

impl FfiFieldFinder {
    fn new(name: String, all_fields: &[FfiField]) -> Self {
        Self {
            name,
            // Skip the first FfiField, which is used for the variant index
            available_fields: all_fields.iter().skip(1).cloned().collect(),
        }
    }

    fn find(&mut self, ffi_type: FfiType) -> Result<FfiField> {
        let ffi_type = match ffi_type {
            // Strings must be nullable (see DESIGN.md for details)
            FfiType::String => FfiType::NullableString,
            ffi_type => ffi_type,
        };

        match self.available_fields.iter().position(|f| f.ty == ffi_type) {
            None => bail!(
                "Can't find FfiField for deconstructable enum: {}: {ffi_type:?}",
                self.name
            ),
            Some(index) => Ok(self.available_fields.swap_remove(index)),
        }
    }
}

/// Create an `EnumFfiFieldSource` vec
///
/// For each FFI field for the enum as a while, there's an entry describing how it's used for a
/// particular variant.
fn create_field_sources(
    unused_fields: Vec<FfiField>,
    variant_fields: &[DeconstructableField],
) -> Vec<EnumFfiFieldSource> {
    let mut field_sources: Vec<(usize, EnumFfiFieldSource)> = unused_fields
        .into_iter()
        .map(|ffi_field| {
            (
                ffi_field.index,
                EnumFfiFieldSource::Default {
                    ffi_type: ffi_field.ty,
                },
            )
        })
        .collect();
    for (i, f) in variant_fields.iter().enumerate() {
        match &f.kind {
            DeconstructableFieldKind::Primitive(ffi_field) => field_sources.push((
                ffi_field.index,
                EnumFfiFieldSource::Primitive { source_field: i },
            )),
            DeconstructableFieldKind::Recursive(ffi_fields) => {
                for f in ffi_fields.iter() {
                    field_sources.push((
                        f.index,
                        EnumFfiFieldSource::Recursive {
                            source_field: i,
                            index: f.index,
                        },
                    ))
                }
            }
        }
    }
    field_sources.sort_unstable_by_key(|(index, _)| *index);
    field_sources
        .into_iter()
        .map(|(_, source)| source)
        .collect()
}
