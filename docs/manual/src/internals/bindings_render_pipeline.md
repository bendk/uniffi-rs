# Rendering Foreign Bindings

UniFFI generates the foreign bindings using a system that works something like a compiler pipeline.
The proc-macros generate metadata that describes the exported interface.
That metadata is then converted by a series of data transformations, finally ending up as generated source code.
This document describes the steps in that process.

## Metadata Generation

When a UniFFI proc-macro wraps a Rust item, it records metadata about the item it's wrapping.
For example, when `#[uniffi::export]` wraps a function, it records the function name, arguments, return type, etc.
This metadata is then serialized into a buffer and stored as an exported symbol in the Rust library.
For example:

```rust
// Medata buffer for the `arithmetic::add()` function
const UNIFFI_META_CONST_ARITHMETIC_FUNC_ADD: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
        ::uniffi::metadata::codes::FUNC, // Code for the item type
    )
    .concat_str("arithmetic") // module name
    .concat_str("add") // function name
    .concat_bool(false) // async?
    .concat_value(2u8) // number of arguments
    .concat_str("a") // 1st argument name
    .concat(<u64 as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META) // 1st argument type
    .concat_bool(false) // default value?
    .concat_str("b") // 2nd argument name
    .concat(<u64 as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META) // 2nd argument type
    .concat_bool(false) // default value?
    .concat(<Result<u64> as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META) // result type
    .concat_long_str(""); // docstring

// Serialize the metadata into a byte array and export the symbol.
//  This can be evaluated at compile time because `UNIFFI_META_ARITHMETIC_FUNC_ADD` is `const`.
#[no_mangle]
#[doc(hidden)]
pub static UNIFFI_META_ARITHMETIC_FUNC_ADD: [u8; UNIFFI_META_CONST_ARITHMETIC_FUNC_ADD
    .size] = UNIFFI_META_CONST_ARITHMETIC_FUNC_ADD.into_array();
```

Notes:

* UniFFI gets the type metadata types using the `TypeId::TYPE_ID_META`.
  By using a trait and associated type rather than relying on the type identifier, we get the correct metadata even through type aliases.
  For example, `Result<u64>` is a type alias with the error type omitted, but `<Result<u64> as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META>` includes metadata about the error type.
* See [Lifting and Lowering](./lifting_and_lowering.md) for details on `UniFfiTag`.

When a UniFFI bindgen command runs, it reads/deserializes the exported metadata symbols from the Rust library.
Then the metadata is passed on to the next stage of the pipeline
See `uniffi_bingen::macro_metadata` and `uniffi_meta::reader` for details.

## Metadata from the UDL file

UniFFI also generates metadata from UDL files and merges this with the proc-macro metadata.

## Metadata -> Bindings IR

This phase transforms the metadata into a `BindingsIr`.
"IR" here is meant to evoke an intermediate-representation in a compiler pipeline, since this representation sits in-between the Rust code and the generated code.
This phase is where derived information about the FFI is added, for example:

* The scaffolding function for each Rust function
* The scaffolding functions to clone/free object references
* The FFI type that each type is lowered into/lifted from
* If a type is used as an error or not

## Bindings IR -> Foreign Language IR

This phase transforms the `BindingsIr` into a language-specific IR like `PythonIr` or `SwiftIr`.
Generic IR items are transformed into ones specific to the foreign language:

* Names/docstrings are transformed to the preferred style
* UniFFI types are transformed to the language's types
* Data about helper classes is added, for example the `FfiConverter` classes that lift/lower types
* New language-specific items are added like imports

## Foreign Language IR -> Generated Code

The final phase takes the language-specific IR and uses it to generate the bindings code.
We use the [Rinja](https://rinja.readthedocs.io/en/stable/) template rendering engine for this.

## Peeking behind the curtains with `peek` and `diff`

Use the `peek` subcommand from any UniFFI bindings generator to inspect this process.
`peek [phase]` will print out the data from any stage of the pipeline.
For example:
  - `uniffi-bindgen [bindgen-args] peek metadata` to inspect the metadata
  - `uniffi-bindgen [bindgen-args] peek bindings-ir` to inspect the bindings IR
  - `uniffi-bindgen [bindgen-args] peek python-ir` to inspect the Python IR
  - `uniffi-bindgen [bindgen-args] peek kotlin` to inspect the Kotlin source
  - `uniffi-bindgen-swift` also supports the `peek` subcommand

Use the `diff` subcommand to see how UniFFI changes affect the pipeline.

For example:
  - `uniffi-bindgen [bindgen-args] diff --save` to save the current pipeline data for later diffing
  - [make some changes to a UniFFIed library, or the UniFFI source itself]
  - `uniffi-bindgen [bindgen-args] diff metadata` to see how those changes affected the metadata phase
  - `uniffi-bindgen [bindgen-args] diff bindings-ir` to see how those changes affected the Bindings IR
  - `uniffi-bindgen [bindgen-args] diff python-ir` to see how those changes affected the Python IR
  - `uniffi-bindgen [bindgen-args] diff python` to see how those changes affected the generated Python code
