/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Debug;
use std::ops::Deref;

use askama::Template;
use heck::{ToLowerCamelCase, ToShoutySnakeCase, ToUpperCamelCase};
use serde::{Deserialize, Serialize};

use crate::{
    anyhow, bail, interface::ffi::ExternalFfiMetadata, interface::*, to_askama_error, Context,
    Result,
};

mod callback_interface;
mod compounds;
mod custom;
mod enum_;
mod miscellany;
mod object;
mod primitives;
mod record;
mod variant;

trait CodeType: Debug {
    /// The language specific label used to reference this type. This will be used in
    /// method signatures and property declarations.
    fn type_label(&self, ci: &KotlinCi<'_>) -> String;

    /// A representation of this type label that can be used as part of another
    /// identifier. e.g. `read_foo()`, or `FooInternals`.
    ///
    /// This is especially useful when creating specialized objects or methods to deal
    /// with this type only.
    fn canonical_name(&self, ci: &KotlinCi<'_>) -> String;

    // default for named types is to assume a ctor exists.
    fn default(&self, default: &DefaultValue, ci: &KotlinCi<'_>) -> Result<String> {
        match default {
            DefaultValue::Default => Ok(format!("{}()", self.type_label(ci))),
            DefaultValue::Literal(_) => bail!("Literals for named types are not supported"),
        }
    }

    /// Name of the FfiConverter
    ///
    /// This is the object that contains the lower, write, lift, and read methods for this type.
    /// Depending on the binding this will either be a singleton or a class with static methods.
    ///
    /// This is the newer way of handling these methods and replaces the lower, write, lift, and
    /// read CodeType methods.  Currently only used by Kotlin, but the plan is to move other
    /// backends to using this.
    fn ffi_converter_name(&self, ci: &KotlinCi<'_>) -> String {
        format!("FfiConverter{}", self.canonical_name(ci))
    }

    /// Function to run at startup
    fn initialization_fn(&self) -> Option<String> {
        None
    }
}

/// The component being rendered, with the config that names the package of every crate it uses.
///
/// Another crate's types are always spelled with their package: a bare-name import would be
/// shadowed by a local type of the same name.
#[derive(Clone, Copy)]
pub struct KotlinCi<'a> {
    ci: &'a ComponentInterface,
    config: &'a Config,
}

impl<'a> KotlinCi<'a> {
    fn new(ci: &'a ComponentInterface, config: &'a Config) -> Self {
        Self { ci, config }
    }

    /// The package of the crate a module path belongs to.
    fn package_for_module_path(&self, module_path: &str) -> String {
        let namespace = self.ci.namespace_for_module_path(module_path).ok();
        self.config.external_package_name(module_path, namespace)
    }

    /// The package of another crate's type, or `None` for a local type.
    fn external_package(&self, ty: &Type) -> Option<String> {
        if !self.ci.is_external(ty) {
            return None;
        }
        ty.module_path().map(|p| self.package_for_module_path(p))
    }

    /// The label of a named type.
    fn named_type_label(&self, ty: &Type) -> String {
        let class_name = KotlinCodeOracle.type_class_name(self.ci, ty);
        match self.external_package(ty) {
            Some(package) => format!("{package}.{class_name}"),
            None => class_name,
        }
    }

    /// The canonical name of a named type, which starts with its namespace when it's another
    /// crate's so the helpers for containers of it don't collide with a local namesake's.
    fn named_canonical_name(&self, ty: &Type) -> String {
        let name = ty.name().expect("a named type");
        if self.ci.is_external(ty) {
            // Outside library mode, an external crate's namespace may be unknown.
            let namespace = self
                .ci
                .namespace_for_type(ty)
                .ok()
                .or(ty.crate_name())
                .expect("an external type has a crate");
            format!("{}Type{name}", namespace.to_upper_camel_case())
        } else {
            format!("Type{name}")
        }
    }

    /// The FfiConverter of a named type, which another crate's bindings define.
    fn named_ffi_converter_name(&self, ty: &Type) -> String {
        let name = ty.name().expect("a named type");
        match self.external_package(ty) {
            Some(package) => format!("{package}.FfiConverterType{name}"),
            None => format!("FfiConverterType{name}"),
        }
    }
}

impl Deref for KotlinCi<'_> {
    type Target = ComponentInterface;

    fn deref(&self) -> &ComponentInterface {
        self.ci
    }
}

// config options to customize the generated Kotlin.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    pub(super) package_name: Option<String>,
    pub(super) cdylib_name: Option<String>,
    generate_immutable_records: Option<bool>,
    #[serde(default)]
    mutable_records: HashSet<String>,
    #[serde(default)]
    omit_checksums: bool,
    #[serde(default)]
    custom_types: HashMap<String, CustomTypeConfig>,
    #[serde(default)]
    pub(super) external_packages: HashMap<String, String>,
    #[serde(default)]
    android: bool,
    #[serde(default)]
    android_cleaner: Option<bool>,
    #[serde(default)]
    kotlin_target_version: Option<String>,
    #[serde(default)]
    disable_java_cleaner: bool,
    #[serde(default)]
    pub(super) rename: toml::Table,
    #[serde(default)]
    pub(super) exclude: Vec<String>,
}

impl Config {
    pub(crate) fn android_cleaner(&self) -> bool {
        self.android_cleaner.unwrap_or(self.android)
    }

