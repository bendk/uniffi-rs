/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::{fs::File, io::Write};

use anyhow::{bail, Result};
use askama::Template;
use camino::{Utf8Path, Utf8PathBuf};

pub use crate::pipeline::*;

pub fn generate(
    initial_root: InitialRoot,
    out_dir: &Utf8Path,
    crate_filter: Option<String>,
) -> Result<()> {
    let mut root = pipeline().execute(initial_root)?;
    match crate_filter.as_deref() {
        Some("uniffi") => {
            // special-case: only render the shared package
            return render_uniffi_package(&root, out_dir);
        }
        Some(crate_filter) => {
            // Only generate bindings for the crate
            let crate_filter = crate_filter.replace("-", "_");
            root.packages.retain(|pkg| pkg.crate_name == crate_filter);
        }
        None => {
            // Render the shared package and don't filter
            render_uniffi_package(&root, out_dir)?;
        }
    }

    // Create a package for each crate
    for package in root.packages {
        let package_dir = out_dir.join(package.name.split(".").collect::<Utf8PathBuf>());
        let filename = match package.ffi_class_name.strip_suffix("Kt") {
            Some(name) => format!("{name}.kt"),
            None => bail!("ffi_class_name doesn't end in Kt: {}", package.ffi_class_name),
        };

        render(
            &out_dir.join("uniffi"),
            &filename,
            PackageFfi { package: &package },
        )?;

        render(
            &package_dir,
            &format!("{}.kt", package.name),
            BindingsPackage { package },
        )?;
    }

    Ok(())
}

pub fn render_uniffi_package(root: &Root, out_dir: &Utf8Path) -> Result<()> {
    render(
        &out_dir.join("uniffi"),
        "Uniffi.kt",
        UniffiShared {
            cdylib: root.cdylib_name()?,
            root,
        },
    )
}

pub fn render<T: Template>(package_dir: &Utf8Path, filename: &str, template: T) -> Result<()> {
    if !package_dir.exists() {
        std::fs::create_dir_all(package_dir)?;
    }
    let output = template.render()?;
    let mut f = File::create(package_dir.join(filename))?;
    write!(f, "{output}")?;
    Ok(())
}

#[derive(Debug, Clone, Template)]
#[template(syntax = "kt", escape = "none", path = "Package.kt")]
pub struct BindingsPackage {
    package: Package,
}

/// FFI code for a package
///
/// This contains functions to lift/lower types defined in that package.
///
/// This is split out into it's own file in the `uniffi` package to keep things a bit cleaner.
/// The FFI code needs to be public, so that other generated code can call it for external types.
/// It's cleaner if that public code doesn't live in the package that users import.
#[derive(Debug, Clone, Template)]
#[template(syntax = "kt", escape = "none", path = "shared/PackageFfi.kt")]
pub struct PackageFfi<'a> {
    package: &'a Package,
}

/// Shared FFI code.
///
/// This contains functions to lift/lower types builtin UniFFI types,
/// including composite types like `Vec<Option<String>>`
#[derive(Debug, Clone, Template)]
#[template(syntax = "kt", escape = "none", path = "shared/Uniffi.kt")]
pub struct UniffiShared<'a> {
    cdylib: String,
    root: &'a Root,
}

// Defining filters modules to make template errors a bit better
mod filters {}
