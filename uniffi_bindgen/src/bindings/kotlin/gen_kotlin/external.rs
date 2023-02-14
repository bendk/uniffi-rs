/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::backend::{CodeOracle, CodeType, Literal};
use crate::interface::ExternalKind;

pub struct ExternalCodeType {
    name: String,
    kind: ExternalKind,
}

impl ExternalCodeType {
    pub fn new(name: String, kind: ExternalKind) -> Self {
        Self { name, kind }
    }
}

impl CodeType for ExternalCodeType {
    fn type_label(&self, _oracle: &dyn CodeOracle) -> String {
        self.name.clone()
    }

    fn canonical_name(&self, _oracle: &dyn CodeOracle) -> String {
        format!("Type{}", self.name)
    }

    fn literal(&self, _oracle: &dyn CodeOracle, _literal: &Literal) -> String {
        unreachable!("Can't have a literal of an external type");
    }

    fn initialization_fn(&self, oracle: &dyn CodeOracle) -> Option<String> {
        match self.kind {
            ExternalKind::CallbackInterface => {
                Some(format!("{}.register", self.ffi_converter_name(oracle)))
            }
            _ => None
        }
    }
}
