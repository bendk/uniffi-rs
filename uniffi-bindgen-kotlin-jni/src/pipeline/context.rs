/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::collections::HashMap;

use super::*;

#[derive(Default, Clone)]
pub struct Context {
    // Map namespace names to Kotlin packages
    pub package_map: HashMap<String, String>,
    // Map namespace names to Rust crates
    pub crate_map: HashMap<String, String>,
    // Map namespace names to Config values
    pub config_map: HashMap<String, Config>,
    // Map namespace / type names the type definition module_path
    pub type_module_paths: HashMap<String, HashMap<String, String>>,
    pub type_id_map: HashMap<Type, usize>,
    pub current_crate_name: Option<String>,
    pub current_namespace_name: Option<String>,
    pub current_enum: Option<general::Enum>,
}

impl Context {
    pub fn update_from_root(&mut self, root: &general::Root) -> Result<()> {
        for namespace in root.namespaces.values() {
            let config = Config::from_toml(namespace.config_toml.as_deref())?;
            let package_name = match &config.package_name {
                Some(name) => name.clone(),
                None => format!("uniffi.{}", namespace.name),
            };
            self.config_map.insert(namespace.name.clone(), config);
            self.package_map
                .insert(namespace.name.clone(), package_name);
            self.crate_map
                .insert(namespace.name.clone(), namespace.crate_name.clone());
            self.type_module_paths
                .insert(namespace.name.clone(), Self::type_module_paths(namespace));
        }
        self.populate_type_id_map(root);
        Ok(())
    }

    fn populate_type_id_map(&mut self, root: &general::Root) {
        let mut counter = 0..;
        root.visit(|ty: &Type| {
            if !self.type_id_map.contains_key(ty) {
                self.type_id_map.insert(ty.clone(), counter.next().unwrap());
            }
        });
    }

    fn type_module_paths(namespace: &general::Namespace) -> HashMap<String, String> {
        namespace
            .type_definitions
            .iter()
            .filter_map(|type_def| match type_def {
                general::TypeDefinition::Record(r) => {
                    Some((r.orig_name.clone(), r.module_path.clone()))
                }
                general::TypeDefinition::Enum(e) => {
                    Some((e.orig_name.clone(), e.module_path.clone()))
                }
                _ => None,
            })
            .collect()
    }

    pub fn update_from_namespace(&mut self, namespace: &general::Namespace) {
        self.current_crate_name = Some(namespace.crate_name.clone());
        self.current_namespace_name = Some(namespace.name.clone());
    }

    pub fn update_from_enum(&mut self, en: &general::Enum) {
        self.current_enum = Some(en.clone());
    }

    pub fn current_crate_name(&self) -> Result<&str> {
        self.current_crate_name
            .as_deref()
            .ok_or_else(|| anyhow!("current_crate_name not set"))
    }

    pub fn namespace_name(&self) -> Result<&str> {
        self.current_namespace_name
            .as_deref()
            .ok_or_else(|| anyhow!("current_namespace_name not set"))
    }

    pub fn config(&self) -> Result<&Config> {
        let namespace_name = self.namespace_name()?;
        self.config_map
            .get(namespace_name)
            .ok_or_else(|| anyhow!("config not found: {namespace_name}"))
    }

    pub fn package_name(&self, namespace_name: &str) -> Result<&str> {
        self.package_map
            .get(namespace_name)
            .map(|s| s.as_str())
            .ok_or_else(|| anyhow!("package name not found: {namespace_name}"))
    }

    pub fn crate_name(&self, namespace_name: &str) -> Result<&str> {
        self.crate_map
            .get(namespace_name)
            .map(|s| s.as_str())
            .ok_or_else(|| anyhow!("crate name not found: {namespace_name}"))
    }

    pub fn module_path_for_type(&self, namespace: &str, type_orig_name: &str) -> Result<&str> {
        self.type_module_paths
            .get(namespace)
            .and_then(|map| map.get(type_orig_name))
            .map(|s| s.as_str())
            .ok_or_else(|| anyhow!("module path not found: {namespace}:{type_orig_name}"))
    }

    pub fn current_enum(&self) -> Result<&general::Enum> {
        self.current_enum
            .as_ref()
            .ok_or_else(|| anyhow!("current_enum not set"))
    }
}
