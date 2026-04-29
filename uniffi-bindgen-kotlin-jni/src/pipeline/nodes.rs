/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::collections::HashSet;

use super::*;

uniffi_pipeline::use_prev_node!(general::EnumShape);
uniffi_pipeline::use_prev_node!(general::FieldsKind);
uniffi_pipeline::use_prev_node!(general::ObjectImpl);
uniffi_pipeline::use_prev_node!(general::Radix);
uniffi_pipeline::use_prev_node!(general::Type);

#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::Root))]
#[map_node(root::map_root)]
pub struct Root {
    pub cdylib: Option<String>,
    pub packages: Vec<Package>,
}

#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::Namespace))]
#[map_node(packages::map_namespace)]
pub struct Package {
    pub name: String,
    pub crate_name: String,
    pub config: Config,
    pub functions: Vec<Function>,
    pub type_definitions: Vec<TypeDefinition>,
    pub scaffolding_functions: Vec<ScaffoldingFunction>,
    pub imports: IndexSet<String>,
}

#[derive(Debug, Clone, Node)]
#[allow(clippy::large_enum_variant)]
pub enum TypeDefinition {
    Record(Record),
    Enum(Enum),
    Interface(Interface),
    Class(Class),
    CallbackInterface(CallbackInterface),
    Custom(CustomType),
    Box(BoxedType),
    Optional(OptionalType),
    Sequence(SequenceType),
    Map(MapType),
    Timestamp(TypeNode),
    Duration(TypeNode),
    Bytes(TypeNode),
}

#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::Record))]
#[map_node(records::map_record)]
pub struct Record {
    pub fields_kind: FieldsKind,
    pub self_type: TypeNode,
    pub immutable: bool,
    pub name: String,
    pub orig_name: String,
    pub uniffi_trait_methods: UniffiTraitMethods,
    pub fields: Vec<Field>,
    pub docstring: Option<String>,
    pub recursive: bool,
    pub deconstructable: Option<DeconstructableRecord>,
}

#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::Enum))]
#[map_node(update_context(context.update_from_enum(&self)))]
#[map_node(enums::map_enum)]
pub struct Enum {
    pub is_flat: bool,
    #[map_node(context.config()?.use_enum_entries())]
    pub use_entries: bool,
    pub self_type: TypeNode,
    pub discr_type: TypeNode,
    pub discr_specified: bool,
    pub variants: Vec<Variant>,
    pub name: String,
    pub orig_name: String,
    pub base_classes: Vec<String>,
    pub uniffi_trait_methods: UniffiTraitMethods,
    pub shape: EnumShape,
    pub kotlin_kind: KotlinEnumKind,
    pub docstring: Option<String>,
    pub recursive: bool,
}

/// Kotlin class that implements an interface by calling into Rust
///
/// This is generated for Objects and trait interfaces
#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::Interface))]
#[map_node(interfaces::map_class)]
pub struct Class {
    pub name: String,
    pub orig_name: String,
    pub uniffi_trait_methods: UniffiTraitMethods,
    pub module_path: String,
    pub self_type: TypeNode,
    pub package_name: String,
    pub base_classes: Vec<String>,
    pub constructors: Vec<Constructor>,
    pub methods: Vec<Method>,
    pub docstring: Option<String>,
    pub crate_name: String,
    pub imp: ObjectImpl,
    /// Callback interface for trait interfaces
    pub callback_interface: Option<CallbackInterface>,
}

/// Kotlin Interface
///
/// This is generated for Objects, trait interfaces and callback interfaces
#[derive(Debug, Clone, Node)]
pub struct Interface {
    pub name: String,
    pub methods: Vec<Method>,
    pub docstring: Option<String>,
}

/// Kotlin Callback interface
///
/// This implements a Rust trait by calling into Kotlin.
/// This is generated for trait interfaces and callback interfaces
#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::CallbackInterface))]
#[map_node(callbacks::map_callback_interface)]
pub struct CallbackInterface {
    pub self_type: TypeNode,
    pub name: String,
    pub orig_name: String,
    pub module_path: String,
    pub docstring: Option<String>,
    pub methods: Vec<CallbackMethod>,
    pub crate_name: String,
    pub for_trait_interface: bool,
}

