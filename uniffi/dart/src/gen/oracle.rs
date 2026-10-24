use genco::lang::dart;
use genco::quote;
use heck::{ToLowerCamelCase, ToUpperCamelCase};
use uniffi_bindgen::interface::{
    Argument, AsType, Callable, FfiType, Object, ObjectImpl, TraitKind, Type,
};
use uniffi_bindgen::ComponentInterface;

// use super::render::{AsRenderable, Renderable};
use super::{callback_interface, compounds, custom, enums, objects, records};
use crate::gen::{primitives, CodeType};

pub struct DartCodeOracle;

impl DartCodeOracle {
    /// The namespace of the crate that defines `ty`, if it is not `ci`'s crate.
    pub fn external_namespace(ty: &Type, ci: &ComponentInterface) -> Option<String> {
        ci.is_external(ty).then(|| {
            ci.namespace_for_type(ty)
                .expect("external type should have a namespace")
                .to_string()
        })
    }

    /// The prefix the library of `namespace` is imported under.
    ///
    /// Another crate's library is always imported under a prefix, so a local type of the same
    /// name cannot shadow its types. The bare namespace would itself be shadowed by a member of
    /// the same name, such as a record field `net` holding `net` types; generated members are
    /// lowerCamelCase, so they never contain the underscore.
    pub fn import_prefix(namespace: &str) -> String {
        format!("uniffi_{namespace}")
    }

    /// Refers to `name` through the library of `namespace` that declares it, if it is not local.
    pub fn qualify(namespace: Option<&str>, name: String) -> String {
        match namespace {
            Some(namespace) => format!("{}.{name}", Self::import_prefix(namespace)),
            None => name,
        }
    }

    /// The canonical name of a type from the library of `namespace`, unique within this one.
    pub fn qualify_canonical(namespace: Option<&str>, canonical: String) -> String {
        match namespace {
            Some(namespace) => format!("{}{canonical}", namespace.to_upper_camel_case()),
            None => canonical,
        }
    }

    /// The handler that lifts `error_type` when a call throws it.
    pub fn error_handler_name(error_type: &Type, ci: &ComponentInterface) -> String {
        let error_name = Self::class_name(error_type.name().unwrap_or("UnknownError"));
        Self::qualify(
            Self::external_namespace(error_type, ci).as_deref(),
            format!("{}ErrorHandler", error_name.to_lower_camel_case()),
        )
    }

    /// Sanitize a Dart identifier, appending an underscore if it's a reserved keyword.
    pub fn sanitize_identifier(id: &str) -> String {
        if Self::is_reserved_identifier(id) {
            format!("{id}_")
        } else {
            id.to_string()
        }
    }

    /// Check if the given identifier is a reserved keyword in Dart.
    pub fn is_reserved_identifier(id: &str) -> bool {
        RESERVED_IDENTIFIERS.contains(&id)
    }

    /// Get the idiomatic Dart rendering of a class name (for enums, records, errors, etc).
    pub fn class_name(nm: &str) -> String {
        let name = Self::sanitize_identifier(&nm.to_upper_camel_case());
        // Handle the special case where the name is exactly "Error" to avoid conflicts with Dart's Exception
        if name == "Error" {
            "ErrorException".to_string()
        } else {
            // Only replace "Error" at the end of the name for other cases
            match name.strip_suffix("Error") {
                None => name,
                Some(stripped) => format!("{stripped}Exception"),
            }
        }
    }

    /// Replace any occurrence of `Error` with `Exception` to avoid Dart naming conflicts.
    pub fn exception_safe_name(name: &str) -> String {
        name.replace("Error", "Exception")
    }

    /// Get the idiomatic Dart rendering of a function name.
    pub fn fn_name(nm: &str) -> String {
        Self::sanitize_identifier(&nm.to_lower_camel_case())
    }

    /// Get the idiomatic Dart rendering of a variable name.
    pub fn var_name(nm: &str) -> String {
        Self::sanitize_identifier(&nm.to_lower_camel_case())
    }

