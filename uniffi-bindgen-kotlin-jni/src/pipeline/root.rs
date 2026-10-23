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

    let mut packages = Vec::from_iter(input.namespaces.map_node(&context)?.into_values());

    // Ensure that type definitions only appear the first package that uses them.
    let mut builtin_type_definitions = vec![];
    let mut type_id_to_jvm_class = HashMap::new();
    for package in packages.iter_mut() {
        for type_def in mem::take(&mut package.type_definitions) {
            let type_node = type_def.self_type();
            if type_id_to_jvm_class.contains_key(&type_node.id) {
                continue;
            }
            if type_node.ty.has_user_type() {
                type_id_to_jvm_class.insert(type_node.id, package.ffi_class_name.to_string());
                package.type_definitions.push(type_def);
            } else {
                type_id_to_jvm_class.insert(type_node.id, "UniffiKt".to_string());
                builtin_type_definitions.push(type_def);
            }
        }
    }

    let mut root = Root {
        cdylib: input.cdylib,
        builtin_type_definitions,
        packages,
        scaffolding_lift_vars: vec![],
    };
    root.populate_scaffolding_lift_vars(type_id_to_jvm_class)?;
    Ok(root)
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
            .chain(self.packages.iter().flat_map(|p| &p.type_definitions))
    }

    fn populate_scaffolding_lift_vars(
        &mut self,
        jvm_class_map: HashMap<u64, String>,
    ) -> Result<()> {
        let mut seen = HashSet::new();
        let mut lift_vars = vec![];
        let mut add_lift_var = |type_node: &TypeNode| {
            if seen.insert(type_node.id) {
                let jvm_class_name = match jvm_class_map.get(&type_node.id) {
                    Some(n) => n.to_string(),
                    None => bail!("Can't find JVM class for {type_node:?}"),
                };
                lift_vars.push(RustStaticLiftVar {
                    jvm_class_name,
                    type_node: type_node.clone(),
                });
            }
            Ok(())
        };

        self.try_visit(|callable: &Callable| {
            if let Some(return_type) = callable.return_type() {
                if !return_type.lowers_to_primitive() {
                    add_lift_var(return_type)?;
                }
            }
            if let Some(throws_type) = callable.throws_type() {
                add_lift_var(throws_type)?;
            }
            Ok(())
        })?;
        self.try_visit(|b: &BoxedType| {
            add_lift_var(&b.inner)
        })?;
        self.scaffolding_lift_vars = lift_vars;
        Ok(())
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
