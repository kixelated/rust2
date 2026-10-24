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
    /// The modules the bindings import.
    imports: BTreeSet<String>,
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
        // Once any crate needs its own module, every crate is one, so the bindings import each
        // crate they use, not only those they name through their module.
        let imports = if config.separate_modules {
            ci.iter_external_types()
                .filter_map(|ty| ty.crate_name())
                .map(|crate_name| {
                    let namespace = ci.namespace_for_module_path(crate_name)?;
                    Ok(config.external_module_name(crate_name, namespace))
                })
                .collect::<Result<_>>()?
        } else {
            qualified.values().map(|q| q.module.clone()).collect()
        };
        Ok(Self {
            crate_name: ci.crate_name().to_string(),
            qualified,
            imports,
        })
    }

    /// Whether some external type is named through its crate's module.
    pub fn is_empty(&self) -> bool {
        self.qualified.is_empty()
    }

    /// The modules the bindings import.
    pub fn modules(&self) -> BTreeSet<String> {
        self.imports.clone()
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

#[cfg(test)]
mod test {
    use super::super::needs_separate_modules;
    use super::*;
    use uniffi_meta::{
        FieldMetadata, FnMetadata, FnParamMetadata, Metadata, MetadataGroup, NamespaceMetadata,
        RecordMetadata,
    };

    fn record(module_path: &str, name: &str) -> Type {
        Type::Record {
            module_path: module_path.to_string(),
            name: name.to_string(),
        }
    }

    fn namespace(name: &str) -> NamespaceMetadata {
        NamespaceMetadata {
            crate_name: name.to_string(),
            name: name.to_string(),
        }
    }

    // `media` has its own `Frame` and uses `net`'s, plus `extra`'s unambiguous `Other`.
    #[test]
    fn separate_modules_import_every_crate() {
        let frame = RecordMetadata {
            module_path: "media".to_string(),
            name: "Frame".to_string(),
            orig_name: None,
            remote: false,
            fields: vec![FieldMetadata {
                name: "raw".to_string(),
                orig_name: None,
                ty: record("net", "Frame"),
                default: None,
                docstring: None,
            }],
            docstring: None,
        };
        let wrap = FnMetadata {
            module_path: "media".to_string(),
            name: "wrap".to_string(),
            orig_name: None,
            is_async: false,
            inputs: vec![FnParamMetadata::simple("other", record("extra", "Other"))],
            return_type: Some(record("media", "Frame")),
            throws: None,
            checksum: None,
            docstring: None,
        };
        let mut ci = ComponentInterface::from_metadata(MetadataGroup {
            namespace: namespace("media"),
            namespace_docstring: None,
            items: [Metadata::Record(frame), Metadata::Func(wrap)].into(),
        })
        .unwrap();
        ci.set_crate_to_namespace_map(
            ["media", "net", "extra"]
                .into_iter()
                .map(|name| (name.to_string(), namespace(name)))
                .collect(),
        );
        let mut config = Config::default();
        // `net`'s `Frame` is ambiguous, so every crate needs its own module.
        assert!(needs_separate_modules(&config, &ci).unwrap());

        // `Other` is named unqualified, so `extra` is imported along with `net`.
        config.separate_modules = true;
        let external = ExternalTypes::new(&config, &ci).unwrap();
        assert_eq!(
            external.modules(),
            BTreeSet::from(["extra".to_string(), "net".to_string()])
        );
    }
}
