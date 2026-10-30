@file:OptIn(kotlin.ExperimentalUnsignedTypes::class)
package uniffi

{%- for import in package.imports %}
import {{ import }}
{%- endfor %}

{%- let type_definitions = package.type_definitions %}
{%- include "TypeDefinitionsFfi.kt" %}
