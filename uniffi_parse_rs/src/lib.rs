/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::{collections::HashMap, str::FromStr};

use camino::Utf8Path;
pub use target_lexicon::Triple;
pub use uniffi_meta::MetadataGroup;

mod attrs;
mod cfg_select;
mod compile_env;
mod custom_types;
mod enums;
mod errors;
mod files;
mod functions;
mod impls;
mod ir;
mod items;
mod kw;
mod macros;
mod modules;
mod objects;
#[macro_use]
mod paths;
mod public_paths;
mod records;
mod trait_impls;
mod traits;
mod types;
mod use_;

pub use compile_env::CompileEnv;
pub use custom_types::CustomType;
pub use enums::{Enum, Variant};
pub use errors::{Error, ErrorContext, ErrorKind};
pub use functions::{Argument, Function, ReturnType};
pub use impls::Impl;
pub use ir::{Ir, IrChanges, VisitUnresolvedItem};
pub use items::{BuiltinItem, Item};
pub use modules::Module;
pub use objects::{Constructor, Method, Object, SelfArg};
pub use paths::{ChildItem, Namespace, RPath, Visibility};
pub use public_paths::ItemNames;
pub use records::{Field, Record};
pub use traits::Trait;
pub use types::{GenericArgs, Type};
pub use use_::{parse_use, UseGlob, UseItem};

pub type Result<T, E = Error> = std::result::Result<T, E>;
pub type MetadataGroupMap = HashMap<String, MetadataGroup>;

/// Top-level API for `uniffi_parse_rs`
///
/// This parses Rust sources and creates a `MetadataGroupMap` from them.
/// All failable methods return `anyhow::Result`
/// to integrate better with the rest of the UniFFI.
pub struct Parser {
    target: Triple,
    ir: Ir,
}

impl Parser {
    pub fn new(target: &str) -> anyhow::Result<Self> {
        Ok(Self {
            target: Triple::from_str(target)
                .map_err(|_| anyhow::anyhow!("Invalid target triple"))?,
            ir: Ir::default(),
        })
    }

    pub fn new_for_host_target() -> Self {
        Self {
            target: Triple::host(),
            ir: Ir::default(),
        }
    }

    pub fn add_crate_root(
        &mut self,
        name: &str,
        path: &Utf8Path,
        features: Vec<String>,
    ) -> anyhow::Result<()> {
        let compile_env = CompileEnv::new(self.target.clone(), features);
        self.ir
            .add_crate_root(name, path, compile_env)
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    /// Consume the parser into a [uniffi_meta::MetadataGroupMap]
    ///
    /// `udl_loader` inputs (crate_name, udl_namespace)
    /// and returns metadata loaded from the UDL file for that crate.
    pub fn into_uniffi_meta<F>(mut self, mut udl_loader: F) -> anyhow::Result<MetadataGroupMap>
    where
        F: FnMut(&str, &str) -> anyhow::Result<MetadataGroup>,
    {
        self.ir
            .resolve_items()
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        for crate_root in self.ir.crate_roots.values_mut() {
            let udl_name = crate_root.udl_name().map_err(|e| anyhow::anyhow!("{e}"))?;
            let module = crate_root.module_mut();
            if let Some(udl_name) = udl_name {
                let metadata = udl_loader(&module.ident.to_string(), &udl_name)?;
                module.add_udl_metadata(metadata.items);
            }
        }
        self.ir
            .into_metadata_group_map()
            .map_err(|e| anyhow::anyhow!("{e}"))
    }
}
