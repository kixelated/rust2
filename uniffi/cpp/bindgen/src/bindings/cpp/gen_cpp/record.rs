use uniffi_bindgen::{interface::Literal, ComponentInterface};

use crate::bindings::cpp::{
    gen_cpp::filters::{external_canonical_prefix, external_namespace_prefix, CppCodeOracle},
    CodeType,
};

#[derive(Debug)]
pub(crate) struct RecordCodeType {
    id: String,
    module_path: String,
}

impl RecordCodeType {
    pub(crate) fn new(id: String, module_path: String) -> Self {
        Self { id, module_path }
    }
}

impl CodeType for RecordCodeType {
    fn type_label(&self, ci: &ComponentInterface) -> String {
        format!(
            "{}{}",
            external_namespace_prefix(ci, &self.module_path),
            CppCodeOracle.class_name(&self.id)
        )
    }

    fn canonical_name(&self, ci: &ComponentInterface) -> String {
        format!(
            "{}Type{}",
            external_canonical_prefix(ci, &self.module_path),
            self.id
        )
    }

    fn literal(&self, _literal: &Literal, _ci: &ComponentInterface) -> String {
        unreachable!();
    }
}
