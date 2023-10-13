{%- let obj = ci|get_object_definition(name) %}
public protocol {{ obj.name() }}Protocol {
    {% for meth in obj.methods() -%}
    func {{ meth.name()|fn_name }}({% call swift::arg_list_protocol(meth) %}) {% call swift::async(meth) %} {% call swift::throws(meth) -%}
    {%- match meth.return_type() -%}
    {%- when Some with (return_type) %} -> {{ return_type|type_name -}}
    {%- else -%}
    {%- endmatch %}
    {% endfor %}
}

public class {{ type_name }}: {{ obj.name() }}Protocol {
    fileprivate let uniffiHandle: UInt32

    // TODO: We'd like this to be `private` but for Swifty reasons,
    // we can't implement `FfiConverter` without making this `required` and we can't
    // make it `required` without making it `public`.
    required init(handle: UInt32) {
        self.uniffiHandle = handle
    }

    {%- match obj.primary_constructor() %}
    {%- when Some with (cons) %}
    public convenience init({% call swift::arg_list_decl(cons) -%}) {% call swift::throws(cons) %} {
        self.init(handle: {% call swift::to_ffi_call(cons) %})
    }
    {%- when None %}
    {%- endmatch %}

    deinit {
        try! rustCall { {{ obj.ffi_object_free().name() }}(self.uniffiHandle, $0) }
    }

    {% for cons in obj.alternate_constructors() %}

    public static func {{ cons.name()|fn_name }}({% call swift::arg_list_decl(cons) %}) {% call swift::throws(cons) %} -> {{ type_name }} {
        return {{ type_name }}(handle: {% call swift::to_ffi_call(cons) %})
    }

    {% endfor %}

    {# // TODO: Maybe merge the two templates (i.e the one with a return type and the one without) #}
    {% for meth in obj.methods() -%}
    {%- if meth.is_async() %}

    public func {{ meth.name()|fn_name }}({%- call swift::arg_list_decl(meth) -%}) async {% call swift::throws(meth) %}{% match meth.return_type() %}{% when Some with (return_type) %} -> {{ return_type|type_name }}{% when None %}{% endmatch %} {
        return {% call swift::try(meth) %} await uniffiRustCallAsync(
            rustFutureFunc: {
                {{ meth.ffi_func().name() }}(
                    self.uniffiHandle
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

    public func {{ meth.name()|fn_name }}({% call swift::arg_list_decl(meth) %}) {% call swift::throws(meth) %} -> {{ return_type|type_name }} {
        return {% call swift::try(meth) %} {{ return_type|lift_fn }}(
            {% call swift::to_ffi_call_with_prefix("self.uniffiHandle", meth) %}
        )
    }

    {%- when None %}

    public func {{ meth.name()|fn_name }}({% call swift::arg_list_decl(meth) %}) {% call swift::throws(meth) %} {
        {% call swift::to_ffi_call_with_prefix("self.uniffiHandle", meth) %}
    }

    {%- endmatch -%}
    {%- endif -%}
    {% endfor %}
}

public struct {{ ffi_converter_name }}: FfiConverter {
    typealias FfiType = UInt32
    typealias SwiftType = {{ type_name }}

    public static func lift(_ handle: UInt32) throws -> {{ type_name }} {
        return {{ type_name }}(handle: handle)
    }

    public static func lower(_ value: {{ type_name }}) -> UInt32 {
        return value.uniffiHandle
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
an external type by another crate
#}
public func {{ ffi_converter_name }}_lift(_ handle: UInt32) throws -> {{ type_name }} {
    return try {{ ffi_converter_name }}.lift(handle)
}

public func {{ ffi_converter_name }}_lower(_ value: {{ type_name }}) -> UInt32 {
    return {{ ffi_converter_name }}.lower(value)
}
