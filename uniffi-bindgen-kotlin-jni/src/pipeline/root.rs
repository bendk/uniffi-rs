/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::mem;

use uniffi_bindgen::pipeline::general::sort::sort_type_definitions_ignore_container_children;

use super::*;

pub fn map_root(mut input: general::Root, context: &Context) -> Result<Root> {
    let mut context = context.clone();
    // Re-sort the type definitions with sort_type_definitions_ignore_container_children.
    //
    // It allows us to handle recursive types when calculating FFI layouts, since:
    //   - box/vec/set have static FFI layouts, we don't care about the children
    //   - Rust requires that one field is a box/vec/set when types are recursive.
    for namespace in input.namespaces.values_mut() {
        namespace.type_definitions =
            sort_type_definitions_ignore_container_children(namespace.type_definitions.drain(..));
    }

    context.update_from_root(&input)?;

    // Map packages, ensuring that type definitions only appear in one of them
    let mut type_definitions_seen = HashSet::new();
    let mut builtin_type_definitions = vec![];
    for namespace in input.namespaces.values_mut() {
        for type_def in mem::take(&mut namespace.type_definitions) {
            // Filter out type_definitions we've already seen
            let type_id = match &type_def {
                general::TypeDefinition::Record(r) => r.self_type.id,
                general::TypeDefinition::Enum(e) => e.self_type.id,
                general::TypeDefinition::Optional(o) => o.self_type.id,
                general::TypeDefinition::Sequence(s) => s.self_type.id,
                general::TypeDefinition::Map(m) => m.self_type.id,
                general::TypeDefinition::Set(s) => s.self_type.id,
                general::TypeDefinition::Interface(c) => c.self_type.id,
                general::TypeDefinition::Custom(c) => c.self_type.id,
                general::TypeDefinition::CallbackInterface(c) => c.self_type.id,
                general::TypeDefinition::Box(b) => b.self_type.id,
                general::TypeDefinition::Simple(tn) => tn.id,
                // uniffi-bindgen-kotlin-jni doesn't generate any code for external types.
                // For example, it fully qualifies all type names so they don't need to be imported.
                general::TypeDefinition::External(_) => continue,
            };
            if type_definitions_seen.insert(type_id) {
                continue;
            }
            // builtin type definitions belong to the root node
            if type_def.self_type().has_user_type() {
                builtin_type_definitions.push(type_def);
            } else {
                namespace.type_definitions.push(type_def);
            }
        }
    }

    let packages = Vec::from_iter(input.namespaces.map_node(&context)?.into_values());
    let builtin_type_definitions = packages::map_type_definitions(builtin_type_definitions, &context)?;

    Ok(Root {
        cdylib: input.cdylib,
        builtin_type_definitions,
        packages,
    })
}

impl Root {
    pub fn cdylib_name(&self) -> Result<String> {
        let config_names: IndexSet<_> = self
            .packages
            .iter()
            .filter_map(|p| p.config.cdylib_name.as_deref())
            .collect();
        Ok(match config_names.len() {
            0 => match &self.cdylib {
                Some(name) => name.to_string(),
                None => bail!("Unknown cdylib name.  Use `src:[crate_name]` to generate bindings or set it in a `uniffi.toml` config"),
            }
            1 => config_names.into_iter().next().unwrap().to_string(),
            _ => bail!("Conflicting cdylib names in `uniffi.toml` files: {:?}", Vec::from_iter(config_names)),
        })
    }

    pub fn jni_methods(&self) -> impl Iterator<Item = (&str, JniMethodKind, &Callable)> {
        self.packages.iter().flat_map(Package::jni_methods)
    }

    pub fn type_definitions(&self) -> impl Iterator<Item = &TypeDefinition> {
        self.builtin_type_definitions
            .iter()
            .chain(
                self.packages
                    .iter()
                    .flat_map(|p| &p.type_definitions)
            )
    }

    /// Types where we need to call the Kotlin lift function from Rust
    ///
    /// We need to do this for:
    ///  * non-primitive types that are returned
    ///  * error types that get thrown from Rust functions.
    ///  * the inner type of boxed types
    pub fn lift_kt_from_rust_types(&self) -> impl Iterator<Item = &TypeNode> {
        let mut seen = HashSet::new();
        let mut type_nodes = vec![];
        self.visit(|callable: &Callable| {
            if let Some(return_type) = callable.return_type() {
                if !return_type.lowers_to_primitive() && seen.insert(return_type.id) {
                    type_nodes.push(return_type);
                }
            }
            if let Some(throws_type) = callable.throws_type() {
                if seen.insert(throws_type.id) {
                    type_nodes.push(throws_type);
                }
            }
        });
        self.visit(|b: &BoxedType| {
            if seen.insert(b.inner.id) {
                type_nodes.push(&b.inner);
            }
        });
        type_nodes.into_iter()
    }

    pub fn rust_async_callable_results(&self) -> impl Iterator<Item = &CallableResult> {
        self.iter_callable_results_that_match(|c| c.is_for_rust_function() && c.is_async)
    }

    pub fn kotlin_sync_callable_results(&self) -> impl Iterator<Item = &CallableResult> {
        self.iter_callable_results_that_match(|c| c.is_for_kotlin_function() && !c.is_async)
    }

    pub fn kotlin_async_callable_results(&self) -> impl Iterator<Item = &CallableResult> {
        self.iter_callable_results_that_match(|c| c.is_for_kotlin_function() && c.is_async)
    }

    fn iter_callable_results_that_match(
        &self,
        filter: impl Fn(&Callable) -> bool,
    ) -> impl Iterator<Item = &CallableResult> {
        let mut seen = HashSet::new();
        let mut results = vec![];
        self.visit(|callable: &Callable| {
            if filter(callable) && seen.insert(callable.result.id()) {
                results.push(&callable.result);
            }
        });
        results.into_iter()
    }

    pub fn classes(&self) -> impl Iterator<Item = &Class> {
        self.packages.iter().flat_map(Package::classes)
    }

    pub fn boxes(&self) -> impl Iterator<Item = &BoxedType> {
        self.packages.iter().flat_map(Package::boxes)
    }

    pub fn disable_java_cleaner(&self) -> bool {
        // Try to merge the different config values as best we can.
        //
        // Maybe we can leverage https://github.com/mozilla/uniffi-rs/issues/2866 to improve this
        self.packages.iter().any(|p| p.config.disable_java_cleaner)
    }

    pub fn enable_android_cleaner(&self) -> bool {
        // Try to merge the different config values as best we can.
        //
        // Maybe we can leverage https://github.com/mozilla/uniffi-rs/issues/2866 to improve this
        self.packages.iter().any(|p| p.config.android_cleaner())
    }
}
