/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use super::KotlinCodeOracle;
use crate::{
    backend::{CodeType, Literal},
    interface::ObjectImpl,
};

#[derive(Debug)]
pub struct ObjectCodeType {
    id: String,
    imp: ObjectImpl,
}

impl ObjectCodeType {
    pub fn new(id: String, imp: ObjectImpl) -> Self {
        Self { id, imp }
    }
}

impl CodeType for ObjectCodeType {
    fn type_label(&self) -> String {
        KotlinCodeOracle.class_name(&self.id)
    }

    fn canonical_name(&self) -> String {
        format!("Type{}", self.id)
    }

    fn literal(&self, _literal: &Literal) -> String {
        unreachable!();
    }

    fn initialization_fn(&self) -> Option<String> {
        match &self.imp {
            ObjectImpl::Trait => Some(format!(
                "{}.initialize",
                KotlinCodeOracle.trait_vtable_obj(&self.id)
            )),
            ObjectImpl::Struct => None,
        }
    }
}
