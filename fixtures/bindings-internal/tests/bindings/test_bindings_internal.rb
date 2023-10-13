# frozen_string_literal: true

# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/. */

require 'test/unit'
require 'bindings_internal'

class TestSlab < Test::Unit::TestCase
  def test_slaB
    slab = BindingsInternal::UniffiSlab.new(0)
    handle1 = slab.insert(0)
    handle2 = slab.insert(1)
    assert_equal(slab.get(handle1), 0)
    assert_equal(slab.get(handle2), 1)
    slab.remove(handle1)
    # Re-using a removed handle should fail
    begin
      slab.get(handle1)
    rescue BindingsInternal::InternalError => err
      # Expected
    else
      raise 'should have thrown'
    end

    # Using a handle with a different slab should fail
    slab2 = BindingsInternal::UniffiSlab.new(1)
    begin
      slab2.get(handle2)
    rescue BindingsInternal::InternalError => err
      # Expected
    else
      raise 'should have thrown'
    end

    # Using a handle from Rust should fail
    begin
      slab.get(handle2 & ~0x0001_0000_0000)
    rescue BindingsInternal::InternalError => err
      # Expected
    else
      raise 'should have thrown'
    end
  end
end
