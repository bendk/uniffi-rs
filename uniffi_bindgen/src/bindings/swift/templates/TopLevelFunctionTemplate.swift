{%- if func.is_async() %}

public func {{ func.name()|fn_name }}({%- call swift::arg_list_decl(func) -%}) async {% call swift::throws(func) %}{% match func.return_type() %}{% when Some with (return_type) %} -> {{ return_type|type_name }}{% when None %}{% endmatch %} {
    // Normally this is handled by rustCall(), but async functions don't go use that since they
    // don't have a RustCallStatusArg
    uniffiEnsureInitialized()
    // Suspend the function and call the scaffolding function, passing it a callback handler from
    // `AsyncTypes.swift`
    //
    // Make sure to hold on to a reference to the continuation in the top-level scope so that
    // it's not freed before the callback is invoked.
    var continuation: {{ func.result_type().borrow()|future_continuation_type }}? = nil
    let rustFutureHandle = {{ func.ffi_func().name() }}(
        {%- for arg in func.arguments() %}
        {{ arg|lower_arg }}{% if !loop.last %},{% endif %}
        {%- endfor %}
    )
    do {
        return {% if func.throws() %}try {% endif %}await withTaskCancellationHandler {
            return {% call swift::try(func) %} await withCheckedThrowingContinuation {
                continuation = $0
                {{ func.rust_future_startup_func().name() }}(
                    rustFutureHandle,
                    FfiConverterForeignExecutor.lower(UniFfiForeignExecutor()),
                    {{ func.result_type().borrow()|future_callback }},
                    &continuation
                )
            }
        } onCancel: {
            {{ func.rust_future_free_func().name() }}(rustFutureHandle)
        }
    } catch let error = CancellationError {
        throw error
    } catch {
        {{ func.rust_future_free_func().name() }}(rustFutureHandle)
        throw error
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
