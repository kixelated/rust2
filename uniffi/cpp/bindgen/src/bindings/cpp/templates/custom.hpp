{%- match config.custom_types.get(name.as_str()) %}
{%- when Some with (type_config) %}
{%- match type_config.type_name %}
{%- when Some with (type_name) %}
struct {{ typ|ffi_converter_name(ci) }} {
    static {{ name }} lift(RustBuffer);
    static RustBuffer lower(const {{ name }} &);
    static {{ name }} read(RustStream &);
    static void write(RustStream &, const {{ name }} &);
    static uint64_t allocation_size(const {{ name }} &);
};
{%- else %}
{%- endmatch %}
{%- else -%}
typedef struct {{ builtin|ffi_converter_name(ci) }} {{ typ|ffi_converter_name(ci) }};
{%- endmatch %}
