use genco::prelude::*;
use uniffi_bindgen::interface::{AsType, Type};

use super::oracle::{AsCodeType, DartCodeOracle};
use super::render::{Renderable, TypeHelperRenderer};
use super::CodeType;

#[derive(Debug)]
pub struct CustomCodeType {
    name: String,
    module_path: String,
    builtin: Box<Type>,
    namespace: Option<String>,
}

impl CustomCodeType {
    pub fn new(name: String, module_path: String, builtin: Box<Type>, namespace: Option<String>) -> Self {
        CustomCodeType { name, module_path, builtin, namespace }
    }
}

impl CodeType for CustomCodeType {
    fn type_label(&self) -> String {
        DartCodeOracle::qualify(self.namespace.as_deref(), DartCodeOracle::class_name(&self.name))
    }

    fn canonical_name(&self) -> String {
        DartCodeOracle::qualify_canonical(
            self.namespace.as_deref(),
            DartCodeOracle::class_name(&self.name),
        )
    }

    fn ffi_converter_name(&self) -> String {
        DartCodeOracle::qualify(
            self.namespace.as_deref(),
            format!("FfiConverter{}", DartCodeOracle::class_name(&self.name)),
        )
    }
}

impl AsType for CustomCodeType {
    fn as_type(&self) -> Type {
        Type::Custom {
            name: self.name.clone(),
            module_path: self.module_path.clone(),
            builtin: self.builtin.clone(),
        }
    }
}

impl Renderable for CustomCodeType {
    fn render_type_helper(&self, type_helper: &dyn TypeHelperRenderer) -> dart::Tokens {
        type_helper.include_once_check(&self.name, &self.as_type());
        // Another crate's library declares its own typedefs.
        if self.namespace.is_some() {
            return quote!();
        }

        let ffi_converter_name = &self.ffi_converter_name();
        let type_name = &self.type_label();
        let ci = type_helper.get_ci();
        let builtin_ffi_converter_name = &(*self.builtin).as_codetype(ci).ffi_converter_name();
        let builtin_name = DartCodeOracle::dart_type_label(Some(&*self.builtin), ci);

        quote! {
            typedef $(type_name) = $(builtin_name);
            typedef $(ffi_converter_name) = $(builtin_ffi_converter_name);
        }
    }
}
