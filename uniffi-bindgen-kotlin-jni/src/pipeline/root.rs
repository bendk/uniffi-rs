/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use super::*;

pub fn map_root(input: general::Root, context: &Context) -> Result<Root> {
    let mut context = context.clone();
    context.update_from_root(&input)?;

    Ok(Root {
        cdylib: input.cdylib,
        packages: input.namespaces.map_node(&context)?.into_values().collect(),
    })
}
