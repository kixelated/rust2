/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use super::{CodeType, KotlinCi};
use crate::interface::Type;

#[derive(Debug)]
pub struct CallbackInterfaceCodeType {
    ty: Type,
}

impl CallbackInterfaceCodeType {
    pub fn new(ty: Type) -> Self {
        Self { ty }
    }
}

impl CodeType for CallbackInterfaceCodeType {
    fn type_label(&self, ci: &KotlinCi<'_>) -> String {
        ci.named_type_label(&self.ty)
    }

    fn canonical_name(&self, ci: &KotlinCi<'_>) -> String {
        ci.named_canonical_name(&self.ty)
    }

    fn ffi_converter_name(&self, ci: &KotlinCi<'_>) -> String {
        ci.named_ffi_converter_name(&self.ty)
    }

    fn initialization_fn(&self) -> Option<String> {
        let name = self.ty.name().expect("a named type");
        Some(format!("uniffiCallbackInterface{name}.register"))
    }
}
