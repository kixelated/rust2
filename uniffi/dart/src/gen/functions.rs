use genco::prelude::*;
use uniffi_bindgen::interface::{AsType, Function};

use super::defaults::render_argument_param;
use super::oracle::AsCodeType;
use super::render::TypeHelperRenderer;
use crate::gen::oracle::DartCodeOracle;
use crate::gen::render::AsRenderable;

pub fn generate_function(func: &Function, type_helper: &dyn TypeHelperRenderer) -> dart::Tokens {
    let args = if func.arguments().is_empty() {
        quote!()
    } else {
        quote!({$(for arg in &func.arguments() =>
            $(render_argument_param(
                arg,
                arg.as_renderable(type_helper.get_ci()).render_type(&arg.as_type(), type_helper),
                type_helper.get_ci()
            )),
        )})
    };

    let (ret, lifter) = if let Some(ret) = func.return_type() {
        (ret.as_renderable(type_helper.get_ci()).render_type(ret, type_helper), quote!($(ret.as_codetype(type_helper.get_ci()).lift())))
    } else {
        (quote!(void), quote!((_) {}))
    };

    // Check if function can throw errors
    let error_handler = if let Some(error_type) = func.throws_type() {
        let handler_name = DartCodeOracle::error_handler_name(error_type, type_helper.get_ci());
        quote!($(handler_name))
    } else {
        quote!(null)
    };

    // Use centralized callback-aware argument lowering
    if func.is_async() {
        // For async methods returning objects, we need to convert the int pointer to Pointer<Void>
        let async_lifter = if let Some(ret_type) = func.return_type() {
            match ret_type {
                uniffi_bindgen::interface::Type::Object { .. } => {
                    quote!((ptr) => $lifter(Pointer<Void>.fromAddress(ptr)))
                }
                _ => lifter.clone(),
            }
        } else {
            lifter.clone()
        };

        quote!(
            UniffiRustFuture<$ret> $(DartCodeOracle::fn_name(func.name()))($args) {
                return uniffiRustCallAsync(
                  () => $(func.ffi_func().name())(
                    $(for arg in &func.arguments() => $(DartCodeOracle::lower_arg_with_callback_handling(arg, type_helper.get_ci())),)
                  ),
                  $(DartCodeOracle::async_poll(func, type_helper.get_ci())),
                  $(DartCodeOracle::async_complete(func, type_helper.get_ci())),
                  $(DartCodeOracle::async_cancel(func, type_helper.get_ci())),
                  $(DartCodeOracle::async_free(func, type_helper.get_ci())),
                  $async_lifter,
                  $error_handler,
                );
            }
        )
    } else if ret == quote!(void) {
        quote!(
            $ret $(DartCodeOracle::fn_name(func.name()))($args) {
                return rustCall((status) {
                    $(func.ffi_func().name())(
                        $(for arg in &func.arguments() => $(DartCodeOracle::lower_arg_with_callback_handling(arg, type_helper.get_ci())),) status
                    );
                }, $error_handler);
            }
        )
    } else {
        quote!(
            $ret $(DartCodeOracle::fn_name(func.name()))($args) {
                return rustCallWithLifter(
                    (status) => $(func.ffi_func().name())(
                        $(for arg in &func.arguments() => $(DartCodeOracle::lower_arg_with_callback_handling(arg, type_helper.get_ci())),) status
                    ),
                    $lifter,
                    $error_handler
                );
            }
        )
    }
}