    /// Get the idiomatic Dart rendering of an individual enum variant.
    pub fn enum_variant_name(nm: &str) -> String {
        Self::sanitize_identifier(&nm.to_lower_camel_case())
    }

    /// Get the idiomatic Dart rendering of an FFI callback function name
    pub fn ffi_callback_name(nm: &str) -> String {
        format!("Pointer<NativeFunction<{}>>", Self::callback_name(&nm.to_upper_camel_case()))
    }

    /// Helper method to generate the callback name based on `Type`.
    fn callback_name(name: &str) -> String {
        format!("Uniffi{}", name.to_upper_camel_case())
    }

    ///// Get the idiomatic Dart rendering of an exception name
    // pub fn error_name(nm: &str) -> String {
    //     let name = Self::class_name(nm);
    //     match name.strip_suffix("Error") {
    //         None => name,
    //         Some(stripped) => format!("{stripped}Exception"),
    //     }
    // }

    pub fn infer_ffi_module<F>(ci: &ComponentInterface, fallback: F) -> String
    where
        F: FnOnce() -> String,
    {
        ci.iter_ffi_function_definitions()
            .next()
            .and_then(|f| {
                let name = f.name();
                name.strip_prefix("uniffi_")
                    .and_then(|rest| rest.split("_fn_").next())
                    .map(|s| s.replace('-', "_"))
            })
            .unwrap_or_else(|| {
                let fallback_value = fallback();
                fallback_value.replace('-', "_")
            })
    }

    // TODO: Replace instances of `generate_ffi_dart_type` with ffi_type_label
    pub fn ffi_dart_type_label(ffi_type: Option<&FfiType>) -> dart::Tokens {
        if let Some(ret_type) = ffi_type {
            match ret_type {
                FfiType::Int8 => quote!(int),
                FfiType::UInt8 => quote!(int),
                FfiType::Int16 => quote!(int),
                FfiType::UInt16 => quote!(int),
                FfiType::Int32 => quote!(int),
                FfiType::UInt32 => quote!(int),
                FfiType::Int64 => quote!(int),
                FfiType::UInt64 => quote!(int),
                FfiType::Float32 => quote!(double),
                FfiType::Float64 => quote!(double),
                FfiType::RustBuffer(_) => quote!(RustBuffer),
                FfiType::ForeignBytes => quote!(ForeignBytes),
                FfiType::Handle => quote!(Pointer<Void>),
                FfiType::Callback(name) => quote!($(Self::ffi_callback_name(name))),
                FfiType::Reference(inner) => quote!($(Self::ffi_type_label_by_reference(inner))),
                _ => panic!("Unimplemented FfiType: {ret_type:?}"), // Fallback implementation
            }
        } else {
            quote!(void)
        }
    }

    pub fn ffi_native_type_label(ffi_ret_type: Option<&FfiType>) -> dart::Tokens {
        if let Some(ret_type) = ffi_ret_type {
            match ret_type {
                FfiType::Int8 => quote!(Int8),
                FfiType::UInt8 => quote!(Uint8),
                FfiType::Int16 => quote!(Int16),
                FfiType::UInt16 => quote!(Uint16),
                FfiType::Int32 => quote!(Int32),
                FfiType::UInt32 => quote!(Uint32),
                FfiType::Int64 => quote!(Int64),
                FfiType::UInt64 => quote!(Uint64),
                FfiType::Float32 => quote!(Float),
                FfiType::Float64 => quote!(Double),
                FfiType::RustBuffer(_) => quote!(RustBuffer),
                FfiType::ForeignBytes => quote!(ForeignBytes),
                FfiType::Handle => quote!(Pointer<Void>),
                FfiType::Callback(name) => quote!($(Self::ffi_callback_name(name))),
                FfiType::Reference(inner) => quote!($(Self::ffi_type_label_by_reference(inner))),
                _ => panic!("Unimplemented FfiType: {ret_type:?}"), // Fallback implementation
            }
        } else {
            quote!(void)
        }
    }

