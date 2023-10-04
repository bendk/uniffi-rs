private struct UniffiImplSwift{{ name }} {
    static var vtable: {{ name|vtable_name }} = {{ name|vtable_name }}(
        {%- for meth in obj.methods() %}
        {{ meth.name()|fn_name }}: { (uniffiHandle: UnsafeMutableRawPointer,
            {%- for arg in meth.arguments() %}
            {{- arg.name() }}: {{ arg|ffi_type|ffi_type_name }},
            {%- endfor -%}
            {%- match meth.return_type() %}
            {%- when Some(return_type) %}outReturn: UnsafeMutablePointer<{{ return_type|ffi_type|ffi_type_name }}>,
            {%- when None %}outReturn:  UnsafeMutableRawPointer?,
            {%- endmatch %}
            uniffiCallStatus: UnsafeMutablePointer<RustCallStatus>
        ) in
            // Take an unretained pointer since we are calling by reference
            let swiftObj = Unmanaged<AnyObject>.fromOpaque(uniffiHandle).takeUnretainedValue() as! {{ type_name }}
            let makeCall = { {% if meth.throws() %}try {% endif %}swiftObj.{{ meth.name()|fn_name }}(
                {%- for arg in meth.arguments() %}
                {{ arg.name()|arg_name }}: try {{ arg|lift_fn }}({{ arg.name() }}){% if !loop.last %},{% endif %}
                {%- endfor %}
            ) }
            {% match meth.return_type() %}
            {%- when Some(t) %}
            let writeReturn = { outReturn.pointee = {{ t|lower_fn }}($0) }
            {%- when None %}
            let writeReturn = { () }
            {%- endmatch %}

            {%- match meth.throws_type() %}
            {%- when None %}
            uniffiTraitInterfaceCall(
                callStatus: uniffiCallStatus,
                makeCall: makeCall,
                writeReturn: writeReturn
            )
            {%- when Some(error_type) %}
            uniffiTraitInterfaceCallWithError(
                callStatus: uniffiCallStatus,
                makeCall: makeCall,
                writeReturn: writeReturn,
                lowerError: {{ error_type|lower_fn }}
            )
            {%- endmatch %}
        },
        {%- endfor %}
        uniffiFree: { (uniffiHandle: UnsafeMutableRawPointer) -> () in
            // Take a retained pointer to balance the pointer released in lower()
            let _ = Unmanaged<AnyObject>.fromOpaque(uniffiHandle).takeRetainedValue()
        }
    )

    static func initialize() {
        {{ obj.ffi_init_trait_callback().name() }}(&vtable)
    }
}
