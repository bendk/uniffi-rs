{%- for lift_var in root.scaffolding_lift_vars %}
static {{ lift_var.type_node.lift_kt_from_rust_var() }}: uniffi_jni::CachedStaticMethod = uniffi_jni::CachedStaticMethod::new(
    c"uniffi/{{ lift_var.jvm_class_name }}",
    c"{{ lift_var.type_node.lift_fn_kt() }}",
    c"{{ lift_var.type_node.lift_fn_jni_signature() }}",
);
{%- endfor %}
