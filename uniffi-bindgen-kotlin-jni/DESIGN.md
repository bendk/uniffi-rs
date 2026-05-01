`uniffi-bindgen-kotlin-jni` is an experimental bindgen system that generates
JNI-based Rust scaffolding and Kotlin bindings to call into it.

# Code generation

`uniffi-bindgen-kotin-jni` uses `uniffi_parse_rs` to parse the Rust source
and create the metadata needed to generate the bindings/scaffolding.

The scaffolding is generated using the `uniffi_pipeline` framework and Askama templates
rather than from the macros.
We avoided generating this via the macros,
since it's expected that some external bindgens will also want language-specific scaffolding
and there's no clear way for them to hook into `uniffi_macros` to do that.

Scaffolding is generated for a single crate only.
The main reason is that we only want to parse the source code once.

# FFI calling convention

We use JNI to make FFI calls.

## FFI Buffers

`uniffi-bindgen-kotin-jni` leverages the `uniffi::FfiBuffer` type to pass values across the FFI.
When possible, we try to re-use the same buffer for arguments, return values, exception data, etc.

FFI buffers are the baseline method for passing values.
It's used as a fallback when we can't pass values as primitives or deconstruct them.

FFI buffers are managed using the following system:

* The caller allocates the buffer
* The caller passes the buffer handle to the callee.
* The callee writes the return value to the same buffer, if there is one.
* The caller frees the buffer

## Primitive arguments / return values

If possible, we try to pass arguments directly using JNI.
This results in a significant speedup since it's faster to pass primitive values
and sometimes we can avoid allocating a buffer altogether.

The following types are passed as primitives:
  * Integers, floats, and bool.  Unsigned ints are converted to their signed counterparts.
  * `String` is passed as a `jstring` after a conversion step.

## Deconstructable types

If we can't pass types as primitives,
the next step is to try deconstructing the type into a fixed set of primitive values.
The primitive values are then passed as multiple FFI arguments.

We currently support this for:
  * Records where all fields are primitive or deconstructable.

# Errors/exceptions

Errors/exceptions are handled using JNI rather than `uniffi::RustCallStatus`:

* Rust calls a Kotlin function to create the exception instance.
   * For deconstructable types, Rust passes all the primitive values.
   * For other types, Rust writes the error value to a FFI buffer.
       * For functions that require a buffer for the arguments/return values, this buffer is re-used.
       * If not, then the callee allocates and frees a new buffer
* Rust calls the Kotlin read method using JNI to construct the exception value
* Rust then causes the current JNI function to throw.

## How calls work

### Kotlin -> Rust sync call

* FFI arguments:
    * FFI buffer handle (if needed)
    * Primitive values for all primitive/deconstructable arguments
* Returning values:
    * Primitive types are returned directly
    * Deconstructable types:
        * Deconstructed the value into primitives
        * Pass the primitives to the Kotlin lift function via JNI, getting back a `jobject`.
        * Return the resulting `jobject` back to Kotlin.
    * Otherwise, Rust inputs a FFI buffer handle and writes the return value to it
* Error handling: Rust constructs and throws an exception as described in `Errors/exceptions`

### Rust -> Kotlin sync calls

* FFI arguments:
    * FFI buffer handle (if needed)
    * Primitive values for all primitive/deconstructable arguments
    * If the return value is deconstructable,
      pass a pointer to a stack-allocated `Option<T>` value.
* Returning values:
    * Primitive types are returned directly
    * Deconstructable types:
        * Deconstructed the value into primitives
        * Pass the primitives alongside the `Option` pointer to a Rust JNI function.
        * The Rust function initializes the `Option` from the primitives.
        * The Kotlin function returns void, then the Rust function unwraps and returns the `Option` value.
    * Otherwise, Rust inputs a FFI buffer handle and writes the return value to it
* Errors/unexpected error handling:
    * Expected errors (the `E` side of a Result) are handled the like a regular return,
      they just call a different JNI function.
    * For unexpected errors, Kotlin throws an exception and Rust catches it

### Kotlin -> Rust async call

* FFI arguments:
    * FFI buffer handle (if needed)
    * Primitive values for all primitive/deconstructable arguments
