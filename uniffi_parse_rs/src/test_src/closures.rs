/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::sync::Arc;

#[derive(uniffi::Error)]
enum TestError { }

// Test resolving closures through type aliases
pub type ArcDynFn = Arc<dyn std::ops::Fn(u8) -> () + ::std::marker::Sync + Send>;
pub type DynFn = dyn std::ops::Fn(u8) -> () + Sync + std::marker::Send;

// uniffi_parse_rs should ignore the `uniffi::closure!` macro
uniffi::closure!(Fn(u8) -> ());
