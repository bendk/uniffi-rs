# Remote types: proc-macros and interfaces

* Status: proposed
* Deciders: Uniffi developers
* Date: 2024-05-24

## Context and Problem Statement

Remote types -- types defined in 3rd-party crates -- require special handling from UniFFI.
One reason, discussed in ADR-0006, is the Rust orphan rule.
Another reason, in some ways more fundamental, is that user's cannot change the type declaration.
For example, there's clearly no way to wrap it with `#[derive(uniffi::Record)]`.
UniFFI currently supports remote records and enums, but does not support interface types.
UniFFI also only supports remote types when using UDL-based code generation.

This ADR will explore:
  - Adding remote record/enum support for proc-macro-based generation
  - Adding interface type support for proc-macro-based generation

This ADR will not discuss:
  - The orphan rule and sharing these types between UniFFI crates.
    This issue will be the same regardless of which option we choose.
  - Adding interface type support for UDL-based generation.
    The assumption is that all options have be a natural/simple way to express in UDL.

## Running examples

This document will focus on 2 types:

- [log::Level](https://docs.rs/log/latest/log/enum.Level.html) as an example of a remote enum.
  Remote structs work essentially the same.
- [anyhow::Error](https://docs.rs/anyhow/latest/anyhow/struct.Error.html) as an example of a remote interface.
  In this case the interface is being used an error type, but this doesn't matter for the purposes of this ADR.

## The current state

UniFFI currently supports declaring remote records/enums in UDL files using the normal syntax.
For example, users can use `Log::Level` in their interface by creating a type alias `type LogLevel = log::Level`, then adding this definition to the UDL:


defining it like so:

```idl
enum LogLevel {
    "Error",
    "Warn",
    "Info",
    "Debug",
    "Trace",
}
```

## Considered Options

### [Option 1] expose remote types directly

We could continue to expose remote types directly, similar to how it currently works in UDL.
One issue here is that proc-macro generation is based attributes that wrap an item, however there's no way for a user to add an attribute to a remote type.
However, macros can work around this issue.

```rust
type LogLevel = log::Level;

uniffi::remote!(
    pub enum LogLevel {
        Error = 1,
        Warn = 2,
        Info = 3,
        Debug = 4,
        Trace = 5,
    }
);
```

The `remote!` macro would generate all scaffolding code needed to handle `LogLevel`.
The `enum LogLevel` item would not end up in the expanded code.

This could also work for interfaces:

```rust
type AnyhowError = anyhow::Error;

uniffi::remote!(
    impl AnyhowError {
        // Expose the `to_string` method (technically, `to_string` comes from the `Display` trait, but that
        // doesn't matter for foreign consumers.  Since the item definition is not used for the
         // scaffolding code and will not be present in the expanded code, it can be left empty.
        pub fn to_string(&self) -> String { }
    }
);
```

One issue with this approach is that we can only export methods that are compatible with UniFFI.
However, users could add an extension trait to create adapter methods that are UniFFI compatible:

```rust
type AnyhowError = anyhow::Error;

pub trait AnyhowErrorExt {
    // [anyhow::Error::is] is a generic method, which can't be exported by UniFFI,
    // but we can export specialized versions for specific types.
    fn is_foo_error(&self) -> bool;
    fn is_bar_error(&self) -> bool;

    // `to_string` is not the best name for the foreign code, let's rename it.
    fn message(&self) -> String;
}

impl AnyhowErrorExt for anyhow::Error {
    fn is_foo_error(&self) -> bool {
        self.is::<foo::Error>()
    }

    fn is_bar_error(&self) -> bool {
        self.is::<bar::Error>()
    }

    fn message(&self) -> String {
        self.to_string()
    }
}

uniffi::remote!(
    impl AnyhowError {
        pub fn is_foo_error(&self) -> bool { }
        pub fn is_bar_error(&self) -> bool { }
        pub fn message(&self) -> String { }
    }
);
```

The above code could be shortened using the [extend](https://crates.io/crates/extend) crate.
UniFFI could also offer syntactic sugar:


```rust
type AnyhowError = anyhow::Error;

// This expands to the equivelent code as the above block
uniffi::remote_extend!(
    impl AnyhowError {
        fn is_foo_error(&self) -> bool {
            self.is::<foo::Error>()
        }

        fn is_bar_error(&self) -> bool {
            self.is::<bar::Error>()
        }

        fn message(&self) -> String {
            self.to_string()
        }
    }
);
```

### [Option 1a] use an attribute macro

The same idea could also be spelled out using an attribute macro rather than a function-like macro:

```rust
#[uniffi::remote]
pub enum LogLevel {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

#[uniffi::remote]
impl AnyhowError {
    // Expose the `to_string` method (technically, `to_string` comes from the `Display` trait, but that
    // doesn't matter for foreign consumers.  Since the item definition is not used for the
     // scaffolding code and will not be present in the expanded code, it can be left empty.
    pub fn to_string(&self) -> String { }
}
```

### [Option 2] use custom-type conversion to expose the type

An alternate strategy would be to use a custom-type conversion from that type into a local type that does implement the UniFFI traits.
These examples will use the custom type syntax from #2087, since I think it looks nicer than the current `UniffiCustomTypeConverter` based code.

```rust
/// Define a type that mirrors `Log::Level`
#[derive(uniffi::Enum)]
pub enum LogLevel {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

/// Define a custom type conversion from `log::Level` to the above type.
uniffi::custom_type!(log::Level, LogLevel, {
  from_custom: |l| match l {
    log::Level::Error => LogLevel::Error,
    log::Level::Warn => LogLevel::Warn,
    log::Level::Info => LogLevel::Info,
    log::Level::Debug => LogLevel::Debug,
    log::Level::Trace => LogLevel::Trace,
  },
  try_into_custom: |l| Ok(match l ({
    LogLevel::Error => log::Level::Error,
    LogLevel::Warn => log::Level::Warn,
    LogLevel::Info => log::Level::Info,
    LogLevel::Debug => log::Level::Debug,
    LogLevel::Trace => log::Level::Trace,
  })
})

/// Interfaces can use the newtype pattern
#[derive(uniffi::Object)]
pub struct AnyhowError(anyhow::Error);

uniffi::custom_newtype!(anyhow::Error, AnyhowError).

// We can define methods directly with this approach, no need for extension traits.
#[uniffi::export]
impl AnyhowError {
    fn is_foo_error(&self) -> bool {
        self.0.is::<foo::Error>()
    }

    fn is_bar_error(&self) -> bool {
        self.0.is::<bar::Error>()
    }

    fn message(&self) -> String {
        self.0.to_string()
    }
}
```

#### Two types

One drawback of this approach is that we have to equivalent, but different types.
Rust code would need to use `anyhow::Error` in their signatures, while foreign code would use `AnyhowError`.
Since the types are almost exactly the same, but named slightly different and with slightly different methods, it can be awkward to document this distinction -- both by UniFFI for library authors and by library authors for their consumers.

### [Option 3] hybrid approach

We could try to combine the best of both worlds by implementing the FFI traits directly for records/structs and using the converter approach for interfaces.

## Pros and Cons of the Options

### [Option 1] expose remote types directly

* Good, because both the foreign code and Rust code can use the same type names.
* Good, because it has a low amount of boilerplate code (assuming we provide the `remote_extend!` macro).
* Bad, because we need to define extension traits for remote interfaces types.
* Bad, because it can be confusing to see a type declaration that the `uniffi::remote!` macro will eventually throw away.

### [Option 1a] use an attribute macro

(compared to option 1)

* Good, because the item declaration looks more natural.
* Bad, since the natural looking item declaration is thrown away, there is even more possibility for confusion.

### [Option 2] use custom-type conversion to expose the type

* Good, because adding methods to remote interface types is natural.
* Bad, because having two equivalent but different types could cause confusion.
* Bad, because users have to write out the trivial struct/enum conversions.

### [Option 3] hybrid approach

* Good, because adding methods to remote interface types is natural.
* Good, because both the foreign code and Rust code can use the same type names for struct/record types.
* Bad, because there will be two types for interface types.
* Good, because it has a low amount of boilerplate code.
* Bad, because mixing the two systems increases the overall complexity and risk of confusion.

## Decision Outcome

???

