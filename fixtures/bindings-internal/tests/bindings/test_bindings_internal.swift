// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

import bindings_internal

var slab = UniffiSlab<Int>(0)
let handle1 = try! slab.insert(0)
let handle2 = try! slab.insert(1)
assert(try! slab.get(handle1) == 0)
assert(try! slab.get(handle2) == 1)
let _ = try! slab.remove(handle1)

do {
    let _ = try slab.get(handle1)
    fatalError("get with a removed handle should fail")
} catch UniffiInternalError.slabUseAfterFree {
    // Expected
}

var slab2 = UniffiSlab<Int>(1)
do {
    let _ = try slab2.get(handle2)
    fatalError("get with a handle from a different slab should fail")
} catch UniffiInternalError.slabError {
    // Expected
}

do {
    let _ = try slab.get(handle2 & ~0x0001_0000_0000)
    fatalError("get with a handle from a Rust slab should fail")
} catch UniffiInternalError.slabError {
    // Expected
}
