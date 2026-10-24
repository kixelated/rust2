{%- if ci.is_used_as_error(type_) %}
{%- let type_name = type_|type_name(ci) %}
{%- let package_name = ci.package_for_module_path(module_path) %}

object {{ type_|canonical_name(ci) }}ExternalErrorHandler : UniffiRustCallStatusErrorHandler<{{ type_name }}> {
    override fun lift(error_buf: RustBuffer.ByValue): {{ type_name }} =
        {{ type_name }}.ErrorHandler.lift(
            {{ package_name }}.RustBuffer.ByValue().apply {
                capacity = error_buf.capacity
                len = error_buf.len
                data = error_buf.data
            }
        )
}

{%- endif %}