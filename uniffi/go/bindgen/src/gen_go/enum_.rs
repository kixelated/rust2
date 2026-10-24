/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use uniffi_bindgen::{
    interface::{Literal, Type},
    ComponentInterface,
};

use super::{filters::oracle, CodeType};

#[derive(Debug)]
pub struct EnumCodeType {
    type_: Type,
}

impl EnumCodeType {
    pub fn new(type_: Type) -> Self {
        Self { type_ }
    }
}

impl CodeType for EnumCodeType {
    fn type_label(&self, ci: &ComponentInterface) -> String {
        let name = self.canonical_name();
        if ci.is_used_as_error(&self.type_) {
            format!("*{name}")
        } else {
            name
        }
    }

    fn canonical_name(&self) -> String {
        oracle().class_name(self.type_.name().expect("an enum is named"))
    }

    fn literal(&self, literal: &Literal, ci: &ComponentInterface) -> String {
        if let Literal::Enum(v, _) = literal {
            format!(
                "{}.{}",
                self.type_label(ci),
                super::GoCodeOracle.enum_variant_name(v)
            )
        } else {
            unreachable!();
        }
    }
}
