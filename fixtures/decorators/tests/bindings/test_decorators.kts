/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

import uniffi.decorators.*

val decorator = MyDecorator()
// Decorators are given to the rust object via a constructor.
val rustObj0 = RustObject(decorator)

val string1 = "placeholder string"
// Alternative constructors take the decorator as the first argument.
// The argument name is a mixed case version of the decorator interface name.
val rustObj1 = RustObject.fromString(string = string1, myDecorator = decorator)

assert(rustObj0.length() == 0) { "generic return" }
assert(rustObj1.length() == string1.length) { "generic return" }

assert(rustObj0.getString() == 1) { "different return type from method's own" }
assert(rustObj0.getString() == 2) { "code is run each time the method is run" }
assert(rustObj1.getString() == 3) { "decorator can be shared between objects" }

val string2 = "meta-syntactic variable values"
assert(rustObj1.identityString(string2) == Unit) { "void return" }
assert(decorator.lastString == string2) { "Decorator is actually called" }
