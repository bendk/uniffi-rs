# Functions

Functions are exposed in a [namespace](./namespace.md), either via [UDL](../udl/functions.md) or [proc-macros](../proc_macro/functions.md)

```rust
#[uniffi::export] // if using proc-macros
fn hello_world() -> String {
    "Hello World!".to_owned()
}
```

Note that everything described here applies to all "callables" - eg, interface methods, constructors etc.

## Optional arguments & default values

You can specify a default value for function arguments (in [UDL](../udl/functions.md#default-values) and [proc-macros](../proc_macro/functions.md#default-values))

The Rust code will still take a required argument:

```rust
fn hello_name(name: String) -> String {
    format!("Hello {}", name)
}
```

The generated foreign-language bindings will use function parameters with default values.

If the default argument value is `"world"`, the generated Kotlin code will be equivalent to:

```kotlin
fun helloName(name: String = "world" ): String {
    // ...
}
```

## Async

Async functions "just work" with proc-macros, while [UDL can use the `[Async]`](../udl/functions.md#async) attribute:

See the [Async/Future support section](../futures.md) for details.

## Closures

Closures can be passed as values.
In order to enable this, call `uniffi::closure!` macro
and pass it the function signature.

Closure values must be wrapped in an `Arc` and also be `Send + Sync`.
Consider creating a type alias so you don't need to type everything out.

Example Rust code:

```rust
/// Define the closure type
uniffi::closure!(Fn() -> String);

/// Create an alias for convenience
pub type NameGenerator = Arc<dyn Fn() -> String + Send + Sync>;

// Define a function that inputs the closure
#[uniffi::export]
pub fn hello_generated_name(name_gen: NameGenerator) -> String {
    format!("Hello {}", name_gen())
}
```

Example foreign code:

```kotlin
// Generated function
fun helloGeneratedName(nameGen: () -> String): String {
    // ...
}

// You can pass Kotlin lambdas to `helloGeneratedName`
helloGeneratedName({ "Bob" })
```

Notes:
* Closure values can be constructed on either the Rust or Foreign side.
* Closure can be used wherever normal types are allowed,
  including arguments, return types, and record/enum fields.
* Only `Fn` is supported.  `FnMut` and `FnOnce` are not.
* Result return types / throwing functions are supported.
