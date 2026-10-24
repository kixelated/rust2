{#/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */#}

{%- import "macros.go" as go %}

{%- for type_ in ci.iter_local_types() %}
{%- let type_name = type_|type_name(ci) %}
{%- let ffi_converter_name = type_|ffi_converter_name(ci) %}
{%- let ffi_converter_instance = type_|ffi_converter_instance(ci) %}
{%- let ffi_destroyer_name = type_|ffi_destroyer_name(ci) %}
{%- let canonical_type_name = type_|canonical_name(ci) %}

{%- match type_ %}

{%- when Type::Boolean %}
{%- include "BooleanHelper.go" %}

{%- when Type::Int8 %}
{%- include "Int8Helper.go" %}

{%- when Type::Int16 %}
{%- include "Int16Helper.go" %}

{%- when Type::Int32 %}
{%- include "Int32Helper.go" %}

{%- when Type::Int64 %}
{%- include "Int64Helper.go" %}

{%- when Type::UInt8 %}
{%- include "UInt8Helper.go" %}

{%- when Type::UInt16 %}
{%- include "UInt16Helper.go" %}

{%- when Type::UInt32 %}
{%- include "UInt32Helper.go" %}

{%- when Type::UInt64 %}
{%- include "UInt64Helper.go" %}

{%- when Type::Float32 %}
{%- include "Float32Helper.go" %}

{%- when Type::Float64 %}
{%- include "Float64Helper.go" %}

{%- when Type::String %}
{%- include "StringHelper.go" %}

{%- when Type::Bytes %}
{%- include "BytesHelper.go" %}

{%- when Type::Timestamp %}
{% include "TimestampHelper.go" %}

{%- when Type::Duration %}
{% include "DurationHelper.go" %}

{%- when Type::Enum { name, module_path } %}
{%- if ci.is_used_as_error(type_) %}
{{- self.error(type_)? }}
{%- else %}
{{- self.enum_(type_)? }}
{% endif %}

{%- when Type::Optional { inner_type } %}
{% include "OptionalTemplate.go" %}

{%- when Type::Object { name, module_path, imp } %}
{{ self.object(type_)? }}

{%- when Type::Record { name, module_path } %}
{{ self.record(type_)? }}

{%- when Type::Sequence { inner_type }  %}
{% include "SequenceTemplate.go" %}

{%- when Type::Set { inner_type } %}
{% include "SetTemplate.go" %}

{%- when Type::Map { key_type, value_type } %}
{% include "MapTemplate.go" %}

{%- when Type::CallbackInterface { name, module_path } %}
{{ self.callback_interface(type_)? }}

{%- when Type::Custom { name, builtin, module_path } %}
{{ self.custom(type_)? }}

{%- else %}
{%- endmatch %}
{%- endfor %}

{%- for type_ in ci.iter_external_types() %}
{%- let type_name = type_|type_name(ci) %}
{%- let ffi_converter_name = type_|ffi_converter_name(ci) %}
{%- let ffi_converter_instance = type_|ffi_converter_instance(ci) %}
{%- let ffi_destroyer_name = type_|ffi_destroyer_name(ci) %}
{%- let canonical_type_name = type_|canonical_name(ci) %}

{%- include "ExternalTemplate.go" %}
{%- endfor %}
