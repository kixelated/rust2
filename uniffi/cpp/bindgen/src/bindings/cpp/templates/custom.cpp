{%- match config.custom_types.get(name.as_str()) %}
{%- when Some with (type_config) %}
{%- match type_config.type_name %}
{%- when Some with (type_name) %}
{{ type_name }} {{ ffi_converter_name }}::lift(RustBuffer buff) {
    auto builtin_val = {{ builtin|lift_fn(ci) }}(buff);

    return {{ type_config.lift("builtin_val") }};
}

RustBuffer {{ ffi_converter_name }}::lower(const {{ type_name }} &val) {
    auto builtin_val = {{ type_config.lower("val") }};

    return {{ builtin|lower_fn(ci) }}(builtin_val);
}

{{ type_name }} {{ ffi_converter_name }}::read(RustStream &stream) {
    auto builtin_val = {{ builtin|read_fn(ci) }}(stream);

    return {{ type_config.lift("builtin_val") }};
}

void {{ ffi_converter_name }}::write(RustStream &stream, const {{ type_name }} &val) {
    auto builtin_val = {{ type_config.lower("val") }};

    {{ builtin|write_fn(ci) }}(stream, builtin_val);
}

uint64_t {{ ffi_converter_name }}::allocation_size(const {{ type_name }} &val) {
    auto builtin_val = {{ type_config.lower("val") }};

    return {{ builtin|allocation_size_fn(ci) }}(builtin_val);
}
{%- else %}
{%- endmatch %}
{%- else %}
{%- endmatch %}
