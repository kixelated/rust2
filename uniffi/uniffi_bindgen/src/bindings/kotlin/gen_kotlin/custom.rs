/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use super::{AsCodeType, CodeType, KotlinCi};
use crate::interface::{DefaultValue, Type};

#[derive(Debug)]
pub struct CustomCodeType {
    ty: Type,
}

impl CustomCodeType {
    pub fn new(ty: Type) -> Self {
        CustomCodeType { ty }
    }
}

impl CodeType for CustomCodeType {
    fn type_label(&self, ci: &KotlinCi<'_>) -> String {
        ci.named_type_label(&self.ty)
    }

    fn canonical_name(&self, ci: &KotlinCi<'_>) -> String {
        ci.named_canonical_name(&self.ty)
    }

    fn ffi_converter_name(&self, ci: &KotlinCi<'_>) -> String {
        ci.named_ffi_converter_name(&self.ty)
    }

    fn default(&self, default: &DefaultValue, ci: &KotlinCi<'_>) -> anyhow::Result<String> {
        let Type::Custom { builtin, .. } = &self.ty else {
            unreachable!("a custom type")
        };
        builtin
            .as_codetype()
            .default(default, ci)
            .map_err(|_e| anyhow::anyhow!("Unsupported default value for {}", self.type_label(ci)))
    }
}
