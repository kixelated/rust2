/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use super::{CodeType, KotlinCi};
use crate::{
    bail,
    interface::{DefaultValue, Literal, Type},
    Result,
};

#[derive(Debug)]
pub struct EnumCodeType {
    ty: Type,
}

impl EnumCodeType {
    pub fn new(ty: Type) -> Self {
        Self { ty }
    }
}

impl CodeType for EnumCodeType {
    fn type_label(&self, ci: &KotlinCi<'_>) -> String {
        ci.named_type_label(&self.ty)
    }

    fn canonical_name(&self, ci: &KotlinCi<'_>) -> String {
        ci.named_canonical_name(&self.ty)
    }

    fn ffi_converter_name(&self, ci: &KotlinCi<'_>) -> String {
        ci.named_ffi_converter_name(&self.ty)
    }

    fn default(&self, default: &DefaultValue, ci: &KotlinCi<'_>) -> Result<String> {
        if let DefaultValue::Literal(Literal::Enum(v, _)) = default {
            Ok(format!(
                "{}.{}",
                self.type_label(ci),
                super::KotlinCodeOracle.enum_variant_name(v)
            ))
        } else {
            bail!("Invalid literal for enum type: {default:?}")
        }
    }
}
