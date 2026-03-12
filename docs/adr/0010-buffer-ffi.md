# Buffer FFI

* Status: proposed
* Deciders:
* Date: 2024-05-24

Discussion and approval: [PR 2816](https://github.com/mozilla/uniffi-rs/pull/2816)

## Context and Problem Statement

Our current FFI is based on passing arguments/return values using the C ABI.
This forces us to convert types that can't be represented by the C ABI
when passing them across the FFI (e.g. enums, Arcs, etc).
Furthermore, languages like Kotlin, Python, and JS that can't make C calls directly
and need to use an intermediate layer to make these calls.
Usually this means a libffi-based library, like ctypes or JNA.

There are several issues with this approach:

* Performance can be poor, especially when using JNA.
* JNA is has been a persistent source of issues on Kotlin.
  There are a couple current issues without clear solutions: #2740, #2624.
* Limitations in these libraries limit how we can design the FFI.
  For example, callback methods can't return values directly because of https://bugs.python.org/issue5710.

We want to rework our FFI approach based on this experience.
This document discusses several general methods for passing values across an FFI.
It proposes 2 general FFIs that can work as a baseline
and sketches out they can be extended to create language-specific FFIs.

The main focus of this ADR is creating a new Kotlin FFI
since we've been seeing both performance issues and crashes with the current FFI.
However other languages are also discussed.

## Passing values over the FFI

There are several different ways for passing arguments and return values across the FFI.
Each can be useful in different scenarios.

### FFI buffer

We could use a single "FFI buffer" to pass arguments, return values, and the call status.
Scaffolding signatures would look like:

```
extern "C" uniffi_buffer_ffi_function_name(ffi_buffer: *u8);
```
Callees should:

 * Read the FFI values for all arguments from the buffer
 * Lift those FFI values into high-level types
 * Call the exported function using the lifted arguments
 * Lower the return value into an FFI type
 * Write the result the buffer:
    * For successful calls, write `0` followed by the return value.
    * For expected errors, write `1` followed by the error value
    * For unexpected errors, write `2` followed by a `RustBuffer` containing a error message.

#### Packing values to the FFI buffer

* Ints and floats are packed in native-endian format.
* Pointers are casted to `u64` values then packed into the buffer.
  Function pointers are handled the same way.
* Structs are packed by serializing each field in order.
* Enums are packed by serializing the discriminant as a `u64` value,
  then packing each field of that variant in order.
* All items are aligned to 64-bit addresses.

#### Allocating the buffer

Callers must ensure the buffer is large enough to hold all arguments as well as the return value
(one or the other, not both at once).
Callers may use different strategies to allocate/free the FFI buffer, including:

* Allocating a new buffer for each call
* Creating an array on the stack
* Statically allocating a single buffer for each thread in a thread-local variable

In order to support all of these strategies, callers must read all data from the buffer immediately
before there's any chance of another call across the FFI.

As long as no dynamically sized values are present in the argument list or return value,
then the buffer will have a fixed and relatively small size.
Buffers for dynamically sized values are discussed in the "RustBuffer and heap data" section below.

#### Performance

The performance of the buffer FFI has been well tested on Kotlin 
with benchmarks showing that it performs much faster than the current FFI.
See the appendix below for details,
the TLDR is that it speeds up most calls by a factor of 100x or so.

### Language-specific bindings layers (JNI/pyo3)

Another approach is using a language-specific bindings layer like JNI, pyo3,
or defining a Python module using the C-API.
So far, we been focused on JNI and this section will reflect that.
However, it's expected that the same logic applies to pyo3 and other systems.

This would mean generating Rust code that looked like this:

```
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_some_package_name_UniffiLibrary_rustFunc(
    mut env: JNIEnv,
    _class: JClass,
    int_arg: i32,
    vec_of_ints: JObject,
    struct_arg: JObject,
    vec_of_structs: JObject,
) -> f64 {
    // Note: no lifting needed for `int_arg`

    // Lift the vec_of_ints arg
    let vec_of_ints_elements = env.get_array_elements_critical(vec_of_ints, ReleaseMode::NoCopyBack).unwrap();
    let vec_of_ints_lifted = vec_elements.iter().collect();

    // Lift the struct arg
    let field_ids = STRUCT_FIELD_IDS.get().unwrap();
    let struct_lifted = TheStruct {
        field_a: env.get_field_unchecked(
            &struct_arg, 
            field_ids.0,
            ReturnType::Primitive(Primitive::Int),
        ).unwrap().i().unwrap(),
        field_b: env.get_field_unchecked(
            &struct_arg, 
            field_ids.1,
            ReturnType::Primitive(Primitive::Double),
        ).unwrap().d().unwrap(),
    };

    // Lift the vec_of_structs arg
    // Unfortunately, we need to get each element one at a time rather in this case.
    let vec_of_structs_length = env.get_array_length(vec_of_structs).unwrap();
    let vec_of_structs_lifted = (0..vec_of_structs_length).iter()
        .map(|i| {
            let struct = env.get_object_array_element(vec_of_structs, i);
            TheStruct {
                field_a: env.get_field_unchecked(
                    &struct, 
                    field_ids.0,
                    ReturnType::Primitive(Primitive::Int),
                ).unwrap().i().unwrap(),
                field_b: env.get_field_unchecked(
                    &struct, 
                    field_ids.1,
                    ReturnType::Primitive(Primitive::Double),
                ).unwrap().d().unwrap(),
            }
        })
        .collect::<Vec<_>>();

    // Call the Rust function
    let result = rust_func(int_arg, vec_of_ints_lifted, struct_lifted, vec_of_structs_lifted);

    // ...lower and return the return value, handle errors, etc.
}
```

The corresponding generated Kotlin code would be fairly simple,
since a lot of the lowering is happening in the JNI layer.

### Performance

JNI code performs better than FFI buffers in some cases:

* **Primitive values** (ints, floats, bools, etc).  Benchmarks show about a 20% speedup.
* **Arrays of primitives**.
  This has not been tested, but it seems safe to assume the JNI approach is faster.

However, FFI buffers perform better in other cases:

* **Structs**.
  It's slightly faster to read elements from a buffer than to make a JNI function call for each field.
* **Enums**.
  In addition to needing JNI calls per field, but you also need JNI calls to figure out the enum variant.
  Calling `IsInstanceOf` for each variant seems very slow (although this has not been tested).
* **Nested data** (arrays of structs, hash maps, structs with enum fields, etc).
  At this point buffers start to significantly out-perform JNI
  since you need to make more than 1 JNI call per item.
  For example, with a vec of structs you need to make an extra JNI call per item in addition to the
  JNI calls for each field.

See https://github.com/mozilla/uniffi-rs/issues/2672 for further discussion and the rough
benchmarks.

### C-ABI

We're currently using the C-ABI to pass primitive arguments.
We could choose to double-down on the C-ABI and use it to pass more values:

* Structs could be passed as the `repr(C)` version of the struct.
* Enums could be passed as tagged unions: one `u64` field for the variant discriminant
and a `repr(C)` union for the variant data.
* Strings could be passed as a (pointer, length) pair.

#### Performance

This would likely improve performance for languages like Swift that natively support C interoperability.

However, it would likely be slower for languages like Kotlin.
One of the main contributors to performance issues right now is reading JNA structs
and adding more structs and introducing unions will almost certainly hurt.

### RustBuffer and heap data

Heap data requires special consideration since it may not fit in an FFI buffer
or on the stack at all.
This means a RustBuffer allocation is required to pass this data.

One issue with our current FFI is that we can allocate multiple RustBuffers per call
since we currently allocate a RustBuffer for values that require heap allocation (vecs, hash-maps, strings).
We also allocate RustBuffers for any structs/enums values (including `Option`).

If we use an FFI buffer to pass values, we can avoid allocating a RustBuffer for structs/enums.
Ironically may increase the total number of allocations
since we'll need to allocate a RustBuffer for any fields that need it.
If a struct or enum has N fields that are heap-allocated, we'll be allocating N RustBuffers instead of 1.

We can avoid this overhead by allocating a single RustBuffer if any argument or a descendent field is heap-allocated.
We then pack each heap-allocated argument into that RustBuffer, in order.
The caller will allocate the RustBuffer before the call and free it afterwards.

In addition to reducing the number of allocations, this also can simplify the generated code.
Currently callees need to free multiple buffers,
with this system the caller will only need to free one.

#### Returning heap values

If the return value is a heap value, the callee will allocate a new RustBuffer to store it.
The caller is responsible for freeing the return buffer once the return value has been lifted.

#### Packing vecs/hash-maps/strings to the FFI buffer

For each of these, first pack the length of the value as a `u64`.
Then pack each vec item, string bytes, or map key/value pair in order.
All items will be aligned to 64-bit boundaries, except the individual string bytes.
Use native-endian when packing items.

#### Optimizing particular cases

There are several optimizations could avoid RustBuffer allocations in some cases.
For example, passing strings as a pointer/length pair or passing small objects using the FFI buffer.
Note however, that there will always be some types that require a RustBuffer, like HashMaps.

### Low-hanging fruit

Finally, there are obvious improvements that we can make for the new FFI.
These all feel obvious, so they're simple listed here without much discussion:

* For functions that can't fail, don't pass in a `RustCallStatus` argument
* Make `RustBuffer` functions use primitive values, rather than a `RustBuffer` struct:
  * `rustbuffer_alloc` can return a pointer since the caller knows the length/capacity
  * `rustbuffer_free` can input the pointer/length/capacity as separate arguments, rather than the struct.

## Options

### [A] Single C-ABI FFI for all languages

This is the current approach

* Bad, because it has poor performance on Kotlin/Python
* Bad, because it has load to JNA crashes on Kotlin
* Bad, because limitations in one language influence the FFI for all languages
* Bad, because it's difficult to pass structs/enums without a RustBuffer.
  One consequence is that we need to include the `call_status` out-pointer in each FFI signature.
* Good, because we only need to generate one version of the scaffolding.

### [B] Generalized FFIs, with opportunities for extensions

Define 2 general FFIs:

* Buffer FFI
  * When no heap allocations are required, FFI calls input a single FFI buffer
    (i.e. their signature is `(ffi_buffer: *u8) -> ()`.
    This buffer is used for both the arguments and return value.
  * For calls that require a RustBuffer allocation for an argument, a single RustBuffer will be allocated.
    The RustBuffer data will be passed instead of the FFI buffer
    (signature: `(data: *u8, len: usize, capacity: usize) -> ()`)
    Callers will allocated and free this RustBuffer.
    This will be instead of the normal FFI buffer for all arguments and the return value.
  * For calls that require a RustBuffer allocation for the return value,
    the callee will allocate a RustBuffer and return it using the normal methods
    (i.e. serializing the fields to the FFI buffer).
    The caller is responsible for freeing the RustBuffer.
  * This FFI will be used for languages like Kotlin, Python, and JS that can't make C calls natively.
* C FFI
  * Primitive values, structs, and enums, will be passed separate `repr(c)` arguments.
    They will also be returned using the same representation.
  * For calls that require a RustBuffer allocation for an argument, a single RustBuffer will be allocated.
    This RustBuffer will be passed as an single extra argument (again as a `repr(c)` struct).
    Callers will allocated and free this RustBuffer.
  * For calls that require a RustBuffer allocation for the return value,
    the callee will allocate a RustBuffer and return it.
    The caller is responsible for freeing the RustBuffer.
  * This FFI will be used for languages like Swift and C++ that can make C calls natively.

Bindings could also use the Buffer FFI with language-specific extensions, for example:
  * Using JNI/pyo3/WASM to make the FFI calls.
  * Passing primitive values using JNI primitives rather than FFI buffers.
  * Passing buffers using language-specific types like ArrayBuffer
    or something like the JNA Pointer class.
  * How this would exactly work is out of scope for this ADR,
    but languages would be encouraged to test out different approaches here.

Pros and cons:

* Good, because the general FFIs will improve performance compared to the current FFI
* Good, because the Buffer FFI will decrease JNA crashes
* Good, because the Buffer FFI can simplify the FFI for languages like Kotlin/Python/JS.
  For example, we can avoid the call status out-pointer.
* Good, because language-specific extensions can be used to maximize performance
* Bad, because language-specific extensions require more code and increase the overall complexity
* Good, because in the buffer FFI creates FFI functions with a known/small number of signatures.
 I think it could significantly improve `uniffi-bindgen-gecko-js`
 which needs to generate a C++ layer to allow JS to call Rust scaffolding functions.
 Maybe we could replace some or all of that layer with a few functions:
   * `get_scaffolding_function(name: String) -> ScaffoldingFunction`
   * `call_scaffolding_function(func: ScaffoldingFunction, buf: ArrayBuffer)`.
   * Maybe we'll need separate versions of these for FFI functions that input/output RustBuffers
     rather than FFI buffers.
     However, there's still only going to be a fairly small number of total functions needed.
* Bad, because we need to cast/serialize data pointers and function pointers to FFI buffers.
 This makes pointer providence trickier and could cause issues on exotic platforms
 where the pointer width is greater than 64 bits.

### [C] Bespoke FFIs for each language

We could just say each bindings generator can implement it's own FFI and leave it at that.
In a technical sense, this is equivalent to [B].
However, without agreeing to some general FFI approaches,
we will probably end up multiplying the complexity unnecessarily.

* Good, for all of the reasons of [B]
* Bad, because it can lead to more fragmentation between bindings and more complexity overall.

### Decision: [B] Generalized FFIs, with opportunities for extensions

## Appendix

### Implementation plan

Our current implementation plan at this point is to:

* Implement the buffer FFI and switch Kotlin to using it
* Investigate implementing a Kotlin-specific JNI FFI to improve performance
  and remove the need for JNA.
* Package the Kotlin JNI in a way that Java bindings can also use it.
* Investigate switching uniffi-bindgen-gecko-js to use something based on the buffer FFI.
  This will have to be a language-specific FFI,
  since we have no way of making C ABI calls from Spidermonkey.

Future work that we hope to do (in no particular order)
* Implement the C FFI and switch Swift to using it
* Switch Python to a new FFI
* Help external bindings authors update their FFI layers
* Do something with Ruby.
  Maybe this is a good to to release ownership of it and turn it into an external binding.
* Mark the current FFI as deprecated and remove it at some point after that.

### Performance testing

When designing this FFI, we did a lot of Kotlin performance testing to compare the different possibilities.
Kotlin was chosen because it has the worst performance of all builtin bindings.
The buffer FFI should also improve performance for Python and other languages,
but the amount will be less than for Kotlin.
Here's a summary of that testing:

* The first step was to generate the Kotlin/Rust code and check that in.
  Other commits made changes to the generated code.
  This made experimentation faster and made it easier to see how changes affected the FFI.
* https://github.com/bendk/uniffi-rs/commit/push-knutvwvsuxxn
* The "low-hanging fruit" changes decreased benchmark times by about 50% in most cases
  (https://github.com/bendk/uniffi-rs/commit/push-xmprvxqrzxpv)
  * Another potential low-hanging fruit change would be to allocate buffers in Kotlin when we can
    instead of using the RustBuffer FFI.
    However, this didn't show any real performance benefits.
    (https://github.com/bendk/uniffi-rs/commit/push-uvylovvzslrl).
* Switching to an buffer FFI further improved performance.
  Many of the times decreased by about 98%, though some decreased less.
  The `nested-data` benchmark regressed because of the issue mentioned above
  where the new FFI caused more RustBuffer allocations. 
  (https://github.com/bendk/uniffi-rs/commit/push-ouylqyrnpnqr).
  * Using JNI to pass JVM values directly was also tested as an alternative.
    Benchmarks showed similar performance to the buffer FFI with JNA,
    but worse performance compared to the buffer FFI with JNI.
    (https://github.com/bendk/uniffi-rs/commit/push-nkkpuuuvonow and
    https://github.com/bendk/uniffi-rs/commit/push-qplztoxoqwny).
  * Testing shows that using `sun.jna.Pointer` was much faster than `java.nio.ByteBuffer`
    for reading/writing to the buffer.
    (https://github.com/bendk/uniffi-rs/commit/push-tvmtokymtoyp).
* Using a single RustBuffer for heap allocations improved performance
  for benchmarks that passed vecs/maps/strings.
  The `strings` benchmark time decreased by about 45%.
  The `nested-data` benchmark decreased by 80% which is an overall speedup compared to the
  low-hanging fruit commit.
  (https://github.com/bendk/uniffi-rs/commit/push-mlzzlsxznylp)
* When compared to the current code, all benchmark times improved
  and criterion usually reported speedups around -99%, meaning the code ran ~100x faster.

### boltffi

[boltffi](https://github.com/boltffi/boltffi) has been gaining popularity recently
and claims to be up to 1000x faster than UniFFI.
Since one of the goals for this ADR is performance, let's investigate how boltffi handles things.

To test this, I modified the `todolist` example to use `boltffi` and added this method to `TodoList`:

```
fn test_method(&self, a: u32, e: TodoEntry, e2: TodoEntry, s: String, b: bool, l: Vec<u32>, l2: Vec<TodoEntry>) -> TodoEntry {
    todo!()
}
```

The goal was to see how boltffi passed these types across the FFI.
A quick review of the generated Kotlin code shows that:

* Object handles are passed across the FFI as JVM Longs (the `self` param)
* Primitives, Strings, and vecs of primitives are passed using JNI (the `a`, `s`, `b`, `l` params)
* The `TodoEntry` struct and `Vec<TodoEntry>` are passed using a buffer (the `e`, `e2`, `l2` params and the return value)
  boltffi calls this the "WireProtocol" and defines `WireReader` and `WireWriter` types.
  boltffi uses a separate buffer for each of these values.

This is pretty much how we imagine the Kotlin JNI bindings will look:
  * Object handles are passed as longs, just like we currently do.
  * Some values are passed as JVM primitives and some are packed into a FFI buffer.
  * The main difference is that boltffi allocates multiple buffers,
    while this ADR recommends using a single buffer for all arguments and for the return values.