* FFI function return value: Rust future handle
* Kotlin calls a poll function, until the future is ready
    * Kotlin calls a poll function with the future handle, passing it a continuation object
    * If the future is ready, the function returns `UNIFFI_RUST_FUTURE_COMPLETE`
      and we move to the next step.
    * If the future is pending, then the poll function stores the continuation
    * When the future is woken up it then calls a Kotlin method to resume the continuation, restarting this loop
* Returning values:
    * Primitive types:
        * Kotlin passes a `Completion` object to the poll function.
        * Before returning `UNIFFI_RUST_FUTURE_COMPLETE`, Rust passes the return value to `completion.complete()`.
        * Kotlin returns the passed value from the async function.
    * Deconstructable types work like primitive values, except Rust passes multiple primitive values to `complete()`
      and Kotlin reconstructs the return value from those.
    * Otherwise, Rust inputs a FFI buffer handle in the initial FFI call
      and writes the return value to it
* Error handling: Rust constructs and throws an exception from the poll function, as described in `Errors/exceptions`

### Rust -> Rust async call

* Rust creates a oneshot sender/receiver.
* FFI arguments:
    * Handle for the oneshot sender
    * FFI buffer handle (if needed)
    * Primitive values for all primitive/deconstructable arguments
* The Kotlin FFI function schedules the async call, then returns
* Rust awaits the oneshot receiver
* When the Kotlin async function completes:
    * Kotlin calls a Rust completion function
    * Kotlin passes the oneshot sender handle, plus return value data
    * Rust constructs the return value and sends it to the oneshot sender
* Returning values from the async function:
    * Primitive types: Kotlin passes the primitive value to the completion function
    * Deconstructable types: Kotlin passes the deconstructed primitive values to the completion function.
    * Otherwise:
        * Rust inputs a FFI buffer handle in the initial FFI call.
        * Kotlin writes the return value to the buffer
* Error handling:
    * Rust defines separate completion functions to handle errors/unexpected errors.
    * Those functions work similarly to the success completion function,
      they input the sender channel, construct the result value, then send it via the channel.

# Kotlin `uniffi` package

This is a generated package that contains all the FFI functions.
Putting these functions here has some advantages over our current system:

* **Less namespace pollution**.
  With `uniffi-bindgen-kotlin` have to add public functions for UniFFI-internal operations.
  These functions need to be public to make external types work.
  However, it feels slightly wrong for them to end up in the consumer-facing package.
  Putting these functions in the `uniffi` package feels better.
* **Less duplication**.
  If multiple crates use `Vec<u8>` in their interfaces, we'll only need to define 1 read and 1 write function.
* **Simplified external types**.
  We don't need to map namespaces to package names to find the right function.

# Custom types

We use the `uniffi::CustomType` trait to handle this.
The `custom_type!` macro implements it for the custom type.

Note: this is one place where wheree we need the macros to generate code.
This is because the `into` and `try_from` expressions need to be executed
where they're defined rather than in the crate where we're generating scaffolding.

# Callback interfaces

Callabck interfaces are implemented by defining a Kotlin dispatch method
for each callback interface method.
This is responsible for finding the object in the handle map,
reading/converting all arguments from the FFI buffer,
calling the real method,
then writing the return value back to the FFI buffer.
We don't need an `init` function or a vtable,
since we can just lookup and call the methods directly from JNI.

Callback interfaces have essentially the same FFI as Rust functions,
except the calling direction is reversed and they also input the callback object handle.

In the future, we could explore more radical ideas,
like storing the callback object pointer directly in Rust.

# Trait interfaces

Trait interfaces are passed across the FFI using 2 64-bit handles.
For Rust-implemented traits, this is casted from the `Arc<dyn Trait>` wide pointer.
For Kotlin-implemented traits, the first handle is always `0`
and the second is the callback interface handle.

This avoids creating extra Box wrapper that we currently create and also gives us a simple way
to check which side of the FFI implements a trait.

# JNI

This crate uses the low-level `jni_sys` crate rather than the high-level `jni` crate.
This allows for more control and better performance in the JNI code.
An earlier version used `jni`, but benchmarks showed something like ~2-4x increase when switching to `jni_sys`.
`jni_sys` is not much harder for us to use, since we're operating at a very low level.

`jni_sys` is also a more light-weight dependency,
which is important since we're forcing all consumer crates to depend on it.
