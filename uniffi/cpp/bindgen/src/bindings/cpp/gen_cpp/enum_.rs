use uniffi_bindgen::{
    interface::{Literal, Type},
    ComponentInterface,
};

use crate::bindings::cpp::{
    gen_cpp::filters::{
        external_canonical_prefix, external_namespace_prefix, is_error_class, CppCodeOracle,
    },
    CodeType,
};
#[derive(Debug)]
pub(crate) struct EnumCodeType {
    id: String,
    module_path: String,
}

impl EnumCodeType {
    pub(crate) fn new(id: String, module_path: String) -> Self {
        Self { id, module_path }
    }

    fn as_type(&self) -> Type {
        Type::Enum {
            name: self.id.clone(),
            module_path: self.module_path.clone(),
        }
    }
}

impl CodeType for EnumCodeType {
    fn type_label(&self, ci: &ComponentInterface) -> String {
        if is_error_class(ci, &self.as_type()) {
            format!(
                "std::shared_ptr<{}{}>",
                external_namespace_prefix(ci, &self.module_path),
                CppCodeOracle.class_name(&self.id)
            )
        } else {
            format!(
                "{}{}",
                external_namespace_prefix(ci, &self.module_path),
                CppCodeOracle.class_name(&self.id)
            )
        }
    }

    fn canonical_name(&self, ci: &ComponentInterface) -> String {
        format!(
            "{}{}",
            external_canonical_prefix(ci, &self.module_path),
            CppCodeOracle.class_name(&self.id)
        )
    }

    fn literal(&self, _: &Literal, _ci: &ComponentInterface) -> String {
        unreachable!();
    }
}
