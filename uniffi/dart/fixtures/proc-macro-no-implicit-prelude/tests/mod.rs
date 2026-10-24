use anyhow::Result;

#[test]
fn proc_macro_no_implicit_prelude() -> Result<()> {
    uniffi_dart::testing::run_test("proc-macro-no-implicit-prelude", "src/api.udl", None)
}
