{%- let obj = ci|get_object_definition(name) %}
{%- let (protocol_name, impl_class_name) = obj|object_names %}
{%- let methods = obj.methods() %}
{%- let protocol_docstring = obj.docstring() %}

{% include "Protocol.swift" %}

{%- call swift::docstring(obj, 0) %}
public class {{ impl_class_name }}:
    {%- for tm in obj.uniffi_traits() %}
    {%-     match tm %}
    {%-         when UniffiTrait::Display { fmt } %}
    CustomStringConvertible,
    {%-         when UniffiTrait::Debug { fmt } %}
    CustomDebugStringConvertible,
    {%-         when UniffiTrait::Eq { eq, ne } %}
    Equatable,
    {%-         when UniffiTrait::Hash { hash } %}
    Hashable,
    {%-         else %}
    {%-    endmatch %}
    {%- endfor %}
    {{ protocol_name }} {
    fileprivate let handle: UInt64

    // TODO: We'd like this to be `private` but for Swifty reasons,
    // we can't implement `FfiConverter` without making this `required` and we can't
    // make it `required` without making it `public`.
    required init(handle: UInt64) {
        self.handle = handle
    }

    public func uniffiCloneHandle() -> UInt64 {
        return try! rustCall { {{ obj.ffi_object_clone().name() }}(self.handle, $0) }
    }

    {%- match obj.primary_constructor() %}
    {%- when Some with (cons) %}
    {%- call swift::docstring(cons, 4) %}
    public convenience init({% call swift::arg_list_decl(cons) -%}) {% call swift::throws(cons) %} {
        self.init(handle: {% call swift::to_ffi_call(cons) %})
    }
    {%- when None %}
    {%- endmatch %}

    deinit {
        try! rustCall { {{ obj.ffi_object_free().name() }}(handle, $0) }
    }

    {% for cons in obj.alternate_constructors() %}
    {%- call swift::docstring(cons, 4) %}
    public static func {{ cons.name()|fn_name }}({% call swift::arg_list_decl(cons) %}) {% call swift::throws(cons) %} -> {{ impl_class_name }} {
        return {{ impl_class_name }}(handle: {% call swift::to_ffi_call(cons) %})
    }

    {% endfor %}

    {# // TODO: Maybe merge the two templates (i.e the one with a return type and the one without) #}
    {% for meth in obj.methods() -%}
    {%- if meth.is_async() %}
    {%- call swift::docstring(meth, 4) %}
    public func {{ meth.name()|fn_name }}({%- call swift::arg_list_decl(meth) -%}) async {% call swift::throws(meth) %}{% match meth.return_type() %}{% when Some with (return_type) %} -> {{ return_type|type_name }}{% when None %}{% endmatch %} {
        return {% call swift::try(meth) %} await uniffiRustCallAsync(
            rustFutureFunc: {
                {{ meth.ffi_func().name() }}(
                    self.uniffiCloneHandle()
                    {%- for arg in meth.arguments() -%}
                    ,
                    {{ arg|lower_fn }}({{ arg.name()|var_name }})
                    {%- endfor %}
                )
            },
            pollFunc: {{ meth.ffi_rust_future_poll(ci) }},
            completeFunc: {{ meth.ffi_rust_future_complete(ci) }},
            freeFunc: {{ meth.ffi_rust_future_free(ci) }},
            {%- match meth.return_type() %}
            {%- when Some(return_type) %}
            liftFunc: {{ return_type|lift_fn }},
            {%- when None %}
            liftFunc: { $0 },
            {%- endmatch %}
            {%- match meth.throws_type() %}
            {%- when Some with (e) %}
            errorHandler: {{ e|ffi_converter_name }}.lift
            {%- else %}
            errorHandler: nil
            {% endmatch %}
        )
    }

    {% else -%}

    {%- match meth.return_type() -%}

    {%- when Some with (return_type) %}
    {%- call swift::docstring(meth, 4) %}
    public func {{ meth.name()|fn_name }}({% call swift::arg_list_decl(meth) %}) {% call swift::throws(meth) %} -> {{ return_type|type_name }} {
        return {% call swift::try(meth) %} {{ return_type|lift_fn }}(
            {% call swift::to_ffi_call_with_prefix("self.uniffiCloneHandle()", meth) %}
        )
    }

    {%- when None %}
    {%- call swift::docstring(meth, 4) %}
    public func {{ meth.name()|fn_name }}({% call swift::arg_list_decl(meth) %}) {% call swift::throws(meth) %} {
        {% call swift::to_ffi_call_with_prefix("self.uniffiCloneHandle()", meth) %}
    }

    {%- endmatch -%}
    {%- endif -%}
    {% endfor %}

    {%- for tm in obj.uniffi_traits() %}
    {%-     match tm %}
    {%-         when UniffiTrait::Display { fmt } %}
    public var description: String {
        return {% call swift::try(fmt) %} {{ fmt.return_type().unwrap()|lift_fn }}(
            {% call swift::to_ffi_call_with_prefix("self.uniffiCloneHandle()", fmt) %}
        )
    }
    {%-         when UniffiTrait::Debug { fmt } %}
    public var debugDescription: String {
        return {% call swift::try(fmt) %} {{ fmt.return_type().unwrap()|lift_fn }}(
            {% call swift::to_ffi_call_with_prefix("self.uniffiCloneHandle()", fmt) %}
        )
    }
    {%-         when UniffiTrait::Eq { eq, ne } %}
    public static func == (lhs: {{ impl_class_name }}, other: {{ impl_class_name }}) -> Bool {
        return {% call swift::try(eq) %} {{ eq.return_type().unwrap()|lift_fn }}(
            {% call swift::to_ffi_call_with_prefix("lhs.uniffiCloneHandle()", eq) %}
        )
    }
    {%-         when UniffiTrait::Hash { hash } %}
    public func hash(into hasher: inout Hasher) {
        let val = {% call swift::try(hash) %} {{ hash.return_type().unwrap()|lift_fn }}(
            {% call swift::to_ffi_call_with_prefix("self.uniffiCloneHandle()", hash) %}
        )
        hasher.combine(val)
    }
    {%-         else %}
    {%-    endmatch %}
    {%- endfor %}

}

{%- if obj.is_trait_interface() %}
{%- let callback_handler = format!("uniffiCallbackInterface{}", name) %}
{%- let callback_init = format!("uniffiCallbackInit{}", name) %}
{%- let ffi_init_callback = obj.ffi_init_callback() %}
{% include "CallbackInterfaceImpl.swift" %}
{%- endif %}

public struct {{ ffi_converter_name }}: FfiConverter {
    {%- if obj.is_trait_interface() %}
    fileprivate static var handleMap = UniffiHandleMap<{{ type_name }}>()
    {%- endif %}

    typealias FfiType = UInt64
    typealias SwiftType = {{ type_name }}

    public static func lift(_ handle: UInt64) throws -> {{ type_name }} {
        {%- if obj.is_trait_interface() %}
        if uniffiHandleIsFromRust(handle) {
            return {{ impl_class_name }}(handle: handle)
        } else {
            return handleMap.consumeHandle(handle: handle)
        }
        {%- else %}
        return {{ impl_class_name }}(handle: handle)
        {%- endif %}
    }

    public static func lower(_ value: {{ type_name }}) -> UInt64 {
        {%- if obj.is_trait_interface() %}
        if let rustImpl = value as? {{ impl_class_name }} {
            // If we're wrapping a trait implemented in Rust, return that handle directly rather
            // than wrapping it again in Swift.
            return rustImpl.uniffiCloneHandle()
        } else {
            return handleMap.newHandle(obj: value)
        }
        {%- else %}
        return value.uniffiCloneHandle()
        {%- endif %}
    }

    public static func read(from buf: inout (data: Data, offset: Data.Index)) throws -> {{ type_name }} {
        return try lift(try readInt(&buf))
    }

    public static func write(_ value: {{ type_name }}, into buf: inout [UInt8]) {
        writeInt(&buf, lower(value))
    }
}

{#
We always write these public functions just in case the enum is used as
an external type by another crate.
#}
public func {{ ffi_converter_name }}_lift(_ handle: UInt64) throws -> {{ type_name }} {
    return try {{ ffi_converter_name }}.lift(handle)
}

public func {{ ffi_converter_name }}_lower(_ value: {{ type_name }}) -> UInt64 {
    return {{ ffi_converter_name }}.lower(value)
}
