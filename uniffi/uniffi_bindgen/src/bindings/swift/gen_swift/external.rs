/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::collections::{BTreeSet, HashMap};

use heck::ToUpperCamelCase;

use super::{CodeType, Config};
use crate::{
    interface::{ComponentInterface, DefaultValue, Type},
    Result,
};

/// The external types a crate's bindings name through their crate's Swift module.
///
/// Swift's documented layout compiles every crate's bindings into one module, where another
/// crate's type is in scope by its bare name, so that is how external types are named. A type
/// whose name is ambiguous here (a local type, or another crate's type, has it too) instead
/// needs each crate in its own module: the bindings name it through its crate's module, which
/// they import.
#[derive(Clone, Debug, Default)]
pub struct ExternalTypes {
    crate_name: String,
    /// By crate name and type name.
    qualified: HashMap<(String, String), Qualifier>,
}

#[derive(Clone, Debug)]
struct Qualifier {
    /// The Swift module of the type's crate.
    module: String,
    /// The type's namespace, which tells this crate's helpers for it apart from its namesake's.
    namespace: String,
}

impl ExternalTypes {
    pub fn new(config: &Config, ci: &ComponentInterface) -> Result<Self> {
        let mut crates_by_name: HashMap<&str, BTreeSet<&str>> = HashMap::new();
        for ty in ci.iter_external_types() {
            if let (Some(crate_name), Some(name)) = (ty.crate_name(), ty.name()) {
                crates_by_name.entry(name).or_default().insert(crate_name);
            }
        }

        let mut qualified = HashMap::new();
        for (name, crates) in crates_by_name {
            if crates.len() == 1 && ci.get_type(name).is_none() {
                continue;
            }
            for crate_name in crates {
                let namespace = ci.namespace_for_module_path(crate_name)?.to_string();
                let module = config.external_module_name(crate_name, &namespace);
                qualified.insert(
                    (crate_name.to_string(), name.to_string()),
                    Qualifier { module, namespace },
                );
            }
        }
        Ok(Self {
            crate_name: ci.crate_name().to_string(),
            qualified,
        })
    }

    /// The modules the bindings import to name their qualified types.
    pub fn modules(&self) -> BTreeSet<String> {
        self.qualified.values().map(|q| q.module.clone()).collect()
    }

    /// The module to qualify `ty` with, if it is an ambiguous external type.
    pub fn module(&self, ty: &Type) -> Option<&str> {
        self.qualifier(ty).map(|q| q.module.as_str())
    }

    /// Whether `ty` is a local type with the name of an external type the bindings import.
    pub fn is_shadowing(&self, ty: &Type) -> bool {
        ty.crate_name() == Some(&self.crate_name)
            && ty
                .name()
                .is_some_and(|name| self.qualified.keys().any(|(_, n)| n == name))
    }

    /// Wrap the code type of `ty`, if it is an ambiguous external type.
    pub fn wrap(&self, ty: &Type, code_type: Box<dyn CodeType>) -> Box<dyn CodeType> {
        match self.qualifier(ty) {
            Some(qualifier) => Box::new(QualifiedCodeType {
                is_record: matches!(ty, Type::Record { .. }),
                qualifier: qualifier.clone(),
                inner: code_type,
            }),
            None => code_type,
        }
    }

    fn qualifier(&self, ty: &Type) -> Option<&Qualifier> {
        let key = (ty.crate_name()?.to_string(), ty.name()?.to_string());
        self.qualified.get(&key)
    }
}

/// An external type named through its crate's module.
#[derive(Debug)]
struct QualifiedCodeType {
    is_record: bool,
    qualifier: Qualifier,
    inner: Box<dyn CodeType>,
}

impl CodeType for QualifiedCodeType {
    fn type_label(&self) -> String {
        format!("{}.{}", self.qualifier.module, self.inner.type_label())
    }

    // Only this crate's helpers for compound types use this, so it needs no module, but it
    // must differ from the canonical name of the local namesake.
    fn canonical_name(&self) -> String {
        format!(
            "{}{}",
            self.qualifier.namespace.to_upper_camel_case(),
            self.inner.canonical_name()
        )
    }

    fn default(&self, default: &DefaultValue) -> Result<String> {
        match default {
            DefaultValue::Default if self.is_record => Ok(format!("{}()", self.type_label())),
            _ => self.inner.default(default),
        }
    }

    fn ffi_converter_name(&self) -> String {
        format!(
            "{}.{}",
            self.qualifier.module,
            self.inner.ffi_converter_name()
        )
    }

    fn initialization_fn(&self) -> Option<String> {
        self.inner.initialization_fn()
    }
}
