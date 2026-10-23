@file:OptIn(kotlin.ExperimentalUnsignedTypes::class)
package uniffi

class InternalException(message: kotlin.String) : Exception(message)

{% include "Async.kt" %}
{% include "FfiBuffer.kt" %}
{% include "LiftLower.kt" %}
{% include "Scaffolding.kt" %}
{% include "Interfaces.kt" %}
{% include "CallbackInterfaces.kt" %}

{%- let type_definitions = root.builtin_type_definitions %}
{%- include "TypeDefinitionsFfi.kt" %}
