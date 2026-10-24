/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use super::*;
use heck::ToUpperCamelCase;

pub fn map_type_node(ty: Type, context: &Context) -> Result<TypeNode> {
    Ok(TypeNode {
        id: context.get_type_id(&ty)?,
        canonical_name: canonical_name(&ty, context.current_namespace_name.as_deref()),
        is_used_as_error: context.type_is_used_as_error(&ty),
        ffi_type: ffi_types::ffi_type(&ty, context)?,
        ty: ty.map_node(context)?,
    })
}

/// A unique name for a type within `namespace`, or among the builtin types when `None`.
///
/// A named type from another namespace is spelled with that namespace inside compound types,
/// whose helpers `namespace` defines itself, so they do not collide with the helpers for a
/// local type of the same name.
pub fn canonical_name(ty: &Type, namespace: Option<&str>) -> String {
    match ty {
        Type::UInt8 => "UInt8".to_string(),
        Type::Int8 => "Int8".to_string(),
        Type::UInt16 => "UInt16".to_string(),
        Type::Int16 => "Int16".to_string(),
        Type::UInt32 => "UInt32".to_string(),
        Type::Int32 => "Int32".to_string(),
        Type::UInt64 => "UInt64".to_string(),
        Type::Int64 => "Int64".to_string(),
        Type::Float32 => "Float32".to_string(),
        Type::Float64 => "Float64".to_string(),
        Type::Boolean => "Boolean".to_string(),
        Type::String => "String".to_string(),
        Type::Bytes => "Bytes".to_string(),
        Type::Timestamp => "Timestamp".to_string(),
        Type::Duration => "Duration".to_string(),
        Type::Interface { name, .. }
        | Type::CallbackInterface { name, .. }
        | Type::Record { name, .. }
        | Type::Enum { name, .. }
        | Type::Custom { name, .. } => format!("Type{name}"),
        Type::Optional { inner_type } => {
            format!("Optional{}", inner_canonical_name(inner_type, namespace))
        }
        Type::Sequence { inner_type } => {
            format!("Sequence{}", inner_canonical_name(inner_type, namespace))
        }
        // Note: this is currently guaranteed to be unique because keys can only be primitive
        // types.  If we allowed user-defined types, there would be potential collisions.  For
        // example "MapTypeFooTypeTypeBar" could be "Foo" -> "TypeBar" or "FooType" -> "Bar".
        Type::Map {
            key_type,
            value_type,
        } => format!(
            "Map{}{}",
            inner_canonical_name(key_type, namespace),
            inner_canonical_name(value_type, namespace),
        ),
        Type::Box { inner_type } => format!("Box{}", inner_canonical_name(inner_type, namespace)),
        Type::Set { inner_type } => {
            format!("Set{}", inner_canonical_name(inner_type, namespace))
        }
    }
}

fn inner_canonical_name(ty: &Type, namespace: Option<&str>) -> String {
    match ty.namespace() {
        Some(other) if namespace.is_some_and(|namespace| namespace != other) => {
            format!(
                "{}{}",
                other.to_upper_camel_case(),
                canonical_name(ty, namespace)
            )
        }
        _ => canonical_name(ty, namespace),
    }
}

pub fn map_type(mut ty: Type, context: &Context) -> Result<Type> {
    Ok(match ty {
        // Map names for top-level types
        Type::Record {
            ref namespace,
            ref mut name,
            ..
        }
        | Type::Enum {
            ref namespace,
            ref mut name,
            ..
        }
        | Type::Interface {
            ref namespace,
            ref mut name,
            ..
        }
        | Type::CallbackInterface {
            ref namespace,
            ref mut name,
            ..
        }
        | Type::Custom {
            ref namespace,
            ref mut name,
            ..
        } => {
            *name = rename::type_(namespace, name.clone(), context)?;
            ty
        }
        // Map inner types
        Type::Optional { inner_type } => Type::Optional {
            inner_type: Box::new(map_type(*inner_type, context)?),
        },
        Type::Sequence { inner_type } => Type::Sequence {
            inner_type: Box::new(map_type(*inner_type, context)?),
        },
        Type::Map {
            key_type,
            value_type,
        } => Type::Map {
            key_type: Box::new(map_type(*key_type, context)?),
            value_type: Box::new(map_type(*value_type, context)?),
        },
        Type::Set { inner_type } => Type::Set {
            inner_type: Box::new(map_type(*inner_type, context)?),
        },
        // All other types can be returned unchanged
        _ => ty,
    })
}

pub fn type_for_record(rec: &initial::Record, context: &Context) -> Result<Type> {
    Ok(Type::Record {
        namespace: context.namespace_name()?,
        name: rec.name.clone(),
        orig_name: rec.orig_name.clone(),
    })
}

pub fn type_for_enum(en: &initial::Enum, context: &Context) -> Result<Type> {
    Ok(Type::Enum {
        namespace: context.namespace_name()?,
        name: en.name.clone(),
        orig_name: en.orig_name.clone(),
    })
}

pub fn type_for_interface(int: &initial::Interface, context: &Context) -> Result<Type> {
    Ok(Type::Interface {
        namespace: context.namespace_name()?,
        name: int.name.clone(),
        orig_name: int.orig_name.clone(),
        imp: int.imp,
    })
}

pub fn type_for_callback_interface(
    cbi: &initial::CallbackInterface,
    context: &Context,
) -> Result<Type> {
    Ok(Type::CallbackInterface {
        namespace: context.namespace_name()?,
        name: cbi.name.clone(),
        orig_name: cbi.orig_name.clone(),
    })
}

pub fn type_for_custom_type(custom: &initial::CustomType, context: &Context) -> Result<Type> {
    Ok(Type::Custom {
        namespace: context.namespace_name()?,
        name: custom.name.clone(),
        orig_name: custom.orig_name.clone(),
        builtin: Box::new(custom.builtin.clone()),
    })
}
