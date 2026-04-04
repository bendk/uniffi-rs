public interface {{ int.name_kt() }} {
    {%- for meth in int.methods %}
    fun {{ meth.callable.name_kt() }}({{ meth.callable.arg_list() }}): {{ meth.callable.result.return_type_kt() }}
    {% endfor %}
}
