/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

import uniffi.bindings_internal.InternalException
import uniffi.bindings_internal.UniffiSlab

val slab = UniffiSlab<Int>(0)
val handle1 = slab.insert(0)
val handle2 = slab.insert(1)
assert(slab.get(handle1) == 0)
assert(slab.get(handle2) == 1)
slab.remove(handle1)

try {
    slab.get(handle1)
    throw AssertionError("get with a removed handle should fail")
} catch (_: InternalException) {
    // Expected
}

val slab2 = UniffiSlab<Int>(1)
try {
    slab2.get(handle2)
    throw AssertionError("get with a handle from a different slab should fail")
} catch (_: InternalException) {
    // Expected
}

try {
    slab.get(handle2 and 0x0001_0000_0000.inv())
    throw AssertionError("get with a handle from a Rust slab should fail")
} catch (_: InternalException) {
    // Expected
}