    fn ffi_type_label_by_reference(ffi_type: &FfiType) -> dart::Tokens {
        match ffi_type {
            FfiType::Int8 => quote!(Int8),
            FfiType::UInt8 => quote!(Uint8),
            FfiType::Int16 => quote!(Int16),
            FfiType::UInt16 => quote!(Uint16),
            FfiType::Int32 => quote!(Int32),
            FfiType::UInt32 => quote!(Uint32),
            FfiType::Int64 => quote!(Int64),
            FfiType::UInt64 => quote!(Uint64),
            FfiType::Float32 => quote!(Float),
            FfiType::Float64 => quote!(Double),
            FfiType::RustBuffer(_) => quote!(RustBuffer),
            FfiType::Callback(name) => quote!(Pointer<$(Self::ffi_callback_name(name))>),
            FfiType::Struct(name) => quote!(Pointer<$(Self::ffi_struct_name(name))>),
            _ => quote!(Pointer<Void>), // Fallback implementation
        }
    }

    pub fn ffi_struct_name(name: &str) -> dart::Tokens {
        quote!($(format!("Uniffi{}", name.to_upper_camel_case())))
    }

    // pub fn convert_from_rust_buffer(ty: &Type, inner: dart::Tokens) -> dart::Tokens {
    //     match ty {
    //         Type::Object { .. } => inner,
    //         Type::String | Type::Optional { .. } => quote!($(inner).asUint8List()),
    //         _ => inner,
    //     }
    // }

    // pub fn convert_to_rust_buffer(ty: &Type, inner: dart::Tokens) -> dart::Tokens {
    //     match ty {
    //         Type::Object { .. } => inner,
    //         Type::String | Type::Optional { .. } | Type::Enum { .. } | Type::Sequence { .. } => {
    //             quote!(toRustBuffer($inner))
    //         }
    //         _ => inner,
    //     }
    // }

    pub fn type_lower_fn(ty: &Type, inner: dart::Tokens, ci: &ComponentInterface) -> dart::Tokens {
        match ty {
            Type::Int8
            | Type::Int16
            | Type::Int32
            | Type::Int64
            | Type::UInt8
            | Type::UInt16
            | Type::UInt32
            | Type::UInt64 => {
                quote!($(ty.as_codetype(ci).ffi_converter_name()).lower($inner))
            }
            Type::Float32 | Type::Float64 => inner,
            Type::Boolean
            | Type::Duration
            | Type::String
            | Type::Object { .. }
            | Type::Enum { .. }
            | Type::Optional { .. }
            | Type::Record { .. } => {
                quote!($(ty.as_codetype(ci).ffi_converter_name()).lower($inner))
            }
            _ => quote!($(ty.as_codetype(ci).ffi_converter_name()).lower($inner)), // Fallback implementation
        }
    }

    /// With @Native, async functions are called directly by name
    pub fn async_poll(callable: impl Callable, ci: &ComponentInterface) -> dart::Tokens {
        let ffi_func = callable.ffi_rust_future_poll(ci);
        quote!($ffi_func)
    }

    /// With @Native, async functions are called directly by name
    pub fn async_complete(callable: impl Callable, ci: &ComponentInterface) -> dart::Tokens {
        let ffi_func = callable.ffi_rust_future_complete(ci);
        quote!($ffi_func)
    }

    /// With @Native, async functions are called directly by name
    pub fn async_cancel(callable: impl Callable, ci: &ComponentInterface) -> dart::Tokens {
        let ffi_func = callable.ffi_rust_future_cancel(ci);
        quote!($ffi_func)
    }

    /// With @Native, async functions are called directly by name
    pub fn async_free(callable: impl Callable, ci: &ComponentInterface) -> dart::Tokens {
        let ffi_func = callable.ffi_rust_future_free(ci);
        quote!($ffi_func)
    }