/// Single method in a vtable
#[derive(Debug, Clone, Node, MapNode)]
pub struct CallbackMethod {
    pub callable: Callable,
    pub jni_signature: String,
    pub jni_method_call_name: String,
    pub dispatch_fn_rs: String,
    pub dispatch_fn_kt: String,
}

#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::CustomType))]
pub struct CustomType {
    #[map_node(context.config()?.custom_types.get(&self.name).cloned())]
    pub config: Option<CustomTypeConfig>,
    #[map_node(context.current_crate_name()?.to_string())]
    pub crate_name: String,
    pub self_type: TypeNode,
    pub name: String,
    pub builtin: TypeNode,
    pub docstring: Option<String>,
}

#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::Variant))]
#[map_node(enums::map_variant)]
pub struct Variant {
    pub name_kt: String,
    pub name: String,
    pub orig_name: String,
    pub discr: LiteralNode,
    pub fields_kind: FieldsKind,
    pub fields: Vec<Field>,
    pub docstring: Option<String>,
}

#[derive(Debug, Clone, Node, MapNode)]
pub enum KotlinEnumKind {
    EnumClass { discr_type: Option<String> },
    FlatError,
    SealedClass,
}

#[derive(Debug, Clone, Node, MapNode)]
pub struct Field {
    pub name: String,
    pub orig_name: String,
    pub index: usize,
    pub ty: TypeNode,
    pub default: Option<DefaultValueNode>,
    pub docstring: Option<String>,
}

#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::Constructor))]
pub struct Constructor {
    #[map_node(callables::constructor_jni_method_name(&self, context)?)]
    pub jni_method_name: String,
    pub callable: Callable,
    pub docstring: Option<String>,
}

#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::Method))]
pub struct Method {
    #[map_node(callables::method_jni_method_name(&self, context)?)]
    pub jni_method_name: String,
    pub callable: Callable,
    pub docstring: Option<String>,
}

#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::Function))]
pub struct Function {
    #[map_node(callables::function_jni_method_name(&self, context)?)]
    pub jni_method_name: String,
    pub module_path: String,
    pub docstring: Option<String>,
    pub callable: Callable,
}

#[derive(Debug, Clone, Node)]
pub struct ScaffoldingFunction {
    pub jni_method_name: String,
    pub callable: Callable,
    pub kind: ScaffoldingFunctionKind,
}

#[derive(Debug, Clone, Node)]
pub enum ScaffoldingFunctionKind {
    // Normal function
    Function,
    // Normal method
    Method,
    // Trait method (most of these require special-cased logic)
    TraitMethodDebugFmt,
    TraitMethodDisplayFmt,
    TraitMethodEqEq,
    TraitMethodEqNe,
    TraitMethodHashHash,
    TraitMethodOrdCmp,
}

#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::Callable))]
#[map_node(callables::map_callable)]
pub struct Callable {
    pub kind: CallableKind,
    pub name: String,
    pub orig_name: String,
    pub is_async: bool,
    pub arguments: Vec<Argument>,
    pub result: CallableResult,
    pub fully_qualified_name_rs: String,
}

#[derive(Debug, Clone, Node)]
pub struct CallableResult {
    pub return_type: Option<TypeNode>,
    pub throws_type: Option<TypeNode>,
    // Unique ID for this CallableResult
    pub id: usize,
}

#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::CallableKind))]
pub enum CallableKind {
    Function,
    Method {
        self_type: TypeNode,
    },
    Constructor {
        self_type: TypeNode,
        primary: bool,
    },
    VTableMethod {
        self_type: TypeNode,
        for_callback_interface: bool,
    },
}

