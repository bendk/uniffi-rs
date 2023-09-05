{%- if func.is_async() %}

public func {{ func.name()|fn_name }}({%- call swift::arg_list_decl(func) -%}) async {% call swift::throws(func) %}{% match func.return_type() %}{% when Some with (return_type) %} -> {{ return_type|type_name }}{% when None %}{% endmatch %} {
    // call uniffiEnsureInitialized(). Normally this is handled by rustCall(), but async
    // functions don't go use that since they don't have a RustCallStatusArg
    uniffiEnsureInitialized()
    // Suspend the function and call the scaffolding function, passing it a callback handler from
    // `AsyncTypes.swift`
    let rustFutureHandle = {{ func.ffi_func().name() }}(
        {%- for arg in func.arguments() %}
        {{ arg|lower_arg }}{% if !loop.last %},{% endif %}
        {%- endfor %}
    )
    defer {
        {{ ci.ffi_rust_future_free().name() }}(rustFutureHandle)
    }

    return {% if func.throws() %}try {% endif %}await withTaskCancellationHandler {
        return {% call swift::try(func) %} await withCheckedThrowingContinuation {
            // The simplest way to pass a pointer to Rust is via an in-out param, make
            // continuation mutable so that we can do that
            var continuation = $0
            {{ ci.ffi_rust_future_startup().name() }}(
                rustFutureHandle,
                FfiConverterForeignExecutor.lower(UniFfiForeignExecutor()),
                {{ func.result_type().borrow()|future_callback }} as UniFfiFutureCallback,
                &continuation
            )
        } 
    } onCancel: {
        {{ ci.ffi_rust_future_cancel().name() }}(rustFutureHandle)
    }
}

{% else %}

{%- match func.return_type() -%}
{%- when Some with (return_type) %}

public func {{ func.name()|fn_name }}({%- call swift::arg_list_decl(func) -%}) {% call swift::throws(func) %} -> {{ return_type|type_name }} {
    return {% call swift::try(func) %} {{ return_type|lift_fn }}(
        {% call swift::to_ffi_call(func) %}
    )
}

{%- when None %}

public func {{ func.name()|fn_name }}({% call swift::arg_list_decl(func) %}) {% call swift::throws(func) %} {
    {% call swift::to_ffi_call(func) %}
}

{% endmatch %}
{%- endif %}