    /// Get the idiomatic Dart rendering of a class name based on `Type`.
    pub fn dart_type_label(type_: Option<&Type>, ci: &ComponentInterface) -> dart::Tokens {
        if let Some(ret_type) = type_ {
            match ret_type {
                Type::UInt8
                | Type::UInt16
                | Type::UInt32
                | Type::UInt64
                | Type::Int8
                | Type::Int16
                | Type::Int32
                | Type::Int64 => quote!(int),
                Type::Float32 | Type::Float64 => quote!(double),
                Type::Boolean => quote!(bool),
                Type::Bytes => quote!(Uint8List),
                Type::String => quote!(String),
                Type::Timestamp => quote!(DateTime),
                Type::Duration => quote!(Duration),
                // Reference types
                Type::Object { .. } | Type::Enum { .. } | Type::Record { .. } | Type::Custom { .. } => {
                    quote!($(ret_type.as_codetype(ci).type_label()))
                }
                Type::Optional { inner_type } => {
                    let inner = DartCodeOracle::dart_type_label(Some(inner_type), ci);
                    quote!($inner?)
                }
                Type::Sequence { inner_type } => {
                    let inner = DartCodeOracle::dart_type_label(Some(inner_type), ci);
                    quote!(List<$inner>)
                }
                Type::Map { key_type, value_type, .. } => {
                    let key = DartCodeOracle::dart_type_label(Some(key_type), ci);
                    let value = DartCodeOracle::dart_type_label(Some(value_type), ci);
                    quote!(Map<$key, $value>)
                }
                _ => quote!(dynamic),
            }
        } else {
            quote!(void)
        }
    }

    /// Get the native Dart FFI type rendering based on `Type`.
    pub fn native_type_label(native_ret_type: Option<&Type>, ci: &ComponentInterface) -> dart::Tokens {
        if let Some(ret_type) = native_ret_type {
            match ret_type {
                Type::UInt8 => quote!(Uint8),
                Type::UInt16 => quote!(Uint16),
                Type::UInt32 => quote!(Uint32),
                Type::UInt64 => quote!(Uint64),
                Type::Int8 => quote!(Int8),
                Type::Int16 => quote!(Int16),
                Type::Int32 => quote!(Int32),
                Type::Int64 => quote!(Int64),
                Type::Float32 => quote!(Float),
                Type::Float64 => quote!(Double),
                Type::Boolean => quote!(Int8),
                Type::Bytes => quote!(RustBuffer),
                Type::String => quote!(RustBuffer),
                Type::Timestamp => quote!(Int64),
                Type::Duration => quote!(Int64),
                Type::Optional { inner_type } => match **inner_type {
                    Type::String => quote!(RustBuffer),
                    _ => quote!(RustBuffer),
                },
                Type::Sequence { .. } => quote!(RustBuffer),
                Type::Map { .. } => quote!(RustBuffer),
                Type::Object { .. } => quote!(Pointer<Void>),
                Type::Enum { .. } => quote!(Int32),
                Type::Record { .. } => quote!(RustBuffer),
                Type::Custom { .. } => quote!($(ret_type.as_codetype(ci).type_label())),
                _ => quote!(Pointer<Void>),
            }
        } else {
            quote!(Void)
        }
    }

    /// Get the native Dart FFI type rendering based on `Type`.
    pub fn native_dart_type_label(native_ret_type: Option<&Type>, ci: &ComponentInterface) -> dart::Tokens {
        if let Some(ret_type) = native_ret_type {
            match ret_type {
                Type::UInt8
                | Type::UInt16
                | Type::UInt32
                | Type::UInt64
                | Type::Int8
                | Type::Int16
                | Type::Int32
                | Type::Int64 => quote!(int), // Adjust based on actual FFI size
                Type::Float32 => quote!(double),
                Type::Float64 => quote!(double),
                Type::Boolean => quote!(int),
                Type::Bytes => quote!(RustBuffer),
                Type::String => quote!(RustBuffer),
                Type::Timestamp => quote!(int),
                Type::Duration => quote!(int),
                Type::Optional { inner_type } => match **inner_type {
                    Type::String => quote!(RustBuffer),
                    _ => quote!(RustBuffer),
                },
                Type::Sequence { .. } => quote!(RustBuffer),
                Type::Map { .. } => quote!(RustBuffer),
                Type::Object { .. } => quote!(Pointer<Void>),
                Type::Enum { .. } => quote!(int),
                Type::Record { .. } => quote!(RustBuffer),
                Type::Custom { .. } => quote!($(ret_type.as_codetype(ci).type_label())),
                _ => quote!(dynamic),
            }
        } else {
            quote!(void)
        }
    }

