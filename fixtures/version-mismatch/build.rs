/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

fn main() {
    // Generate scaffolding for both UDL files, and pick one at compile time

    // Used if the `api_v2` feature is disabled
    uniffi::generate_scaffolding("./src/api_v1.udl").unwrap();
    // Used if the `api_v2` feature is enabled
    uniffi::generate_scaffolding("./src/api_v2.udl").unwrap();
}
