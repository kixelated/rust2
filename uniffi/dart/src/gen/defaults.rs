use genco::prelude::*;
use uniffi_bindgen::interface::{Argument, AsType, DefaultValue, Type};

use uniffi_bindgen::ComponentInterface;

use super::oracle::{AsCodeType, DartCodeOracle};
use super::primitives::render_interface_default_value;

fn enum_name_from_type(ty: &Type, ci: &ComponentInterface) -> Option<String> {
    match ty {
        Type::Enum { .. } => Some(ty.as_codetype(ci).type_label()),
        Type::Optional { inner_type } => enum_name_from_type(inner_type, ci),
        _ => None,
    }
}

pub(crate) fn render_default_value(
    default_value: &DefaultValue,
    ty: &Type,
    ci: &ComponentInterface,
) -> Option<String> {
    render_interface_default_value(default_value, ty, |ty, variant| {
        let enum_name = enum_name_from_type(ty, ci)?;
        let variant_name = DartCodeOracle::enum_variant_name(variant);
        Some(format!("{enum_name}.{variant_name}"))
    })
}

pub(crate) fn render_argument_param(
    arg: &Argument,
    type_tokens: dart::Tokens,
    ci: &ComponentInterface,
) -> dart::Tokens {
    let name = DartCodeOracle::var_name(arg.name());

    if let Some(default_value) = arg.default_value() {
        if let Some(default_expr) = render_default_value(default_value, &arg.as_type(), ci) {
            quote!($type_tokens $name = $(default_expr))
        } else {
            quote!(required $type_tokens $name)
        }
    } else {
        quote!(required $type_tokens $name)
    }
}