    // Method to get the appropriate callback parameter type
    pub fn callback_param_type(
        arg_type: &Type,
        arg_name: &str,
        ci: &ComponentInterface,
    ) -> dart::Tokens {
        let type_label = DartCodeOracle::native_dart_type_label(Some(arg_type), ci);
        quote!($type_label $arg_name)
    }

    // Method to generate code for handling callback return values
    pub fn callback_return_handling(
        ret_type: &Type,
        method_name: &str,
        args: Vec<dart::Tokens>,
        ci: &ComponentInterface,
    ) -> dart::Tokens {
        match ret_type {
            Type::Boolean => {
                // For boolean return values
                quote!(
                    final result = obj.$method_name($(for arg in &args => $arg,));
                    outReturn.value = result ? 1 : 0;
                )
            }
            Type::UInt8
            | Type::UInt16
            | Type::UInt32
            | Type::UInt64
            | Type::Int8
            | Type::Int16
            | Type::Int32
            | Type::Int64 => {
                // For primitive integer return values
                let lowered = ret_type.as_codetype(ci).ffi_converter_name();
                quote!(
                    final result = obj.$method_name($(for arg in &args => $arg,));
                    outReturn.value = $lowered.lower(result);
                )
            }
            Type::Float32 | Type::Float64 => {
                // For float return values
                quote!(
                    final result = obj.$method_name($(for arg in &args => $arg,));
                    outReturn.value = result;
                )
            }
            Type::Optional { .. } => {
                // For optional return values, use the Optional FfiConverter which already
                // handles the discriminant byte encoding correctly.
                let ffi_converter = ret_type.as_codetype(ci).ffi_converter_name();
                quote!(
                    final result = obj.$method_name($(for arg in &args => $arg,));
                    if (result == null) {
                        outReturn.ref = toRustBuffer(Uint8List.fromList([0]));
                    } else {
                        final lowered = $ffi_converter.lower(result);
                        outReturn.ref = toRustBuffer(lowered.asUint8List());
                    }
                )
            }
            Type::String => {
                // For string return values
                quote!(
                    final result = obj.$method_name($(for arg in &args => $arg,));
                    outReturn.ref = FfiConverterString.lower(result);
                    status.code = CALL_SUCCESS;
                )
            }
            Type::Object { .. } => {
                let lowered = ret_type.as_codetype(ci).ffi_converter_name();
                quote!(
                    final result = obj.$method_name($(for arg in &args => $arg,));
                    outReturn.value = $lowered.lower(result);
                )
            }
            Type::Sequence { inner_type } => {
                if let Type::Int32 = **inner_type {
                    // For int32 sequence return values
                    quote!(
                        final result = obj.$method_name($(for arg in &args => $arg,));
                        outReturn.ref = FfiConverterSequenceInt32.lower(result);
                    )
                } else {
                    // For other sequence types
                    let lowered = ret_type.as_codetype(ci).ffi_converter_name();
                    quote!(
                        final result = obj.$method_name($(for arg in &args => $arg,));
                        outReturn.ref = $lowered.lower(result);
                    )
                }
            }
            _ => {
                // For other return types
                let lowered = ret_type.as_codetype(ci).ffi_converter_name();
                quote!(
                    final result = obj.$method_name($(for arg in &args => $arg,));
                    outReturn.ref = $lowered.lower(result);
                )
            }
        }
    }

