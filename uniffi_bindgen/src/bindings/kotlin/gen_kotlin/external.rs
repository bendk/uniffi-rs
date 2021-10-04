/* This Source Code Form is subject to the terms of the Mozilla Publie
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use super::KotlinCodeType;
use crate::interface::types::ExternalTypeHandler;

impl KotlinCodeType for ExternalTypeHandler<'_> {
    fn nm(&self) -> String {
        unimplemented!();
    }
}
