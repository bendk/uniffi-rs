# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/. */

import contextlib
import unittest
from bindings_internal import *
from bindings_internal import UniffiSlab

class InternalsTest(unittest.TestCase):
    def test_slab(self):
        slab = UniffiSlab(0)
        handle1 = slab.insert(0)
        handle2 = slab.insert(1)
        self.assertEqual(slab.get(handle1), 0)
        self.assertEqual(slab.get(handle2), 1)
        slab.remove(handle1)
        # Re-using a removed handle should fail
        with self.assertRaises(InternalError):
            slab.get(handle1)

        # Using a handle with a different slab should fail
        slab2 = UniffiSlab(1)
        with self.assertRaises(InternalError):
            slab2.get(handle2)

        # Using a handle from rust should fail
        with self.assertRaises(InternalError):
            slab.get(handle2 & ~UniffiSlab.FOREIGN_BIT)

if __name__=='__main__':
    unittest.main()