    // Method to get the appropriate return type for callback functions
    pub fn callback_out_return_type(ret_type: Option<&Type>) -> dart::Tokens {
        if let Some(ret) = ret_type {
            match ret {
                Type::UInt8 => quote!(Pointer<Uint8>),
                Type::UInt16 => quote!(Pointer<Uint16>),
                Type::UInt32 => quote!(Pointer<Uint32>),
                Type::UInt64 => quote!(Pointer<Uint64>),
                Type::Int8 => quote!(Pointer<Int8>),
                Type::Int16 => quote!(Pointer<Int16>),
                Type::Int32 => quote!(Pointer<Int32>),
                Type::Int64 => quote!(Pointer<Int64>),
                Type::Float32 => quote!(Pointer<Float>),
                Type::Float64 => quote!(Pointer<Double>),
                Type::Boolean => quote!(Pointer<Int8>),
                Type::Object { .. } => quote!(Pointer<Pointer<Void>>),
                _ => quote!(Pointer<RustBuffer>),
            }
        } else {
            quote!(Pointer<Void>)
        }
    }

    // Method to handle void return values in callbacks
    pub fn callback_void_handling(method_name: &str, args: Vec<dart::Tokens>) -> dart::Tokens {
        quote!(
            obj.$method_name($(for arg in &args => $arg,));
            status.code = CALL_SUCCESS;
        )
    }

    // Method to get the appropriate lift expression for callback arguments with indexed variable names
    pub fn callback_arg_lift_indexed(
        arg_type: &Type,
        arg_name: &str,
        arg_idx: usize,
        ci: &ComponentInterface,
    ) -> dart::Tokens {
        // Use index-based variable names to avoid conflicts
        if let Type::Boolean = arg_type {
            quote!(final bool_arg$(arg_idx) = $arg_name == 1;)
        } else if let Type::Enum { .. } = arg_type {
            let converter = arg_type.as_codetype(ci).ffi_converter_name();
            quote!(final arg$(arg_idx) = $converter.read(createUint8ListFromInt($arg_name)).value;)
        } else if let Type::Bytes = arg_type {
            quote!(final arg$(arg_idx) = FfiConverterUint8List.lift($arg_name);)
        } else if let Type::String = arg_type {
            quote!(final arg$(arg_idx) = FfiConverterString.lift($arg_name);)
        } else if let Type::Optional { inner_type } = arg_type {
            if let Type::String = **inner_type {
                quote!(final arg$(arg_idx) = FfiConverterOptionalString.lift($arg_name);)
            } else {
                let converter = arg_type.as_codetype(ci).ffi_converter_name();
                quote!(final arg$(arg_idx) = $converter.lift($arg_name);)
            }
        } else if let Type::Sequence { inner_type } = arg_type {
            if let Type::Int32 = **inner_type {
                quote!(final arg$(arg_idx) = FfiConverterSequenceInt32.lift($arg_name);)
            } else {
                let converter = arg_type.as_codetype(ci).ffi_converter_name();
                quote!(final arg$(arg_idx) = $converter.lift($arg_name);)
            }
        } else {
            let converter = arg_type.as_codetype(ci).ffi_converter_name();
            quote!(final arg$(arg_idx) = $converter.lift($arg_name);)
        }
    }

    // Method to get argument name for a callback method based on type and index
    pub fn callback_arg_name(arg_type: &Type, arg_idx: usize) -> dart::Tokens {
        if let Type::Boolean = arg_type {
            quote!(bool_arg$(arg_idx))
        } else {
            quote!(arg$(arg_idx))
        }
    }

    /// Lower argument with special handling for callback traits
    pub fn lower_arg_with_callback_handling(arg: &Argument, ci: &ComponentInterface) -> dart::Tokens {
        // Borrowed `&[u8]` / `[ByRef] bytes` arguments take the `ForeignBytes`
        // FFI path, not the owned `RustBuffer` path. uniffi 0.32 selects
        // `FfiType::ForeignBytes` for these (see `Argument::is_borrowed_bytes`),
        // so the FFI signature already expects `ForeignBytes`; lower the
        // `Uint8List` into one instead of a `RustBuffer`. "Borrowed" is the
        // Rust view (it borrows for the call rather than owning a RustBuffer);
        // the emitted `lowerForeignBytes` still copies into native memory since
        // a GC-managed `Uint8List` has no stable address. Mirrors the ByRef-bytes
        // converter in uniffi's Kotlin/Python backends.
        if arg.is_borrowed_bytes() {
            return quote!(lowerForeignBytes($(Self::var_name(arg.name()))));
        }
        let base_lower = Self::type_lower_fn(&arg.as_type(), quote!($(Self::var_name(arg.name()))), ci);
        match arg.as_type() {
            Type::Object {
                imp: ObjectImpl::Trait(TraitKind::Both | TraitKind::ForeignOnly),
                ..
            } => base_lower,
            Type::CallbackInterface { .. } => quote!($base_lower.address),
            _ => base_lower,
        }
    }