    pub(crate) fn use_enum_entries(&self) -> bool {
        self.get_kotlin_version() >= KotlinVersion::new(1, 9, 0)
    }

    /// Returns a `Version` with the contents of `kotlin_target_version`.
    /// If `kotlin_target_version` is not defined, version `0.0.0` will be used as a fallback.
    /// If it's not valid, this function will panic.
    fn get_kotlin_version(&self) -> KotlinVersion {
        self.kotlin_target_version
            .clone()
            .map(|v| {
                KotlinVersion::parse(&v).unwrap_or_else(|_| {
                    panic!("Provided Kotlin target version is not valid: {}", v)
                })
            })
            .unwrap_or(KotlinVersion::new(0, 0, 0))
    }

    // Get the package name for an external type
    fn external_package_name(&self, module_path: &str, namespace: Option<&str>) -> String {
        // config overrides are keyed by the crate name, default fallback is the namespace.
        let crate_name = module_path.split("::").next().unwrap();
        match self.external_packages.get(crate_name) {
            Some(name) => name.clone(),
            // If the module path is not in `external_packages`, we need to fall back to a default
            // with the namespace, which we hopefully have.  This is quite fragile, but it's
            // unreachable in library mode - all deps get an entry in `external_packages` with the
            // correct namespace.
            None => format!("uniffi.{}", namespace.unwrap_or(module_path)),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct KotlinVersion((u16, u16, u16));

impl KotlinVersion {
    fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self((major, minor, patch))
    }

    fn parse(version: &str) -> Result<Self> {
        let components = version
            .split('.')
            .map(|n| {
                n.parse::<u16>()
                    .map_err(|_| anyhow!("Invalid version string ({n} is not an integer)"))
            })
            .collect::<Result<Vec<u16>>>()?;

        match components.as_slice() {
            [major, minor, patch] => Ok(Self((*major, *minor, *patch))),
            [major, minor] => Ok(Self((*major, *minor, 0))),
            [major] => Ok(Self((*major, 0, 0))),
            _ => bail!("Invalid version string (expected 1-3 components): {version}"),
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomTypeConfig {
    imports: Option<Vec<String>>,
    type_name: Option<String>,
    into_custom: String, // b/w compat alias for lift
    lift: String,
    from_custom: String, // b/w compat alias for lower
    lower: String,
}

// functions replace literal "{}" in strings with a specified value.
impl CustomTypeConfig {
    fn lift(&self, name: &str) -> String {
        let converter = if self.lift.is_empty() {
            &self.into_custom
        } else {
            &self.lift
        };
        converter.replace("{}", name)
    }
    fn lower(&self, name: &str) -> String {
        let converter = if self.lower.is_empty() {
            &self.from_custom
        } else {
            &self.lower
        };
        converter.replace("{}", name)
    }
}

impl Config {
    // We insist someone has already configured us - any defaults we supply would be wrong.
    pub fn package_name(&self) -> String {
        self.package_name
            .as_ref()
            .expect("package name should have been set in update_component_configs")
            .clone()
    }

    pub fn cdylib_name(&self) -> String {
        self.cdylib_name
            .as_ref()
            .expect("cdylib name should have been set in update_component_configs")
            .clone()
    }

    /// Whether to generate immutable records (`val` instead of `var`)
    fn generate_immutable_records(&self) -> bool {
        self.generate_immutable_records.unwrap_or(false)
    }

    /// Whether a specific record should be generated with immutable fields.
    /// A record is immutable only if `generate_immutable_records` is enabled
    /// and the record is not listed in `mutable_records`.
    pub fn is_record_immutable(&self, name: &str) -> bool {
        self.generate_immutable_records() && !self.mutable_records.contains(name)
    }

    pub fn disable_java_cleaner(&self) -> bool {
        self.disable_java_cleaner
    }
}

// Generate kotlin bindings for the given ComponentInterface, as a string.
pub fn generate_bindings(config: &Config, ci: &ComponentInterface) -> Result<String> {
    ensure_flat_enum_trait_methods_supported(ci)?;
    KotlinWrapper::new(config, ci)
        .context("failed to create a binding generator")?
        .render()
        .context("failed to render kotlin bindings")
}

fn ensure_flat_enum_trait_methods_supported(ci: &ComponentInterface) -> Result<()> {
    for enum_def in ci.enum_definitions() {
        if ci.is_used_as_error(enum_def) || !enum_def.is_flat() {
            continue;
        }

        let trait_methods = enum_def.uniffi_trait_methods();
        if trait_methods.eq_eq.is_some()
            || trait_methods.hash_hash.is_some()
            || trait_methods.ord_cmp.is_some()
        {
            anyhow::bail!(
                "Kotlin bindings do not support exporting Eq/Ord/Hash for flat enum `{}`",
                enum_def.name()
            );
        }
    }
    Ok(())
}

/// A struct to record a Kotlin import statement.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ImportRequirement {
    /// The name we are importing.
    Import { name: String },
}

impl ImportRequirement {
    /// Render the Kotlin import statement.
    fn render(&self) -> String {
        match &self {
            ImportRequirement::Import { name } => format!("import {name}"),
        }
    }
}

/// Renders Kotlin helper code for all types
///
/// This template is a bit different than others in that it stores internal state from the render
/// process.  Make sure to only call `render()` once.
#[derive(Template)]
#[template(escape = "none", path = "kotlin/Types.kt")]
pub struct TypeRenderer<'a> {
    config: &'a Config,
    ci: KotlinCi<'a>,
    // Track imports added with the `add_import()` macro
    imports: RefCell<BTreeSet<ImportRequirement>>,
}

impl<'a> TypeRenderer<'a> {
    fn new(config: &'a Config, ci: &'a ComponentInterface) -> Self {
        Self {
            config,
            ci: KotlinCi::new(ci, config),
            imports: RefCell::new(BTreeSet::new()),
        }
    }

    // The following methods are used by the `Types.kt` macros.

    // Helper to add an import statement
    //
    // Call this inside your template to cause an import statement to be added at the top of the
    // file.  Imports will be sorted and de-deuped.
    //
    // Returns an empty string so that it can be used inside an askama `{{ }}` block.
    fn add_import(&self, name: &str) -> &str {
        self.imports.borrow_mut().insert(ImportRequirement::Import {
            name: name.to_owned(),
        });
        ""
    }
}

#[derive(Template)]
#[template(escape = "none", path = "kotlin/wrapper.kt")]
pub struct KotlinWrapper<'a> {
    config: &'a Config,
    ci: KotlinCi<'a>,
    type_helper_code: String,
    type_imports: BTreeSet<ImportRequirement>,
}

impl<'a> KotlinWrapper<'a> {
    pub fn new(config: &'a Config, ci: &'a ComponentInterface) -> Result<Self> {
        let type_renderer = TypeRenderer::new(config, ci);
        let type_helper_code = type_renderer.render()?;
        let type_imports = type_renderer.imports.into_inner();
        Ok(Self {
            config,
            ci: KotlinCi::new(ci, config),
            type_helper_code,
            type_imports,
        })
    }

    pub fn initialization_fns(&self) -> Vec<String> {
        let init_fns = self
            .ci
            .iter_local_types()
            .map(|t| KotlinCodeOracle.find(t))
            .filter_map(|ct| ct.initialization_fn())
            .map(|fn_name| format!("{fn_name}(this)"));

        // Also call global initialization function for any external type we use.
        // For example, we need to make sure that all callback interface vtables are registered
        // (#2343).
        let extern_module_init_fns = self
            .ci
            .iter_external_types()
            .filter_map(|ty| ty.crate_name())
            .map(|crate_name| {
                let package_name = self.ci.package_for_module_path(crate_name);
                format!("{package_name}.uniffiEnsureInitialized()")
            })
            // Collect into a btree set to de-dup and order
            .collect::<BTreeSet<_>>();

        init_fns.chain(extern_module_init_fns).collect()
    }

    pub fn imports(&self) -> Vec<ImportRequirement> {
        self.type_imports.iter().cloned().collect()
    }
}

/// Get the name of the interface and class name for a trait.
///
/// For a regular `struct Foo` or `trait Foo`, there's `FooInterface` with `Foo` as
/// the name of the (Rust implemented) object. But if it's a foreign trait:
/// * The name `Foo` is the name of the interface used by a the Kotlin implementation of the trait.
/// * The Rust implemented object is `FooImpl`.
///
/// This all impacts what types `FfiConverter.lower()` inputs.  If it's a "foreign trait"
/// `lower` must lower anything that implements the interface (ie, a kotlin implementation).
/// If not, then lower only lowers the concrete class (ie, our simple instance with the pointer).
fn object_interface_name(ci: &KotlinCi<'_>, obj: &Object) -> String {
    let class_name = KotlinCodeOracle.class_name(ci, obj.name());
    if obj.has_callback_interface() {
        class_name
    } else {
        format!("{class_name}Interface")
    }
}

// *sigh* - same thing for a trait, which might be either Object or CallbackInterface.
// (we should either fold it into object or kill it!)
fn trait_interface_name(ci: &KotlinCi<'_>, trait_ty: &Type) -> Result<String> {
    let Some(crate_name) = trait_ty.crate_name() else {
        bail!("Invalid trait_type: {trait_ty:?}");
    };
    let Some(ci_look) = ci.find_component_interface(crate_name) else {
        anyhow::bail!("no interface with crate_name: {}", crate_name);
    };

    let (obj_name, has_callback_interface) = match trait_ty {
        Type::Object { name, .. } => {
            let Some(obj) = ci_look.get_object_definition(name) else {
                bail!("trait interface not found: {}", name);
            };
            (name, obj.has_callback_interface())
        }
        Type::CallbackInterface { name, .. } => (name, true),
        _ => {
            bail!("Invalid trait_type: {trait_ty:?}")
        }
    };

    let class_name = KotlinCodeOracle.class_name(ci_look, obj_name);
    let interface_name = if has_callback_interface {
        class_name
    } else {
        format!("{class_name}Interface")
    };
    Ok(match ci.external_package(trait_ty) {
        Some(package) => format!("{package}.{interface_name}"),
        None => interface_name,
    })
}

// The name of the object exposing a Rust implementation.
fn object_impl_name(ci: &KotlinCi<'_>, obj: &Object) -> String {
    let class_name = KotlinCodeOracle.class_name(ci, obj.name());
    if obj.has_callback_interface() {
        format!("{class_name}Impl")
    } else {
        class_name
    }
}

#[derive(Clone)]
pub struct KotlinCodeOracle;

impl KotlinCodeOracle {
    fn find(&self, type_: &Type) -> Box<dyn CodeType> {
        type_.clone().as_type().as_codetype()
    }

    /// Get the idiomatic Kotlin rendering of a class name (for enums, records, errors, etc).
    ///
    /// `nm` names a local item; see `type_class_name` for a type of any crate.
    fn class_name(&self, ci: &ComponentInterface, nm: &str) -> String {
        match ci.get_type(nm) {
            Some(ty) => self.type_class_name(ci, &ty),
            None => nm.to_string().to_upper_camel_case(),
        }
    }

    /// The unqualified class name of a named type, which ends in `Exception` for an error.
    fn type_class_name(&self, ci: &ComponentInterface, ty: &Type) -> String {
        let name = ty
            .name()
            .expect("a named type")
            .to_string()
            .to_upper_camel_case();
        // fixup errors.
        if ci.is_used_as_error(ty) {
            self.convert_error_suffix(&name)
        } else {
            name
        }
    }

    fn convert_error_suffix(&self, nm: &str) -> String {
        match nm.strip_suffix("Error") {
            None => nm.to_string(),
            Some(stripped) => format!("{stripped}Exception"),
        }
    }

    /// Get the idiomatic Kotlin rendering of a function name.
    fn fn_name(&self, nm: &str) -> String {
        format!("`{}`", nm.to_string().to_lower_camel_case())
    }

    /// Get the idiomatic Kotlin rendering of a variable name.
    fn var_name(&self, nm: &str) -> String {
        format!("`{}`", self.var_name_raw(nm))
    }

    /// `var_name` without the backticks.  Useful for using in `@Structure.FieldOrder`.
    pub fn var_name_raw(&self, nm: &str) -> String {
        nm.to_string().to_lower_camel_case()
    }

    /// Get the idiomatic Kotlin rendering of an individual enum variant.
    fn enum_variant_name(&self, nm: &str) -> String {
        nm.to_string().to_shouty_snake_case()
    }

    /// Get the idiomatic Kotlin rendering of an FFI callback function name
    fn ffi_callback_name(&self, nm: &str) -> String {
        format!("Uniffi{}", nm.to_upper_camel_case())
    }

    /// Get the idiomatic Kotlin rendering of an FFI struct name
    fn ffi_struct_name(&self, nm: &str) -> String {
        format!("Uniffi{}", nm.to_upper_camel_case())
    }

    fn ffi_type_label_by_value(&self, ffi_type: &FfiType, ci: &KotlinCi<'_>) -> String {
        match ffi_type {
            FfiType::RustBuffer(_) => format!("{}.ByValue", self.ffi_type_label(ffi_type, ci)),
            FfiType::Struct(name) => format!("{}.UniffiByValue", self.ffi_struct_name(name)),
            _ => self.ffi_type_label(ffi_type, ci),
        }
    }

    /// Kotlin/JNA direct mapping can mis-handle unsigned 8/16-bit direct return values
    /// on some runtimes, so widen the raw carrier to Int and let the generated
    /// converters lift it back into the public UByte/UShort API types.
    fn ffi_type_label_for_direct_return(&self, ffi_type: &FfiType, ci: &KotlinCi<'_>) -> String {
        match ffi_type {
            FfiType::UInt8 | FfiType::UInt16 => "Int".to_string(),
            _ => self.ffi_type_label_by_value(ffi_type, ci),
        }
    }

    /// FFI type name to use inside structs
    ///
    /// The main requirement here is that all types must have default values or else the struct
    /// won't work in some JNA contexts.
    fn ffi_type_label_for_ffi_struct(&self, ffi_type: &FfiType, ci: &KotlinCi<'_>) -> String {
        match ffi_type {
            // Make callbacks function pointers nullable. This matches the semantics of a C
            // function pointer better and allows for `null` as a default value.
            FfiType::Callback(name) => format!("{}?", self.ffi_callback_name(name)),
            _ => self.ffi_type_label_by_value(ffi_type, ci),
        }
    }

    /// Default values for FFI
    ///
    /// This is used to:
    ///   - Set a default return value for error results
    ///   - Set a default for structs, which JNA sometimes requires
    fn ffi_default_value(&self, ffi_type: &FfiType) -> String {
        match ffi_type {
            FfiType::UInt8 | FfiType::Int8 => "0.toByte()".to_owned(),
            FfiType::UInt16 | FfiType::Int16 => "0.toShort()".to_owned(),
            FfiType::UInt32 | FfiType::Int32 => "0".to_owned(),
            FfiType::UInt64 | FfiType::Int64 => "0.toLong()".to_owned(),
            FfiType::Float32 => "0.0f".to_owned(),
            FfiType::Float64 => "0.0".to_owned(),
            FfiType::Handle => "0.toLong()".to_owned(),
            FfiType::RustBuffer(_) => "RustBuffer.ByValue()".to_owned(),
            FfiType::Callback(_) => "null".to_owned(),
            FfiType::RustCallStatus => "UniffiRustCallStatus.ByValue()".to_owned(),
            _ => unimplemented!("ffi_default_value: {ffi_type:?}"),
        }
    }

    fn ffi_type_label_by_reference(&self, ffi_type: &FfiType, ci: &KotlinCi<'_>) -> String {
        match ffi_type {
            FfiType::Int8
            | FfiType::UInt8
            | FfiType::Int16
            | FfiType::UInt16
            | FfiType::Int32
            | FfiType::UInt32
            | FfiType::Int64
            | FfiType::UInt64
            | FfiType::Float32
            | FfiType::Float64
            | FfiType::Handle => format!("{}ByReference", self.ffi_type_label(ffi_type, ci)),
            // JNA structs default to ByReference
            FfiType::RustBuffer(_) | FfiType::Struct(_) => self.ffi_type_label(ffi_type, ci),
            _ => panic!("{ffi_type:?} by reference is not implemented"),
        }
    }

    fn ffi_type_label(&self, ffi_type: &FfiType, ci: &KotlinCi<'_>) -> String {
        match ffi_type {
            // Note that unsigned integers in Kotlin are currently experimental, but java.nio.ByteBuffer does not
            // support them yet. Thus, we use the signed variants to represent both signed and unsigned
            // types from the component API.
            FfiType::Int8 | FfiType::UInt8 => "Byte".to_string(),
            FfiType::Int16 | FfiType::UInt16 => "Short".to_string(),
            FfiType::Int32 | FfiType::UInt32 => "Int".to_string(),
            FfiType::Int64 | FfiType::UInt64 => "Long".to_string(),
            FfiType::Float32 => "Float".to_string(),
            FfiType::Float64 => "Double".to_string(),
            FfiType::Handle => "Long".to_string(),
            // Another crate's buffers are its own JNA structure.
            FfiType::RustBuffer(maybe_external) => match maybe_external {
                Some(external_meta) if external_meta.crate_name() != ci.crate_name() => {
                    format!(
                        "{}.RustBuffer",
                        ci.package_for_module_path(&external_meta.module_path)
                    )
                }
                _ => "RustBuffer".to_string(),
            },
            FfiType::RustCallStatus => "UniffiRustCallStatus.ByValue".to_string(),
            FfiType::ForeignBytes => "ForeignBytes.ByValue".to_string(),
            FfiType::Callback(name) => self.ffi_callback_name(name),
            FfiType::Struct(name) => self.ffi_struct_name(name),
            FfiType::Reference(inner) | FfiType::MutReference(inner) => {
                self.ffi_type_label_by_reference(inner, ci)
            }
            FfiType::VoidPointer => "Pointer".to_string(),
        }
    }
}

trait AsCodeType {
    fn as_codetype(&self) -> Box<dyn CodeType>;
}

impl<T: AsType> AsCodeType for T {
    fn as_codetype(&self) -> Box<dyn CodeType> {
        // Map `Type` instances to a `Box<dyn CodeType>` for that type.
        //
        // There is a companion match in `templates/Types.kt` which performs a similar function for the
        // template code.
        //
        //   - When adding additional types here, make sure to also add a match arm to the `Types.kt` template.
        //   - To keep things manageable, let's try to limit ourselves to these 2 mega-matches
        match self.as_type() {
            Type::UInt8 => Box::new(primitives::UInt8CodeType),
            Type::Int8 => Box::new(primitives::Int8CodeType),
            Type::UInt16 => Box::new(primitives::UInt16CodeType),
            Type::Int16 => Box::new(primitives::Int16CodeType),
            Type::UInt32 => Box::new(primitives::UInt32CodeType),
            Type::Int32 => Box::new(primitives::Int32CodeType),
            Type::UInt64 => Box::new(primitives::UInt64CodeType),
            Type::Int64 => Box::new(primitives::Int64CodeType),
            Type::Float32 => Box::new(primitives::Float32CodeType),
            Type::Float64 => Box::new(primitives::Float64CodeType),
            Type::Boolean => Box::new(primitives::BooleanCodeType),
            Type::String => Box::new(primitives::StringCodeType),
            Type::Bytes => Box::new(primitives::BytesCodeType),

            Type::Timestamp => Box::new(miscellany::TimestampCodeType),
            Type::Duration => Box::new(miscellany::DurationCodeType),

            ty @ Type::Enum { .. } => Box::new(enum_::EnumCodeType::new(ty)),
            ty @ Type::Object { .. } => Box::new(object::ObjectCodeType::new(ty)),
            ty @ Type::Record { .. } => Box::new(record::RecordCodeType::new(ty)),
            ty @ Type::CallbackInterface { .. } => {
                Box::new(callback_interface::CallbackInterfaceCodeType::new(ty))
            }
            Type::Optional { inner_type } => {
                Box::new(compounds::OptionalCodeType::new(*inner_type))
            }
            Type::Sequence { inner_type } => {
                Box::new(compounds::SequenceCodeType::new(*inner_type))
            }
            Type::Map {
                key_type,
                value_type,
            } => Box::new(compounds::MapCodeType::new(*key_type, *value_type)),
            Type::Set { inner_type } => Box::new(compounds::SetCodeType::new(*inner_type)),
            ty @ Type::Custom { .. } => Box::new(custom::CustomCodeType::new(ty)),
            Type::Box { inner_type } => inner_type.as_codetype(),
        }
    }
}

mod filters {
    use super::*;
    use uniffi_meta::LiteralMetadata;

    #[askama::filter_fn]
    pub(super) fn type_name(
        as_ct: &impl AsCodeType,
        _: &dyn askama::Values,
        ci: &KotlinCi<'filter>,
    ) -> Result<String, askama::Error> {
        Ok(as_ct.as_codetype().type_label(ci))
    }

    #[askama::filter_fn]
    pub(super) fn canonical_name(
        as_ct: &impl AsCodeType,
        _: &dyn askama::Values,
        ci: &KotlinCi<'filter>,
    ) -> Result<String, askama::Error> {
        Ok(as_ct.as_codetype().canonical_name(ci))
    }

    #[askama::filter_fn]
    pub(super) fn qualified_type_name<T>(
        as_type: &T,
        _: &dyn askama::Values,
        ci: &KotlinCi<'filter>,
    ) -> Result<String, askama::Error>
    where
        T: AsCodeType + AsType,
    {
        Ok(fully_qualified_type_label(&as_type.as_type(), ci))
    }

    #[askama::filter_fn]
    pub(super) fn ffi_converter_name(
        as_ct: &impl AsCodeType,
        _: &dyn askama::Values,
        ci: &KotlinCi<'filter>,
    ) -> Result<String, askama::Error> {
        Ok(as_ct.as_codetype().ffi_converter_name(ci))
    }

    #[askama::filter_fn]
    pub(super) fn ffi_type(
        type_: &impl AsType,
        _: &dyn askama::Values,
    ) -> askama::Result<FfiType, askama::Error> {
        Ok(type_.as_type().into())
    }

    #[askama::filter_fn]
    pub(super) fn lower_fn(
        as_ct: &impl AsCodeType,
        _: &dyn askama::Values,
        ci: &KotlinCi<'filter>,
    ) -> Result<String, askama::Error> {
        Ok(format!(
            "{}.lower",
            as_ct.as_codetype().ffi_converter_name(ci)
        ))
    }

    #[askama::filter_fn]
    pub(super) fn allocation_size_fn(
        as_ct: &impl AsCodeType,
        _: &dyn askama::Values,
        ci: &KotlinCi<'filter>,
    ) -> Result<String, askama::Error> {
        Ok(format!(
            "{}.allocationSize",
            as_ct.as_codetype().ffi_converter_name(ci)
        ))
    }

    #[askama::filter_fn]
    pub(super) fn write_fn(
        as_ct: &impl AsCodeType,
        _: &dyn askama::Values,
        ci: &KotlinCi<'filter>,
    ) -> Result<String, askama::Error> {
        Ok(format!(
            "{}.write",
            as_ct.as_codetype().ffi_converter_name(ci)
        ))
    }

    #[askama::filter_fn]
    pub(super) fn lift_fn(
        as_ct: &impl AsCodeType,
        _: &dyn askama::Values,
        ci: &KotlinCi<'filter>,
    ) -> Result<String, askama::Error> {
        Ok(format!(
            "{}.lift",
            as_ct.as_codetype().ffi_converter_name(ci)
        ))
    }

    #[askama::filter_fn]
    pub(super) fn read_fn(
        as_ct: &impl AsCodeType,
        _: &dyn askama::Values,
        ci: &KotlinCi<'filter>,
    ) -> Result<String, askama::Error> {
        Ok(format!(
            "{}.read",
            as_ct.as_codetype().ffi_converter_name(ci)
        ))
    }

    fn fully_qualified_type_label(ty: &Type, ci: &KotlinCi<'_>) -> String {
        match ty {
            Type::Optional { inner_type } => {
                format!("{}?", fully_qualified_type_label(inner_type, ci))
            }
            Type::Sequence { inner_type } => {
                format!("List<{}>", fully_qualified_type_label(inner_type, ci))
            }
            Type::Map {
                key_type,
                value_type,
            } => format!(
                "Map<{}, {}>",
                fully_qualified_type_label(key_type, ci),
                fully_qualified_type_label(value_type, ci)
            ),
            Type::Set { inner_type } => {
                format!("Set<{}>", fully_qualified_type_label(inner_type, ci))
            }
            Type::Enum { .. }
            | Type::Record { .. }
            | Type::Object { .. }
            | Type::CallbackInterface { .. }
            | Type::Custom { .. } => match ci.is_external(ty) {
                true => ci.named_type_label(ty),
                false => format!(
                    "{}.{}",
                    ci.config.package_name(),
                    KotlinCodeOracle.type_class_name(ci, ty)
                ),
            },
            _ => KotlinCodeOracle.find(ty).type_label(ci),
        }
    }

    #[askama::filter_fn]
    pub fn render_default<T: AsType>(
        default: &DefaultValue,
        _: &dyn askama::Values,
        as_ct: &T,
        ci: &KotlinCi<'filter>,
    ) -> Result<String, askama::Error> {
        as_ct
            .as_codetype()
            .default(default, ci)
            .map_err(|e| to_askama_error(&e))
    }

    // Get the idiomatic Kotlin rendering of an integer.
    fn int_literal(t: &Option<Type>, base10: String) -> Result<String, askama::Error> {
        if let Some(t) = t {
            match t {
                Type::Int8 | Type::Int16 | Type::Int32 | Type::Int64 => Ok(base10),
                Type::UInt8 | Type::UInt16 | Type::UInt32 | Type::UInt64 => Ok(base10 + "u"),
                _ => Err(to_askama_error(&format!(
                    "Only ints are supported for enum literals: {t:?}"
                ))),
            }
        } else {
            Err(to_askama_error(&format!(
                "Enum hasn't defined a repr: {t:?}"
            )))
        }
    }

    // Get the idiomatic Kotlin rendering of an individual enum variant's discriminant
    #[askama::filter_fn]
    pub fn variant_discr_literal(
        e: &Enum,
        _: &dyn askama::Values,
        index: &usize,
    ) -> Result<String, askama::Error> {
        let literal = e.variant_discr(*index).expect("invalid index");
        match literal {
            // Kotlin doesn't convert between signed and unsigned by default
            // so we'll need to make sure we define the type as appropriately
            LiteralMetadata::UInt(v, _, _) => int_literal(e.variant_discr_type(), v.to_string()),
            LiteralMetadata::Int(v, _, _) => int_literal(e.variant_discr_type(), v.to_string()),
            _ => Err(to_askama_error(&format!(
                "Only ints are supported: {literal:?}"
            ))),
        }
    }

    #[askama::filter_fn]
    pub fn ffi_type_name_by_value(
        type_: &FfiType,
        _: &dyn askama::Values,
        ci: &KotlinCi<'filter>,
    ) -> Result<String, askama::Error> {
        Ok(KotlinCodeOracle.ffi_type_label_by_value(type_, ci))
    }

    #[askama::filter_fn]
    pub fn ffi_type_name_for_direct_return(
        type_: &FfiType,
        _: &dyn askama::Values,
        ci: &KotlinCi<'filter>,
    ) -> Result<String, askama::Error> {
        Ok(KotlinCodeOracle.ffi_type_label_for_direct_return(type_, ci))
    }

    #[askama::filter_fn]
    pub fn ffi_type_name_for_ffi_struct(
        type_: &FfiType,
        _: &dyn askama::Values,
        ci: &KotlinCi<'filter>,
    ) -> Result<String, askama::Error> {
        Ok(KotlinCodeOracle.ffi_type_label_for_ffi_struct(type_, ci))
    }

    #[askama::filter_fn]
    pub fn ffi_default_value(
        type_: FfiType,
        _: &dyn askama::Values,
    ) -> Result<String, askama::Error> {
        Ok(KotlinCodeOracle.ffi_default_value(&type_))
    }

    /// Get the idiomatic Kotlin rendering of a function name.
    #[askama::filter_fn]
    pub fn class_name<S: AsRef<str>>(
        nm: S,
        _: &dyn askama::Values,
        ci: &KotlinCi<'filter>,
    ) -> Result<String, askama::Error> {
        Ok(KotlinCodeOracle.class_name(ci, nm.as_ref()))
    }

    /// Get the idiomatic Kotlin rendering of a function name.
    #[askama::filter_fn]
    pub fn fn_name<S: AsRef<str>>(nm: S, _: &dyn askama::Values) -> Result<String, askama::Error> {
        Ok(KotlinCodeOracle.fn_name(nm.as_ref()))
    }

    /// Get the idiomatic Kotlin rendering of a variable name.
    #[askama::filter_fn]
    pub fn var_name<S: AsRef<str>>(nm: S, _: &dyn askama::Values) -> Result<String, askama::Error> {
        Ok(KotlinCodeOracle.var_name(nm.as_ref()))
    }

    /// Get the idiomatic Kotlin rendering of a variable name.
    #[askama::filter_fn]
    pub fn var_name_raw<S: AsRef<str>>(
        nm: S,
        _: &dyn askama::Values,
    ) -> Result<String, askama::Error> {
        Ok(KotlinCodeOracle.var_name_raw(nm.as_ref()))
    }

    /// Per-argument override of `type_name` for the foreign->Rust (lower)
    /// direction only. Routes borrowed `Bytes` to `java.nio.ByteBuffer` —
    /// the only Kotlin type JNA can expose a native pointer to. Other args
    /// take the per-Type `type_name` path. Not used for Rust->Kotlin
    /// (callback / lift) positions.
    #[askama::filter_fn]
    pub(super) fn lower_type_name_for_arg(
        arg: &Argument,
        _: &dyn askama::Values,
        ci: &KotlinCi<'filter>,
    ) -> Result<String, askama::Error> {
        if arg.is_borrowed_bytes() {
            Ok("java.nio.ByteBuffer".to_string())
        } else {
            Ok(arg.as_codetype().type_label(ci))
        }
    }

    /// Per-argument override of `lower_fn` that routes borrowed `Bytes`
    /// through `FfiConverterByRefBytes.lower` (zero-copy). Other args take
    /// the per-Type `lower_fn` path.
    #[askama::filter_fn]
    pub(super) fn lower_fn_for_arg(
        arg: &Argument,
        _: &dyn askama::Values,
        ci: &KotlinCi<'filter>,
    ) -> Result<String, askama::Error> {
        if arg.is_borrowed_bytes() {
            Ok("FfiConverterByRefBytes.lower".to_string())
        } else {
            Ok(format!(
                "{}.lower",
                arg.as_codetype().ffi_converter_name(ci)
            ))
        }
    }

    /// Get a String representing the name used for an individual enum variant.
    #[askama::filter_fn]
    pub fn variant_name(v: &Variant, _: &dyn askama::Values) -> Result<String, askama::Error> {
        Ok(KotlinCodeOracle.enum_variant_name(v.name()))
    }

    #[askama::filter_fn]
    pub fn error_variant_name(
        v: &Variant,
        _: &dyn askama::Values,
    ) -> Result<String, askama::Error> {
        let name = v.name().to_string().to_upper_camel_case();
        Ok(KotlinCodeOracle.convert_error_suffix(&name))
    }

    /// Get the idiomatic Kotlin rendering of an FFI callback function name
    #[askama::filter_fn]
    pub fn ffi_callback_name<S: AsRef<str>>(
        nm: S,
        _: &dyn askama::Values,
    ) -> Result<String, askama::Error> {
        Ok(KotlinCodeOracle.ffi_callback_name(nm.as_ref()))
    }

    /// Get the idiomatic Kotlin rendering of an FFI struct name
    #[askama::filter_fn]
    pub fn ffi_struct_name<S: AsRef<str>>(
        nm: S,
        _: &dyn askama::Values,
    ) -> Result<String, askama::Error> {
        Ok(KotlinCodeOracle.ffi_struct_name(nm.as_ref()))
    }

    #[askama::filter_fn]
    pub fn async_poll(
        callable: impl Callable,
        _: &dyn askama::Values,
        ci: &KotlinCi<'filter>,
    ) -> Result<String, askama::Error> {
        let ffi_func = callable.ffi_rust_future_poll(ci);
        Ok(format!(
            "{{ future, callback, continuation -> UniffiLib.{ffi_func}(future, callback, continuation) }}"
        ))
    }

    #[askama::filter_fn]
    pub fn async_complete(
        callable: impl Callable,
        _: &dyn askama::Values,
        ci: &KotlinCi<'filter>,
    ) -> Result<String, askama::Error> {
        let ffi_func = callable.ffi_rust_future_complete(ci);
        let call = format!("UniffiLib.{ffi_func}(future, continuation)");
        // May need to convert the RustBuffer from our package to the RustBuffer of the external package.
        let call = match callable.return_type() {
            Some(return_type) => match FfiType::from(return_type) {
                FfiType::RustBuffer(Some(external_meta))
                    if external_meta.crate_name() != ci.crate_name() =>
                {
                    let ExternalFfiMetadata { module_path, .. } = external_meta;
                    let package = ci.package_for_module_path(&module_path);
                    format!("{call}.let {{ {package}.RustBuffer.create(it.capacity.toULong(), it.len.toULong(), it.data) }}")
                }
                _ => call,
            },
            _ => call,
        };
        Ok(format!("{{ future, continuation -> {call} }}"))
    }

    #[askama::filter_fn]
    pub fn async_free(
        callable: impl Callable,
        _: &dyn askama::Values,
        ci: &KotlinCi<'filter>,
    ) -> Result<String, askama::Error> {
        let ffi_func = callable.ffi_rust_future_free(ci);
        Ok(format!("{{ future -> UniffiLib.{ffi_func}(future) }}"))
    }

    /// Remove the "`" chars we put around function/variable names
    ///
    /// These are used to avoid name clashes with kotlin identifiers, but sometimes you want to
    /// render the name unquoted.  One example is the message property for errors where we want to
    /// display the name for the user.
    #[askama::filter_fn]
    pub fn unquote<S: AsRef<str>>(nm: S, _: &dyn askama::Values) -> Result<String, askama::Error> {
        Ok(nm.as_ref().trim_matches('`').to_string())
    }

    /// Get the idiomatic Kotlin rendering of docstring
    #[askama::filter_fn]
    pub fn docstring<S: AsRef<str>>(
        docstring: S,
        _: &dyn askama::Values,
        spaces: &i32,
    ) -> Result<String, askama::Error> {
        let middle = textwrap::indent(&textwrap::dedent(docstring.as_ref()), " * ");
        let wrapped = format!("/**\n{middle}\n */");

        let spaces = usize::try_from(*spaces).unwrap_or_default();
        Ok(textwrap::indent(&wrapped, &" ".repeat(spaces)))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_kotlin_version() {
        assert_eq!(
            KotlinVersion::parse("1.2.3").unwrap(),
            KotlinVersion::new(1, 2, 3)
        );
        assert_eq!(
            KotlinVersion::parse("2.3").unwrap(),
            KotlinVersion::new(2, 3, 0),
        );
        assert_eq!(
            KotlinVersion::parse("2").unwrap(),
            KotlinVersion::new(2, 0, 0),
        );
        assert!(KotlinVersion::parse("2.").is_err());
        assert!(KotlinVersion::parse("").is_err());
        assert!(KotlinVersion::parse("A.B.C").is_err());
        assert!(KotlinVersion::new(1, 2, 3) > KotlinVersion::new(0, 1, 2));
        assert!(KotlinVersion::new(1, 2, 3) > KotlinVersion::new(0, 100, 0));
        assert!(KotlinVersion::new(10, 0, 0) > KotlinVersion::new(1, 10, 0));
    }

    #[test]
    fn checksum_functions_use_direct_return_carriers() {
        let ci = ComponentInterface::from_webidl(
            r#"
            namespace test_crate {
                u16 get_value();
            };
            "#,
            "test_crate",
        )
        .unwrap();
        let config = Config {
            package_name: Some("uniffi.test_crate".to_string()),
            cdylib_name: Some("test_crate".to_string()),
            ..Config::default()
        };

        let bindings = generate_bindings(&config, &ci).unwrap();
        let checksum_checks = bindings
            .split("private fun uniffiCheckApiChecksums")
            .nth(1)
            .expect("generated checksum checks")
            .split("/**")
            .next()
            .expect("end of generated checksum checks");

        assert!(
            bindings
                .contains("external fun uniffi_test_crate_checksum_func_get_value(\n    ): Int"),
            "checksum functions should use Int as the JNA direct return carrier"
        );
        assert!(
            !checksum_checks.contains(".toShort())"),
            "checksum comparisons should compare the widened Int carrier directly"
        );
    }
}
