use uniffi_bindgen::{
    interface::{Literal, ObjectImpl},
    ComponentInterface,
};

use crate::bindings::cpp::{
    gen_cpp::filters::{
        callback_interface_name, external_canonical_prefix, external_namespace_prefix,
        CppCodeOracle,
    },
    CodeType,
};

#[derive(Debug)]
pub(crate) struct ObjectCodeType {
    id: String,
    imp: ObjectImpl,
    module_path: String,
}

impl ObjectCodeType {
    pub(crate) fn new(id: String, imp: ObjectImpl, module_path: String) -> Self {
        Self {
            id,
            imp,
            module_path,
        }
    }
}

impl CodeType for ObjectCodeType {
    fn type_label(&self, ci: &ComponentInterface) -> String {
        format!(
            "std::shared_ptr<{}{}>",
            external_namespace_prefix(ci, &self.module_path),
            CppCodeOracle.class_name(&self.id)
        )
    }

    fn canonical_name(&self, ci: &ComponentInterface) -> String {
        format!(
            "{}{}",
            external_canonical_prefix(ci, &self.module_path),
            CppCodeOracle.class_name(&self.id)
        )
    }

    fn literal(&self, _literal: &Literal, _ci: &ComponentInterface) -> String {
        unreachable!();
    }

    fn initialization_fn(&self) -> Option<String> {
        self.imp.has_callback_interface().then(|| {
            format!(
                "uniffi::{}::init",
                callback_interface_name(&CppCodeOracle.class_name(&self.id)).unwrap()
            )
        })
    }
}