    pub fn object_interface_name(_ci: &ComponentInterface, obj: &Object) -> String {
        let class_name = Self::class_name(obj.name());
        if obj.has_callback_interface() || obj.is_trait_interface() {
            class_name
        } else {
            format!("{class_name}Interface")
        }
    }

    pub fn trait_interface_name(ci: &ComponentInterface, name: &str) -> String {
        if let Some(obj) = ci.get_object_definition(name) {
            let class_name = Self::class_name(obj.name());
            if obj.has_callback_interface() || obj.is_trait_interface() {
                class_name
            } else {
                Self::object_interface_name(ci, obj)
            }
        } else if let Some(callback) = ci.get_callback_interface_definition(name) {
            Self::class_name(callback.name())
        } else {
            Self::class_name(name)
        }
    }
}

// https://dart.dev/guides/language/language-tour#keywords
pub static RESERVED_IDENTIFIERS: [&str; 63] = [
    "abstract",
    "as",
    "assert",
    "async",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "covariant",
    "default",
    "deferred",
    "do",
    "dynamic",
    "else",
    "enum",
    "export",
    "extends",
    "extension",
    "external",
    "factory",
    "false",
    "final",
    "finally",
    "for",
    "Function",
    "get",
    "hide",
    "if",
    "implements",
    "import",
    "in",
    "interface",
    "is",
    "late",
    "library",
    "mixin",
    "new",
    "null",
    "on",
    "operator",
    "part",
    "required",
    "rethrow",
    "return",
    "set",
    "show",
    "static",
    "super",
    "switch",
    "sync",
    "this",
    "throw",
    "true",
    "try",
    "typedef",
    "var",
    "void",
    "while",
    "with",
    "yield",
];

pub trait AsCodeType {
    /// The code type of this type as `ci`'s library sees it.
    fn as_codetype(&self, ci: &ComponentInterface) -> Box<dyn CodeType>;
}

impl<T: AsType> AsCodeType for T {
    fn as_codetype(&self, ci: &ComponentInterface) -> Box<dyn CodeType> {
        let namespace = DartCodeOracle::external_namespace(&self.as_type(), ci);
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
            Type::Duration => Box::new(primitives::DurationCodeType),
            Type::Bytes => Box::new(primitives::BytesCodeType),
            Type::Object { name, imp, .. } => {
                Box::new(objects::ObjectCodeType::new(name, imp, namespace))
            }
            Type::Optional { inner_type } => {
                Box::new(compounds::OptionalCodeType::new(self.as_type(), *inner_type, ci))
            }
            Type::Sequence { inner_type } => {
                Box::new(compounds::SequenceCodeType::new(self.as_type(), *inner_type, ci))
            }
            Type::Map { key_type, value_type, .. } => {
                Box::new(compounds::MapCodeType::new(self.as_type(), *key_type, *value_type, ci))
            }
            Type::Enum { name, .. } => Box::new(enums::EnumCodeType::new(name, namespace)),
            Type::Record { name, .. } => Box::new(records::RecordCodeType::new(name, namespace)),
            Type::CallbackInterface { name, .. } => Box::new(
                callback_interface::CallbackInterfaceCodeType::new(name, self.as_type(), namespace),
            ),
            Type::Custom { name, module_path, builtin } => {
                Box::new(custom::CustomCodeType::new(name, module_path, builtin, namespace))
            }
            _ => todo!("As Type for Type::{:?}", self.as_type()),
        }
    }
}