pub enum ReturnStrategy<'a> {
    FfiBuffer(&'a TypeNode),
    Primitive(&'a TypeNode, FfiType),
    Void,
}

#[derive(Debug, Clone, Node, MapNode)]
pub struct Argument {
    pub name: String,
    pub orig_name: String,
    pub ty: TypeNode,
    pub by_ref: bool,
    pub optional: bool,
    pub default: Option<DefaultValueNode>,
    pub strategy: ArgStrategy,
}

#[derive(Debug, Clone, Node)]
pub enum ArgStrategy {
    /// Argument passed via a FFI buffer
    FfiBuffer,
    /// Primitive type passed as a JNI argument
    Primitive(FfiArgument),
    /// Deconstructable type that's passed as multiple JNI arguments
    Deconstruct(Vec<FfiArgument>),
}

/// Argument on the JNI FFI function
#[derive(Debug, Clone, Node, MapNode)]
pub struct FfiArgument {
    pub name: String,
    pub ty: FfiType,
}

/// Type that can lowered and passed across the FFI
#[derive(Debug, Clone, Node)]
pub enum LowerableType {
    /// Primitive type can be lowered and passed directly using JNI
    Primitive(FfiType),
    /// High-level type can be deconstructed into multiple primitive types.
    Deconstructable(Vec<FfiType>),
}

/// Record that can be deconstructed into primitive values
#[derive(Debug, Clone, Node)]
pub struct DeconstructableRecord {
    /// Fields of the high-level type with info on how to lift/lower them.
    pub source_fields: Vec<DeconstructableField>,
}

/// Field of a deconstructable type
///
/// This represents the field of the high-level type, which gets mapped to multiple FFI fields.
#[derive(Debug, Clone, Node)]
pub struct DeconstructableField {
    pub name: String,
    pub orig_name: String,
    pub index: usize,
    pub ty: TypeNode,
    pub kind: DeconstructableFieldKind,
}

#[derive(Debug, Clone, Node)]
pub enum DeconstructableFieldKind {
    // Primitive type
    Primitive(FfiField),
    // Type that we should recursively deconstruct
    Recursive(Vec<FfiField>),
}

/// Field of a lowered type
///
/// This represents the field of the high-level type, which gets mapped to multiple FFI fields.
#[derive(Debug, Clone, Node)]
pub struct FfiField {
    pub index: usize,
    pub ty: FfiType,
}

/// Type that's passed across the FFI using JNI
#[derive(Debug, Clone, Copy, Node)]
pub enum FfiType {
    UInt8,
    Int8,
    UInt16,
    Int16,
    UInt32,
    Int32,
    UInt64,
    Int64,
    Float32,
    Float64,
    Boolean,
    String,
}

#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::BoxedType))]
pub struct BoxedType {
    pub inner: TypeNode,
    pub self_type: TypeNode,
}

#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::OptionalType))]
pub struct OptionalType {
    pub inner: TypeNode,
    pub self_type: TypeNode,
}

#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::SequenceType))]
pub struct SequenceType {
    pub inner: TypeNode,
    pub self_type: TypeNode,
}

#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::MapType))]
pub struct MapType {
    pub key: TypeNode,
    pub value: TypeNode,
    pub self_type: TypeNode,
}

#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::TypeNode))]
#[map_node(types::map_type_node)]
pub struct TypeNode {
    pub is_used_as_error: bool,
    pub has_from_unexpected_callback_error_impl: bool,
    pub ty: Type,
    pub type_kt: String,
    pub type_rs: String,
    /// Unique ID for this type node
    pub id: usize,
    // Extra info for lowerable types
    pub lowerable: Option<LowerableType>,
}

#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::DefaultValue))]
#[map_node(defaults::map_default)]
pub struct DefaultValueNode {
    pub default_kt: String,
    pub default: DefaultValue,
}

#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::Literal))]
#[map_node(defaults::map_literal)]
pub struct LiteralNode {
    pub lit_kt: String,
    pub lit: Literal,
}

/// Default value for a field/argument
///
/// This sets the arg/field type in the case where the user just specified `default`.
#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::DefaultValue))]
pub enum DefaultValue {
    Literal(Literal),
    Default(TypeNode),
}

#[derive(Debug, Clone, Node, MapNode)]
#[map_node(from(general::Literal))]
pub enum Literal {
    Boolean(bool),
    String(String),
    UInt(u64, Radix, TypeNode),
    Int(i64, Radix, TypeNode),
    Float(String, TypeNode),
    Enum(String, TypeNode),
    EmptySequence,
    EmptyMap,
    None,
    Some { inner: Box<DefaultValue> },
}

/// Set of methods for builtin traits
///
/// This gets mapped from a list of `initial::UniffiTrait` items.
#[derive(Default, Debug, Clone, Node, MapNode)]
#[map_node(from(general::UniffiTraitMethods))]
pub struct UniffiTraitMethods {
    pub debug_fmt: Option<Method>,
    pub display_fmt: Option<Method>,
    pub eq_eq: Option<Method>,
    pub eq_ne: Option<Method>,
    pub hash_hash: Option<Method>,
    pub ord_cmp: Option<Method>,
}

impl Root {
    pub fn cdylib_name(&self) -> Result<String> {
        let config_names: IndexSet<_> = self
            .packages
            .iter()
            .filter_map(|p| p.config.cdylib_name.as_deref())
            .collect();
        Ok(match config_names.len() {
            0 => match &self.cdylib {
                Some(name) => name.to_string(),
                None => bail!("Unknown cdylib name.  Use `src:[crate_name]` to generate bindings or set it in a `uniffi.toml` config"),
            }
            1 => config_names.into_iter().next().unwrap().to_string(),
            _ => bail!("Conflicting cdylib names in `uniffi.toml` files: {:?}", Vec::from_iter(config_names)),
        })
    }

    /// Type definitions to generate FFI functions for
    ///
    /// This de-dupes the type definitions for all packages so we only don't generate duplicate
    /// functions for types that may be used in multiple packages like `Vec<u32>`.
    pub fn ffi_type_definitions(&self) -> impl Iterator<Item = &TypeDefinition> {
        let mut seen = HashSet::new();
        self.packages
            .iter()
            .flat_map(|p| &p.type_definitions)
            .filter(move |type_def| {
                seen.insert(match type_def {
                    TypeDefinition::Record(r) => &r.self_type.id,
                    TypeDefinition::Enum(e) => &e.self_type.id,
                    TypeDefinition::Optional(o) => &o.self_type.id,
                    TypeDefinition::Sequence(s) => &s.self_type.id,
                    TypeDefinition::Map(m) => &m.self_type.id,
                    TypeDefinition::Class(c) => &c.self_type.id,
                    TypeDefinition::Custom(c) => &c.self_type.id,
                    TypeDefinition::CallbackInterface(c) => &c.self_type.id,
                    TypeDefinition::Box(b) => &b.self_type.id,
                    TypeDefinition::Timestamp(self_type) => &self_type.id,
                    TypeDefinition::Duration(self_type) => &self_type.id,
                    TypeDefinition::Bytes(self_type) => &self_type.id,
                    TypeDefinition::Interface(_) => return false,
                })
            })
    }

    /// Unique throws_types for Rust functions
    pub fn rust_throws_types(&self) -> impl Iterator<Item = &TypeNode> {
        let mut seen = HashSet::new();
        let mut throws_types = vec![];
        self.visit(|callable: &Callable| {
            if callable.is_for_rust_function() {
                if let Some(throws_type) = callable.throws_type() {
                    if seen.insert(&throws_type.id) {
                        throws_types.push(throws_type);
                    }
                }
            }
        });
        throws_types.into_iter()
    }

    pub fn rust_async_callable_results(&self) -> impl Iterator<Item = &CallableResult> {
        let mut unique_types = IndexMap::new();
        self.visit(|callable: &Callable| {
            if callable.is_async && callable.is_for_rust_function() {
                unique_types.insert(callable.result.id, &callable.result);
            }
        });
        unique_types.into_values()
    }

    pub fn kotlin_async_callable_results(&self) -> impl Iterator<Item = &CallableResult> {
        let mut unique_types = IndexMap::new();
        self.visit(|callable: &Callable| {
            if callable.is_async && callable.is_for_kotlin_function() {
                unique_types.insert(callable.result.id, &callable.result);
            }
        });
        unique_types.into_values()
    }

    pub fn disable_java_cleaner(&self) -> bool {
        // Try to merge the different config values as best we can.
        // https://github.com/mozilla/uniffi-rs/issues/2866 would help here.
        self.packages.iter().any(|p| p.config.disable_java_cleaner)
    }

    pub fn enable_android_cleaner(&self) -> bool {
        // Try to merge the different config values as best we can.
        // https://github.com/mozilla/uniffi-rs/issues/2866 would help here.
        self.packages.iter().any(|p| p.config.android_cleaner())
    }
}

impl Package {
    pub fn jni_class(&self) -> String {
        format!("`{}`", self.crate_name.to_upper_camel_case())
    }

    pub fn name_jni(&self) -> String {
        self.name.replace(".", "/")
    }

    pub fn classes(&self) -> impl Iterator<Item = &Class> {
        self.type_definitions
            .iter()
            .filter_map(|type_def| match type_def {
                TypeDefinition::Class(cls) => Some(cls),
                _ => None,
            })
    }

    pub fn uniffi_trait_methods(&self) -> impl Iterator<Item = &UniffiTraitMethods> {
        self.type_definitions
            .iter()
            .filter_map(|type_def| match type_def {
                TypeDefinition::Class(c) => Some(&c.uniffi_trait_methods),
                TypeDefinition::Record(r) => Some(&r.uniffi_trait_methods),
                TypeDefinition::Enum(e) => Some(&e.uniffi_trait_methods),
                _ => None,
            })
    }
}

impl Callable {
    pub fn name_rs(&self) -> String {
        names::escape_rust(&self.orig_name)
    }

    pub fn name_kt(&self) -> String {
        format!("`{}`", self.name.to_lower_camel_case())
    }

    pub fn has_receiver(&self) -> bool {
        self.receiver_type().is_some()
    }

    pub fn receiver_type(&self) -> Option<&TypeNode> {
        match &self.kind {
            CallableKind::Method { self_type, .. }
            | CallableKind::VTableMethod { self_type, .. } => Some(self_type),
            _ => None,
        }
    }

    pub fn is_constructor(&self) -> bool {
        matches!(self.kind, CallableKind::Constructor { .. })
    }

    pub fn is_primary_constructor(&self) -> bool {
        matches!(self.kind, CallableKind::Constructor { primary: true, .. })
    }

    /// Get an argument list for the function/method
    pub fn arg_list(&self) -> String {
        self.arguments
            .iter()
            .map(|a| match &a.default {
                None => format!("{}: {}", a.name_kt(), a.ty.type_kt),
                Some(d) => format!("{}: {} = {}", a.name_kt(), a.ty.type_kt, d.default_kt),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn ffi_arguments(&self) -> impl Iterator<Item = &FfiArgument> {
        let mut ffi_args = vec![];
        for a in self.arguments.iter() {
            match &a.strategy {
                ArgStrategy::Primitive(arg) => ffi_args.push(arg),
                ArgStrategy::Deconstruct(args) => ffi_args.extend(args),
                _ => (),
            }
        }
        ffi_args.into_iter()
    }

    /// Get an argument list without any defaults
    ///
    /// Used when implementing a method for an interface, in that case you can't specify a default.
    pub fn arg_list_no_defaults(&self) -> String {
        self.arguments
            .iter()
            .map(|a| format!("{}: {}", a.name_kt(), a.ty.type_kt))
            .collect::<Vec<_>>()
            .join(" , ")
    }

    pub fn is_for_rust_function(&self) -> bool {
        matches!(
            self.kind,
            CallableKind::Function
                | CallableKind::Method { .. }
                | CallableKind::Constructor { .. }
                | CallableKind::VTableMethod {
                    for_callback_interface: false,
                    ..
                }
        )
    }

    pub fn is_for_kotlin_function(&self) -> bool {
        matches!(self.kind, CallableKind::VTableMethod { .. })
    }

    pub fn uses_buffer(&self) -> bool {
        if self.kind.is_callback_method() && !self.is_async && self.throws_type().is_some() {
            // Sync callback methods currently always need to use buffer if they throw.
            // TODO: remove this kludge.
            return true;
        }

        self.arguments.iter().any(Argument::uses_buffer)
            || self.has_receiver()
            || self
                .return_type()
                .is_some_and(|ty| !matches!(ty.lowerable, Some(LowerableType::Primitive(_))))
    }

    pub fn return_strategy(&self) -> ReturnStrategy<'_> {
        self.result.return_strategy()
    }

    pub fn has_ffi_buffer_arg(&self) -> bool {
        self.arguments.iter().any(Argument::uses_buffer)
    }

    pub fn return_type(&self) -> Option<&TypeNode> {
        self.result.return_type.as_ref()
    }

    pub fn throws_type(&self) -> Option<&TypeNode> {
        self.result.throws_type.as_ref()
    }
}

impl CallableResult {
    pub fn return_type_rs(&self) -> String {
        match (&self.return_type, &self.throws_type) {
            (None, None) => "()".into(),
            (Some(ty), None) => ty.type_rs.clone(),
            (None, Some(err_ty)) => format!("::std::result::Result<(), {}>", err_ty.type_rs),
            (Some(ty), Some(err_ty)) => {
                format!("::std::result::Result<{}, {}>", ty.type_rs, err_ty.type_rs)
            }
        }
    }

    pub fn return_type_kt(&self) -> &str {
        match &self.return_type {
            None => "Unit",
            Some(ty) => &ty.type_kt,
        }
    }

    pub fn return_strategy(&self) -> ReturnStrategy<'_> {
        match &self.return_type {
            Some(type_node) => match &type_node.lowerable {
                Some(LowerableType::Primitive(ffi_type)) => {
                    ReturnStrategy::Primitive(type_node, *ffi_type)
                }
                Some(LowerableType::Deconstructable(_ffi_types)) => {
                    // We don't support deconstructing return values yet
                    ReturnStrategy::FfiBuffer(type_node)
                }
                None => ReturnStrategy::FfiBuffer(type_node),
            },
            _ => ReturnStrategy::Void,
        }
    }

    pub fn async_await_future_fn(&self) -> String {
        format!("awaitRustFuture{}", self.id)
    }

    pub fn async_poll_fn(&self) -> String {
        format!("rustFuturePoll{}", self.id)
    }

    pub fn async_cancel_fn(&self) -> String {
        format!("rustFutureCancel{}", self.id)
    }

    pub fn async_free_fn(&self) -> String {
        format!("rustFutureFree{}", self.id)
    }

    pub fn async_complete_class(&self) -> String {
        format!("CompleteRustFuture{}", self.id)
    }

    pub fn async_complete_success_fn(&self) -> String {
        format!("completeCallbackSuccess{}", self.id)
    }

    pub fn async_complete_error_fn(&self) -> String {
        format!("completeCallbackError{}", self.id)
    }

    pub fn async_complete_unexpected_error_fn(&self) -> String {
        format!("completeCallbackUnexpectedError{}", self.id)
    }

    /// oneshot Sender/Receiver generic type, for async callback functions
    pub fn async_oneshot_type(&self) -> String {
        let return_ty = self
            .return_type
            .as_ref()
            .map(|type_node| &type_node.type_rs);
        let throws_ty = self
            .throws_type
            .as_ref()
            .map(|type_node| &type_node.type_rs);

        let inner_type = match (return_ty, throws_ty) {
            (Some(return_ty), Some(throws_ty)) => {
                format!("::std::result::Result<{return_ty}, {throws_ty}>")
            }
            (Some(return_ty), None) => return_ty.clone(),
            (None, Some(throws_ty)) => format!("::std::result::Result<(), {throws_ty}>"),
            (None, None) => "()".into(),
        };
        // Wrap the normal result type in another Result<> to handle unexpected errors.
        format!("uniffi::Result<{inner_type}>")
    }

    pub fn async_rust_future_output(&self) -> String {
        let ok_type = match self.return_strategy() {
            ReturnStrategy::Primitive(type_node, _) => &type_node.type_rs,
            _ => "()",
        };
        let expected_result_type = if let Some(throws_type) = &self.throws_type {
            let throws_type_rs = &throws_type.type_rs;
            // For errors, we return the E type, plus the FFI buffer if the caller sent us one.
            format!("::std::result::Result<{ok_type}, ({throws_type_rs}, ::std::option::Option<uniffi::FfiBuffer>)>")
        } else {
            ok_type.to_string()
        };

        // Everything gets wrapped in an anyhow::Result to handle unexpected errors
        format!("uniffi::Result<{expected_result_type}>")
    }
}

impl<'a> ReturnStrategy<'a> {
    pub fn is_primitive(&self) -> bool {
        matches!(&self, ReturnStrategy::Primitive(_, _))
    }

    pub fn is_ffi_buffer(&self) -> bool {
        matches!(&self, ReturnStrategy::FfiBuffer(_))
    }
}

impl Class {
    pub fn name_kt(&self) -> String {
        if self.imp.has_callback_interface() {
            format!("{}Impl", self.name.to_upper_camel_case())
        } else {
            names::class_name_kt(&self.name, self.self_type.is_used_as_error)
        }
    }

    pub fn name_rs(&self) -> String {
        names::escape_rust(&self.orig_name)
    }

    pub fn jni_free_name(&self) -> String {
        format!(
            "objectFree{}{}",
            self.crate_name.to_upper_camel_case(),
            self.name.to_upper_camel_case(),
        )
    }

    pub fn jni_addref_name(&self) -> String {
        format!(
            "objectAddReff{}{}",
            self.crate_name.to_upper_camel_case(),
            self.name.to_upper_camel_case(),
        )
    }

    pub fn handle_map_kt(&self) -> String {
        format!(
            "callbackInterfaceHandleMap{}{}",
            self.crate_name.to_upper_camel_case(),
            self.name.to_upper_camel_case(),
        )
    }

    pub fn impl_struct_rs(&self) -> String {
        format!(
            "UniffiCallbackImpl{}{}",
            self.crate_name.to_upper_camel_case(),
            self.orig_name.to_upper_camel_case(),
        )
    }

    pub fn primary_constructor(&self) -> Option<&Constructor> {
        self.constructors.iter().find(|c| {
            matches!(
                c.callable.kind,
                CallableKind::Constructor { primary: true, .. }
            )
        })
    }

    pub fn secondary_constructors(&self) -> impl Iterator<Item = &Constructor> {
        self.constructors.iter().filter(|c| {
            matches!(
                c.callable.kind,
                CallableKind::Constructor { primary: false, .. }
            )
        })
    }
}

impl Interface {
    pub fn name_kt(&self) -> String {
        format!("`{}`", self.name.to_upper_camel_case())
    }
}

impl CallbackInterface {
    pub fn name_kt(&self) -> String {
        format!("`{}`", self.name.to_upper_camel_case())
    }

    pub fn name_rs(&self) -> String {
        names::escape_rust(&self.orig_name)
    }

    pub fn has_async_method(&self) -> bool {
        self.methods.iter().any(|m| m.callable.is_async)
    }

    pub fn free_fn_kt(&self) -> String {
        format!(
            "callbackInterfaceFree{}{}",
            self.crate_name.to_upper_camel_case(),
            self.name.to_upper_camel_case(),
        )
    }

    pub fn handle_map_kt(&self) -> String {
        format!(
            "callbackInterfaceHandleMap{}{}",
            self.crate_name.to_upper_camel_case(),
            self.name.to_upper_camel_case(),
        )
    }

    pub fn impl_struct_rs(&self) -> String {
        format!(
            "UniffiCallbackImpl{}{}",
            self.crate_name.to_upper_camel_case(),
            self.orig_name.to_upper_camel_case(),
        )
    }
}

impl Record {
    pub fn name_kt(&self) -> String {
        names::class_name_kt(&self.name, self.self_type.is_used_as_error)
    }

    pub fn name_rs(&self) -> String {
        names::escape_rust(&self.orig_name)
    }
}

impl Enum {
    pub fn name_kt(&self) -> String {
        names::class_name_kt(&self.name, self.self_type.is_used_as_error)
    }

    pub fn name_rs(&self) -> String {
        names::escape_rust(&self.orig_name)
    }

    pub fn is_flat_error(&self) -> bool {
        matches!(self.shape, EnumShape::Error { flat: true })
    }
}

impl CustomType {
    pub fn name_kt(&self) -> String {
        names::class_name_kt(&self.name, self.self_type.is_used_as_error)
    }
}

impl CallableKind {
    pub fn is_vtable_method(&self) -> bool {
        matches!(self, CallableKind::VTableMethod { .. })
    }

    pub fn is_callback_method(&self) -> bool {
        matches!(
            self,
            CallableKind::VTableMethod {
                for_callback_interface: true,
                ..
            }
        )
    }
}

impl Variant {
    pub fn name_rs(&self) -> String {
        names::escape_rust(&self.orig_name)
    }
}

impl Field {
    pub fn name_kt(&self) -> String {
        if self.name.is_empty() {
            format!("v{}", self.index + 1)
        } else {
            format!("`{}`", self.name.to_lower_camel_case())
        }
    }

    pub fn name_rs(&self) -> String {
        names::escape_rust(&self.orig_name)
    }
}

impl Argument {
    pub fn name_kt(&self) -> String {
        format!("`{}`", self.name.to_lower_camel_case())
    }

    pub fn name_rs(&self) -> String {
        names::escape_rust(&self.orig_name)
    }

    /// Generate code to pass this argument to the Rust function
    ///
    /// This is the argument name, optionally prefixed with things like `&`/`*`
    pub fn pass_to_rust_fn(&self) -> String {
        let name = self.name_rs();
        match (self.by_ref, &self.ty.ty) {
            // Interface refs: use `&*` to go from the `Arc<T>` to `&T`
            (true, Type::Interface { imp, .. }) if imp.is_trait_interface() => format!("&*{name}"),
            // All other refs just need `&`
            (true, _) => format!("&{name}"),
            _ => name,
        }
    }

    pub fn uses_buffer(&self) -> bool {
        matches!(self.strategy, ArgStrategy::FfiBuffer)
    }
}

impl LowerableType {
    pub fn ffi_types(&self) -> Vec<&FfiType> {
        match self {
            LowerableType::Primitive(ffi_type) => vec![ffi_type],
            LowerableType::Deconstructable(ffi_types) => ffi_types.iter().collect(),
        }
    }

    pub fn is_deconstructable(&self) -> bool {
        matches!(self, LowerableType::Deconstructable(_))
    }
}

impl DeconstructableRecord {
    pub fn ffi_types(&self) -> Vec<&FfiType> {
        let mut ffi_types = vec![];
        for f in self.source_fields.iter() {
            match &f.kind {
                DeconstructableFieldKind::Primitive(f) => ffi_types.push(&f.ty),
                DeconstructableFieldKind::Recursive(fs) => {
                    ffi_types.extend(fs.iter().map(|f| &f.ty))
                }
            }
        }
        ffi_types
    }
}

impl DeconstructableField {
    pub fn is_recursive(&self) -> bool {
        matches!(&self.kind, DeconstructableFieldKind::Recursive(_))
    }

    pub fn name_kt(&self) -> String {
        if self.name.is_empty() {
            format!("v{}", self.index + 1)
        } else {
            format!("`{}`", self.name.to_lower_camel_case())
        }
    }

    pub fn name_rs(&self) -> String {
        names::escape_rust(&self.orig_name.to_snake_case())
    }
}

impl FfiArgument {
    pub fn name_kt(&self) -> String {
        format!("`{}`", self.name.to_lower_camel_case())
    }

    pub fn name_rs(&self) -> String {
        names::escape_rust(&self.name)
    }
}

impl UniffiTraitMethods {
    // Rust has 2 display traits, while Kotlin has one.
    // Prefer `Display` but use `Debug` otherwise
    pub fn to_string(&self) -> Option<&Method> {
        self.display_fmt.as_ref().or(self.debug_fmt.as_ref())
    }

    pub fn scaffolding_functions(&self) -> impl Iterator<Item = ScaffoldingFunction> + '_ {
        // We only need to generate one of `Display` or `Debug`
        let to_string = match (&self.display_fmt, &self.debug_fmt) {
            (Some(meth), _) => Some((meth, ScaffoldingFunctionKind::TraitMethodDisplayFmt)),
            (None, Some(meth)) => Some((meth, ScaffoldingFunctionKind::TraitMethodDebugFmt)),
            _ => None,
        };

        to_string
            .into_iter()
            .chain(
                self.eq_eq
                    .as_ref()
                    .map(|meth| (meth, ScaffoldingFunctionKind::TraitMethodEqEq)),
            )
            .chain(
                self.eq_ne
                    .as_ref()
                    .map(|meth| (meth, ScaffoldingFunctionKind::TraitMethodEqNe)),
            )
            .chain(
                self.hash_hash
                    .as_ref()
                    .map(|meth| (meth, ScaffoldingFunctionKind::TraitMethodHashHash)),
            )
            .chain(
                self.ord_cmp
                    .as_ref()
                    .map(|meth| (meth, ScaffoldingFunctionKind::TraitMethodOrdCmp)),
            )
            .map(|(meth, kind)| ScaffoldingFunction {
                jni_method_name: meth.jni_method_name.clone(),
                callable: meth.callable.clone(),
                kind,
            })
    }
}
